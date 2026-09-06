//! Read-only owner viewer. Tokens are ephemeral and never become app credentials.
use crate::{
    api,
    capabilities::{Command, CommandResult},
    credentials,
    writer::WriteError,
};
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const REQUEST_LIMIT: usize = 16 * 1024;
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'; object-src 'none'";
struct Pending {
    hash: [u8; 32],
    expires: Instant,
    seconds: u32,
    attempts: u8,
}
struct Session {
    hash: [u8; 32],
    expires: Instant,
}
struct State {
    pending: Option<Pending>,
    session: Option<Session>,
    backup: Value,
}
pub(crate) struct Runtime(Mutex<State>);
impl Runtime {
    pub(crate) fn new() -> Self {
        Self(Mutex::new(State {
            pending: None,
            session: None,
            backup: json!({"scope":"current_service_process","status":"unknown","last_success":null,"last_attempt_at_ms":null}),
        }))
    }
    pub(crate) fn approve(&self, seconds: u32, port: u16) -> CommandResult {
        if !(5..=600).contains(&seconds) {
            return Err(WriteError::InvalidRequest);
        }
        let code = credentials::random_hex(32).map_err(|_| WriteError::PersistenceRejected)?;
        let hash = credentials::token_hash(&code).ok_or(WriteError::PersistenceRejected)?;
        let mut state = self.0.lock().map_err(|_| WriteError::Stopped)?;
        state.session = None;
        state.pending = Some(Pending {
            hash,
            expires: Instant::now() + Duration::from_secs(90),
            seconds,
            attempts: 0,
        });
        Ok(
            json!({"url":format!("http://127.0.0.1:{port}/viewer/"),"code":code.as_str(),"code_expires_in_seconds":90,"session_seconds":seconds}),
        )
    }
    fn exchange(&self, code: &str) -> CommandResult {
        let hash = credentials::token_hash(code).ok_or(WriteError::Unauthorized)?;
        let mut state = self.0.lock().map_err(|_| WriteError::Stopped)?;
        let pending = state.pending.take().ok_or(WriteError::Unauthorized)?;
        if Instant::now() >= pending.expires {
            return Err(WriteError::Unauthorized);
        }
        if !same(&hash, &pending.hash) {
            if pending.attempts < 7 {
                state.pending = Some(Pending {
                    attempts: pending.attempts + 1,
                    ..pending
                });
            }
            return Err(WriteError::Unauthorized);
        }
        let token = credentials::random_hex(32).map_err(|_| WriteError::PersistenceRejected)?;
        state.session = Some(Session {
            hash: credentials::token_hash(&token).ok_or(WriteError::PersistenceRejected)?,
            expires: Instant::now() + Duration::from_secs(u64::from(pending.seconds)),
        });
        Ok(json!({"token":token.as_str(),"expires_in_seconds":pending.seconds}))
    }
    pub(crate) fn clear(&self) -> Result<(), WriteError> {
        let mut state = self.0.lock().map_err(|_| WriteError::Stopped)?;
        state.pending = None;
        state.session = None;
        Ok(())
    }
    fn remaining(&self, hash: &[u8; 32]) -> Result<f64, WriteError> {
        let mut state = self.0.lock().map_err(|_| WriteError::Stopped)?;
        if state
            .session
            .as_ref()
            .is_some_and(|s| Instant::now() >= s.expires)
        {
            state.session = None;
        }
        let session = state.session.as_ref().ok_or(WriteError::Unauthorized)?;
        if !same(hash, &session.hash) {
            return Err(WriteError::Unauthorized);
        }
        Ok(session
            .expires
            .saturating_duration_since(Instant::now())
            .as_secs_f64())
    }
    fn logout(&self, hash: &[u8; 32]) -> CommandResult {
        let mut state = self.0.lock().map_err(|_| WriteError::Stopped)?;
        let session = state.session.as_ref().ok_or(WriteError::Unauthorized)?;
        if Instant::now() >= session.expires || !same(hash, &session.hash) {
            return Err(WriteError::Unauthorized);
        }
        state.session = None;
        state.pending = None;
        Ok(json!({"closed":true}))
    }
    pub(crate) fn backup_result(&self, result: Option<&Value>) -> Result<(), WriteError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| WriteError::PersistenceRejected)?
            .as_millis();
        let mut state = self.0.lock().map_err(|_| WriteError::Stopped)?;
        state.backup["last_attempt_at_ms"] = json!(now);
        state.backup["status"] = json!(if result.is_some() {
            "succeeded"
        } else {
            "failed"
        });
        if let Some(result) = result {
            state.backup["last_success"] = json!({"snapshot_id":result["snapshot_id"],"completed_at_ms":now,"bytes":result["bytes"],"watermark":result["watermark"]});
        }
        Ok(())
    }
    pub(crate) fn read(&self, db: &Connection, hash: &[u8; 32], query: Read) -> CommandResult {
        self.remaining(hash)?;
        let value = match query {
            Read::Ping => json!({"state":"unlocked","expires_in_seconds":self.remaining(hash)?}),
            Read::Index => crate::embedding::status(db)?,
            Read::Backup => self
                .0
                .lock()
                .map_err(|_| WriteError::Stopped)?
                .backup
                .clone(),
            Read::Search { query } => crate::retrieval::search(db, query)?,
            Read::List { page } => crate::retrieval::list(db, page)?,
            Read::History { query } => crate::retrieval::history(db, query)?,
            Read::Inspect { query } => {
                if query.expected_revision == Some(0) {
                    return Err(WriteError::InvalidRequest);
                }
                crate::lifecycle::inspect(db, query)?
            }
            Read::Namespaces { offset } => {
                offset_check(offset)?;
                let total: i64 =
                    db.query_row("SELECT count(*) FROM namespaces", [], |r| r.get(0))?;
                let names = db
                    .prepare("SELECT name FROM namespaces ORDER BY name LIMIT 50 OFFSET ?1")?
                    .query_map([offset], |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                json!({"namespaces":names,"total":total,"next_offset":next(offset,total)})
            }
            Read::Clients { offset } => {
                offset_check(offset)?;
                let total: i64 = db.query_row("SELECT count(*) FROM clients", [], |r| r.get(0))?;
                let mut clients=db.prepare("SELECT id,label,role,revoked,grant_revision FROM clients ORDER BY id LIMIT 50 OFFSET ?1")?.query_map([offset],|r|Ok(json!({"client_id":r.get::<_,String>(0)?,"label":r.get::<_,String>(1)?,"role":r.get::<_,String>(2)?,"revoked":r.get::<_,bool>(3)?,"grant_revision":r.get::<_,u32>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
                for client in &mut clients {
                    let id = client["client_id"]
                        .as_str()
                        .ok_or(WriteError::PersistenceRejected)?;
                    let namespaces=db.prepare("SELECT namespace FROM client_grants WHERE client_id=?1 ORDER BY namespace LIMIT 33")?.query_map([id],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                    if namespaces.len() > 32 {
                        return Err(WriteError::PersistenceRejected);
                    }
                    client["namespaces"] = json!(namespaces);
                }
                json!({"clients":clients,"total":total,"next_offset":next(offset,total)})
            }
            Read::Records { namespace, offset } => {
                offset_check(offset)?;
                if !crate::schema::valid_identifier(&namespace, true) {
                    return Err(WriteError::InvalidRequest);
                }
                let total: i64 = db.query_row(
                    "SELECT count(*) FROM records WHERE namespace=?1",
                    [&namespace],
                    |r| r.get(0),
                )?;
                let records=db.prepare("SELECT r.id,r.current_revision,v.kind,v.state,EXISTS(SELECT 1 FROM visible_records a WHERE a.namespace=r.namespace AND a.id=r.id) FROM records r JOIN revisions v ON v.namespace=r.namespace AND v.record_id=r.id AND v.revision=r.current_revision WHERE r.namespace=?1 ORDER BY r.id LIMIT 50 OFFSET ?2")?.query_map(params![namespace,offset],|r|Ok(json!({"namespace":namespace,"id":r.get::<_,String>(0)?,"revision":r.get::<_,u32>(1)?,"kind":r.get::<_,String>(2)?,"state":r.get::<_,String>(3)?,"visible":r.get::<_,bool>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
                json!({"records":records,"total":total,"next_offset":next(offset,total)})
            }
        };
        self.remaining(hash)?;
        Ok(value)
    }
}
fn same(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |v, (a, b)| v | (a ^ b)) == 0
}
fn offset_check(offset: u32) -> Result<(), WriteError> {
    if offset > 100_000 {
        Err(WriteError::InvalidRequest)
    } else {
        Ok(())
    }
}
fn next(offset: u32, total: i64) -> Option<u32> {
    let next = offset + 50;
    (i64::from(next) < total).then_some(next)
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Read {
    Ping,
    Index,
    Backup,
    Namespaces { offset: u32 },
    Clients { offset: u32 },
    Records { namespace: String, offset: u32 },
    Search { query: crate::retrieval::Search },
    List { page: crate::retrieval::Page },
    History { query: crate::retrieval::History },
    Inspect { query: crate::lifecycle::Inspect },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exchange {
    code: String,
}

pub(crate) fn headers(response: &mut Response) {
    response
        .headers_mut()
        .insert("content-security-policy", CSP.parse().expect("fixed CSP"));
    response
        .headers_mut()
        .insert("x-frame-options", "DENY".parse().expect("fixed header"));
    response.headers_mut().insert(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
            .parse()
            .expect("fixed header"),
    );
    response.headers_mut().insert(
        "cross-origin-resource-policy",
        "same-origin".parse().expect("fixed header"),
    );
    response.headers_mut().insert(
        "cross-origin-opener-policy",
        "same-origin".parse().expect("fixed header"),
    );
    response
        .headers_mut()
        .insert("pragma", "no-cache".parse().expect("fixed header"));
}
fn exactly(headers: &axum::http::HeaderMap, key: &str, value: &str) -> bool {
    headers.get_all(key).iter().count() == 1
        && headers.get(key).and_then(|v| v.to_str().ok()) == Some(value)
}
pub(crate) async fn dispatch(
    runtime: Arc<Runtime>,
    shared: api::SharedWriter,
    host: &str,
    request: Request<Body>,
) -> Response {
    let path = request.uri().path().to_owned();
    if request.method() == "GET" {
        let asset = match path.as_str() {
            "/viewer" | "/viewer/" => Some((
                "text/html; charset=utf-8",
                include_str!("viewer/index.html"),
            )),
            "/viewer/viewer.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("viewer/viewer.js"),
            )),
            "/viewer/viewer.css" => {
                Some(("text/css; charset=utf-8", include_str!("viewer/viewer.css")))
            }
            _ => None,
        };
        if let Some((kind, body)) = asset {
            return ([(header::CONTENT_TYPE, kind)], body).into_response();
        }
    }
    if request.method() != "POST"
        || !matches!(
            path.as_str(),
            "/viewer/api/session" | "/viewer/api/read" | "/viewer/api/logout"
        )
    {
        return api::error(StatusCode::NOT_FOUND, "not_found");
    }
    let h = request.headers();
    if !exactly(h, "origin", &format!("http://{host}"))
        || !exactly(h, "sec-fetch-site", "same-origin")
        || !exactly(h, "x-hotr-viewer", "1")
    {
        return api::error(StatusCode::FORBIDDEN, "origin_rejected");
    }
    if !exactly(h, "content-type", "application/json") || h.contains_key(header::COOKIE) {
        return api::error(StatusCode::BAD_REQUEST, "invalid_request");
    }
    let writer = shared.read().ok().and_then(|s| s.clone());
    let Some(writer) = writer else {
        return api::error(StatusCode::SERVICE_UNAVAILABLE, "locked");
    };
    let hash = if path != "/viewer/api/session" {
        if h.get_all(header::AUTHORIZATION).iter().count() != 1 {
            return api::error(StatusCode::UNAUTHORIZED, "unauthorized");
        }
        let hash = h
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .and_then(credentials::token_hash);
        let Some(hash) = hash else {
            return api::error(StatusCode::UNAUTHORIZED, "unauthorized");
        };
        if let Err(error) = runtime.remaining(&hash) {
            return api::service_error(error);
        }
        Some(hash)
    } else {
        if h.contains_key(header::AUTHORIZATION) {
            return api::error(StatusCode::BAD_REQUEST, "invalid_request");
        }
        None
    };
    let bytes = match to_bytes(request.into_body(), REQUEST_LIMIT).await {
        Ok(bytes) => Zeroizing::new(bytes.to_vec()),
        Err(_) => return api::error(StatusCode::PAYLOAD_TOO_LARGE, "request_limit"),
    };
    let result = match path.as_str() {
        "/viewer/api/session" => match api::decode::<Exchange>(&bytes) {
            Ok(exchange) => runtime.exchange(&Zeroizing::new(exchange.code)),
            Err(error) => Err(error),
        },
        "/viewer/api/read" => match (hash, api::decode::<Read>(&bytes)) {
            (Some(hash), Ok(query)) => {
                let result = writer
                    .command(Command::Viewer {
                        runtime: runtime.clone(),
                        hash,
                        query,
                    })
                    .await;
                match runtime.remaining(&hash) {
                    Ok(_) => result,
                    Err(error) => Err(error),
                }
            }
            (_, Err(error)) => Err(error),
            _ => Err(WriteError::Unauthorized),
        },
        "/viewer/api/logout" => match (hash, api::decode::<Value>(&bytes)) {
            (Some(hash), Ok(body)) if body == json!({}) => runtime.logout(&hash),
            _ => Err(WriteError::InvalidRequest),
        },
        _ => Err(WriteError::InvalidRequest),
    };
    if shared.read().ok().is_none_or(|s| s.is_none()) {
        return api::error(StatusCode::SERVICE_UNAVAILABLE, "locked");
    }
    match result {
        Ok(value) => api::response(StatusCode::OK, value),
        Err(error) => api::service_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approvals_are_single_use_replace_sessions_and_bound_lifetime() {
        let runtime = Runtime::new();
        assert!(runtime.approve(4, 1).is_err());
        assert!(runtime.approve(601, 1).is_err());
        let code = runtime.approve(5, 1).unwrap()["code"]
            .as_str()
            .unwrap()
            .to_owned();
        let token = runtime.exchange(&code).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(runtime.exchange(&code).is_err());
        let hash = credentials::token_hash(&token).unwrap();
        assert!(runtime.remaining(&hash).is_ok());
        runtime.approve(5, 1).unwrap();
        assert!(runtime.remaining(&hash).is_err());
        let code = runtime.approve(5, 1).unwrap()["code"]
            .as_str()
            .unwrap()
            .to_owned();
        for _ in 0..8 {
            assert!(runtime.exchange(&"0".repeat(64)).is_err());
        }
        assert!(runtime.exchange(&code).is_err());
        runtime.clear().unwrap();
    }
}
