//! Bounded Ollama embedding transport. Every request is sent over a verified
//! same-owner IPv4 loopback socket; there is no URL parsing or cloud fallback.

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, header},
};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io, net::Ipv4Addr, time::Duration};
use tokio::time::timeout;

pub const MODEL: &str = "nomic-embed-text:v1.5";
pub const MODEL_DIGEST: &str = "0a109f422b47e3a30ba2b10eca18548e944e8a23073ee3f3e947efcf3c45e59f";
pub const DIMENSIONS: usize = 768;

const MAX_TEXT_BYTES: usize = 65_536;
const MAX_CHUNK_BYTES: usize = 2_048;
const MAX_CHUNKS: usize = 33;
// JSON control-character escaping can expand an accepted 64 KiB input by 6x.
const MAX_REQUEST_BYTES: usize = 512 * 1024;
// A full 33 x 768 JSON batch is larger than the service API's ordinary bound.
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const OVERALL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_COMPONENT_MAGNITUDE: f32 = 1_000_000.0;
const MIN_NORM_SQUARED: f64 = 1.0e-24;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    /// Actual TCP peer observed on the embedding request, after SID validation.
    pub peer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    InvalidInput,
    Unavailable,
    PeerRejected,
    TimedOut,
    ResponseTooLarge,
    HttpRejected,
    ResponseRejected,
    ModelMismatch,
    MetadataMismatch,
    DimensionMismatch,
    VectorRejected,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "embedding_invalid_input",
            Self::Unavailable => "embedding_unavailable",
            Self::PeerRejected => "embedding_peer_rejected",
            Self::TimedOut => "embedding_timeout",
            Self::ResponseTooLarge => "embedding_response_too_large",
            Self::HttpRejected => "embedding_http_rejected",
            Self::ResponseRejected => "embedding_response_rejected",
            Self::ModelMismatch => "embedding_model_mismatch",
            Self::MetadataMismatch => "embedding_metadata_mismatch",
            Self::DimensionMismatch => "embedding_dimension_mismatch",
            Self::VectorRejected => "embedding_vector_rejected",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

struct LocalResponse {
    value: Value,
    peer: String,
}

struct Driver(tokio::task::JoinHandle<()>);

impl Drop for Driver {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Embed one complete document or query. Inputs are never silently truncated.
pub async fn embed(port: u16, text: &str, query: bool) -> Result<Embedding, Error> {
    timeout(OVERALL_TIMEOUT, embed_inner(port, text, query))
        .await
        .map_err(|_| Error::TimedOut)?
}

async fn embed_inner(port: u16, text: &str, query: bool) -> Result<Embedding, Error> {
    let chunks = chunk(text, query)?;

    verify_tags(&request(port, Method::GET, "/api/tags", None).await?.value)?;
    verify_show(
        &request(
            port,
            Method::POST,
            "/api/show",
            Some(&json!({"model": MODEL})),
        )
        .await?
        .value,
    )?;

    let response = request(
        port,
        Method::POST,
        "/api/embed",
        Some(&json!({
            "model": MODEL,
            "input": chunks,
            "truncate": false,
            "dimensions": DIMENSIONS,
            "options": {"num_thread": 4, "num_gpu": 0}
        })),
    )
    .await?;

    // Recheck the mutable tag after inference before accepting any vector.
    verify_tags(&request(port, Method::GET, "/api/tags", None).await?.value)?;
    let vector = combine_embeddings(&response.value, chunks.len())?;
    Ok(Embedding {
        vector,
        peer: response.peer,
    })
}

async fn request(
    port: u16,
    method: Method,
    path: &'static str,
    value: Option<&Value>,
) -> Result<LocalResponse, Error> {
    let body = value
        .map(serde_json::to_vec)
        .transpose()
        .map_err(|_| Error::InvalidInput)?
        .unwrap_or_default();
    if body.len() > MAX_REQUEST_BYTES {
        return Err(Error::InvalidInput);
    }

    timeout(REQUEST_TIMEOUT, async move {
        let stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|_| Error::Unavailable)?;
        let local = stream.local_addr().map_err(|_| Error::Unavailable)?;
        let peer = stream.peer_addr().map_err(|_| Error::Unavailable)?;
        if peer.ip() != Ipv4Addr::LOCALHOST || peer.port() != port {
            return Err(Error::PeerRejected);
        }

        let expected = crate::windows_security::current_sid().map_err(|_| Error::PeerRejected)?;
        let mut verified = false;
        for _ in 0..20 {
            match crate::windows_security::tcp_peer_sid(local, peer) {
                Ok(sid) if sid == expected => {
                    verified = true;
                    break;
                }
                Ok(_) => return Err(Error::PeerRejected),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(_) => return Err(Error::PeerRejected),
            }
        }
        if !verified {
            return Err(Error::PeerRejected);
        }

        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .map_err(|_| Error::Unavailable)?;
        let _driver = Driver(tokio::spawn(async move {
            let _ = connection.await;
        }));
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, format!("127.0.0.1:{port}"))
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .map_err(|_| Error::InvalidInput)?;
        let response = sender
            .send_request(request)
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::HttpRejected);
        }
        if response
            .headers()
            .get(header::CONTENT_LENGTH)
            .map(|length| {
                length
                    .to_str()
                    .ok()
                    .and_then(|length| length.parse::<usize>().ok())
                    .filter(|length| *length <= MAX_RESPONSE_BYTES)
                    .is_none()
            })
            .unwrap_or(false)
        {
            return Err(Error::ResponseTooLarge);
        }
        let bytes = to_bytes(Body::new(response.into_body()), MAX_RESPONSE_BYTES)
            .await
            .map_err(|_| Error::ResponseRejected)?;
        let value = serde_json::from_slice(&bytes).map_err(|_| Error::ResponseRejected)?;
        Ok(LocalResponse {
            value,
            peer: peer.to_string(),
        })
    })
    .await
    .map_err(|_| Error::TimedOut)?
}

fn chunk(text: &str, query: bool) -> Result<Vec<String>, Error> {
    if text.is_empty() || text.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput);
    }
    let prefix = if query {
        "search_query: "
    } else {
        "search_document: "
    };
    let payload_limit = MAX_CHUNK_BYTES - prefix.len();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + payload_limit).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == start || chunks.len() == MAX_CHUNKS {
            return Err(Error::InvalidInput);
        }
        chunks.push(format!("{prefix}{}", &text[start..end]));
        start = end;
    }
    Ok(chunks)
}

fn verify_tags(value: &Value) -> Result<(), Error> {
    let models = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or(Error::ResponseRejected)?;
    let mut matching = models.iter().filter(|model| {
        model.get("name").and_then(Value::as_str) == Some(MODEL)
            && model
                .get("model")
                .is_none_or(|name| name.as_str() == Some(MODEL))
    });
    let model = matching.next().ok_or(Error::ModelMismatch)?;
    if matching.next().is_some()
        || model.get("digest").and_then(Value::as_str) != Some(MODEL_DIGEST)
    {
        return Err(Error::ModelMismatch);
    }
    Ok(())
}

fn verify_show(value: &Value) -> Result<(), Error> {
    let license = value
        .get("license")
        .and_then(Value::as_str)
        .filter(|license| {
            let normalized = license.to_ascii_lowercase();
            normalized.contains("apache") && normalized.contains("2.0")
        })
        .ok_or(Error::MetadataMismatch)?;
    if license.len() > MAX_RESPONSE_BYTES {
        return Err(Error::MetadataMismatch);
    }

    let details = value
        .get("details")
        .and_then(Value::as_object)
        .ok_or(Error::MetadataMismatch)?;
    if details.get("format").and_then(Value::as_str) != Some("gguf")
        || details.get("family").and_then(Value::as_str) != Some("nomic-bert")
    {
        return Err(Error::MetadataMismatch);
    }

    let info = value
        .get("model_info")
        .and_then(Value::as_object)
        .ok_or(Error::MetadataMismatch)?;
    if info.get("general.architecture").and_then(Value::as_str) != Some("nomic-bert") {
        return Err(Error::MetadataMismatch);
    }
    let dimensions: Vec<u64> = info
        .iter()
        .filter(|(key, _)| key.ends_with(".embedding_length"))
        .filter_map(|(_, value)| value.as_u64())
        .collect();
    if dimensions.as_slice() != [DIMENSIONS as u64] {
        return Err(Error::DimensionMismatch);
    }
    let contexts: Vec<u64> = info
        .iter()
        .filter(|(key, _)| key.ends_with(".context_length"))
        .filter_map(|(_, value)| value.as_u64())
        .collect();
    if contexts.len() != 1 || contexts[0] < MAX_CHUNK_BYTES as u64 {
        return Err(Error::MetadataMismatch);
    }
    if value.get("capabilities").is_some_and(|capabilities| {
        !capabilities
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("embedding")))
    }) {
        return Err(Error::MetadataMismatch);
    }
    // This is a local consistency check. The pinned manifest, not provider-
    // reported license text, remains the licensing authority.
    Ok(())
}

fn combine_embeddings(value: &Value, expected: usize) -> Result<Vec<f32>, Error> {
    if value.get("model").and_then(Value::as_str) != Some(MODEL) {
        return Err(Error::ModelMismatch);
    }
    let embeddings = value
        .get("embeddings")
        .and_then(Value::as_array)
        .filter(|embeddings| embeddings.len() == expected)
        .ok_or(Error::DimensionMismatch)?;
    let mut sum = vec![0.0_f64; DIMENSIONS];
    for embedding in embeddings {
        let values = embedding.as_array().ok_or(Error::VectorRejected)?;
        if values.len() != DIMENSIONS {
            return Err(Error::DimensionMismatch);
        }
        let mut vector = Vec::with_capacity(DIMENSIONS);
        let mut norm_squared = 0.0_f64;
        for value in values {
            let component =
                serde_json::from_value::<f32>(value.clone()).map_err(|_| Error::VectorRejected)?;
            if !component.is_finite() || component.abs() > MAX_COMPONENT_MAGNITUDE {
                return Err(Error::VectorRejected);
            }
            norm_squared += f64::from(component) * f64::from(component);
            vector.push(component);
        }
        if !norm_squared.is_finite() || norm_squared <= MIN_NORM_SQUARED {
            return Err(Error::VectorRejected);
        }
        let norm = norm_squared.sqrt();
        for (total, component) in sum.iter_mut().zip(vector) {
            *total += f64::from(component) / norm;
        }
    }

    let norm_squared: f64 = sum.iter().map(|component| component * component).sum();
    if !norm_squared.is_finite() || norm_squared <= MIN_NORM_SQUARED {
        return Err(Error::VectorRejected);
    }
    let norm = norm_squared.sqrt();
    let vector: Vec<f32> = sum
        .into_iter()
        .map(|component| (component / norm) as f32)
        .collect();
    if vector.iter().any(|component| !component.is_finite()) {
        return Err(Error::VectorRejected);
    }
    Ok(vector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    struct Reply {
        status: u16,
        body: Vec<u8>,
        declared_length: Option<usize>,
    }

    impl Reply {
        fn json(value: Value) -> Self {
            Self {
                status: 200,
                body: serde_json::to_vec(&value).unwrap(),
                declared_length: None,
            }
        }

        fn raw(body: impl Into<Vec<u8>>) -> Self {
            Self {
                status: 200,
                body: body.into(),
                declared_length: None,
            }
        }
    }

    async fn read_request(stream: &mut TcpStream) {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let Ok(read) = stream.read(&mut buffer).await else {
                return;
            };
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buffer[..read]);
            let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|length| length.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                return;
            }
        }
    }

    async fn serve(replies: Vec<Reply>) -> (u16, Arc<AtomicUsize>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&hits);
        tokio::spawn(async move {
            for reply in replies {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                observed.fetch_add(1, Ordering::SeqCst);
                read_request(&mut stream).await;
                let reason = if reply.status == 200 { "OK" } else { "Found" };
                let length = reply.declared_length.unwrap_or(reply.body.len());
                let headers = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    reply.status, reason, length
                );
                if stream.write_all(headers.as_bytes()).await.is_ok() {
                    let _ = stream.write_all(&reply.body).await;
                }
            }
        });
        (port, hits)
    }

    fn tags(digest: &str) -> Reply {
        Reply::json(json!({
            "models": [{"name": MODEL, "model": MODEL, "digest": digest}]
        }))
    }

    fn show(dimensions: usize) -> Reply {
        Reply::json(json!({
            "license": "Apache License 2.0",
            "details": {"format": "gguf", "family": "nomic-bert"},
            "model_info": {
                "general.architecture": "nomic-bert",
                "nomic-bert.context_length": 8192,
                "nomic-bert.embedding_length": dimensions
            },
            "capabilities": ["embedding"]
        }))
    }

    async fn fixture(replies: Vec<Reply>) -> (Result<Embedding, Error>, usize) {
        let (port, hits) = serve(replies).await;
        let result = embed(port, "fixture", false).await;
        (result, hits.load(Ordering::SeqCst))
    }

    #[test]
    fn chunking_preserves_complete_unicode_input() {
        let text = "aé🦀".repeat(9_000);
        let chunks = chunk(&text, false).unwrap();
        assert!(chunks.len() <= MAX_CHUNKS);
        assert!(chunks.iter().all(|chunk| chunk.len() <= MAX_CHUNK_BYTES));
        let rebuilt: String = chunks
            .iter()
            .map(|chunk| chunk.strip_prefix("search_document: ").unwrap())
            .collect();
        assert_eq!(rebuilt, text);

        let exact_max = "x".repeat(MAX_TEXT_BYTES);
        let chunks = chunk(&exact_max, true).unwrap();
        let rebuilt: String = chunks
            .iter()
            .map(|chunk| chunk.strip_prefix("search_query: ").unwrap())
            .collect();
        assert_eq!(rebuilt, exact_max);
    }

    #[test]
    fn rejects_oversize_and_invalid_vectors() {
        assert_eq!(
            chunk(&"x".repeat(MAX_TEXT_BYTES + 1), false),
            Err(Error::InvalidInput)
        );
        assert_eq!(
            combine_embeddings(
                &json!({"model": MODEL, "embeddings": [vec![0.0_f32; DIMENSIONS]]}),
                1
            ),
            Err(Error::VectorRejected)
        );
        assert_eq!(
            combine_embeddings(&json!({"model": MODEL, "embeddings": [[1.0]]}), 1),
            Err(Error::DimensionMismatch)
        );
    }

    #[test]
    fn normalized_chunk_average_is_unit_length() {
        let mut first = vec![0.0_f32; DIMENSIONS];
        let mut second = vec![0.0_f32; DIMENSIONS];
        first[0] = 3.0;
        second[1] = 4.0;
        let vector =
            combine_embeddings(&json!({"model": MODEL, "embeddings": [first, second]}), 2).unwrap();
        let norm: f64 = vector
            .iter()
            .map(|value| f64::from(*value) * f64::from(*value))
            .sum::<f64>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1.0e-6);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loopback_fixture_rejects_model_metadata_and_redirects() {
        let (result, hits) = fixture(vec![tags("wrong")]).await;
        assert_eq!(result, Err(Error::ModelMismatch));
        assert_eq!(hits, 1);

        let (result, hits) = fixture(vec![tags(MODEL_DIGEST), show(DIMENSIONS - 1)]).await;
        assert_eq!(result, Err(Error::DimensionMismatch));
        assert_eq!(hits, 2);

        let (result, hits) = fixture(vec![Reply {
            status: 302,
            body: Vec::new(),
            declared_length: None,
        }])
        .await;
        assert_eq!(result, Err(Error::HttpRejected));
        assert_eq!(hits, 1, "redirect must not be followed");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loopback_fixture_rechecks_tag_and_response_model() {
        let (result, hits) = fixture(vec![
            tags(MODEL_DIGEST),
            show(DIMENSIONS),
            Reply::json(json!({"model": MODEL, "embeddings": [[1.0]]})),
            tags("changed"),
        ])
        .await;
        assert_eq!(result, Err(Error::ModelMismatch));
        assert_eq!(hits, 4);

        let (result, hits) = fixture(vec![
            tags(MODEL_DIGEST),
            show(DIMENSIONS),
            Reply::json(json!({"model": "other", "embeddings": []})),
            tags(MODEL_DIGEST),
        ])
        .await;
        assert_eq!(result, Err(Error::ModelMismatch));
        assert_eq!(hits, 4);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loopback_fixture_rejects_malformed_and_bounded_responses() {
        for malformed in [
            format!(r#"{{"model":"{MODEL}","embeddings":[[NaN]]}}"#),
            format!(r#"{{"model":"{MODEL}","embeddings":[[1e400]]}}"#),
        ] {
            let (result, hits) = fixture(vec![
                tags(MODEL_DIGEST),
                show(DIMENSIONS),
                Reply::raw(malformed),
            ])
            .await;
            assert_eq!(result, Err(Error::ResponseRejected));
            assert_eq!(hits, 3);
        }

        let (result, hits) = fixture(vec![Reply {
            status: 200,
            body: vec![b' '; MAX_RESPONSE_BYTES + 1],
            declared_length: None,
        }])
        .await;
        assert_eq!(result, Err(Error::ResponseTooLarge));
        assert_eq!(hits, 1);

        let body = br#"{"models":[]}"#.to_vec();
        let (result, hits) = fixture(vec![Reply {
            status: 200,
            declared_length: Some(body.len() + 10),
            body,
        }])
        .await;
        assert_eq!(result, Err(Error::ResponseRejected));
        assert_eq!(hits, 1);
    }
}
