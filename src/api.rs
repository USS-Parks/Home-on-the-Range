//! Loopback application API. Owner operations are absent from this transport.
use crate::{
    capabilities::{Command, Lookup},
    credentials,
    writer::{WriteError, WriteRequest, WriterHandle},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    io,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, net::TcpListener, sync::Semaphore, task::JoinSet, time::timeout};

pub const MAX_REQUEST: usize = 256 * 1024;
pub const MAX_RESPONSE: usize = 1024 * 1024;
pub const MAX_CONNECTIONS: usize = 128;
pub const MAX_ACTIVE_REQUESTS: usize = 64;
pub(crate) type SharedWriter = Arc<RwLock<Option<WriterHandle>>>;

#[derive(Clone)]
struct ApiState {
    writer: SharedWriter,
    host: String,
    requests: Arc<Semaphore>,
}

pub(crate) async fn run(listener: TcpListener, writer: SharedWriter) -> io::Result<()> {
    let address = listener.local_addr()?;
    if address.ip() != std::net::IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(io::Error::other("only IPv4 loopback is permitted"));
    }
    let state = ApiState {
        writer,
        host: address.to_string(),
        requests: Arc::new(Semaphore::new(MAX_ACTIVE_REQUESTS)),
    };
    let app = Router::new().fallback(endpoint).with_state(state);
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _=connections.join_next(),if !connections.is_empty()=>{},
            incoming=listener.accept()=>{
                let (mut stream,peer)=incoming?;
                if !peer.ip().is_loopback() {continue;}
                if connections.len()>=MAX_CONNECTIONS {
                    let _=timeout(Duration::from_millis(100),stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: 31\r\n\r\n{\"error\":{\"code\":\"overloaded\"}}" )).await;
                    continue;
                }
                stream.set_nodelay(true)?;
                let service=TowerToHyperService::new(app.clone());
                connections.spawn(async move {
                    let mut builder=hyper::server::conn::http1::Builder::new();
                    builder.timer(TokioTimer::new()).header_read_timeout(Duration::from_secs(5)).max_headers(32).max_buf_size(8192);
                    let _=builder.serve_connection(TokioIo::new(stream),service).await;
                });
            }
        }
    }
}

fn response(status: StatusCode, value: Value) -> Response {
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() <= MAX_RESPONSE => {
            (status, [(header::CONTENT_TYPE, "application/json")], bytes).into_response()
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            "{\"error\":{\"code\":\"response_limit\"}}",
        )
            .into_response(),
    }
}
fn error(status: StatusCode, code: &str) -> Response {
    response(status, json!({"error":{"code":code}}))
}

fn service_error(value: WriteError) -> Response {
    let (status, code) = match value {
        WriteError::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
        WriteError::RevisionConflict => (StatusCode::CONFLICT, "revision_conflict"),
        WriteError::IdempotencyConflict => (StatusCode::CONFLICT, "idempotency_conflict"),
        WriteError::Overloaded => (StatusCode::TOO_MANY_REQUESTS, "overloaded"),
        WriteError::Stopped => (StatusCode::SERVICE_UNAVAILABLE, "locked"),
        WriteError::PersistenceRejected => {
            (StatusCode::INTERNAL_SERVER_ERROR, "persistence_rejected")
        }
        WriteError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
        WriteError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
        WriteError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        WriteError::OutcomeUnknown => (StatusCode::GATEWAY_TIMEOUT, "outcome_unknown"),
    };
    error(status, code)
}

pub(crate) fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, WriteError> {
    if body.len() > MAX_REQUEST {
        return Err(WriteError::InvalidRequest);
    }
    let (mut depth, mut quoted, mut escape) = (0u32, false, false);
    for byte in body {
        if quoted {
            if escape {
                escape = false;
            } else if *byte == b'\\' {
                escape = true;
            } else if *byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 32 {
                        return Err(WriteError::InvalidRequest);
                    }
                }
                b'}' | b']' => {
                    depth = depth.checked_sub(1).ok_or(WriteError::InvalidRequest)?;
                }
                _ => {}
            }
        }
    }
    serde_json::from_slice(body).map_err(|_| WriteError::InvalidRequest)
}

async fn endpoint(State(state): State<ApiState>, request: Request) -> Response {
    let mut result = match timeout(Duration::from_secs(10), dispatch(state, request)).await {
        Ok(result) => result,
        Err(_) => error(StatusCode::GATEWAY_TIMEOUT, "outcome_unknown"),
    };
    result
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    result.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    result.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if result.status() == StatusCode::GATEWAY_TIMEOUT {
        result
            .headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    result
}

async fn dispatch(state: ApiState, request: Request<Body>) -> Response {
    let headers = request.headers();
    if headers.get_all(header::HOST).iter().count() != 1
        || headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(&state.host)
    {
        return error(StatusCode::FORBIDDEN, "host_rejected");
    }
    if headers.contains_key(header::ORIGIN) {
        return error(StatusCode::FORBIDDEN, "origin_rejected");
    }
    if request.uri().scheme().is_some()
        || request.uri().query().is_some()
        || request.uri().path().len() > 1024
    {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    }
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let hash = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(credentials::token_hash);
    let Some(hash) = hash else {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    let Ok(_permit) = state.requests.clone().try_acquire_owned() else {
        return error(StatusCode::TOO_MANY_REQUESTS, "overloaded");
    };
    let writer = match state.writer.read() {
        Ok(lock) => lock.clone(),
        Err(_) => None,
    };
    let Some(writer) = writer else {
        return error(StatusCode::SERVICE_UNAVAILABLE, "locked");
    };
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    if !matches!(
        (method.as_str(), path.as_str()),
        ("GET", "/v1/status")
            | ("POST", "/v1/records")
            | ("POST", "/v1/records/get")
            | ("POST", "/v1/search")
            | ("POST", "/v1/records/list")
            | ("POST", "/v1/records/count")
            | ("POST", "/v1/records/history")
    ) {
        return error(StatusCode::NOT_FOUND, "not_found");
    }
    // Refuse unknown/revoked credentials before waiting for a body. Every store
    // operation still rechecks authorization on the queue after body parsing.
    if let Err(error) = writer.command(Command::Status { hash }).await {
        return service_error(error);
    }
    if method == axum::http::Method::POST
        && headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .map(str::trim)
            != Some("application/json")
    {
        return error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "json_required");
    }
    if headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|size| size > MAX_REQUEST as u64)
    {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "request_limit");
    }
    let body = match to_bytes(request.into_body(), MAX_REQUEST).await {
        Ok(body) => body,
        Err(_) => return error(StatusCode::PAYLOAD_TOO_LARGE, "request_limit"),
    };
    let result = match path.as_str() {
        "/v1/search" => match decode(&body) {
            Ok(query) => writer.command(Command::Search { hash, query }).await,
            Err(error) => Err(error),
        },
        "/v1/records/list" => match decode(&body) {
            Ok(query) => writer.command(Command::List { hash, query }).await,
            Err(error) => Err(error),
        },
        "/v1/records/count" => match decode(&body) {
            Ok(query) => writer.command(Command::Count { hash, query }).await,
            Err(error) => Err(error),
        },
        "/v1/records/history" => match decode(&body) {
            Ok(query) => writer.command(Command::History { hash, query }).await,
            Err(error) => Err(error),
        },
        "/v1/status" => {
            if !body.is_empty() {
                return error(StatusCode::BAD_REQUEST, "invalid_request");
            }
            writer.command(Command::Status { hash }).await
        }
        "/v1/records/get" => {
            let query: Lookup = match decode(&body) {
                Ok(query) => query,
                Err(e) => return service_error(e),
            };
            writer.command(Command::Get { hash, query }).await
        }
        "/v1/records" => {
            let record: WriteRequest = match decode(&body) {
                Ok(record) => record,
                Err(e) => return service_error(e),
            };
            let pending = match writer.submit_authenticated(hash, record) {
                Ok(pending) => pending,
                Err(e) => return service_error(e),
            };
            pending.wait().await.and_then(|outcome| {
                serde_json::to_value(outcome).map_err(|_| WriteError::PersistenceRejected)
            })
        }
        _ => unreachable!(),
    };
    match result {
        Ok(value) => response(StatusCode::OK, value),
        Err(value) => service_error(value),
    }
}

/// Send exactly once to a server owned by this Windows account. Authentication
/// is decrypted only after the established socket's server identity is checked.
/// Third-party HTTP clients need an equivalent trusted endpoint boundary.
pub async fn scoped_request(
    profile: &credentials::CredentialProfile,
    method: &str,
    path: &str,
    value: Option<&Value>,
) -> io::Result<(u16, Value)> {
    if !matches!(method, "GET" | "POST")
        || !path.starts_with("/v1/")
        || path.len() > 1024
        || path.contains(['?', '#', '\r', '\n'])
    {
        return Err(io::Error::other("client request rejected"));
    }
    let body = value
        .map(serde_json::to_vec)
        .transpose()
        .map_err(|_| io::Error::other("client JSON rejected"))?
        .unwrap_or_default();
    if body.len() > MAX_REQUEST {
        return Err(io::Error::other("client request limit"));
    }
    timeout(Duration::from_secs(10), async {
        let stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, profile.port)).await?;
        let expected = crate::windows_security::current_sid()?;
        let mut verified = false;
        for _ in 0..20 {
            match crate::windows_security::tcp_peer_sid(stream.local_addr()?, stream.peer_addr()?) {
                Ok(sid) if sid == expected => {
                    verified = true;
                    break;
                }
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "server owner rejected",
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
                Err(error) => return Err(error),
            }
        }
        if !verified {
            return Err(io::Error::other("server identity unavailable"));
        }
        let token = credentials::unprotect(profile)?;
        let secret = zeroize::Zeroizing::new(format!("Bearer {}", token.as_str()));
        let mut authorization =
            HeaderValue::from_str(&secret).map_err(|_| io::Error::other("credential rejected"))?;
        authorization.set_sensitive(true);
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .map_err(|_| io::Error::other("local connection failed"))?;
        struct Driver(tokio::task::JoinHandle<()>);
        impl Drop for Driver {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let _driver = Driver(tokio::spawn(async move {
            let _ = connection.await;
        }));
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, format!("127.0.0.1:{}", profile.port))
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .map_err(|_| io::Error::other("client request rejected"))?;
        let response = sender
            .send_request(request)
            .await
            .map_err(|_| io::Error::other("request outcome unknown"))?;
        let status = response.status().as_u16();
        let bytes = to_bytes(Body::new(response.into_body()), MAX_RESPONSE)
            .await
            .map_err(|_| io::Error::other("response limit or transport failure"))?;
        let value = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::other("invalid local response"))?;
        Ok((status, value))
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "request outcome unknown"))?
}
