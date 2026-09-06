//! Current, authorized hybrid retrieval over encrypted lexical and vector state.
use crate::{
    embedding_transport::{DIMENSIONS, MODEL_DIGEST},
    retrieval::{Page, Search},
    schema,
    writer::WriteError,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const MAX_SEMANTIC_SCOPE: i64 = 100_000;
const MAX_SNIPPET_BODY_BYTES: usize = 2_048;
const RRF_K: f64 = 60.0;

#[derive(Debug)]
struct Candidate {
    id: String,
    revision: u32,
    priority: u8,
    score: f64,
}

#[derive(Clone, Copy)]
struct ResponseMeta<'a> {
    page: &'a Page,
    retrieval_mode: &'static str,
    degraded_reason: Option<&'static str>,
    visible: i64,
    indexed: i64,
    lexical: i64,
    semantic: i64,
    fused: i64,
    total: i64,
    next_offset: Option<u32>,
}

fn check(deadline: Instant, stopped: &AtomicBool) -> Result<(), WriteError> {
    if stopped.load(Ordering::SeqCst) {
        Err(WriteError::Stopped)
    } else if Instant::now() >= deadline {
        Err(WriteError::OutcomeUnknown)
    } else {
        Ok(())
    }
}

fn degraded_reason(status: &str, has_vector: bool) -> Option<&'static str> {
    match (status, has_vector) {
        ("ready", true) => None,
        ("disabled", _) => Some("disabled"),
        ("model_unavailable", _) => Some("model_unavailable"),
        ("embedding_timeout", _) => Some("embedding_timeout"),
        ("embedding_busy", _) => Some("embedding_busy"),
        ("embedding_changed", _) => Some("embedding_changed"),
        // Never copy an arbitrary transport or provider string into a response.
        _ => Some("model_unavailable"),
    }
}

fn valid_unit_vector(vector: &[f32]) -> bool {
    if vector.len() != DIMENSIONS || vector.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let norm_squared = vector
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>();
    norm_squared.is_finite() && (0.999..=1.001).contains(&norm_squared)
}

fn decode_vector(bytes: &[u8]) -> Result<Vec<f32>, WriteError> {
    let (chunks, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() || chunks.len() != DIMENSIONS {
        return Err(WriteError::PersistenceRejected);
    }
    let vector = chunks
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    if !valid_unit_vector(&vector) {
        return Err(WriteError::PersistenceRejected);
    }
    Ok(vector)
}

fn lexical_candidates(
    db: &Connection,
    page: &Page,
    query: &str,
    literal: &str,
    take: i64,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<(i64, Vec<Candidate>), WriteError> {
    check(deadline, stopped)?;
    let total = db.query_row(
        "SELECT count(*) FROM record_fts JOIN visible_records v ON v.record_rowid=record_fts.rowid WHERE record_fts MATCH ?1 AND v.namespace=?2",
        params![literal, page.namespace],
        |row| row.get::<_, i64>(0),
    )?;
    check(deadline, stopped)?;
    let mut statement = db.prepare(
        "SELECT v.id,v.current_revision,CASE WHEN v.id=?3 COLLATE NOCASE THEN 2 WHEN EXISTS(SELECT 1 FROM revision_sources s WHERE s.namespace=v.namespace AND s.record_id=v.id AND s.revision=v.current_revision AND s.reference=?3) THEN 1 ELSE 0 END AS direct_priority FROM record_fts JOIN visible_records v ON v.record_rowid=record_fts.rowid WHERE record_fts MATCH ?1 AND v.namespace=?2 ORDER BY direct_priority DESC,v.id LIMIT ?4",
    )?;
    let mut rows = statement.query(params![literal, page.namespace, query, take])?;
    let mut candidates = Vec::with_capacity(usize::try_from(take.min(total)).unwrap_or(0));
    let mut rank = 0usize;
    while let Some(row) = rows.next()? {
        check(deadline, stopped)?;
        rank += 1;
        candidates.push(Candidate {
            id: row.get(0)?,
            revision: row.get(1)?,
            priority: row.get(2)?,
            score: 1.0 / (RRF_K + rank as f64),
        });
    }
    Ok((total, candidates))
}

fn semantic_candidates(
    db: &Connection,
    namespace: &str,
    query: &str,
    query_vector: &[f32],
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Vec<Candidate>, WriteError> {
    let mut statement = db.prepare(
        "SELECT e.record_id,e.revision,e.vector,CASE WHEN e.record_id=?3 COLLATE NOCASE THEN 2 WHEN EXISTS(SELECT 1 FROM revision_sources s WHERE s.namespace=e.namespace AND s.record_id=e.record_id AND s.revision=e.revision AND s.reference=?3) THEN 1 ELSE 0 END FROM current_embeddings e WHERE e.namespace=?1 AND e.model_digest=?2 ORDER BY e.record_id LIMIT 100001",
    )?;
    let mut rows = statement.query(params![namespace, MODEL_DIGEST, query])?;
    let mut scored = Vec::new();
    while let Some(row) = rows.next()? {
        check(deadline, stopped)?;
        if scored.len() >= MAX_SEMANTIC_SCOPE as usize {
            return Err(WriteError::PersistenceRejected);
        }
        let vector = decode_vector(&row.get::<_, Vec<u8>>(2)?)?;
        let score = query_vector
            .iter()
            .zip(vector)
            .map(|(left, right)| f64::from(*left) * f64::from(right))
            .sum::<f64>();
        if !score.is_finite() {
            return Err(WriteError::PersistenceRejected);
        }
        scored.push((
            row.get::<_, String>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u8>(3)?,
            score,
        ));
    }
    check(deadline, stopped)?;
    scored.sort_by(|left, right| {
        right
            .3
            .total_cmp(&left.3)
            .then_with(|| left.0.cmp(&right.0))
    });
    check(deadline, stopped)?;
    Ok(scored
        .into_iter()
        .enumerate()
        .map(|(index, (id, revision, priority, _))| Candidate {
            id,
            revision,
            priority,
            score: 1.0 / (RRF_K + (index + 1) as f64),
        })
        .collect())
}

fn fuse(
    lexical: Vec<Candidate>,
    semantic: Vec<Candidate>,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Vec<Candidate>, WriteError> {
    let mut fused = BTreeMap::<String, Candidate>::new();
    for candidate in lexical.into_iter().chain(semantic) {
        check(deadline, stopped)?;
        match fused.entry(candidate.id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let current = entry.get_mut();
                if current.revision != candidate.revision {
                    return Err(WriteError::PersistenceRejected);
                }
                current.priority = current.priority.max(candidate.priority);
                current.score += candidate.score;
            }
        }
    }
    let mut fused = fused.into_values().collect::<Vec<_>>();
    check(deadline, stopped)?;
    fused.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.id.cmp(&right.id))
    });
    check(deadline, stopped)?;
    Ok(fused)
}

fn still_current(
    db: &Connection,
    namespace: &str,
    id: &str,
    revision: u32,
) -> Result<bool, WriteError> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM visible_records WHERE namespace=?1 AND id=?2 AND current_revision=?3)",
        params![namespace, id, revision],
        |row| row.get(0),
    )?)
}

fn response_value(
    meta: ResponseMeta<'_>,
    records: &[Value],
    omitted: u32,
    estimate: usize,
) -> Value {
    json!({
        "context_mode": "current",
        "retrieval_mode": meta.retrieval_mode,
        "degraded_reason": meta.degraded_reason,
        "freshness": {"visible": meta.visible, "indexed": meta.indexed},
        "candidates": {"lexical": meta.lexical, "semantic": meta.semantic, "fused": meta.fused},
        "records": records,
        "total": meta.total,
        "next_offset": meta.next_offset,
        "omitted_for_budget": omitted,
        "byte_budget": meta.page.byte_budget,
        "token_budget": meta.page.token_budget,
        "estimated_tokens": estimate,
        "token_estimate": "one_per_serialized_utf8_byte_conservative_not_provider_billing"
    })
}

fn settled_response(
    meta: ResponseMeta<'_>,
    records: &[Value],
    omitted: u32,
) -> Result<(Value, usize), WriteError> {
    let mut estimate = 0usize;
    for _ in 0..8 {
        let response = response_value(meta, records, omitted, estimate);
        let actual = serde_json::to_vec(&response)
            .map_err(|_| WriteError::PersistenceRejected)?
            .len();
        if actual == estimate {
            return Ok((response, actual));
        }
        estimate = actual;
    }
    Err(WriteError::PersistenceRejected)
}

fn snippet(revision: &schema::Revision, body_end: usize) -> Value {
    json!({
        "namespace": revision.record.namespace,
        "id": revision.record.id,
        "revision": revision.revision,
        "state": revision.record.state,
        "sources": revision.record.sources,
        "tags": revision.record.tags,
        "body": &revision.record.body[..body_end],
        "truncated": body_end < revision.record.body.len()
    })
}

fn snippet_boundaries(body: &str) -> Vec<usize> {
    let limit = body.len().min(MAX_SNIPPET_BODY_BYTES);
    let mut boundaries = vec![0];
    boundaries.extend(
        body.char_indices()
            .map(|(index, value)| index + value.len_utf8())
            .take_while(|end| *end <= limit),
    );
    boundaries
}

fn fit_snippet(
    revision: &schema::Revision,
    packed: &[Value],
    meta: ResponseMeta<'_>,
    worst_omitted: u32,
    bound: usize,
) -> Result<Option<Value>, WriteError> {
    let boundaries = snippet_boundaries(&revision.record.body);
    let fits = |body_end: usize| -> Result<(bool, Value), WriteError> {
        let value = snippet(revision, body_end);
        let mut proposed = Vec::with_capacity(packed.len() + 1);
        proposed.extend_from_slice(packed);
        proposed.push(value.clone());
        let (_, size) = settled_response(meta, &proposed, worst_omitted)?;
        Ok((size <= bound, value))
    };

    let maximum = *boundaries.last().unwrap_or(&0);
    let (maximum_fits, maximum_value) = fits(maximum)?;
    if maximum_fits {
        return Ok(Some(maximum_value));
    }
    if !fits(0)?.0 {
        return Ok(None);
    }

    let mut low = 0usize;
    let mut high = boundaries.len() - 1;
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if fits(boundaries[middle])?.0 {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(Some(snippet(revision, boundaries[low])))
}

fn pack(
    db: &Connection,
    page_rows: &[(String, u32)],
    meta: ResponseMeta<'_>,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Value, WriteError> {
    let bound = meta.page.byte_budget.min(meta.page.token_budget) as usize;
    let worst_omitted = u32::try_from(page_rows.len()).map_err(|_| WriteError::InvalidRequest)?;
    let mut packed = Vec::<(String, u32, Value)>::new();
    let mut omitted = 0u32;

    for (id, revision) in page_rows {
        check(deadline, stopped)?;
        if !still_current(db, &meta.page.namespace, id, *revision)? {
            continue;
        }
        let record = schema::revision(db, &meta.page.namespace, id, Some(*revision))?
            .ok_or(WriteError::PersistenceRejected)?;
        if !still_current(db, &meta.page.namespace, id, *revision)? {
            continue;
        }
        let values = packed
            .iter()
            .map(|(_, _, value)| value.clone())
            .collect::<Vec<_>>();
        if let Some(value) = fit_snippet(&record, &values, meta, worst_omitted, bound)? {
            packed.push((id.clone(), *revision, value));
        } else {
            omitted = omitted.saturating_add(1);
        }
    }

    // Time-based visibility can change during CPU packing. Recheck every row at
    // the final return boundary; revisions and sources are never served stale.
    let mut records = Vec::with_capacity(packed.len());
    for (id, revision, value) in packed {
        check(deadline, stopped)?;
        if still_current(db, &meta.page.namespace, &id, revision)? {
            records.push(value);
        }
    }
    check(deadline, stopped)?;
    let (response, actual) = settled_response(meta, &records, omitted)?;
    if actual > bound {
        return Err(WriteError::InvalidRequest);
    }
    Ok(response)
}

/// Search one already-authorized namespace. The caller rechecks identity and
/// grants around this operation; this layer rechecks current visibility/revision
/// before every returned record.
pub(crate) fn search(
    db: &Connection,
    query: Search,
    vector: Option<&[f32]>,
    semantic_status: &str,
    deadline: Instant,
    stopped: &AtomicBool,
) -> Result<Value, WriteError> {
    query.page.validate()?;
    let literal = crate::retrieval::literal_query(&query.query)?;
    check(deadline, stopped)?;

    let visible = db.query_row(
        "SELECT count(*) FROM visible_records WHERE namespace=?1",
        [&query.page.namespace],
        |row| row.get::<_, i64>(0),
    )?;
    let indexed = db.query_row(
        "SELECT count(*) FROM current_embeddings WHERE namespace=?1 AND model_digest=?2",
        params![query.page.namespace, MODEL_DIGEST],
        |row| row.get::<_, i64>(0),
    )?;
    check(deadline, stopped)?;

    let requested_hybrid = semantic_status == "ready" && vector.is_some();
    if requested_hybrid && !valid_unit_vector(vector.expect("checked above")) {
        return Err(WriteError::InvalidRequest);
    }
    let scope_limited = requested_hybrid && visible > MAX_SEMANTIC_SCOPE;
    let hybrid = requested_hybrid && !scope_limited;
    let reason = if scope_limited {
        Some("scope_limit")
    } else {
        degraded_reason(semantic_status, vector.is_some())
    };
    let retrieval_mode = if hybrid { "hybrid" } else { "lexical_only" };

    let lexical_take = if hybrid {
        MAX_SEMANTIC_SCOPE
    } else {
        i64::from(query.page.offset) + i64::from(query.page.limit)
    };
    let (lexical_total, lexical) = lexical_candidates(
        db,
        &query.page,
        &query.query,
        &literal,
        lexical_take,
        deadline,
        stopped,
    )?;
    let semantic = if hybrid {
        semantic_candidates(
            db,
            &query.page.namespace,
            &query.query,
            vector.expect("hybrid requires a vector"),
            deadline,
            stopped,
        )?
    } else {
        Vec::new()
    };
    let semantic_count =
        i64::try_from(semantic.len()).map_err(|_| WriteError::PersistenceRejected)?;
    let ranked = fuse(lexical, semantic, deadline, stopped)?;
    let total = if hybrid {
        i64::try_from(ranked.len()).map_err(|_| WriteError::PersistenceRejected)?
    } else {
        lexical_total
    };
    let start = usize::try_from(query.page.offset).map_err(|_| WriteError::InvalidRequest)?;
    let page_rows = ranked
        .iter()
        .skip(start)
        .take(query.page.limit as usize)
        .map(|candidate| (candidate.id.clone(), candidate.revision))
        .collect::<Vec<_>>();
    let consumed = u32::try_from(page_rows.len()).map_err(|_| WriteError::InvalidRequest)?;
    let next = query.page.offset.saturating_add(consumed);
    let next_offset = (consumed > 0 && i64::from(next) < total).then_some(next);
    let meta = ResponseMeta {
        page: &query.page,
        retrieval_mode,
        degraded_reason: reason,
        visible,
        indexed,
        lexical: lexical_total,
        semantic: semantic_count,
        fused: total,
        total,
        next_offset,
    };
    pack(db, &page_rows, meta, deadline, stopped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        owner,
        schema::{Kind, RecordInput, SourceReference, State},
    };
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    const KEY: &[u8] = b"HOTR-16-synthetic-key-718634";
    const BODY_CANARY: &str = "HOTR16canary";

    fn run_dir() -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        let base = owner::safe_absolute(&root.join("work/hotr-tests")).unwrap();
        fs::create_dir_all(&base).unwrap();
        let base = base.canonicalize().unwrap();
        assert!(base.starts_with(&root));
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let run = base.join(format!("HOTR-16-{}-{stamp}", std::process::id()));
        fs::create_dir(&run).unwrap();
        write_new(
            &run.join("SYNTHETIC-ONLY"),
            b"HOTR-16; synthetic hybrid retrieval fixture\n",
        );
        run
    }

    fn write_new(path: &Path, bytes: &[u8]) {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }

    fn fixture() -> rusqlite::Connection {
        let run = run_dir();
        owner::create(&run.join("vault"), KEY).unwrap();
        let db = schema::open(&run.join("vault/vault.db"), KEY).unwrap();
        db.execute("UPDATE embedding_config SET port=47822", [])
            .unwrap();
        db
    }

    fn stop() -> AtomicBool {
        AtomicBool::new(false)
    }

    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(3)
    }

    fn page(namespace: &str, limit: u32, byte_budget: u32, token_budget: u32) -> Page {
        Page {
            namespace: namespace.into(),
            limit,
            offset: 0,
            byte_budget,
            token_budget,
        }
    }

    fn append(
        db: &mut Connection,
        namespace: &str,
        id: &str,
        revision: u32,
        body: &str,
        sources: Vec<SourceReference>,
    ) {
        let tx = db.transaction().unwrap();
        crate::writer::append_revision(
            &tx,
            "owner",
            &RecordInput {
                namespace: namespace.into(),
                id: id.into(),
                kind: Kind::Note,
                body: body.into(),
                state: State::Accepted,
                sources,
                tags: vec!["fixture".into()],
            },
            (revision > 1).then_some(revision - 1),
            revision,
            1_788_700_000_000 + i64::from(revision),
        )
        .unwrap();
        tx.commit().unwrap();
    }

    fn unit(component: usize) -> Vec<f32> {
        let mut vector = vec![0.0; DIMENSIONS];
        vector[component] = 1.0;
        vector
    }

    fn seed_vector(db: &Connection, namespace: &str, id: &str, revision: u32, vector: &[f32]) {
        let blob = vector
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        db.execute(
            "INSERT INTO embedding_index(namespace,record_id,revision,generation,model_digest,attempts,due_ms,vector,completed_at_ms) VALUES(?1,?2,?3,0,?4,1,0,?5,1788700000000)",
            params![namespace, id, revision, MODEL_DIGEST, blob],
        )
        .unwrap();
    }

    #[test]
    fn hybrid_ranking_is_current_scoped_direct_first_and_stably_tied() {
        let mut db = fixture();
        append(
            &mut db,
            "project/test",
            "needle",
            1,
            &format!("{BODY_CANARY} needle exact"),
            vec![],
        );
        append(
            &mut db,
            "project/test",
            "source-hit",
            1,
            &format!("{BODY_CANARY} needle source"),
            vec![SourceReference {
                reference: "needle".into(),
                label: "opaque".into(),
            }],
        );
        append(
            &mut db,
            "project/test",
            "lexical",
            1,
            &format!("{BODY_CANARY} needle lexical"),
            vec![],
        );
        append(
            &mut db,
            "project/test",
            "semantic-a",
            1,
            &format!("{BODY_CANARY} vector only a"),
            vec![],
        );
        append(
            &mut db,
            "project/test",
            "semantic-b",
            1,
            &format!("{BODY_CANARY} vector only b"),
            vec![],
        );
        append(
            &mut db,
            "project/test",
            "stale",
            1,
            &format!("{BODY_CANARY} old needle"),
            vec![],
        );
        seed_vector(&db, "project/test", "stale", 1, &unit(0));
        append(
            &mut db,
            "project/test",
            "stale",
            2,
            &format!("{BODY_CANARY} corrected current"),
            vec![],
        );
        append(
            &mut db,
            "project/secret",
            "secret",
            1,
            &format!("{BODY_CANARY} needle secret"),
            vec![],
        );
        seed_vector(&db, "project/secret", "secret", 1, &unit(0));
        for id in ["needle", "source-hit", "lexical"] {
            seed_vector(&db, "project/test", id, 1, &unit(1));
        }
        for id in ["semantic-a", "semantic-b"] {
            seed_vector(&db, "project/test", id, 1, &unit(0));
        }

        let result = search(
            &db,
            Search {
                page: page("project/test", 50, 65_536, 65_536),
                query: "needle".into(),
            },
            Some(&unit(0)),
            "ready",
            deadline(),
            &stop(),
        )
        .unwrap();
        let ids = result["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(ids[0], "needle");
        assert_eq!(ids[1], "source-hit");
        assert!(
            ids.iter().position(|id| *id == "semantic-a").unwrap()
                < ids.iter().position(|id| *id == "semantic-b").unwrap()
        );
        assert!(!ids.contains(&"stale"));
        assert!(!ids.contains(&"secret"));
        assert_eq!(result["retrieval_mode"], "hybrid");
        assert!(result["degraded_reason"].is_null());
        assert_eq!(result["freshness"], json!({"visible": 6, "indexed": 5}));
        assert_eq!(result["candidates"]["semantic"], 5);
        assert_eq!(result["records"][1]["sources"][0]["reference"], "needle");
    }

    #[test]
    fn lexical_degradation_and_invalid_query_vectors_fail_closed() {
        let mut db = fixture();
        append(
            &mut db,
            "project/test",
            "lexical",
            1,
            &format!("{BODY_CANARY} fallback phrase"),
            vec![],
        );
        let query = Search {
            page: page("project/test", 10, 4_096, 4_096),
            query: "fallback".into(),
        };
        let result = search(&db, query.clone(), None, "disabled", deadline(), &stop()).unwrap();
        assert_eq!(result["retrieval_mode"], "lexical_only");
        assert_eq!(result["degraded_reason"], "disabled");
        assert_eq!(result["candidates"]["semantic"], 0);
        assert_eq!(result["records"][0]["id"], "lexical");
        let empty = search(
            &db,
            Search {
                page: page("project/test", 10, 1_024, 512),
                query: "no-such-term".into(),
            },
            None,
            "model_unavailable",
            deadline(),
            &stop(),
        )
        .unwrap();
        let empty_bytes = serde_json::to_vec(&empty).unwrap();
        assert!(empty_bytes.len() <= 512);
        assert_eq!(empty["estimated_tokens"], empty_bytes.len());
        assert_eq!(
            search(&db, query, Some(&[1.0]), "ready", deadline(), &stop(),),
            Err(WriteError::InvalidRequest)
        );
    }

    #[test]
    fn unicode_snippets_fit_the_whole_envelope_without_truncating_sources() {
        let mut db = fixture();
        append(
            &mut db,
            "project/test",
            "a-provenance",
            1,
            &format!("{BODY_CANARY} budget source-heavy"),
            vec![SourceReference {
                reference: "s".repeat(1_800),
                label: "complete but too large".into(),
            }],
        );
        append(
            &mut db,
            "project/test",
            "b-unicode",
            1,
            &format!("{BODY_CANARY} budget {}", "界".repeat(19_000)),
            vec![SourceReference {
                reference: "opaque://fixture/unicode".into(),
                label: "complete".into(),
            }],
        );
        let result = search(
            &db,
            Search {
                page: page("project/test", 2, 1_024, 1_024),
                query: "budget".into(),
            },
            None,
            "model_unavailable",
            deadline(),
            &stop(),
        )
        .unwrap();
        let serialized = serde_json::to_vec(&result).unwrap();
        assert!(serialized.len() <= 1_024);
        assert_eq!(result["estimated_tokens"], serialized.len());
        assert_eq!(result["total"], 2);
        assert_eq!(result["omitted_for_budget"], 1);
        assert_eq!(result["records"].as_array().unwrap().len(), 1);
        let record = &result["records"][0];
        assert_eq!(record["id"], "b-unicode");
        assert_eq!(record["truncated"], true);
        assert!(!record["body"].as_str().unwrap().is_empty());
        assert!(
            record["body"]
                .as_str()
                .unwrap()
                .is_char_boundary(record["body"].as_str().unwrap().len())
        );
        assert_eq!(
            record["sources"][0]["reference"],
            "opaque://fixture/unicode"
        );
        assert!(result["next_offset"].is_null());
    }
}
