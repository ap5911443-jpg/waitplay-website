//! The one-shot loopback listener that receives the OAuth redirect (RFC 8252, "loopback interface redirection").
//!
//! Security properties:
//! - bound to `127.0.0.1` only, random port, one flow per listener;
//! - the redirect path contains a random nonce (`/auth/callback/<nonce>`): a request without the right nonce is refused and does
//!   **not** end the flow, so a stray or hostile request cannot cancel or complete it;
//! - the `Host` header must be `127.0.0.1:<port>` (DNS-rebinding guard); only `GET`;
//! - the first valid callback ends the flow and the listener is dropped: a replay finds a closed port;
//! - the page the browser shows loads nothing from anywhere and sends no referrer.

use crate::error::{AuthError, Result};
use crate::pkce::{constant_time_eq, random_b64url, urldecode};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

const MAX_REQUEST: usize = 8 * 1024;
const MAX_REJECTED: usize = 25;

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Valid callback carrying the authorization code.
    Code(String),
    /// Valid callback (right nonce) in which the provider or Supabase reports an error, e.g. the user pressed Cancel.
    ProviderError(String),
    /// Not our callback; answer with this status and keep waiting.
    Reject(u16),
}

/// Judges the raw request head. Pure, so every hostile shape is unit-tested.
pub fn judge(head: &str, port: u16, nonce: &str) -> Verdict {
    let mut lines = head.split("\r\n");
    let Some(request_line) = lines.next() else { return Verdict::Reject(400) };
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Verdict::Reject(400);
    };
    if !version.starts_with("HTTP/1.") {
        return Verdict::Reject(400);
    }
    let host = lines.filter_map(|l| l.split_once(':')).find(|(k, _)| k.eq_ignore_ascii_case("host")).map(|(_, v)| v.trim().to_string());
    if host.as_deref() != Some(&format!("127.0.0.1:{port}")) {
        return Verdict::Reject(400);
    }
    if method != "GET" {
        return Verdict::Reject(405);
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let Some(got) = path.strip_prefix("/auth/callback/") else { return Verdict::Reject(404) };
    if !constant_time_eq(got, nonce) {
        return Verdict::Reject(400);
    }
    let mut code = None;
    let mut error = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "code" => code = urldecode(v),
            "error_description" => error = urldecode(v).or(error),
            "error" if error.is_none() => error = urldecode(v),
            _ => {}
        }
    }
    match (code, error) {
        (_, Some(e)) => Verdict::ProviderError(e),
        (Some(c), None) if !c.is_empty() && c.len() <= 2048 => Verdict::Code(c),
        _ => Verdict::Reject(400),
    }
}

pub struct CallbackServer {
    listener: TcpListener,
    port: u16,
    nonce: String,
}

impl CallbackServer {
    pub fn start() -> Result<CallbackServer> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| AuthError::Callback(format!("could not open the local sign-in listener: {e}")))?;
        listener.set_nonblocking(true).map_err(|e| AuthError::Callback(e.to_string()))?;
        let port = listener.local_addr().map_err(|e| AuthError::Callback(e.to_string()))?.port();
        Ok(CallbackServer { listener, port, nonce: random_b64url(16) })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The `redirect_to` value to register with the authorize request.
    pub fn redirect_url(&self) -> String {
        format!("http://127.0.0.1:{}/auth/callback/{}", self.port, self.nonce)
    }

    /// Waits for the valid callback (consumes the server: it is single use). `cancel` is polled so the UI can abort.
    pub fn wait(self, timeout: Duration, cancel: &dyn Fn() -> bool) -> Result<String> {
        let deadline = Instant::now() + timeout;
        let mut rejected = 0usize;
        loop {
            if cancel() {
                return Err(AuthError::Callback("Sign-in cancelled.".into()));
            }
            if Instant::now() >= deadline {
                return Err(AuthError::Callback("Sign-in timed out. Try again.".into()));
            }
            match self.listener.accept() {
                Ok((stream, _)) => match self.serve(stream) {
                    Some(Ok(code)) => return Ok(code),
                    Some(Err(e)) => return Err(e),
                    None => {
                        rejected += 1;
                        if rejected >= MAX_REJECTED {
                            return Err(AuthError::Callback("Sign-in was interrupted by unexpected requests. Try again.".into()));
                        }
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(25)),
                Err(_) => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    }

    /// `Some(result)` ends the flow, `None` means "not ours, keep waiting".
    fn serve(&self, mut stream: TcpStream) -> Option<Result<String>> {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let mut buf = Vec::with_capacity(1024);
        let mut chunk = [0u8; 1024];
        while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.len() > MAX_REQUEST {
                        respond(&mut stream, 431, "Request too large.");
                        return None;
                    }
                }
            }
        }
        let head = String::from_utf8_lossy(&buf).into_owned();
        match judge(&head, self.port, &self.nonce) {
            Verdict::Code(code) => {
                respond(&mut stream, 200, "You are signed in. You can close this tab and return to Vibe Play.");
                Some(Ok(code))
            }
            Verdict::ProviderError(e) => {
                respond(&mut stream, 200, "Sign-in was not completed. You can close this tab and return to Vibe Play.");
                Some(Err(AuthError::Callback(format!("Sign-in was not completed: {e}"))))
            }
            Verdict::Reject(status) => {
                respond(&mut stream, status, "Not found.");
                None
            }
        }
    }
}

fn respond(stream: &mut TcpStream, status: u16, message: &str) {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        _ => "Bad Request",
    };
    let body = format!(
        "<!doctype html><meta charset=utf-8><meta name=referrer content=no-referrer><title>Vibe Play</title>\
         <body style=\"font:16px system-ui,sans-serif;display:grid;place-items:center;height:90vh;color:#222\"><p>{message}</p></body>"
    );
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         Referrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\nX-Content-Type-Options: nosniff\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(path: &str, host: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: test\r\n\r\n")
    }

    #[test]
    fn a_valid_callback_yields_the_code() {
        assert_eq!(judge(&req("/auth/callback/N0nce?code=abc%2Fdef", "127.0.0.1:5000"), 5000, "N0nce"), Verdict::Code("abc/def".into()));
    }

    #[test]
    fn provider_errors_with_the_right_nonce_end_the_flow() {
        let v = judge(&req("/auth/callback/N0nce?error=access_denied&error_description=User+cancelled", "127.0.0.1:5000"), 5000, "N0nce");
        assert_eq!(v, Verdict::ProviderError("User cancelled".into()));
    }

    #[test]
    fn hostile_or_stray_requests_never_complete_the_flow() {
        let cases: Vec<(String, u16)> = vec![
            (req("/auth/callback/WRONG?code=abc", "127.0.0.1:5000"), 400),   // wrong nonce
            (req("/auth/callback/?code=abc", "127.0.0.1:5000"), 400),        // empty nonce
            (req("/auth/callback/N0nce", "127.0.0.1:5000"), 400),            // no code
            (req("/auth/callback/N0nce?code=", "127.0.0.1:5000"), 400),      // empty code
            (req("/auth/callback/N0nce?code=abc", "evil.example"), 400),     // rebinding host
            (req("/auth/callback/N0nce?code=abc", "127.0.0.1:5001"), 400),   // other port
            (req("/auth/callback/N0nce?code=abc", "localhost:5000"), 400),   // not the literal loopback address
            (req("/favicon.ico", "127.0.0.1:5000"), 404),
            (req("/", "127.0.0.1:5000"), 404),
            ("POST /auth/callback/N0nce?code=abc HTTP/1.1\r\nHost: 127.0.0.1:5000\r\n\r\n".into(), 405),
            ("GET /auth/callback/N0nce?code=abc HTTP/2\r\nHost: 127.0.0.1:5000\r\n\r\n".into(), 400),
            ("GET /auth/callback/N0nce?code=abc\r\nHost: 127.0.0.1:5000\r\n\r\n".into(), 400),
            ("garbage".into(), 400),
            ("".into(), 400),
        ];
        for (head, status) in cases {
            assert_eq!(judge(&head, 5000, "N0nce"), Verdict::Reject(status), "{head:?}");
        }
        let long = format!("/auth/callback/N0nce?code={}", "a".repeat(3000));
        assert_eq!(judge(&req(&long, "127.0.0.1:5000"), 5000, "N0nce"), Verdict::Reject(400), "absurdly long code");
    }

    fn get(port: u16, path: &str, host: Option<&str>) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let host = host.map(String::from).unwrap_or_else(|| format!("127.0.0.1:{port}"));
        write!(s, "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    #[test]
    fn the_listener_ignores_junk_then_accepts_the_real_callback_once() {
        let server = CallbackServer::start().unwrap();
        let (port, redirect) = (server.port(), server.redirect_url());
        assert!(redirect.starts_with(&format!("http://127.0.0.1:{port}/auth/callback/")));
        let nonce = redirect.rsplit('/').next().unwrap().to_string();
        let waiter = std::thread::spawn(move || server.wait(Duration::from_secs(10), &|| false));

        assert!(get(port, "/auth/callback/nope?code=stolen", None).starts_with("HTTP/1.1 400"));
        assert!(get(port, &format!("/auth/callback/{nonce}?code=stolen"), Some("evil.example")).starts_with("HTTP/1.1 400"));
        assert!(get(port, "/favicon.ico", None).starts_with("HTTP/1.1 404"));
        let ok = get(port, &format!("/auth/callback/{nonce}?code=the-code"), None);
        assert!(ok.starts_with("HTTP/1.1 200") && ok.contains("Referrer-Policy: no-referrer") && !ok.contains("the-code"), "{ok}");

        assert_eq!(waiter.join().unwrap().unwrap(), "the-code");
        // replay: the listener is gone with the flow
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err(), "a second callback must find a closed port");
    }

    #[test]
    fn a_cancelled_or_expired_flow_ends_without_a_code() {
        let server = CallbackServer::start().unwrap();
        let err = server.wait(Duration::from_secs(5), &|| true).unwrap_err();
        assert!(matches!(err, AuthError::Callback(m) if m.contains("cancelled")));
        let server = CallbackServer::start().unwrap();
        let err = server.wait(Duration::from_millis(100), &|| false).unwrap_err();
        assert!(matches!(err, AuthError::Callback(m) if m.contains("timed out")));
    }

    #[test]
    fn a_flood_of_bad_requests_ends_the_flow_instead_of_spinning_forever() {
        let server = CallbackServer::start().unwrap();
        let port = server.port();
        let waiter = std::thread::spawn(move || server.wait(Duration::from_secs(20), &|| false));
        for _ in 0..MAX_REJECTED + 2 {
            let _ = TcpStream::connect(("127.0.0.1", port)).map(|mut s| { let _ = write!(s, "GET /x HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"); let mut o = String::new(); let _ = s.read_to_string(&mut o); });
        }
        assert!(matches!(waiter.join().unwrap(), Err(AuthError::Callback(m)) if m.contains("unexpected requests")));
    }
}
