#![allow(dead_code)]
//! Shared helpers for the integration tests: environment isolation, a raw HTTP/1.1 client (so tests control the exact
//! bytes, e.g. Host headers) and a bridge bound to an ephemeral port.

use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Once;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use waitplay_bridge::bridge::{self, Bridge};

static ISOLATE: Once = Once::new();

/// Points TEMP and the Vibe Play checkout at a throwaway directory so tests never touch a live Vibe Play's files.
pub fn isolate_env() {
    ISOLATE.call_once(|| {
        let dir = std::env::temp_dir().join(format!("waitplay-bridge-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        for key in ["TEMP", "TMP", "TMPDIR", "WAITPLAY_HOME"] {
            std::env::set_var(key, &dir);
        }
        std::env::remove_var("BRIDGE_AUTH_TOKEN");
    });
}

pub async fn start_bridge(token: &str) -> Bridge {
    isolate_env();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    bridge::start(listener, token.to_string(), None)
}

/// A bridge with explicit limits (long windows, so tests do not depend on how fast the machine is).
pub async fn start_bridge_with_limits(token: &str, limits: waitplay_bridge::limits::Limits) -> Bridge {
    isolate_env();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    bridge::start_with_limits(listener, token.to_string(), None, limits)
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| panic!("body is not JSON ({e}): {}", String::from_utf8_lossy(&self.body)))
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub fn parse_response(raw: &[u8]) -> HttpResponse {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("response has a header block");
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.lines();
    let status = lines.next().unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
    let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
    HttpResponse { status, headers, body: raw[split + 4..].to_vec() }
}

pub fn build_request(method: &str, path: &str, host: &str, headers: &[(&str, &str)], body: Option<&[u8]>) -> Vec<u8> {
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    if let Some(b) = body {
        req.push_str(&format!("Content-Length: {}\r\n", b.len()));
    }
    req.push_str("\r\n");
    let mut bytes = req.into_bytes();
    if let Some(b) = body {
        bytes.extend_from_slice(b);
    }
    bytes
}

/// One request on a fresh connection; the Host header defaults to the server address.
pub async fn request(addr: SocketAddr, method: &str, path: &str, headers: &[(&str, &str)], body: Option<&[u8]>) -> HttpResponse {
    request_with_host(addr, method, path, &addr.to_string(), headers, body).await
}

pub async fn request_with_host(addr: SocketAddr, method: &str, path: &str, host: &str, headers: &[(&str, &str)], body: Option<&[u8]>) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    // The server may answer and close before an oversized body is fully written; the response is still readable.
    let _ = stream.write_all(&build_request(method, path, host, headers, body)).await;
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw).await;
    parse_response(&raw)
}

pub async fn get(addr: SocketAddr, path: &str) -> HttpResponse {
    request(addr, "GET", path, &[], None).await
}

pub async fn post_json(addr: SocketAddr, path: &str, headers: &[(&str, &str)], body: &Value) -> HttpResponse {
    let mut all = vec![("Content-Type", "application/json")];
    all.extend_from_slice(headers);
    request(addr, "POST", path, &all, Some(body.to_string().as_bytes())).await
}

/// The token `start_bridge("tok")` runs with.
pub const TOKEN: (&str, &str) = ("x-bridge-token", "tok");

/// GET with the bridge token (every route except /health needs it).
pub async fn get_auth(addr: SocketAddr, path: &str) -> HttpResponse {
    request(addr, "GET", path, &[TOKEN], None).await
}

/// POST JSON with the bridge token.
pub async fn post_auth(addr: SocketAddr, path: &str, body: &Value) -> HttpResponse {
    post_json(addr, path, &[TOKEN], body).await
}

pub fn event(session: &str, event: &str) -> Value {
    serde_json::json!({
        "agent": "claude-code",
        "sessionId": session,
        "event": event,
        "timestamp": waitplay_core::now_ms(),
    })
}
