//! Optional SMS on task completion through Twilio (port of phoneNotifier.ts). Entirely env-driven; failures are
//! logged by the caller and never affect event handling.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneNotifierConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub from: String,
    pub to: String,
}

pub struct CompletedTaskNotification {
    pub agent: String,
    pub session_id: String,
    pub duration_ms: Option<i64>,
}

/// Where the Twilio API lives. Production is `api.twilio.com:443` over TLS; tests point it at a plain local socket.
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl Endpoint {
    pub fn twilio() -> Endpoint {
        Endpoint { host: "api.twilio.com".into(), port: 443, tls: true }
    }
}

/// All four values must be set and non-empty.
pub fn get_config() -> Option<PhoneNotifierConfig> {
    let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    Some(PhoneNotifierConfig {
        account_sid: get("TWILIO_ACCOUNT_SID")?,
        auth_token: get("TWILIO_AUTH_TOKEN")?,
        from: get("TWILIO_FROM_NUMBER")?,
        to: get("WAITPLAY_NOTIFICATION_PHONE")?,
    })
}

pub fn is_phone_notification_configured() -> bool {
    get_config().is_some()
}

/// `Vibe Play: <agent> finished work for session <id>. Duration: <n>s.`
pub fn completion_message(task: &CompletedTaskNotification) -> String {
    let duration = match task.duration_ms {
        None => String::new(),
        Some(ms) => format!(" Duration: {}s.", js_round(ms as f64 / 1000.0)),
    };
    format!("Vibe Play: {} finished work for session {}.{}", task.agent, task.session_id, duration)
}

/// `Math.round`: halves round toward +infinity.
fn js_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// No-op when SMS is not configured.
pub async fn notify_task_completed(task: &CompletedTaskNotification) -> Result<(), String> {
    let Some(config) = get_config() else { return Ok(()) };
    send_sms(&Endpoint::twilio(), &config, &completion_message(task)).await
}

fn form_urlencode(pairs: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        push_encoded(&mut out, k);
        out.push('=');
        push_encoded(&mut out, v);
    }
    out
}

/// `application/x-www-form-urlencoded` encoding as URLSearchParams does it (space becomes '+').
fn push_encoded(out: &mut String, s: &str) {
    for b in s.bytes() {
        match b {
            b'*' | b'-' | b'.' | b'_' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
}

/// `encodeURIComponent`.
fn encode_uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn base64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn build_request(endpoint: &Endpoint, config: &PhoneNotifierConfig, body: &str) -> Vec<u8> {
    let form = form_urlencode(&[("To", &config.to), ("From", &config.from), ("Body", body)]);
    let auth = base64(format!("{}:{}", config.account_sid, config.auth_token).as_bytes());
    format!(
        "POST /2010-04-01/Accounts/{}/Messages.json HTTP/1.1\r\nHost: {}\r\nAuthorization: Basic {}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        encode_uri_component(&config.account_sid),
        endpoint.host,
        auth,
        form.len(),
        form
    )
    .into_bytes()
}

/// POSTs the SMS; the whole exchange (connect, TLS, request, response) is bounded by 10 seconds.
pub async fn send_sms(endpoint: &Endpoint, config: &PhoneNotifierConfig, body: &str) -> Result<(), String> {
    match tokio::time::timeout(REQUEST_TIMEOUT, exchange(endpoint, config, body)).await {
        Ok(r) => r,
        Err(_) => Err("Twilio request timed out".to_string()),
    }
}

async fn exchange(endpoint: &Endpoint, config: &PhoneNotifierConfig, body: &str) -> Result<(), String> {
    let request = build_request(endpoint, config, body);
    let tcp = TcpStream::connect((endpoint.host.as_str(), endpoint.port)).await.map_err(|e| e.to_string())?;
    let response = if endpoint.tls {
        let mut roots = tokio_rustls::rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let provider = Arc::new(tokio_rustls::rustls::crypto::ring::default_provider());
        let tls_config = tokio_rustls::rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| e.to_string())?
            .with_root_certificates(roots)
            .with_no_client_auth();
        let name = tokio_rustls::rustls::pki_types::ServerName::try_from(endpoint.host.clone()).map_err(|e| e.to_string())?;
        let stream = tokio_rustls::TlsConnector::from(Arc::new(tls_config)).connect(name, tcp).await.map_err(|e| e.to_string())?;
        roundtrip(stream, &request).await?
    } else {
        roundtrip(tcp, &request).await?
    };
    let (status, text) = parse_response(&response);
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(format!("Twilio returned HTTP {}: {}", status, text.chars().take(300).collect::<String>()))
    }
}

async fn roundtrip<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, request: &[u8]) -> Result<Vec<u8>, String> {
    stream.write_all(request).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    // A peer that closes TLS without close_notify surfaces as an error after the full body arrived; keep what we have.
    let _ = stream.read_to_end(&mut buf).await;
    if buf.is_empty() {
        return Err("empty response from Twilio".to_string());
    }
    Ok(buf)
}

/// Status code and body text of a `Connection: close` HTTP/1.1 response (de-chunked when needed).
fn parse_response(raw: &[u8]) -> (u16, String) {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n");
    let (head, body) = match split {
        Some(i) => (&raw[..i], &raw[i + 4..]),
        None => (raw, &raw[raw.len()..]),
    };
    let head = String::from_utf8_lossy(head);
    let status = head.lines().next().and_then(|l| l.split_whitespace().nth(1)).and_then(|s| s.parse().ok()).unwrap_or(0);
    let chunked = head.lines().skip(1).any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    });
    let body = if chunked { dechunk(body) } else { body.to_vec() };
    (status, String::from_utf8_lossy(&body).into_owned())
}

fn dechunk(mut data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let Some(eol) = data.windows(2).position(|w| w == b"\r\n") else { break };
        let size_text = String::from_utf8_lossy(&data[..eol]);
        let Ok(size) = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16) else { break };
        data = &data[eol + 2..];
        if size == 0 || data.len() < size {
            out.extend_from_slice(&data[..data.len().min(size)]);
            break;
        }
        out.extend_from_slice(&data[..size]);
        data = data.get(size + 2..).unwrap_or(&[]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    fn config() -> PhoneNotifierConfig {
        PhoneNotifierConfig { account_sid: "AC123".into(), auth_token: "secret".into(), from: "+15550001".into(), to: "+15550002".into() }
    }

    #[test]
    fn message_text_matches_typescript() {
        let t = |d| CompletedTaskNotification { agent: "codex".into(), session_id: "s-1".into(), duration_ms: d };
        assert_eq!(completion_message(&t(Some(12_499))), "Vibe Play: codex finished work for session s-1. Duration: 12s.");
        assert_eq!(completion_message(&t(Some(12_500))), "Vibe Play: codex finished work for session s-1. Duration: 13s.");
        assert_eq!(completion_message(&t(None)), "Vibe Play: codex finished work for session s-1.");
    }

    #[test]
    fn form_encoding_and_basic_auth() {
        assert_eq!(form_urlencode(&[("To", "+1 5"), ("Body", "a&b=c")]), "To=%2B1+5&Body=a%26b%3Dc");
        assert_eq!(base64(b"AC123:secret"), "QUMxMjM6c2VjcmV0");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(encode_uri_component("A C/1"), "A%20C%2F1");
    }

    async fn fake_twilio(reply: String) -> (Endpoint, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let mut got = Vec::new();
            loop {
                let n = s.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some(i) = text.find("\r\n\r\n") {
                    let cl: usize = text.lines().find_map(|l| l.strip_prefix("Content-Length: ")).unwrap().trim().parse().unwrap();
                    if got.len() >= i + 4 + cl {
                        break;
                    }
                }
            }
            s.write_all(reply.as_bytes()).await.unwrap();
            String::from_utf8(got).unwrap()
        });
        (Endpoint { host: "127.0.0.1".into(), port, tls: false }, h)
    }

    #[tokio::test]
    async fn posts_form_to_messages_endpoint() {
        let (endpoint, server) = fake_twilio("HTTP/1.1 201 Created\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_string()).await;
        send_sms(&endpoint, &config(), "hello there").await.unwrap();
        let req = server.await.unwrap();
        assert!(req.starts_with("POST /2010-04-01/Accounts/AC123/Messages.json HTTP/1.1\r\n"));
        assert!(req.contains("Authorization: Basic QUMxMjM6c2VjcmV0\r\n"));
        assert!(req.contains("Content-Type: application/x-www-form-urlencoded\r\n"));
        assert!(req.ends_with("To=%2B15550002&From=%2B15550001&Body=hello+there"));
    }

    #[tokio::test]
    async fn non_2xx_is_an_error_with_truncated_body() {
        let body = "x".repeat(400);
        let reply = format!("HTTP/1.1 401 Unauthorized\r\nContent-Length: 400\r\nConnection: close\r\n\r\n{body}");
        let (endpoint, server) = fake_twilio(reply).await;
        let err = send_sms(&endpoint, &config(), "m").await.unwrap_err();
        server.await.unwrap();
        assert_eq!(err, format!("Twilio returned HTTP 401: {}", "x".repeat(300)));
    }

    #[tokio::test]
    async fn chunked_error_bodies_are_decoded() {
        let (endpoint, server) =
            fake_twilio("HTTP/1.1 500 Oops\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nhello\r\n0\r\n\r\n".to_string()).await;
        let err = send_sms(&endpoint, &config(), "m").await.unwrap_err();
        server.await.unwrap();
        assert_eq!(err, "Twilio returned HTTP 500: hello");
    }

    #[tokio::test(start_paused = true)]
    async fn silent_server_times_out_after_ten_seconds() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let _hold = tokio::spawn(async move {
            let (_s, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        let endpoint = Endpoint { host: "127.0.0.1".into(), port, tls: false };
        let err = send_sms(&endpoint, &config(), "m").await.unwrap_err();
        assert_eq!(err, "Twilio request timed out");
    }
}
