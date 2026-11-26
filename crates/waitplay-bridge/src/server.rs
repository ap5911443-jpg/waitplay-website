//! HTTP side of the bridge: the loopback filter, CORS policy, 32 KB JSON body parsing and the route table, in the same
//! order Express ran them (loopback -> CORS -> JSON body -> router). WebSocket upgrades on `/ws` are handled here too.

use crate::event_validator::validate_event_input;
use crate::limits::{self, Limits};
use crate::security::{self, LoopbackVerdict};
use crate::session_manager::SessionManager;
use crate::ws::{self, WsHub};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::header::{self, HeaderName, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Map, Value};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;
use waitplay_core::protocol::MAX_PAYLOAD_BYTES;
use waitplay_core::{now_ms, state, BridgeHealth};

pub type HealthFn = Box<dyn Fn() -> BridgeHealth + Send + Sync>;

/// Everything a request handler needs (the arguments of the original `createExpressApp`).
pub struct BridgeState {
    pub sessions: Arc<SessionManager>,
    /// `None` runs the HTTP API without a WebSocket endpoint.
    pub ws: Option<Arc<WsHub>>,
    pub get_health: HealthFn,
    pub token: String,
    pub limits: Limits,
}

impl BridgeState {
    pub fn new(sessions: Arc<SessionManager>, ws: Option<Arc<WsHub>>, get_health: HealthFn, token: String) -> BridgeState {
        BridgeState::with_limits(sessions, ws, get_health, token, Limits::default())
    }

    pub fn with_limits(sessions: Arc<SessionManager>, ws: Option<Arc<WsHub>>, get_health: HealthFn, token: String, limits: Limits) -> BridgeState {
        BridgeState { sessions, ws, get_health, token, limits }
    }
}

type Resp = Response<Full<Bytes>>;

const JSON_TYPE: &str = "application/json; charset=utf-8";
const HTML_TYPE: &str = "text/html; charset=utf-8";

fn json_response(status: u16, body: &Value) -> Resp {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, JSON_TYPE)
        .body(Full::new(Bytes::from(bytes)))
        .expect("static response parts are valid")
}

fn error_json(status: u16, error: &str, message: &str) -> Resp {
    json_response(status, &json!({ "error": error, "message": message }))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

fn not_found_page(method: &Method, target: &str) -> Resp {
    let body = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Error</title>\n</head>\n<body>\n<pre>Cannot {} {}</pre>\n</body>\n</html>\n",
        html_escape(method.as_str()),
        html_escape(target)
    );
    Response::builder()
        .status(404)
        .header(header::CONTENT_TYPE, HTML_TYPE)
        .body(Full::new(Bytes::from(body)))
        .expect("static response parts are valid")
}

fn with_headers(mut resp: Resp, extra: &[(HeaderName, HeaderValue)]) -> Resp {
    for (k, v) in extra {
        resp.headers_mut().append(k.clone(), v.clone());
    }
    resp
}

/// Accept loop. Runs until the task is dropped/aborted.
pub async fn serve(listener: TcpListener, state: Arc<BridgeState>) {
    loop {
        let (stream, remote) = match listener.accept().await {
            Ok(x) => x,
            Err(_) => {
                // Transient accept failures (e.g. EMFILE) must not kill the bridge.
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }
        };
        let _ = stream.set_nodelay(true);
        tokio::spawn(serve_connection(stream, remote, state.clone()));
    }
}

/// Serves one HTTP/1.1 connection; `remote` is what the loopback filter judges.
pub async fn serve_connection<I>(io: I, remote: SocketAddr, state: Arc<BridgeState>)
where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let service = hyper::service::service_fn(move |req| {
        let state = state.clone();
        async move { Ok::<_, Infallible>(handle(state, remote, req).await) }
    });
    let conn = hyper::server::conn::http1::Builder::new().serve_connection(TokioIo::new(io), service).with_upgrades();
    let _ = conn.await;
}

fn header_str(headers: &hyper::HeaderMap, name: HeaderName) -> Option<&str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

fn wants_upgrade(headers: &hyper::HeaderMap) -> bool {
    headers.contains_key(header::UPGRADE)
        && header_str(headers, header::CONNECTION).is_some_and(|c| c.split(',').any(|t| t.trim().eq_ignore_ascii_case("upgrade")))
}

async fn handle(state: Arc<BridgeState>, remote: SocketAddr, mut req: Request<Incoming>) -> Resp {
    // WebSocket upgrades bypass the HTTP middleware chain (the original `ws` server hooked the raw upgrade event).
    if wants_upgrade(req.headers()) {
        return handle_ws_upgrade(state, remote, &mut req);
    }
    let (parts, body) = req.into_parts();

    // 1. Loopback only.
    match security::loopback_only(&remote, header_str(&parts.headers, header::HOST)) {
        LoopbackVerdict::Allowed => {}
        LoopbackVerdict::NonLoopback => {
            return error_json(403, "Forbidden", "Vibe Play bridge only accepts connections from localhost/loopback.");
        }
        LoopbackVerdict::BadHost(host) => {
            return error_json(400, "Bad Request", &format!("Invalid host \"{}\". Requests must target localhost or 127.0.0.1.", host));
        }
    }

    // 2. CORS: no Origin (curl, hooks, native fetch) is fine; browsers must come from localhost.
    let mut cors: Vec<(HeaderName, HeaderValue)> = Vec::new();
    if let Some(origin) = parts.headers.get(header::ORIGIN).cloned() {
        match origin.to_str() {
            Ok(o) if security::origin_allowed(o) => cors.push((header::ACCESS_CONTROL_ALLOW_ORIGIN, origin)),
            _ => return error_json(500, "Internal Server Error", "Not allowed by Vibe Play Bridge CORS policy"),
        }
    }
    cors.push((header::VARY, HeaderValue::from_static("Origin")));
    if parts.method == Method::OPTIONS {
        cors.push((header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET,POST,OPTIONS")));
        cors.push((header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type,x-bridge-token,Authorization")));
        cors.push((header::CONTENT_LENGTH, HeaderValue::from_static("0")));
        let resp = Response::builder().status(StatusCode::NO_CONTENT).body(Full::new(Bytes::new())).expect("static response parts are valid");
        return with_headers(resp, &cors);
    }

    // 3. JSON body (32 KB ceiling).
    let json_body = match read_json_body(&parts.headers, body).await {
        Ok(v) => v,
        // The oversized body is not read, so the connection cannot be reused for another request.
        Err(BodyError::TooLarge) => {
            let mut close = cors.clone();
            close.push((header::CONNECTION, HeaderValue::from_static("close")));
            return with_headers(error_json(413, "Payload Too Large", "request entity too large"), &close);
        }
        Err(BodyError::Invalid(msg)) => return with_headers(error_json(400, "Bad Request", &msg), &cors),
    };

    // 4. Routes.
    with_headers(route(&state, &parts, json_body), &cors)
}

enum BodyError {
    TooLarge,
    Invalid(String),
}

fn is_json_content_type(headers: &hyper::HeaderMap) -> bool {
    header_str(headers, header::CONTENT_TYPE)
        .map(|ct| ct.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/json"))
        .unwrap_or(false)
}

fn parsed_content_length(headers: &hyper::HeaderMap) -> Option<u64> {
    header_str(headers, header::CONTENT_LENGTH).and_then(|v| v.trim().parse::<u64>().ok())
}

/// Mirrors `express.json({ limit: "32kb" })`: only `application/json` bodies are read; empty or absent bodies become
/// `{}`; strict mode accepts only objects and arrays.
async fn read_json_body(headers: &hyper::HeaderMap, body: Incoming) -> Result<Value, BodyError> {
    let empty = || Value::Object(Map::new());
    let has_body = headers.contains_key(header::TRANSFER_ENCODING) || parsed_content_length(headers).is_some();
    if !has_body || !is_json_content_type(headers) {
        return Ok(empty());
    }
    if parsed_content_length(headers).is_some_and(|len| len > MAX_PAYLOAD_BYTES as u64) {
        return Err(BodyError::TooLarge);
    }
    let bytes = match Limited::new(body, MAX_PAYLOAD_BYTES).collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return Err(if e.downcast_ref::<http_body_util::LengthLimitError>().is_some() {
                BodyError::TooLarge
            } else {
                BodyError::Invalid("request aborted".to_string())
            })
        }
    };
    match bytes.iter().copied().find(|b| !b.is_ascii_whitespace()) {
        None => Ok(empty()),
        Some(b'{') | Some(b'[') => serde_json::from_slice(&bytes).map_err(|e| BodyError::Invalid(e.to_string())),
        Some(_) => Err(BodyError::Invalid("Unexpected token in JSON body: expected an object or array".to_string())),
    }
}

/// The token a request presents: `x-bridge-token` wins when present at all, else `Authorization: Bearer`.
fn presented_token(headers: &hyper::HeaderMap) -> Option<String> {
    let joined = |name: &str| -> Option<String> {
        let values: Vec<&str> = headers.get_all(name).iter().filter_map(|v| v.to_str().ok()).collect();
        if values.is_empty() {
            None
        } else {
            Some(values.join(", "))
        }
    };
    security::presented_token(joined("x-bridge-token").as_deref(), joined("authorization").as_deref())
}

/// `Ok` for the right token. Otherwise the answer to send: 401, or 429 once too many wrong attempts arrive (the rejection is
/// logged, at most one line every ten seconds, so a flood leaves a trace without filling the log).
fn check_auth(state: &BridgeState, headers: &hyper::HeaderMap, what: &str) -> Result<(), Resp> {
    if presented_token(headers).is_some_and(|t| security::tokens_equal(&t, &state.token)) {
        Ok(())
    } else {
        Err(reject(state, what))
    }
}

fn reject(state: &BridgeState, what: &str) -> Resp {
    if !state.limits.failed_auth.allow() {
        return too_many("Too many failed authentication attempts. Try again in a minute.");
    }
    if state.limits.auth_log.allow() {
        eprintln!("[Vibe Play Bridge] rejected unauthenticated request: {}", what);
    }
    unauthorized()
}

fn too_many(message: &str) -> Resp {
    with_headers(error_json(429, "Too Many Requests", message), &[(header::RETRY_AFTER, HeaderValue::from_static("1"))])
}

fn unauthorized() -> Resp {
    error_json(
        401,
        "Unauthorized",
        "Missing or invalid bridge authentication token. Provide x-bridge-token or Authorization: Bearer <token>",
    )
}


/// Express matches routes case-insensitively and ignores one trailing slash.
fn path_segments(path: &str) -> Vec<&str> {
    let trimmed = path.strip_prefix('/').unwrap_or(path);
    let trimmed = trimmed.strip_suffix('/').unwrap_or(trimmed);
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn route(state: &BridgeState, parts: &hyper::http::request::Parts, body: Value) -> Resp {
    let method = &parts.method;
    let is_get = method == Method::GET || method == Method::HEAD;
    let is_post = method == Method::POST;
    let segs = path_segments(parts.uri.path());
    let lower: Vec<String> = segs.iter().map(|s| s.to_ascii_lowercase()).collect();
    let lower: Vec<&str> = lower.iter().map(String::as_str).collect();

    // Everything except /health needs the bridge token: session data and the automation switch are not public.
    macro_rules! auth {
        ($what:expr) => {
            if let Err(resp) = check_auth(state, &parts.headers, $what) {
                return resp;
            }
        };
    }

    match (lower.as_slice(), is_get, is_post) {
        (["health"], true, _) => json_response(200, &serde_json::to_value((state.get_health)()).unwrap_or(Value::Null)),
        (["state"], true, _) => {
            auth!("GET /state");
            json_response(200, &json!({ "enabled": state::is_enabled() }))
        }
        (["state"], _, true) => {
            auth!("POST /state");
            let requested = body.get("enabled").and_then(Value::as_bool).unwrap_or_else(|| !state::is_enabled());
            state::set_enabled(requested);
            if let Some(ws) = &state.ws {
                ws.broadcast(&waitplay_core::BridgeClientMessage::StateChange { enabled: requested });
            }
            json_response(200, &json!({ "status": "ok", "enabled": requested }))
        }
        (["sessions"], true, _) => {
            auth!("GET /sessions");
            json_response(200, &json!({ "sessions": state.sessions.get_all_sessions() }))
        }
        (["sessions", _], true, _) => {
            auth!("GET /sessions/:id");
            let Some(id) = percent_decode(segs[1]) else { return error_json(400, "Bad Request", "Failed to decode param") };
            match state.sessions.get_session(&id) {
                Some(session) => json_response(200, &json!({ "session": session })),
                None => session_not_found(&id),
            }
        }
        (["sessions", _, "reset"], _, true) => {
            auth!("POST /sessions/:id/reset");
            let Some(id) = percent_decode(segs[1]) else { return error_json(400, "Bad Request", "Failed to decode param") };
            if !state.sessions.reset_session(&id) {
                return session_not_found(&id);
            }
            if let Some(ws) = &state.ws {
                ws.broadcast(&waitplay_core::BridgeClientMessage::SessionList { sessions: state.sessions.get_all_sessions() });
            }
            json_response(200, &json!({ "status": "reset", "sessionId": id }))
        }
        (["events"], _, true) => {
            auth!("POST /events");
            if !state.limits.events.allow() {
                return too_many("Too many events per second. Slow down.");
            }
            let content_length = header_str(&parts.headers, header::CONTENT_LENGTH).and_then(|v| v.trim().parse::<usize>().ok());
            let payload = match validate_event_input(&body, content_length) {
                Ok(p) => p,
                Err(message) => return error_json(400, "ValidationError", &message),
            };
            let (session, status) = state.sessions.process_event(&payload);
            if let Some(ws) = &state.ws {
                ws.broadcast_event(&payload, &session);
            }
            json_response(
                200,
                &json!({
                    "status": "received",
                    "sessionId": payload.session_id,
                    "event": payload.event,
                    "agentStatus": status,
                    "timestamp": now_ms(),
                }),
            )
        }
        _ => not_found_page(method, parts.uri.path_and_query().map_or("/", |pq| pq.as_str())),
    }
}

fn session_not_found(id: &str) -> Resp {
    error_json(404, "SessionNotFound", &format!("Session {} not found", id))
}

fn bad_request() -> Resp {
    Response::builder()
        .status(400)
        .header(header::CONNECTION, "close")
        .header(header::CONTENT_TYPE, "text/html")
        .body(Full::new(Bytes::from_static(b"Bad Request")))
        .expect("static response parts are valid")
}

/// `ws`-compatible handshake on exactly `/ws`; a malformed handshake is a bare 400. Before the socket is upgraded the request must
/// pass the same checks as HTTP (loopback peer, local Host header) plus two that only matter for WebSockets: a browser `Origin`
/// must be a local one (pages on other sites can open sockets to 127.0.0.1) and the bridge token must be presented, either as
/// `x-bridge-token` / `Authorization: Bearer` or, for browsers, in `Sec-WebSocket-Protocol: waitplay.v1, waitplay.token.<token>`.
fn handle_ws_upgrade(state: Arc<BridgeState>, remote: SocketAddr, req: &mut Request<Incoming>) -> Resp {
    let Some(hub) = state.ws.clone() else { return bad_request() };
    let headers = req.headers();
    let upgrade_ok = header_str(headers, header::UPGRADE).is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
    let key = header_str(headers, header::SEC_WEBSOCKET_KEY).filter(|k| valid_ws_key(k)).map(str::to_string);
    let version_ok = matches!(header_str(headers, header::SEC_WEBSOCKET_VERSION), Some("8") | Some("13"));
    if req.method() != Method::GET || !upgrade_ok || !version_ok || req.uri().path() != "/ws" {
        return bad_request();
    }
    let Some(key) = key else { return bad_request() };

    match security::loopback_only(&remote, header_str(headers, header::HOST)) {
        LoopbackVerdict::Allowed => {}
        LoopbackVerdict::NonLoopback => return error_json(403, "Forbidden", "Vibe Play bridge only accepts connections from localhost/loopback."),
        LoopbackVerdict::BadHost(host) => return error_json(400, "Bad Request", &format!("Invalid host \"{}\". Requests must target localhost or 127.0.0.1.", host)),
    }
    if let Some(origin) = headers.get(header::ORIGIN) {
        if !origin.to_str().is_ok_and(security::origin_allowed) {
            return error_json(403, "Forbidden", "Origin not allowed by the Vibe Play bridge.");
        }
    }
    let offered = header_str(headers, header::SEC_WEBSOCKET_PROTOCOL).unwrap_or("");
    let presented = presented_token(headers).or_else(|| security::ws_subprotocol_token(offered));
    if !presented.is_some_and(|t| security::tokens_equal(&t, &state.token)) {
        return reject(&state, "WebSocket /ws");
    }
    if hub.connected_count() >= limits::MAX_WS_CLIENTS {
        return error_json(503, "Service Unavailable", "Too many WebSocket clients are connected to the Vibe Play bridge.");
    }
    let select_v1 = offered.split(',').any(|p| p.trim() == "waitplay.v1");

    let on_upgrade = hyper::upgrade::on(req);
    tokio::spawn(async move {
        if let Ok(upgraded) = on_upgrade.await {
            let socket = WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, Some(ws::websocket_config())).await;
            ws::run_connection(socket, remote, hub, state).await;
        }
    });

    let mut resp = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header(header::SEC_WEBSOCKET_ACCEPT, derive_accept_key(key.as_bytes()));
    if select_v1 {
        // a browser that offered sub-protocols insists on one being selected; never echo the one that carries the token
        resp = resp.header(header::SEC_WEBSOCKET_PROTOCOL, "waitplay.v1");
    }
    resp.body(Full::new(Bytes::new())).expect("static response parts are valid")
}

/// `Sec-WebSocket-Key`: 16 random bytes in base64 (22 chars + "==").
fn valid_ws_key(key: &str) -> bool {
    key.len() == 24 && key.ends_with("==") && key[..22].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
}
