//! Hand-off files between hooks, the widget and companion windows. Formats are unchanged from the C# app:
//!
//! * `.waitplay-launch-<id>.txt`    `sessionId|terminal|unixMs|hwnd`   written by a later Vibe Play launch for the running companion
//! * `.waitplay-summon-<site>.txt`  same line, for non-default site companions
//! * `.waitplay-close-<id>.json`    `{"sessionId":..,"ts":..}`         written by agent hooks at turn end
//! * `.waitplay-quit.txt`           unix ms                            "Quit companion" / Vibe Play off
//! * `.waitplay-dismissed.json`     `{id:{dismissed,dismissedAt}}`     the user closed the window for this prompt

use crate::paths;
use crate::protocol::now_ms;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;

/// File-name-safe form of a session id: letters, digits, `-` and `_` stay, everything else becomes `_`.
pub fn signal_file_id(session_id: &str) -> String {
    let s: String = session_id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if s.is_empty() {
        "_summon".to_string()
    } else {
        s
    }
}

pub fn close_file_path(session_id: &str) -> PathBuf {
    paths::signal_dir().join(format!(".waitplay-close-{}.json", signal_file_id(session_id)))
}

/// Tells the companion the agent's turn is over. Works without the bridge.
pub fn write_close_signal(session_id: &str) {
    let body = json!({ "sessionId": session_id, "ts": now_ms() }).to_string();
    let _ = paths::write_atomic(&close_file_path(session_id), body.as_bytes());
}

pub fn quit_file_path() -> PathBuf {
    paths::signal_dir().join(".waitplay-quit.txt")
}

pub fn write_quit_signal() {
    let _ = paths::write_atomic(&quit_file_path(), now_ms().to_string().as_bytes());
}

/// A request for the running companion of `site` to show itself.
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchSignal {
    pub session_id: String,
    pub terminal: String,
    pub ts: i64,
    pub hwnd: i64,
}

impl LaunchSignal {
    pub fn path(site_key: &str, session_id: &str) -> PathBuf {
        let name = if site_key == "instagram" {
            format!(".waitplay-launch-{}.txt", signal_file_id(session_id))
        } else {
            format!(".waitplay-summon-{}.txt", site_key)
        };
        paths::signal_dir().join(name)
    }

    pub fn write(&self, site_key: &str) {
        let line = format!("{}|{}|{}|{}", self.session_id, self.terminal, self.ts, self.hwnd);
        let _ = paths::write_atomic(&Self::path(site_key, &self.session_id), line.as_bytes());
    }

    /// `None` while the file is half written (the caller retries next tick) or malformed.
    pub fn parse(text: &str) -> Option<LaunchSignal> {
        let parts: Vec<&str> = text.trim().split('|').collect();
        if parts.len() < 4 {
            return None;
        }
        Some(LaunchSignal {
            session_id: parts[0].to_string(),
            terminal: parts[1].to_string(),
            ts: parts[2].parse().ok()?,
            hwnd: parts[3].parse().ok()?,
        })
    }
}

fn dismissed_path() -> PathBuf {
    paths::signal_dir().join(".waitplay-dismissed.json")
}

const DISMISS_TTL_MS: i64 = 300_000;

fn read_dismissals() -> Map<String, Value> {
    fs::read_to_string(dismissed_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        })
        .unwrap_or_default()
}

fn write_dismissals(m: &Map<String, Value>) {
    let _ = paths::write_atomic(&dismissed_path(), Value::Object(m.clone()).to_string().as_bytes());
}

/// True while the user's "not for this prompt" is fresh (5 minutes); an expired entry is cleared.
pub fn is_prompt_dismissed(session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    let m = read_dismissals();
    let Some(entry) = m.get(session_id) else { return false };
    if entry.get("dismissed").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    let at = entry.get("dismissedAt").and_then(Value::as_i64).unwrap_or(0);
    if at > 0 && now_ms() - at > DISMISS_TTL_MS {
        clear_dismissal(session_id);
        return false;
    }
    true
}

pub fn record_dismissal(session_id: &str) {
    if session_id.is_empty() {
        return;
    }
    let mut m = read_dismissals();
    m.insert(session_id.to_string(), json!({ "dismissed": true, "dismissedAt": now_ms() }));
    write_dismissals(&m);
}

pub fn clear_dismissal(session_id: &str) {
    if session_id.is_empty() {
        return;
    }
    let mut m = read_dismissals();
    if m.remove(session_id).is_some() {
        write_dismissals(&m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(signal_file_id("claude-code-session"), "claude-code-session");
        assert_eq!(signal_file_id("a b/c:d"), "a_b_c_d");
        assert_eq!(signal_file_id(""), "_summon");
    }

    #[test]
    fn launch_line() {
        let s = LaunchSignal::parse("omp-session|Windows Terminal|1700000000000|394822\n").unwrap();
        assert_eq!(s.terminal, "Windows Terminal");
        assert_eq!(s.hwnd, 394822);
        assert!(LaunchSignal::parse("omp-session|Windows Terminal|17000").is_none());
        assert!(LaunchSignal::parse("a|b|notanumber|1").is_none());
    }
}
