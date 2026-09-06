//! HOTR-17 frozen-corpus retrieval evaluation.
//!
//! The ignored integration test below is intentionally unusable until an
//! independent reviewer has approved a byte-exact corpus freeze.  Corpus text
//! is used only to seed a disposable encrypted vault and to issue the frozen
//! queries; reports contain identifiers and measurements, never query or body
//! text.

use super::local_embedding::{Ollama, configure_cli, status};
use super::*;
use hotr::lifecycle::{Action, Request};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::os::windows::fs::MetadataExt;

const CORPUS_SCHEMA: u32 = 1;
const CORPUS_SEED: u64 = 47_821;
const SHARED_NAMESPACE: &str = "evaluation/shared";
const PRIVATE_NAMESPACE: &str = "evaluation/private";
const BODY_PREFIX: &str = "HOTR07canary\n";
const MAX_CORPUS_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: u32,
    seed: u64,
    records: Vec<CorpusRecord>,
    queries: Vec<CorpusQuery>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusRecord {
    namespace: String,
    id: String,
    kind: Kind,
    tags: Vec<String>,
    sources: Vec<SourceReference>,
    revisions: Vec<CorpusRevision>,
    visibility: Visibility,
    superseded_by: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusRevision {
    body: String,
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Visibility {
    Current,
    Deleted,
    Expired,
    Future,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusQuery {
    id: String,
    split: Split,
    category: Category,
    namespace: String,
    query: String,
    expected_ids: Vec<String>,
    prohibited_ids: Vec<String>,
    rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Split {
    Development,
    HeldOut,
}

impl Split {
    fn label(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::HeldOut => "held_out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Category {
    ExactId,
    Paraphrase,
    Temporal,
    Conflict,
    NoAnswer,
    Access,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Self::ExactId => "exact_id",
            Self::Paraphrase => "paraphrase",
            Self::Temporal => "temporal",
            Self::Conflict => "conflict",
            Self::NoAnswer => "no_answer",
            Self::Access => "access",
        }
    }
}

#[derive(Debug, Clone)]
struct ExpectedRecord {
    namespace: String,
    revision: u32,
    body: String,
    sources: Vec<SourceReference>,
    visible: bool,
}

#[derive(Debug)]
struct FrozenCorpus {
    corpus: Corpus,
    corpus_sha256: String,
    reviewer: String,
    source_labels_sha256: String,
    record_summary_sha256: String,
}

fn fail(code: &str) -> ! {
    panic!("{code}")
}

fn sha256(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

fn checked_project_file(relative: &str, max_bytes: u64, code: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if !root.is_absolute() {
        fail("HOTR17_PROJECT_PATH_NOT_ABSOLUTE");
    }
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| fail("HOTR17_PROJECT_PATH_UNRESOLVED"));
    let requested = root.join(relative);
    for ancestor in requested
        .ancestors()
        .take_while(|path| path.starts_with(&root))
    {
        if let Ok(metadata) = fs::symlink_metadata(ancestor)
            && metadata.file_attributes() & 0x400 != 0
        {
            fail(code);
        }
    }
    let canonical = requested.canonicalize().unwrap_or_else(|_| fail(code));
    if !canonical.starts_with(&root) || !canonical.is_file() {
        fail(code);
    }
    let metadata = fs::metadata(&canonical).unwrap_or_else(|_| fail(code));
    if metadata.len() > max_bytes {
        fail(code);
    }
    canonical
}

fn load_frozen_corpus() -> FrozenCorpus {
    let corpus_path = checked_project_file(
        "tests/fixtures/hotr17/corpus.json",
        MAX_CORPUS_BYTES,
        "HOTR17_CORPUS_PATH_REJECTED",
    );
    let freeze_path = checked_project_file(
        "tests/fixtures/hotr17/freeze.json",
        64 * 1024,
        "HOTR17_FREEZE_PATH_REJECTED",
    );
    let corpus_bytes = fs::read(corpus_path).unwrap_or_else(|_| fail("HOTR17_CORPUS_READ"));
    let corpus_text =
        std::str::from_utf8(&corpus_bytes).unwrap_or_else(|_| fail("HOTR17_CORPUS_UTF8"));
    let normalized = corpus_text.replace("\r\n", "\n");
    let corpus_sha256 = sha256(normalized.as_bytes());

    let freeze: Value = serde_json::from_slice(
        &fs::read(freeze_path).unwrap_or_else(|_| fail("HOTR17_FREEZE_READ")),
    )
    .unwrap_or_else(|_| fail("HOTR17_FREEZE_JSON"));
    if freeze.get("schema").and_then(Value::as_u64) != Some(1)
        || freeze.get("review_status").and_then(Value::as_str) != Some("approved")
        || freeze
            .get("frozen_before_evaluation")
            .and_then(Value::as_bool)
            != Some(true)
    {
        fail("HOTR17_FREEZE_NOT_APPROVED");
    }
    let frozen_hash = freeze
        .get("corpus_sha256")
        .and_then(Value::as_str)
        .unwrap_or_else(|| fail("HOTR17_FREEZE_HASH_MISSING"));
    if !frozen_hash.eq_ignore_ascii_case(&corpus_sha256) {
        fail("HOTR17_FREEZE_HASH_MISMATCH");
    }
    let reviewer = freeze
        .get("reviewer")
        .and_then(Value::as_str)
        .filter(|value| {
            !value.trim().is_empty()
                && value.len() <= 256
                && !value
                    .chars()
                    .any(|character| matches!(character, '\r' | '\n' | '\0'))
        })
        .unwrap_or_else(|| fail("HOTR17_FREEZE_REVIEWER_INVALID"))
        .to_owned();

    let corpus: Corpus =
        serde_json::from_str(&normalized).unwrap_or_else(|_| fail("HOTR17_CORPUS_SHAPE"));
    validate_corpus(&corpus);

    let mut labels = Vec::new();
    let mut summaries = Vec::new();
    for record in &corpus.records {
        for source in &record.sources {
            labels.push(source.label.as_str());
        }
        summaries.push(format!(
            "{}\0{}\0{:?}\0{:?}\0{}\0{}",
            record.namespace,
            record.id,
            record.kind,
            record.visibility,
            record.revisions.len(),
            record.superseded_by.as_deref().unwrap_or("")
        ));
    }
    labels.sort_unstable();
    summaries.sort_unstable();
    let source_labels_sha256 = sha256(labels.join("\n"));
    FrozenCorpus {
        corpus,
        corpus_sha256,
        reviewer,
        source_labels_sha256,
        record_summary_sha256: sha256(summaries.join("\n")),
    }
}

fn validate_corpus(corpus: &Corpus) {
    if corpus.schema != CORPUS_SCHEMA || corpus.seed != CORPUS_SEED {
        fail("HOTR17_CORPUS_VERSION_OR_SEED");
    }
    if !(100..=160).contains(&corpus.records.len()) || corpus.queries.len() != 144 {
        fail("HOTR17_CORPUS_COUNT");
    }
    let namespaces = [SHARED_NAMESPACE, PRIVATE_NAMESPACE];
    let mut records = HashMap::new();
    let mut references = HashSet::new();
    for record in &corpus.records {
        if !namespaces.contains(&record.namespace.as_str())
            || !hotr::schema::valid_identifier(&record.id, false)
            || !record.id.is_ascii()
            || records.insert(record.id.as_str(), record).is_some()
            || record.revisions.is_empty()
            || record.sources.is_empty()
        {
            fail("HOTR17_CORPUS_RECORD_INVALID");
        }
        for revision in &record.revisions {
            if revision.state != State::Proposed
                || !(64..=4000).contains(&revision.body.len())
                || revision.body.contains('\0')
            {
                fail("HOTR17_CORPUS_REVISION_INVALID");
            }
        }
        let probe = RecordInput {
            namespace: record.namespace.clone(),
            id: record.id.clone(),
            kind: record.kind,
            body: format!("{BODY_PREFIX}{}", record.revisions.last().unwrap().body),
            state: State::Proposed,
            sources: record.sources.clone(),
            tags: record.tags.clone(),
        };
        if probe.validate().is_err() {
            fail("HOTR17_CORPUS_RECORD_BOUNDS");
        }
        for source in &record.sources {
            if !references.insert(source.reference.as_str()) {
                fail("HOTR17_CORPUS_REFERENCE_NOT_UNIQUE");
            }
        }
    }
    for record in &corpus.records {
        if let Some(replacement) = &record.superseded_by {
            let replacement = records
                .get(replacement.as_str())
                .unwrap_or_else(|| fail("HOTR17_CORPUS_SUPERSESSION_TARGET"));
            if record.visibility != Visibility::Current
                || replacement.visibility != Visibility::Current
                || replacement.namespace != record.namespace
                || replacement.id == record.id
            {
                fail("HOTR17_CORPUS_SUPERSESSION_INVALID");
            }
        }
    }

    let expected_counts = [
        ((Split::Development, Category::ExactId), 16),
        ((Split::Development, Category::Paraphrase), 48),
        ((Split::Development, Category::Temporal), 12),
        ((Split::Development, Category::Conflict), 8),
        ((Split::Development, Category::NoAnswer), 8),
        ((Split::Development, Category::Access), 4),
        ((Split::HeldOut, Category::ExactId), 8),
        ((Split::HeldOut, Category::Paraphrase), 24),
        ((Split::HeldOut, Category::Temporal), 6),
        ((Split::HeldOut, Category::Conflict), 4),
        ((Split::HeldOut, Category::NoAnswer), 4),
        ((Split::HeldOut, Category::Access), 2),
    ];
    let mut actual_counts = HashMap::new();
    let mut query_ids = HashSet::new();
    let mut query_texts = HashSet::new();
    let mut positive_partition = HashMap::new();
    for query in &corpus.queries {
        *actual_counts
            .entry((query.split, query.category))
            .or_insert(0usize) += 1;
        if !hotr::schema::valid_identifier(&query.id, false)
            || !query.id.is_ascii()
            || !query_ids.insert(query.id.as_str())
            || !query_texts.insert(query.query.as_str())
            || !namespaces.contains(&query.namespace.as_str())
            || query.query.trim().is_empty()
            || query.query.len() > 512
            || query.query.split_whitespace().count() > 32
            || query.query.contains('\0')
            || query.rationale.trim().is_empty()
            || query.expected_ids.iter().collect::<HashSet<_>>().len() != query.expected_ids.len()
            || query.prohibited_ids.iter().collect::<HashSet<_>>().len()
                != query.prohibited_ids.len()
            || query
                .expected_ids
                .iter()
                .any(|id| query.prohibited_ids.contains(id))
        {
            fail("HOTR17_CORPUS_QUERY_INVALID");
        }
        if matches!(query.category, Category::NoAnswer | Category::Access) {
            if !query.expected_ids.is_empty() {
                fail("HOTR17_CORPUS_NEGATIVE_EXPECTED_IDS");
            }
        } else if query.expected_ids.is_empty() || query.namespace != SHARED_NAMESPACE {
            fail("HOTR17_CORPUS_POSITIVE_EXPECTED_IDS");
        }
        if query.category == Category::Access && query.namespace != PRIVATE_NAMESPACE {
            fail("HOTR17_CORPUS_ACCESS_NAMESPACE");
        }
        if query.category == Category::ExactId
            && (query.expected_ids.len() != 1 || query.expected_ids.first() != Some(&query.query))
        {
            fail("HOTR17_CORPUS_EXACT_ID_CARDINALITY");
        }
        for id in &query.expected_ids {
            if positive_partition
                .insert(id, query.split)
                .is_some_and(|previous| previous != query.split)
            {
                fail("HOTR17_CORPUS_SPLIT_OVERLAP");
            }
            let record = records
                .get(id.as_str())
                .unwrap_or_else(|| fail("HOTR17_CORPUS_EXPECTED_UNKNOWN"));
            if record.namespace != query.namespace
                || record.visibility != Visibility::Current
                || record.superseded_by.is_some()
            {
                fail("HOTR17_CORPUS_EXPECTED_NOT_VISIBLE_CURRENT");
            }
        }
        for id in &query.prohibited_ids {
            if !records.contains_key(id.as_str()) || query.expected_ids.contains(id) {
                fail("HOTR17_CORPUS_PROHIBITED_UNKNOWN");
            }
        }
    }
    for (key, expected) in expected_counts {
        if actual_counts.get(&key).copied().unwrap_or(0) != expected {
            fail("HOTR17_CORPUS_QUERY_COUNTS");
        }
    }
}

fn record_input(record: &CorpusRecord, revision: &CorpusRevision) -> RecordInput {
    RecordInput {
        namespace: record.namespace.clone(),
        id: record.id.clone(),
        kind: record.kind,
        body: format!("{BODY_PREFIX}{}", revision.body),
        state: State::Proposed,
        sources: record.sources.clone(),
        tags: record.tags.clone(),
    }
}

fn lifecycle(run: &Path, request: &Request) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hotr"))
        .arg("lifecycle")
        .arg(run.join("vault"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| fail("HOTR17_LIFECYCLE_START"));
    child
        .stdin
        .take()
        .unwrap_or_else(|| fail("HOTR17_LIFECYCLE_STDIN"))
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap_or_else(|_| fail("HOTR17_LIFECYCLE_WRITE"));
    let output = child
        .wait_with_output()
        .unwrap_or_else(|_| fail("HOTR17_LIFECYCLE_WAIT"));
    if !output.status.success() {
        fail("HOTR17_LIFECYCLE_FAILED");
    }
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| fail("HOTR17_LIFECYCLE_REPLY"))
}

fn owner_action(run: &Path, idempotency_key: String, action: Action) -> Value {
    let reply = lifecycle(
        run,
        &Request {
            idempotency_key,
            action,
        },
    );
    if !reply.get("error").is_some_and(Value::is_null) {
        fail("HOTR17_OWNER_ACTION_REJECTED");
    }
    reply
}

async fn seed_corpus(
    run: &Path,
    server: &Server,
    corpus: &Corpus,
) -> (
    HashMap<String, ExpectedRecord>,
    Zeroizing<String>,
    Zeroizing<String>,
    Zeroizing<String>,
) {
    let (_, shared_writer) = issue_cli(
        run,
        "hotr17-shared-contributor",
        "contributor",
        SHARED_NAMESPACE,
    );
    let (_, private_writer) = issue_cli(
        run,
        "hotr17-private-contributor",
        "contributor",
        PRIVATE_NAMESPACE,
    );
    let (_, reader) = issue_cli(run, "hotr17-reader", "reader", SHARED_NAMESPACE);
    let client = local_client();
    let mut expected = HashMap::new();

    for (index, record) in corpus.records.iter().enumerate() {
        let token = if record.namespace == SHARED_NAMESPACE {
            &*shared_writer
        } else {
            &*private_writer
        };
        let request = WriteRequest {
            record: record_input(record, &record.revisions[0]),
            expected_revision: None,
            idempotency_key: format!("hotr17-seed-{index}-1"),
        };
        let response = post(&client, server.port, token, "/v1/records", &request).await;
        if response.0 != 200 || response.1["receipt"]["revision"] != 1 {
            fail("HOTR17_BASE_WRITE_FAILED");
        }
        let mut revision = 1u32;
        for (revision_index, body) in record.revisions.iter().enumerate().skip(1) {
            let reply = owner_action(
                run,
                format!("hotr17-correct-{index}-{}", revision_index + 1),
                Action::Correct {
                    record: record_input(record, body),
                    expected_revision: revision,
                },
            );
            revision += 1;
            if reply["data"]["receipt"]["revision"] != revision {
                fail("HOTR17_CORRECTION_REVISION");
            }
        }
        expected.insert(
            record.id.clone(),
            ExpectedRecord {
                namespace: record.namespace.clone(),
                revision,
                body: record_input(record, record.revisions.last().unwrap()).body,
                sources: record.sources.clone(),
                visible: record.visibility == Visibility::Current && record.superseded_by.is_none(),
            },
        );
    }

    let now_ms: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap();
    for (index, record) in corpus.records.iter().enumerate() {
        if record.visibility == Visibility::Current {
            continue;
        }
        let current = expected.get(&record.id).unwrap().revision;
        let (tombstoned, valid_from_ms, expires_at_ms) = match record.visibility {
            Visibility::Deleted => (true, None, None),
            Visibility::Expired => (false, None, Some(now_ms - 1000)),
            Visibility::Future => (false, Some(now_ms + 60 * 60 * 1000), None),
            Visibility::Current => unreachable!(),
        };
        let reply = owner_action(
            run,
            format!("hotr17-visibility-{index}"),
            Action::Visibility {
                namespace: record.namespace.clone(),
                id: record.id.clone(),
                expected_revision: current,
                tombstoned,
                valid_from_ms,
                expires_at_ms,
            },
        );
        let next = current + 1;
        if reply["data"]["receipt"]["revision"] != next {
            fail("HOTR17_VISIBILITY_REVISION");
        }
        expected.get_mut(&record.id).unwrap().revision = next;
    }

    for (index, record) in corpus.records.iter().enumerate() {
        let Some(replacement_id) = &record.superseded_by else {
            continue;
        };
        let old_revision = expected.get(&record.id).unwrap().revision;
        let replacement_revision = expected.get(replacement_id).unwrap().revision;
        let reply = owner_action(
            run,
            format!("hotr17-supersede-{index}"),
            Action::Supersede {
                namespace: record.namespace.clone(),
                old_id: record.id.clone(),
                old_revision,
                replacement_id: replacement_id.clone(),
                replacement_revision,
            },
        );
        if reply["data"]["receipt"]["old_revision"] != old_revision + 1
            || reply["data"]["receipt"]["replacement_revision"] != replacement_revision + 1
        {
            fail("HOTR17_SUPERSESSION_REVISION");
        }
        expected.get_mut(&record.id).unwrap().revision += 1;
        expected.get_mut(replacement_id).unwrap().revision += 1;
    }

    (expected, reader, shared_writer, private_writer)
}

async fn wait_for_index(run: &Path, expected: u64) -> Value {
    let started = Instant::now();
    loop {
        let value = status(run).await;
        if value["failed"].as_u64().unwrap_or(0) != 0 {
            fail("HOTR17_INDEX_FAILED_ROWS");
        }
        if value["indexed"] == expected {
            return value;
        }
        if started.elapsed() >= Duration::from_secs(240) {
            fail("HOTR17_INDEX_TIMEOUT");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[derive(Debug)]
struct HttpObservation {
    status: Option<u16>,
    value: Option<Value>,
    error_code: Option<String>,
    latency_us: u64,
    no_store: bool,
    cors_absent: bool,
}

async fn timed_post(
    client: &reqwest::Client,
    port: u16,
    token: &str,
    path: &str,
    body: &Value,
) -> HttpObservation {
    let started = Instant::now();
    let sent = client
        .post(format!("http://127.0.0.1:{port}{path}"))
        .bearer_auth(token)
        .json(body)
        .send()
        .await;
    let latency = || started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let response = match sent {
        Ok(response) => response,
        Err(error) => {
            return HttpObservation {
                status: None,
                value: None,
                error_code: Some(
                    if error.is_timeout() {
                        "transport_timeout"
                    } else if error.is_connect() {
                        "transport_connect"
                    } else {
                        "transport_error"
                    }
                    .into(),
                ),
                latency_us: latency(),
                no_store: false,
                cors_absent: false,
            };
        }
    };
    let status = response.status().as_u16();
    let no_store = response
        .headers()
        .get("cache-control")
        .is_some_and(|v| v == "no-store");
    let cors_absent = !response
        .headers()
        .contains_key("access-control-allow-origin");
    let bytes = match response.bytes().await {
        Ok(bytes) if bytes.len() <= api::MAX_RESPONSE => bytes,
        Ok(_) => {
            return HttpObservation {
                status: Some(status),
                value: None,
                error_code: Some("response_too_large".into()),
                latency_us: latency(),
                no_store,
                cors_absent,
            };
        }
        Err(_) => {
            return HttpObservation {
                status: Some(status),
                value: None,
                error_code: Some("response_body_error".into()),
                latency_us: latency(),
                no_store,
                cors_absent,
            };
        }
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => HttpObservation {
            status: Some(status),
            error_code: value.pointer("/error/code").and_then(Value::as_str).map(
                |code| match code {
                    "forbidden" | "unauthorized" | "invalid_request" | "not_found" | "busy"
                    | "timeout" => code.to_owned(),
                    _ => "other_service_error".to_owned(),
                },
            ),
            value: Some(value),
            latency_us: latency(),
            no_store,
            cors_absent,
        },
        Err(_) => HttpObservation {
            status: Some(status),
            value: None,
            error_code: Some("response_json_error".into()),
            latency_us: latency(),
            no_store,
            cors_absent,
        },
    }
}

fn contains_claim_field(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "answer" | "answered" | "no_answer" | "abstain" | "confidence" | "probability"
            ) || contains_claim_field(value)
        }),
        Value::Array(values) => values.iter().any(contains_claim_field),
        _ => false,
    }
}

fn observation_json(
    mode: &str,
    query: &CorpusQuery,
    observation: HttpObservation,
    expected: &HashMap<String, ExpectedRecord>,
) -> Value {
    let mut ids = Vec::new();
    let mut revisions = Vec::new();
    let mut scores = Vec::new();
    let mut violations = Vec::new();
    if let Some(value) = &observation.value {
        let serialized = serde_json::to_string(value).unwrap();
        if serialized.contains("PRIVATE-DETAIL-")
            || serialized.contains("RETIRED-FACT-")
            || query
                .prohibited_ids
                .iter()
                .any(|id| serialized.contains(id))
        {
            violations.push("prohibited_text".to_owned());
        }
    }
    let access_denied = query.category == Category::Access
        && query.namespace == PRIVATE_NAMESPACE
        && observation.status == Some(403);
    if !observation.no_store {
        violations.push("response_missing_no_store".to_owned());
    }
    if !observation.cors_absent {
        violations.push("response_has_cors".to_owned());
    }
    if observation.value.is_none() {
        violations.push("response_unreadable".to_owned());
    } else if !access_denied && observation.status != Some(200) {
        violations.push("response_status".to_owned());
    } else if access_denied {
        if observation.value.as_ref() != Some(&json!({"error":{"code":"forbidden"}})) {
            violations.push("access_error_shape".to_owned());
        }
    } else if let Some(value) = &observation.value {
        if contains_claim_field(value) {
            violations.push("answer_claim_field".to_owned());
        }
        if mode == "hybrid" && value["retrieval_mode"] != "hybrid" {
            violations.push("hybrid_not_fully_indexed".to_owned());
        }
        let records = value.get("records").and_then(Value::as_array);
        if records.is_none() {
            violations.push("candidate_list_missing".to_owned());
        }
        for record in records.into_iter().flatten() {
            let Some(id) = record.get("id").and_then(Value::as_str) else {
                violations.push("record_id_missing".to_owned());
                continue;
            };
            ids.push(id.to_owned());
            let revision = record.get("revision").and_then(Value::as_u64);
            revisions.push(revision);
            let score = record.get("rrf_score").and_then(Value::as_f64);
            scores.push(score);
            if mode == "hybrid" && !score.is_some_and(|score| score.is_finite() && score > 0.0) {
                violations.push(format!("invalid_rrf_score:{id}"));
            }
            let Some(known) = expected.get(id) else {
                violations.push(format!("unknown_record:{id}"));
                continue;
            };
            if record["namespace"] != known.namespace
                || !known.visible
                || known.namespace != SHARED_NAMESPACE
                || known.namespace != query.namespace
            {
                violations.push(format!("forbidden_record:{id}"));
            }
            if revision != Some(u64::from(known.revision)) {
                violations.push(format!("wrong_revision:{id}"));
            }
            if !record
                .get("body")
                .and_then(Value::as_str)
                .is_some_and(|body| {
                    !body.is_empty()
                        && known.body.starts_with(body)
                        && (body == known.body || record["truncated"] == true)
                })
            {
                violations.push(format!("wrong_text:{id}"));
            }
            if record.get("sources") != Some(&serde_json::to_value(&known.sources).unwrap()) {
                violations.push(format!("wrong_sources:{id}"));
            }
        }
        if ids.len() > 5 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            violations.push("invalid_candidate_count".to_owned());
        }
        for prohibited in &query.prohibited_ids {
            if ids.contains(prohibited) {
                violations.push(format!("prohibited_record:{prohibited}"));
            }
        }
    }
    let recall = if query.expected_ids.is_empty() {
        None
    } else {
        let hits = query
            .expected_ids
            .iter()
            .filter(|id| ids.contains(id))
            .count();
        Some(hits as f64 / query.expected_ids.len() as f64)
    };
    let hit = recall.map(|value| value > 0.0);
    let top1_exact =
        (query.category == Category::ExactId).then(|| ids.first() == query.expected_ids.first());
    json!({
        "mode":mode,
        "status":observation.status,
        "error_safe_code":observation.error_code,
        "latency_us":observation.latency_us,
        "ids":ids,
        "revisions":revisions,
        "scores":scores,
        "expected_ids":query.expected_ids,
        "recall_at_5":recall,
        "query_hit_at_5":hit,
        "top1_exact":top1_exact,
        "candidate_count":ids.len(),
        "contract_valid":violations.is_empty(),
        "violations":violations
    })
}

async fn exact_get_json(
    client: &reqwest::Client,
    port: u16,
    token: &str,
    query: &CorpusQuery,
    expected: &HashMap<String, ExpectedRecord>,
) -> Value {
    if query.category != Category::ExactId {
        return Value::Null;
    }
    let id = &query.expected_ids[0];
    let observation = timed_post(
        client,
        port,
        token,
        "/v1/records/get",
        &json!({"namespace":query.namespace,"id":id}),
    )
    .await;
    let mut violations = Vec::new();
    if observation.status != Some(200) || observation.value.is_none() {
        violations.push("exact_get_failed".to_owned());
    } else if let Some(value) = &observation.value {
        let known = expected.get(id).unwrap();
        if value["namespace"] != known.namespace
            || value["id"] != *id
            || value["revision"] != known.revision
            || value["body"] != known.body
            || value.get("sources") != Some(&serde_json::to_value(&known.sources).unwrap())
        {
            violations.push("exact_get_mismatch".to_owned());
        }
    }
    if !observation.no_store || !observation.cors_absent {
        violations.push("exact_get_headers".to_owned());
    }
    json!({
        "status":observation.status,
        "error_safe_code":observation.error_code,
        "latency_us":observation.latency_us,
        "id":id,
        "expected_revision":expected.get(id).unwrap().revision,
        "valid":violations.is_empty(),
        "violations":violations
    })
}

fn deterministic_order(corpus: &Corpus, split: Split) -> Vec<usize> {
    let mut indices: Vec<_> = corpus
        .queries
        .iter()
        .enumerate()
        .filter_map(|(index, query)| (query.split == split).then_some(index))
        .collect();
    let mut state = CORPUS_SEED
        ^ match split {
            Split::Development => 0xd3_71_10_9a,
            Split::HeldOut => 0x48_e1_d0_17,
        };
    for end in (1..indices.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        indices.swap(end, state as usize % (end + 1));
    }
    indices
}

async fn evaluate_split(
    split: Split,
    corpus: &Corpus,
    client: &reqwest::Client,
    port: u16,
    token: &str,
    expected: &HashMap<String, ExpectedRecord>,
) -> Vec<Value> {
    let mut results = Vec::new();
    let mut seen_queries = HashSet::new();
    for (position, index) in deterministic_order(corpus, split).into_iter().enumerate() {
        let query = &corpus.queries[index];
        let potential_repeat_query_key = !seen_queries.insert(query.query.clone());
        let request = json!({"page":page(&query.namespace,5,0),"query":query.query});
        let endpoints = if position % 2 == 0 {
            [("lexical", "/v1/search"), ("hybrid", "/v1/search/hybrid")]
        } else {
            [("hybrid", "/v1/search/hybrid"), ("lexical", "/v1/search")]
        };
        let mut modes = BTreeMap::new();
        for (mode, endpoint) in endpoints {
            let observation = timed_post(client, port, token, endpoint, &request).await;
            modes.insert(mode, observation_json(mode, query, observation, expected));
        }
        let exact_get = exact_get_json(client, port, token, query, expected).await;
        results.push(json!({
            "query_id":query.id,
            "split":query.split.label(),
            "category":query.category.label(),
            "namespace":query.namespace,
            "expected_ids":query.expected_ids,
            "prohibited_ids":query.prohibited_ids,
            "potential_repeat_query_key":potential_repeat_query_key,
            "lexical":modes.remove("lexical").unwrap(),
            "hybrid":modes.remove("hybrid").unwrap(),
            "exact_get":exact_get
        }));
    }
    results
}

#[derive(Debug, Default, Clone)]
struct QualityInput {
    exact_total: usize,
    exact_correct: usize,
    paraphrase_total: usize,
    lexical_paraphrase_recall_sum: f64,
    hybrid_paraphrase_recall_sum: f64,
    wrong_revision_hits: usize,
    forbidden_hits: usize,
    response_errors: usize,
}

fn mode_violations(mode: &Value, quality: &mut QualityInput) {
    for violation in mode["violations"].as_array().into_iter().flatten() {
        let value = violation.as_str().unwrap_or("");
        if value.starts_with("wrong_revision:") {
            quality.wrong_revision_hits += 1;
        }
        if value == "prohibited_text"
            || value.starts_with("unknown_record:")
            || value.starts_with("forbidden_record:")
            || value.starts_with("prohibited_record:")
            || value.starts_with("wrong_text:")
            || value.starts_with("wrong_sources:")
        {
            quality.forbidden_hits += 1;
        }
    }
    if mode["contract_valid"] != true {
        quality.response_errors += 1;
    }
}

fn summarize_split(results: &[Value]) -> (QualityInput, Value) {
    let mut quality = QualityInput::default();
    let mut expected_queries = 0usize;
    let mut lexical_recall_sum = 0.0;
    let mut hybrid_recall_sum = 0.0;
    let mut lexical_hits = 0usize;
    let mut hybrid_hits = 0usize;
    let mut lexical_latencies = Vec::new();
    let mut hybrid_latencies = Vec::new();
    let mut no_answer_lexical_returned = 0usize;
    let mut no_answer_hybrid_returned = 0usize;
    let mut no_answer_total = 0usize;
    let mut wins = Vec::new();
    let mut regressions = Vec::new();
    let mut ties = Vec::new();
    let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
    let mut categories: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
    for result in results {
        let lexical = &result["lexical"];
        let hybrid = &result["hybrid"];
        mode_violations(lexical, &mut quality);
        mode_violations(hybrid, &mut quality);
        for (mode, value, latencies) in [
            ("lexical", lexical, &mut lexical_latencies),
            ("hybrid", hybrid, &mut hybrid_latencies),
        ] {
            latencies.push(value["latency_us"].as_u64().unwrap_or(0));
            let status = value["status"]
                .as_u64()
                .map(|status| status.to_string())
                .unwrap_or_else(|| "transport".into());
            *statuses.entry(format!("{mode}:{status}")).or_insert(0) += 1;
        }
        let category = result["category"].as_str().unwrap();
        if !matches!(category, "no_answer" | "access") {
            expected_queries += 1;
            let category_totals = categories.entry(category.to_owned()).or_default();
            category_totals.0 += 1;
            category_totals.1 += lexical["recall_at_5"].as_f64().unwrap_or(0.0);
            category_totals.2 += hybrid["recall_at_5"].as_f64().unwrap_or(0.0);
            let lexical_recall = lexical["recall_at_5"].as_f64().unwrap_or(0.0);
            let hybrid_recall = hybrid["recall_at_5"].as_f64().unwrap_or(0.0);
            lexical_recall_sum += lexical_recall;
            hybrid_recall_sum += hybrid_recall;
            lexical_hits += usize::from(lexical_recall > 0.0);
            hybrid_hits += usize::from(hybrid_recall > 0.0);
        }
        if category == "exact_id" {
            quality.exact_total += 1;
            let get_valid = result["exact_get"]["valid"] == true;
            if !get_valid {
                quality.response_errors += 1;
            }
            quality.exact_correct += usize::from(
                lexical["top1_exact"] == true && hybrid["top1_exact"] == true && get_valid,
            );
        }
        if category == "paraphrase" {
            quality.paraphrase_total += 1;
            let lexical_recall = lexical["recall_at_5"].as_f64().unwrap_or(0.0);
            let hybrid_recall = hybrid["recall_at_5"].as_f64().unwrap_or(0.0);
            quality.lexical_paraphrase_recall_sum += lexical_recall;
            quality.hybrid_paraphrase_recall_sum += hybrid_recall;
            let id = result["query_id"].clone();
            if hybrid_recall > lexical_recall {
                wins.push(id);
            } else if hybrid_recall < lexical_recall {
                regressions.push(id);
            } else {
                ties.push(id);
            }
        }
        if category == "no_answer" {
            no_answer_total += 1;
            no_answer_lexical_returned +=
                usize::from(lexical["candidate_count"].as_u64().unwrap_or(0) > 0);
            no_answer_hybrid_returned +=
                usize::from(hybrid["candidate_count"].as_u64().unwrap_or(0) > 0);
        }
    }
    let latency = |mut samples: Vec<u64>| {
        samples.sort_unstable();
        let count = samples.len();
        let percentile = |p: usize| {
            if count == 0 {
                0
            } else {
                samples[((count - 1) * p).div_ceil(100)]
            }
        };
        json!({
            "count":count,
            "mean_us":if count == 0 {0.0} else {samples.iter().map(|v|*v as f64).sum::<f64>()/count as f64},
            "p50_us":percentile(50),
            "p95_us":percentile(95)
        })
    };
    let denominator = |sum: f64, count: usize| if count == 0 { 0.0 } else { sum / count as f64 };
    let category_summary: BTreeMap<_, _> = categories.into_iter().map(|(name, (count, lexical, hybrid))|
        (name, json!({"query_count":count,"lexical_recall_at_5":denominator(lexical,count),"hybrid_recall_at_5":denominator(hybrid,count)}))).collect();
    let summary = json!({
        "query_count":results.len(),
        "categories":category_summary,
        "expected_query_count":expected_queries,
        "mean_recall_at_5":{
            "lexical":denominator(lexical_recall_sum,expected_queries),
            "hybrid":denominator(hybrid_recall_sum,expected_queries)
        },
        "query_hit_at_5":{
            "lexical":denominator(lexical_hits as f64,expected_queries),
            "hybrid":denominator(hybrid_hits as f64,expected_queries)
        },
        "paraphrase_recall_at_5":{
            "lexical":denominator(quality.lexical_paraphrase_recall_sum,quality.paraphrase_total),
            "hybrid":denominator(quality.hybrid_paraphrase_recall_sum,quality.paraphrase_total)
        },
        "paraphrase_comparison":{"wins":wins,"regressions":regressions,"ties":ties},
        "exact":{"correct":quality.exact_correct,"total":quality.exact_total},
        "wrong_revision_hits":quality.wrong_revision_hits,
        "forbidden_or_integrity_hits":quality.forbidden_hits,
        "response_errors":quality.response_errors,
        "statuses":statuses,
        "latency":{"lexical":latency(lexical_latencies),"hybrid":latency(hybrid_latencies)},
        "no_answer":{
            "candidate_return_rate":{
                "lexical":denominator(no_answer_lexical_returned as f64,no_answer_total),
                "hybrid":denominator(no_answer_hybrid_returned as f64,no_answer_total)
            },
            "false_positive_rate":Value::Null,
            "false_positive_rate_applicable":false,
            "threshold_pass":Value::Null
        }
    });
    (quality, summary)
}

fn quality_failures(input: &QualityInput) -> Vec<&'static str> {
    let mut failures = Vec::new();
    if input.exact_total == 0 || input.exact_correct != input.exact_total {
        failures.push("exact_id_not_100_percent");
    }
    if input.wrong_revision_hits != 0 {
        failures.push("wrong_revision_hit");
    }
    if input.forbidden_hits != 0 {
        failures.push("forbidden_or_integrity_hit");
    }
    if input.response_errors != 0 {
        failures.push("response_error");
    }
    let lexical = input.lexical_paraphrase_recall_sum / input.paraphrase_total.max(1) as f64;
    let hybrid = input.hybrid_paraphrase_recall_sum / input.paraphrase_total.max(1) as f64;
    if input.paraphrase_total == 0 || hybrid + 1.0e-12 < 0.90 {
        failures.push("hybrid_paraphrase_recall_below_90_percent");
    }
    if (lexical + 1.0e-12 < 0.90 && hybrid - lexical + 1.0e-12 < 0.10)
        || (lexical + 1.0e-12 >= 0.90 && hybrid + 1.0e-12 < lexical)
    {
        failures.push("hybrid_paraphrase_gain_or_baseline_exception_failed");
    }
    failures
}

fn readonly_audit(path: &Path) -> rusqlite::Connection {
    unsafe extern "C" {
        fn sqlite3_key(
            db: *mut rusqlite::ffi::sqlite3,
            key: *const std::ffi::c_void,
            length: i32,
        ) -> i32;
    }
    let db = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap_or_else(|_| fail("HOTR17_DB_AUDIT_OPEN"));
    // SAFETY: live exclusively owned connection; SQLCipher copies bounded static fixture key bytes.
    let rc = unsafe { sqlite3_key(db.handle(), KEY.as_ptr().cast(), KEY.len() as i32) };
    if rc != rusqlite::ffi::SQLITE_OK {
        fail("HOTR17_DB_AUDIT_KEY");
    }
    db
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires independently frozen HOTR-17 corpus and pinned local model; use the bounded HOTR-17 gate"]
async fn hotr17_pinned_model_retrieval_evaluation() {
    // This is deliberately the first operation.  No run directory, service,
    // model process, or query exists before the independent freeze is proven.
    let frozen = load_frozen_corpus();
    let run = run_dir();
    let model_started = Instant::now();
    let ollama = Ollama::start(&run).await;
    let model_start_ms = model_started.elapsed().as_millis();
    let owned_ollama_pid = ollama.child.id();
    let cold_started = Instant::now();
    let cold = hotr::embedding_transport::embed(
        ollama.port,
        "synthetic HOTR-17 cold embedding transport probe",
        true,
    )
    .await
    .unwrap_or_else(|_| fail("HOTR17_COLD_EMBEDDING_FAILED"));
    let cold_embedding_ms = cold_started.elapsed().as_millis();

    owner::create(&run.join("vault"), KEY).unwrap_or_else(|_| fail("HOTR17_VAULT_CREATE"));
    let mut server = Server::start(&run, "hotr17-evaluation");
    unlock(&run).await;
    let (expected, reader, shared_writer, private_writer) =
        seed_corpus(&run, &server, &frozen.corpus).await;
    let expected_visible = expected.values().filter(|record| record.visible).count() as u64;
    let index_started = Instant::now();
    let configured = configure_cli(&run, Some(ollama.port), 0);
    if configured["generation"] != 1 {
        fail("HOTR17_INDEX_CONFIGURATION");
    }
    let indexed = wait_for_index(&run, expected_visible).await;
    let index_duration_ms = index_started.elapsed().as_millis();
    if indexed["failed"] != 0 || indexed["indexed"] != expected_visible {
        fail("HOTR17_INDEX_INCOMPLETE");
    }

    let client = local_client();
    let development = evaluate_split(
        Split::Development,
        &frozen.corpus,
        &client,
        server.port,
        &reader,
        &expected,
    )
    .await;
    let (development_quality, development_summary) = summarize_split(&development);
    let development_failures = quality_failures(&development_quality);
    let held_out = if development_failures.is_empty() {
        evaluate_split(
            Split::HeldOut,
            &frozen.corpus,
            &client,
            server.port,
            &reader,
            &expected,
        )
        .await
    } else {
        Vec::new()
    };
    let (held_quality, held_summary) = summarize_split(&held_out);
    let held_failures = if held_out.is_empty() {
        Vec::new()
    } else {
        quality_failures(&held_quality)
    };
    let mut quality_pass =
        development_failures.is_empty() && held_failures.is_empty() && !held_out.is_empty();

    server.stop(&run).await;
    let vault_path = run.join("vault/vault.db");
    let vault_file_bytes = fs::metadata(&vault_path).unwrap().len();
    let db = readonly_audit(&vault_path);
    let (index_rows, indexed_vector_bytes): (i64, i64) = db
        .query_row(
            "SELECT count(*),coalesce(sum(length(vector)),0) FROM current_embeddings",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or_else(|_| fail("HOTR17_DB_INDEX_AUDIT"));
    let integrity_check: String = db
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap_or_else(|_| fail("HOTR17_DB_INTEGRITY_AUDIT"));
    drop(db);
    let audit_valid = integrity_check == "ok"
        && index_rows == expected_visible as i64
        && indexed_vector_bytes
            == (expected_visible * hotr::embedding_transport::DIMENSIONS as u64 * 4) as i64;
    quality_pass &= audit_valid;

    let unload_status = client
        .post(format!("http://127.0.0.1:{}/api/embed", ollama.port))
        .json(&json!({
            "model":hotr::embedding_transport::MODEL,
            "input":[],
            "keep_alive":0
        }))
        .send()
        .await
        .ok()
        .map(|response| response.status().as_u16());
    drop(ollama);

    let report = json!({
        "prompt":"HOTR-17",
        "result":if quality_pass {"PASS"} else {"FAIL"},
        "binary_sha256":sha256(fs::read(env!("CARGO_BIN_EXE_hotr")).unwrap()),
        "corpus":{
            "schema":frozen.corpus.schema,
            "seed":frozen.corpus.seed,
            "sha256":frozen.corpus_sha256,
            "independent_reviewer":frozen.reviewer,
            "frozen_before_evaluation":true,
            "source_labels_sha256":frozen.source_labels_sha256,
            "record_summary_sha256":frozen.record_summary_sha256,
            "record_count":frozen.corpus.records.len(),
            "query_count":frozen.corpus.queries.len()
        },
        "model":{
            "name":hotr::embedding_transport::MODEL,
            "digest":hotr::embedding_transport::MODEL_DIGEST,
            "ollama_version":"0.32.6",
            "owned_pid":owned_ollama_pid,
            "observed_peer":indexed["last_peer"],
            "cold_observed_peer":cold.peer,
            "dimensions":hotr::embedding_transport::DIMENSIONS,
            "model_start_ms":model_start_ms,
            "first_direct_embedding_cold_ms":cold_embedding_ms,
            "unload_status":unload_status
        },
        "index":{
            "expected_visible_records":expected_visible,
            "status":indexed,
            "duration_ms":index_duration_ms,
            "rows":index_rows,
            "vector_bytes":indexed_vector_bytes
        },
        "vault":{
            "file_bytes_after_service_stop":vault_file_bytes,
            "integrity_check":integrity_check,
            "audit_valid":audit_valid
        },
        "development":{
            "thresholds_applied_before_held_out":true,
            "failures":development_failures,
            "summary":development_summary,
            "queries":development
        },
        "held_out":{
            "evaluated":!held_out.is_empty(),
            "failures":held_failures,
            "summary":held_summary,
            "queries":held_out
        },
        "measurement_notes":{
            "query_vector_cache_hit_observable":false,
            "potential_repeat_query_key_is_derived_only":true,
            "no_answer_abstention_feature":false,
            "no_answer_threshold_claimed":false,
            "runtime_control_telemetry_available":false,
            "cost_or_savings_claim":Value::Null,
            "query_or_body_text_in_report":false
        }
    });
    write_new(
        &run.join("HOTR-17-evaluation.json"),
        &serde_json::to_vec_pretty(&report).unwrap(),
    );
    scan(&run, &[&reader, &shared_writer, &private_writer]);
    assert!(quality_pass, "HOTR17_QUALITY_THRESHOLDS_FAILED");
}

#[cfg(test)]
mod metric_tests {
    use super::*;

    #[test]
    fn corpus_structure_has_unique_queries_and_disjoint_positive_partitions() {
        let path = checked_project_file(
            "tests/fixtures/hotr17/corpus.json",
            MAX_CORPUS_BYTES,
            "HOTR17_CORPUS_PATH_REJECTED",
        );
        let corpus: Corpus = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        validate_corpus(&corpus);
    }

    #[test]
    fn denial_with_context_and_wrong_revision_are_rejected() {
        let mut query = CorpusQuery {
            id: "test-query".into(),
            split: Split::Development,
            category: Category::Access,
            namespace: PRIVATE_NAMESPACE.into(),
            query: "restricted".into(),
            expected_ids: vec![],
            prohibited_ids: vec!["private-item".into()],
            rationale: "negative control".into(),
        };
        let observe = |status, value| HttpObservation {
            status: Some(status),
            value: Some(value),
            error_code: None,
            latency_us: 1,
            no_store: true,
            cors_absent: true,
        };
        let denied = observation_json(
            "hybrid",
            &query,
            observe(
                403,
                json!({"error":{"code":"forbidden"},"body":"PRIVATE-DETAIL-01"}),
            ),
            &HashMap::new(),
        );
        assert_eq!(denied["contract_valid"], false);
        query.category = Category::Paraphrase;
        query.namespace = SHARED_NAMESPACE.into();
        query.expected_ids = vec!["visible-item".into()];
        let expected = HashMap::from([(
            "visible-item".into(),
            ExpectedRecord {
                namespace: SHARED_NAMESPACE.into(),
                revision: 2,
                body: "current body".into(),
                sources: vec![],
                visible: true,
            },
        )]);
        let wrong = observation_json(
            "hybrid",
            &query,
            observe(
                200,
                json!({"retrieval_mode":"hybrid","records":[{"id":"visible-item","namespace":SHARED_NAMESPACE,"revision":1,"body":"old body","sources":[],"rrf_score":0.01}]}),
            ),
            &expected,
        );
        assert_eq!(wrong["contract_valid"], false);
        assert!(
            wrong["violations"]
                .as_array()
                .unwrap()
                .contains(&json!("wrong_revision:visible-item"))
        );
        let mut quality = QualityInput::default();
        mode_violations(&denied, &mut quality);
        mode_violations(&wrong, &mut quality);
        assert_eq!(quality.response_errors, 2);
        assert_eq!(quality.wrong_revision_hits, 1);
        assert!(quality.forbidden_hits >= 2);
    }

    fn input(lexical: f64, hybrid: f64) -> QualityInput {
        QualityInput {
            exact_total: 8,
            exact_correct: 8,
            paraphrase_total: 10,
            lexical_paraphrase_recall_sum: lexical * 10.0,
            hybrid_paraphrase_recall_sum: hybrid * 10.0,
            ..QualityInput::default()
        }
    }

    #[test]
    fn baseline_exception_and_gain_use_exact_threshold_math() {
        assert!(quality_failures(&input(0.90, 0.90)).is_empty());
        assert!(quality_failures(&input(0.89, 0.99)).is_empty());
        assert!(
            quality_failures(&input(0.89, 0.98))
                .contains(&"hybrid_paraphrase_gain_or_baseline_exception_failed")
        );
        assert!(
            quality_failures(&input(0.91, 0.90))
                .contains(&"hybrid_paraphrase_gain_or_baseline_exception_failed")
        );
    }

    #[test]
    fn misses_transport_errors_and_revision_leaks_fail_closed() {
        let mut failed = input(0.90, 0.90);
        failed.exact_correct = 7;
        failed.response_errors = 1;
        failed.wrong_revision_hits = 1;
        failed.forbidden_hits = 1;
        let failures = quality_failures(&failed);
        assert!(failures.contains(&"exact_id_not_100_percent"));
        assert!(failures.contains(&"response_error"));
        assert!(failures.contains(&"wrong_revision_hit"));
        assert!(failures.contains(&"forbidden_or_integrity_hit"));
    }
}
