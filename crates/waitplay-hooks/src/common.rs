//! Pieces every subcommand shares: JavaScript-flavoured value helpers (the old scripts leaned on JS truthiness and
//! UTF-16 string slicing, and the payloads must stay byte-compatible), the event payload builder, and launching the app.

use serde_json::{Map, Value};
use std::path::PathBuf;
use std::time::Duration;
use waitplay_core::{paths, process, protocol::now_ms};

/// JavaScript truthiness of a JSON value (`value || fallback`).
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map_or(true, |f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `v` when it is truthy, else the fallback (`extra?.model || 'Claude 3.7 Sonnet'`).
pub fn or_default(v: Option<&Value>, fallback: &str) -> Value {
    match v {
        Some(v) if truthy(v) => v.clone(),
        _ => Value::String(fallback.to_string()),
    }
}

/// `String(value)` as JavaScript renders it (template literals, `+=`).
pub fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(|x| if x.is_null() { String::new() } else { js_string(x) }).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// Length in UTF-16 code units, which is what JavaScript's `string.length` counts.
pub fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `s.slice(0, max)` with JavaScript's UTF-16 indexing. Fast path: ASCII or short strings are returned untouched.
pub fn js_slice(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let units: Vec<u16> = s.encode_utf16().take(max).collect();
    String::from_utf16_lossy(&units)
}

/// Prompt shaping used by Claude Code: first 100 characters.
pub fn prompt_100(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => js_slice(s, 100),
        _ => String::new(),
    }
}

pub fn obj(pairs: Vec<(&str, Value)>) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

/// The JSON body POSTed to `/events`. `None` fields are omitted, like `undefined` in `JSON.stringify`.
pub fn event_payload(
    agent: &str,
    session_id: &str,
    event: &str,
    model: Option<Value>,
    terminal: Option<&str>,
    data: Option<Map<String, Value>>,
) -> Value {
    let mut m = Map::new();
    m.insert("agent".into(), Value::String(agent.to_string()));
    m.insert("sessionId".into(), Value::String(session_id.to_string()));
    m.insert("event".into(), Value::String(event.to_string()));
    m.insert("timestamp".into(), Value::from(now_ms()));
    if let Some(model) = model {
        m.insert("model".into(), model);
    }
    if let Some(t) = terminal {
        m.insert("terminal".into(), Value::String(t.to_string()));
    }
    if let Some(d) = data {
        m.insert("data".into(), Value::Object(d));
    }
    Value::Object(m)
}

/// POST one event to the bridge.
///
/// Windows takes about two seconds to report a refused loopback connection, which would stall every hook while the bridge
/// is not running. A listening bridge accepts in well under a millisecond, so a short probe first lets a missing bridge fail
/// fast; the request itself keeps the 1 s limit of `BridgeClient`.
pub fn post_event(payload: &Value) -> Result<(), String> {
    let port = paths::bridge_port();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(150)).map_err(|e| format!("bridge not reachable: {}", e))?;
    waitplay_core::BridgeClient::with_port(port).post_event_json(payload)
}

/// `post_event`, swallowing every failure: a hook must never hurt the agent.
pub fn post_event_quiet(payload: &Value) {
    let _ = post_event(payload);
}

/// What to tell the companion app when launching it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PopRequest {
    pub session: String,
    pub terminal: Option<String>,
    pub prompt_time: Option<i64>,
    /// Process whose ancestry the app walks to find the agent's terminal window.
    pub parent_pid: u32,
}

impl PopRequest {
    pub fn args(&self) -> Vec<String> {
        let mut a = vec![format!("--session={}", self.session)];
        if let Some(t) = &self.terminal {
            a.push(format!("--terminal={}", t));
        }
        a.push(format!("--parentPid={}", self.parent_pid));
        if let Some(p) = self.prompt_time.filter(|p| *p > 0) {
            a.push(format!("--promptTime={}", p));
        }
        a
    }
}

/// Launches the Vibe Play app detached. Absent binary or spawn failure: nothing happens.
pub fn spawn_app(req: &PopRequest) {
    if let Some(exe) = paths::app_exe() {
        let _ = process::spawn_detached(&exe, &req.args());
    }
}

/// Launches the app with raw arguments (the simulators pass flags the hooks never use).
pub fn spawn_app_raw(args: &[String]) {
    if let Some(exe) = paths::app_exe() {
        let _ = process::spawn_detached(&exe, args);
    }
}

/// The process the agent started this hook from. The hook itself is gone before the app starts walking the process tree,
/// so the parent is the closest still-living stand-in for Node's `process.pid`.
pub fn parent_pid_or_self() -> u32 {
    parent_pid().unwrap_or_else(std::process::id)
}

#[cfg(unix)]
fn parent_pid() -> Option<u32> {
    Some(std::os::unix::process::parent_id())
}

#[cfg(windows)]
fn parent_pid() -> Option<u32> {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct ProcessEntry32W {
        dwSize: u32,
        cntUsage: u32,
        th32ProcessID: u32,
        th32DefaultHeapID: usize,
        th32ModuleID: u32,
        cntThreads: u32,
        th32ParentProcessID: u32,
        pcPriClassBase: i32,
        dwFlags: u32,
        szExeFile: [u16; 260],
    }
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }
    const TH32CS_SNAPPROCESS: u32 = 2;
    let me = std::process::id();
    // SAFETY: plain Win32 toolhelp calls on a zero-initialised, correctly sized entry; the snapshot handle is closed.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == -1 {
            return None;
        }
        let mut entry: ProcessEntry32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<ProcessEntry32W>() as u32;
        let mut found = None;
        let mut ok = Process32FirstW(snap, &mut entry);
        while ok != 0 {
            if entry.th32ProcessID == me {
                found = Some(entry.th32ParentProcessID).filter(|p| *p != 0);
                break;
            }
            ok = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(not(any(unix, windows)))]
fn parent_pid() -> Option<u32> {
    None
}

/// Old hook protocol for "was this prompt dismissed": the dismissal file maps the session to a number and the prompt is
/// dismissed when that number is at least the prompt's timestamp. The app now writes objects there, which JavaScript
/// compared as NaN (never dismissed); only a plain number ever matched, and so it does here.
pub fn legacy_dismissed_since(session_id: &str, prompt_ts: i64) -> bool {
    let Ok(text) = std::fs::read_to_string(paths::signal_dir().join(".waitplay-dismissed.json")) else { return false };
    let Ok(v) = serde_json::from_str::<Value>(&text) else { return false };
    legacy_dismissed_value(v.get(session_id), prompt_ts)
}

pub fn legacy_dismissed_value(entry: Option<&Value>, prompt_ts: i64) -> bool {
    match entry {
        Some(Value::Number(n)) => prompt_ts > 0 && n.as_f64().map_or(false, |f| f >= prompt_ts as f64),
        _ => false,
    }
}

/// `~`, as `os.homedir()` reports it.
pub fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("HOME").filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// An override path from the environment, else the default under the home directory.
pub fn env_path_or(var: &str, default: impl FnOnce() -> PathBuf) -> PathBuf {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v.trim()),
        _ => default(),
    }
}

/// Reads `s` as a JavaScript `parseInt(s, 10)` would, `None` for NaN.
pub fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let (neg, rest) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let n: i64 = digits.parse().ok()?;
    Some(if neg { -n } else { n })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn truthiness() {
        assert!(!truthy(&json!(null)));
        assert!(!truthy(&json!("")));
        assert!(!truthy(&json!(0)));
        assert!(truthy(&json!("x")));
        assert!(truthy(&json!({})));
        assert_eq!(or_default(Some(&json!("gpt")), "d"), json!("gpt"));
        assert_eq!(or_default(Some(&json!("")), "d"), json!("d"));
        assert_eq!(or_default(None, "d"), json!("d"));
    }

    #[test]
    fn utf16_slicing_matches_javascript() {
        assert_eq!(js_slice("hello", 100), "hello");
        assert_eq!(js_slice(&"a".repeat(150), 100).len(), 100);
        // U+1F600 is two UTF-16 units: slice(0, 3) of "ab😀" keeps "ab" and half the pair (lossy here)
        assert_eq!(js_len("ab\u{1F600}"), 4);
        assert_eq!(js_slice("ab\u{1F600}", 4), "ab\u{1F600}");
        assert_eq!(js_slice("\u{1F600}\u{1F600}", 2), "\u{1F600}");
    }

    #[test]
    fn js_strings() {
        assert_eq!(js_string(&json!(5)), "5");
        assert_eq!(js_string(&json!(null)), "null");
        assert_eq!(js_string(&json!({"a":1})), "[object Object]");
        assert_eq!(js_string(&json!(["a", null, 2])), "a,,2");
    }

    #[test]
    fn payload_omits_missing_fields() {
        let p = event_payload("claude-code", "claude-code-session", "session_ended", Some(json!("m")), None, None);
        assert_eq!(p["agent"], "claude-code");
        assert_eq!(p["sessionId"], "claude-code-session");
        assert_eq!(p["model"], "m");
        assert!(p.get("data").is_none());
        assert!(p.get("terminal").is_none());
        assert!(p["timestamp"].as_i64().unwrap() > 1_700_000_000_000);
    }

    #[test]
    fn pop_args() {
        let r = PopRequest { session: "omp-session".into(), terminal: Some("Term - 1".into()), prompt_time: Some(5), parent_pid: 42 };
        assert_eq!(r.args(), vec!["--session=omp-session", "--terminal=Term - 1", "--parentPid=42", "--promptTime=5"]);
        let r = PopRequest { session: "s".into(), terminal: None, prompt_time: None, parent_pid: 1 };
        assert_eq!(r.args(), vec!["--session=s", "--parentPid=1"]);
    }

    #[test]
    fn legacy_dismissal_only_honours_numbers() {
        assert!(legacy_dismissed_value(Some(&json!(2000)), 1000));
        assert!(!legacy_dismissed_value(Some(&json!(500)), 1000));
        assert!(!legacy_dismissed_value(Some(&json!({"dismissed": true, "dismissedAt": 9999999999999i64})), 1000));
        assert!(!legacy_dismissed_value(None, 1000));
        assert!(!legacy_dismissed_value(Some(&json!(2000)), 0));
    }

    #[test]
    fn parse_int_like_js() {
        assert_eq!(parse_int("3999"), Some(3999));
        assert_eq!(parse_int(" 12abc"), Some(12));
        assert_eq!(parse_int("abc"), None);
    }

    #[test]
    fn parent_pid_resolves() {
        assert!(parent_pid_or_self() > 0);
    }
}
