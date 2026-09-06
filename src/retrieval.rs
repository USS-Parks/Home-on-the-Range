//! Current, scoped FTS5 retrieval. This module is called only after authorization.
use crate::{schema, writer::WriteError};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

fn default_limit() -> u32 {
    10
}
fn default_bytes() -> u32 {
    65536
}
fn default_tokens() -> u32 {
    32768
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub namespace: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_bytes")]
    pub byte_budget: u32,
    #[serde(default = "default_tokens")]
    pub token_budget: u32,
}

impl Page {
    fn validate(&self) -> Result<(), WriteError> {
        if !schema::valid_identifier(&self.namespace, true)
            || !(1..=50).contains(&self.limit)
            || self.offset > 100_000
            || !(1024..=262144).contains(&self.byte_budget)
            || !(512..=262144).contains(&self.token_budget)
        {
            return Err(WriteError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub page: Page,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub page: Page,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Count {
    pub namespace: String,
}

pub(crate) fn visible(db: &Connection, namespace: &str, id: &str) -> Result<bool, WriteError> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM visible_records WHERE namespace=?1 AND id=?2)",
        params![namespace, id],
        |row| row.get(0),
    )?)
}

pub(crate) fn reindex(db: &Connection, record: &schema::RecordInput) -> Result<(), WriteError> {
    let rowid: i64 = db.query_row(
        "SELECT rowid FROM records WHERE namespace=?1 AND id=?2",
        params![record.namespace, record.id],
        |row| row.get(0),
    )?;
    db.execute("DELETE FROM record_fts WHERE rowid=?1", [rowid])?;
    let sources = record
        .sources
        .iter()
        .map(|s| s.reference.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    db.execute(
        "INSERT INTO record_fts(rowid,namespace,id,body,tags,sources) VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            rowid,
            record.namespace,
            record.id,
            record.body,
            record.tags.join(" "),
            sources
        ],
    )?;
    Ok(())
}

fn literal_query(query: &str) -> Result<String, WriteError> {
    if query.is_empty() || query.len() > 512 || query.contains('\0') {
        return Err(WriteError::InvalidRequest);
    }
    let words: Vec<_> = query.split_whitespace().collect();
    if words.is_empty() || words.len() > 32 {
        return Err(WriteError::InvalidRequest);
    }
    // FTS operators, quotes and column selectors are data. No raw query fragment
    // or identifier is interpolated into SQL; only these quoted phrases enter MATCH.
    Ok(words
        .into_iter()
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND "))
}

fn pack(
    db: &Connection,
    page: &Page,
    rows: Vec<(String, u32)>,
    total: i64,
) -> Result<Value, WriteError> {
    let mut records = Vec::new();
    let mut omitted = 0u32;
    let mut consumed = 0u32;
    // A deliberately conservative byte-based token estimate, independent of
    // provider tokenizer. Envelope bytes count too; this is not billed tokens.
    let bound = page.byte_budget.min(page.token_budget) as usize;
    let mut bytes = 512usize;
    for (id, revision) in rows {
        consumed += 1;
        let record = schema::revision(db, &page.namespace, &id, Some(revision))?
            .ok_or(WriteError::PersistenceRejected)?;
        let value = serde_json::to_value(record).map_err(|_| WriteError::PersistenceRejected)?;
        let size = serde_json::to_vec(&value)
            .map_err(|_| WriteError::PersistenceRejected)?
            .len()
            + 1;
        if bytes + size <= bound {
            bytes += size;
            records.push(value);
        } else {
            omitted += 1;
        }
    }
    let next = page.offset + consumed;
    let response = json!({"records":records,"total":total,"next_offset":if i64::from(next)<total && consumed>0 {Some(next)} else {None},"omitted_for_budget":omitted,"byte_budget":page.byte_budget,"token_budget":page.token_budget,"estimated_tokens":bytes,"token_estimate":"one per serialized UTF-8 byte including envelope reserve; not provider accounting","pagination":"offset over current authorized results; concurrent writes may change pages"});
    let actual = serde_json::to_vec(&response)
        .map_err(|_| WriteError::PersistenceRejected)?
        .len();
    if actual > bound {
        return Err(WriteError::InvalidRequest);
    }
    Ok(response)
}

pub(crate) fn search(db: &Connection, query: Search) -> Result<Value, WriteError> {
    query.page.validate()?;
    let literal = literal_query(&query.query)?;
    let total:i64=db.query_row("SELECT count(*) FROM record_fts JOIN visible_records v ON v.record_rowid=record_fts.rowid WHERE record_fts MATCH ?1 AND v.namespace=?2",params![literal,query.page.namespace],|row|row.get(0))?;
    // Only authorized visible candidates participate in ordering. Do not expose
    // global-corpus BM25 statistics: exact ID/source matches get fixed boosts,
    // followed by stable ID order. FTS supplies literal term intersection.
    let rows=db.prepare("SELECT v.id,v.current_revision FROM record_fts JOIN visible_records v ON v.record_rowid=record_fts.rowid WHERE record_fts MATCH ?1 AND v.namespace=?2 ORDER BY (v.id=?3 COLLATE NOCASE) DESC, EXISTS(SELECT 1 FROM revision_sources s WHERE s.namespace=v.namespace AND s.record_id=v.id AND s.revision=v.current_revision AND s.reference=?3) DESC, v.id LIMIT ?4 OFFSET ?5")?
        .query_map(params![literal,query.page.namespace,query.query,query.page.limit,query.page.offset],|row|Ok((row.get(0)?,row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    pack(db, &query.page, rows, total)
}

pub(crate) fn list(db: &Connection, page: Page) -> Result<Value, WriteError> {
    page.validate()?;
    let total: i64 = db.query_row(
        "SELECT count(*) FROM visible_records WHERE namespace=?1",
        [&page.namespace],
        |row| row.get(0),
    )?;
    let rows=db.prepare("SELECT id,current_revision FROM visible_records WHERE namespace=?1 ORDER BY id LIMIT ?2 OFFSET ?3")?
        .query_map(params![page.namespace,page.limit,page.offset],|row|Ok((row.get(0)?,row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    pack(db, &page, rows, total)
}

pub(crate) fn count(db: &Connection, query: Count) -> Result<Value, WriteError> {
    let count: i64 = db.query_row(
        "SELECT count(*) FROM visible_records WHERE namespace=?1",
        [query.namespace],
        |row| row.get(0),
    )?;
    Ok(json!({"count":count}))
}

pub(crate) fn history(db: &Connection, query: History) -> Result<Value, WriteError> {
    query.page.validate()?;
    if !schema::valid_identifier(&query.id, false) {
        return Err(WriteError::InvalidRequest);
    }
    let exists = db
        .query_row(
            "SELECT current_revision FROM records WHERE namespace=?1 AND id=?2",
            params![query.page.namespace, query.id],
            |row| row.get::<_, u32>(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(WriteError::NotFound);
    }
    let total: i64 = db.query_row(
        "SELECT count(*) FROM revisions WHERE namespace=?1 AND record_id=?2",
        params![query.page.namespace, query.id],
        |row| row.get(0),
    )?;
    let rows=db.prepare("SELECT record_id,revision FROM revisions WHERE namespace=?1 AND record_id=?2 ORDER BY revision LIMIT ?3 OFFSET ?4")?
        .query_map(params![query.page.namespace,query.id,query.page.limit,query.page.offset],|row|Ok((row.get(0)?,row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    pack(db, &query.page, rows, total)
}
