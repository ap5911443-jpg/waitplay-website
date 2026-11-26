//! `waitplay-hook claude-code <start|tool|stop|notify|end>`: the Claude Code lifecycle hook (port of
//! integrations/claude-code/waitplay-hook.cjs, the one `~/.claude/settings.json` points at; waitplay-hook.js is a generic
//! reference dispatcher that nothing calls). Claude Code pipes the hook JSON to stdin.

use crate::common::{self, obj, or_default, PopRequest};
use serde_json::{Map, Value};
use std::io::Read;
use std::sync::mpsc;
use std::time::Duration;
use waitplay_core::{protocol::now_ms, signals, state};

pub const AGENT: &str = "claude-code";
pub const SESSION: &str = "claude-code-session";
const DEFAULT_MODEL: &str = "Claude 3.7 Sonnet";

/// What one hook invocation must do. Pure result of the hook name and the stdin JSON.
#[derive(Debug, PartialEq)]
pub enum Action {
    Start { model: Value, prompt: String, transcript: String },
    Tool { tool: Value, model: Value },
    /// A tool finished (also: the user approved it): the agent is working again.
    ToolDone { tool: Value, model: Value },
    Stop { model: Value },
    WaitingForInput { prompt: String },
    End,
    Ignore,
}

/// `event_type` is the CLI argument (`start`, `tool`, ...); `data` is the stdin JSON (an empty object when absent).
pub fn classify(event_type: &str, data: &Value) -> Action {
    let field = |k: &str| data.get(k);
    // `data.hook_event_name || eventType`
    let hook_name: &str = match field("hook_event_name") {
        Some(Value::String(s)) if !s.is_empty() => s,
        Some(v) if common::truthy(v) => "",
        _ => event_type,
    };
    let model = || or_default(field("model"), DEFAULT_MODEL);

    if hook_name == "UserPromptSubmit" || hook_name == "user_prompt_submit" || event_type == "start" {
        let transcript = field("transcript_path").and_then(Value::as_str).unwrap_or("").to_string();
        Action::Start { model: model(), prompt: common::prompt_100(field("prompt")), transcript }
    } else if hook_name == "PreToolUse" || hook_name == "pre_tool_use" || event_type == "tool" {
        let tool = match field("tool_name") {
            Some(v) if common::truthy(v) => v.clone(),
            _ => or_default(field("tool"), "Tool"),
        };
        Action::Tool { tool, model: model() }
    } else if hook_name == "PostToolUse" || hook_name == "post_tool_use" || event_type == "tool_done" {
        let tool = match field("tool_name") {
            Some(v) if common::truthy(v) => v.clone(),
            _ => or_default(field("tool"), "Tool"),
        };
        Action::ToolDone { tool, model: model() }
    } else if hook_name == "Stop" || hook_name == "stop" || event_type == "stop" {
        Action::Stop { model: model() }
    } else if hook_name == "Notification" || event_type == "notify" {
        let msg = match field("message") {
            Some(Value::String(s)) => s.as_str(),
            _ => "",
        };
        // Claude Code says what kind of notification it is: a permission prompt or a question for the user need the user;
        // `idle_prompt` ("waiting for your input") comes after the turn is already over and must not look like an approval.
        let kind = field("notification_type").and_then(Value::as_str).unwrap_or("");
        let lower = msg.to_lowercase();
        let needs_user = match kind {
            "permission_prompt" | "elicitation_dialog" => true,
            "" => lower.contains("permission") || lower.contains("approve") || lower.contains("allow"),
            _ => false,
        };
        if needs_user {
            Action::WaitingForInput { prompt: msg.to_string() }
        } else {
            Action::Ignore
        }
    } else if hook_name == "SessionEnd" || event_type == "end" {
        Action::End
    } else {
        Action::Ignore
    }
}

fn post(session: &str, event: &str, model: Value, data: Option<Map<String, Value>>) {
    common::post_event_quiet(&common::event_payload(AGENT, session, event, Some(model), None, data));
}

/// One session per Claude Code conversation (the hook JSON carries its `session_id`), so two terminals running Claude Code are
/// tracked independently: one finishing never closes the other's window. Without an id the old shared session is used.
pub fn session_for(data: &Value) -> String {
    let id: String = data
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(12)
        .collect();
    if id.is_empty() {
        SESSION.to_string()
    } else {
        format!("claude-code-{}", id)
    }
}

pub fn execute(action: Action, session: &str) {
    match action {
        Action::Start { model, prompt, transcript } => {
            let prompt_time = now_ms();
            signals::clear_dismissal(session);
            start_heartbeat(session, prompt_time, &transcript);
            // The companion window takes a moment to appear; start it before the (possibly slow) bridge call.
            if state::is_enabled() && !signals::is_prompt_dismissed(session) {
                common::spawn_app(&PopRequest {
                    session: session.into(),
                    terminal: None,
                    prompt_time: Some(prompt_time),
                    parent_pid: common::parent_pid_or_self(),
                });
            }
            post(
                session,
                "task_started",
                model.clone(),
                Some(obj(vec![("model", model), ("prompt", Value::String(prompt)), ("promptTime", Value::from(prompt_time))])),
            );
        }
        Action::Tool { tool, model } | Action::ToolDone { tool, model } => post(session, "task_running", model.clone(), Some(obj(vec![("tool", tool), ("model", model)]))),
        Action::Stop { model } => {
            stop_heartbeat(session);
            signals::clear_dismissal(session);
            post(session, "task_completed", model.clone(), Some(obj(vec![("model", model)])));
            signals::write_close_signal(session);
        }
        Action::WaitingForInput { prompt } => {
            post(session, "waiting_for_input", Value::String(DEFAULT_MODEL.into()), Some(obj(vec![("prompt", Value::String(prompt))])))
        }
        Action::End => {
            stop_heartbeat(session);
            signals::clear_dismissal(session);
            post(session, "session_ended", Value::String(DEFAULT_MODEL.into()), None);
        }
        Action::Ignore => {}
    }
}

/// Parses stdin text: an empty or malformed body is an empty object, anything that is not an object reads as empty too.
pub fn parse_stdin(input: &str) -> Value {
    if input.trim().is_empty() {
        return Value::Object(Map::new());
    }
    match serde_json::from_str::<Value>(input) {
        Ok(v @ Value::Object(_)) => v,
        _ => Value::Object(Map::new()),
    }
}

/// Reads all of stdin, giving up after `limit` (the old hook exited 0 after one second of silence).
pub fn read_stdin_with_timeout(limit: Duration) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::stdin().lock().read_to_end(&mut buf);
        let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
    });
    rx.recv_timeout(limit).ok()
}

// ── heartbeat ───────────────────────────────────────────────────────────────────────────────────────────────────────────
// Claude Code only runs a hook when a tool starts or ends, so a long answer or a long build is silent for minutes. A tiny
// detached helper started with the prompt says "still going" every few seconds until the turn ends, so the bridge's silence
// watchdog never calls a working agent finished. It stops on its own when the turn ends (Stop/SessionEnd remove its token file),
// when the user interrupts (the transcript gets "[Request interrupted by user"), or when the transcript has been silent for
// [`STALL_AFTER`] (a crashed agent), and never runs longer than [`MAX_BEAT`].

const BEAT_EVERY: Duration = Duration::from_secs(5);
const STALL_AFTER: Duration = Duration::from_secs(15 * 60);
const MAX_BEAT: Duration = Duration::from_secs(6 * 3600);

fn beat_file(session: &str) -> std::path::PathBuf {
    waitplay_core::paths::signal_dir().join(format!(".waitplay-beat-{}.txt", signals::signal_file_id(session)))
}

fn start_heartbeat(session: &str, token: i64, transcript: &str) {
    let dir = waitplay_core::paths::ensure_signal_dir();
    let _ = dir; // the helper's token file lives next to the other signal files
    let _ = waitplay_core::paths::write_atomic(&beat_file(session), token.to_string().as_bytes());
    let Ok(exe) = std::env::current_exe() else { return };
    let offset = if transcript.is_empty() { 0 } else { std::fs::metadata(transcript).map(|m| m.len()).unwrap_or(0) };
    let args = vec![
        "claude-code".to_string(),
        "beat".to_string(),
        format!("--session={}", session),
        format!("--token={}", token),
        format!("--offset={}", offset),
        format!("--transcript={}", transcript),
    ];
    let _ = waitplay_core::process::spawn_detached(&exe, &args);
}

fn stop_heartbeat(session: &str) {
    let _ = std::fs::remove_file(beat_file(session));
}

/// Did the user interrupt this turn? Looks only at what the transcript gained since the prompt (`offset`).
pub fn interrupted_since(transcript: &std::path::Path, offset: u64) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(transcript) else { return false };
    let Ok(len) = f.metadata().map(|m| m.len()) else { return false };
    if len <= offset {
        return false;
    }
    let from = offset.max(len.saturating_sub(256 * 1024));
    if f.seek(SeekFrom::Start(from)).is_err() {
        return false;
    }
    let mut buf = Vec::new();
    let _ = f.take(256 * 1024).read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).contains("[Request interrupted by user")
}

/// `waitplay-hook claude-code beat --session=ID --token=T [--offset=N] [--transcript=PATH]`
pub fn run_beat(args: &[String]) {
    let arg = |k: &str| args.iter().find_map(|a| a.strip_prefix(&format!("--{}=", k))).map(str::to_string);
    let (Some(session), Some(token)) = (arg("session"), arg("token")) else { return };
    let transcript = arg("transcript").filter(|t| !t.is_empty()).map(std::path::PathBuf::from);
    let offset: u64 = arg("offset").and_then(|o| o.parse().ok()).unwrap_or(0);
    let file = beat_file(&session);
    let began = std::time::Instant::now();
    let mut last_len = transcript.as_ref().and_then(|t| std::fs::metadata(t).ok()).map(|m| m.len()).unwrap_or(0);
    let mut last_activity = began;
    while began.elapsed() < MAX_BEAT {
        std::thread::sleep(BEAT_EVERY);
        // a newer prompt, a stop, or the end of the session replaces or removes the token: this turn is over
        if std::fs::read_to_string(&file).map(|t| t.trim() != token).unwrap_or(true) {
            return;
        }
        if let Some(t) = &transcript {
            if let Ok(m) = std::fs::metadata(t) {
                if m.len() != last_len {
                    last_len = m.len();
                    last_activity = std::time::Instant::now();
                }
            }
            if interrupted_since(t, offset) {
                let _ = std::fs::remove_file(&file);
                post(&session, "task_completed", Value::String(DEFAULT_MODEL.into()), None);
                signals::write_close_signal(&session);
                return;
            }
            if last_activity.elapsed() > STALL_AFTER {
                return;
            }
        }
        common::post_event_quiet(&common::event_payload(AGENT, &session, "heartbeat", None, None, None));
    }
}

pub fn run(args: &[String]) {
    if args.first().map(String::as_str) == Some("beat") {
        return run_beat(&args[1..]);
    }
    let event_type = args.first().map(String::as_str).unwrap_or("unknown");
    let Some(input) = read_stdin_with_timeout(Duration::from_millis(1000)) else { return };
    let data = parse_stdin(&input);
    execute(classify(event_type, &data), &session_for(&data));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn c(arg: &str, data: Value) -> Action {
        classify(arg, &data)
    }

    #[test]
    fn start_from_hook_json_and_from_arg() {
        let long = "x".repeat(150);
        assert_eq!(
            c("start", json!({"hook_event_name": "UserPromptSubmit", "prompt": long, "model": "opus"})),
            Action::Start { model: json!("opus"), prompt: "x".repeat(100), transcript: String::new() }
        );
        // the CLI argument alone is enough
        assert_eq!(c("start", json!({})), Action::Start { model: json!("Claude 3.7 Sonnet"), prompt: String::new(), transcript: String::new() });
        // hook name alone, whatever the argument is
        assert_eq!(c("unknown", json!({"hook_event_name": "user_prompt_submit"})), Action::Start { model: json!("Claude 3.7 Sonnet"), prompt: String::new(), transcript: String::new() });
    }

    #[test]
    fn tool_name_fallbacks() {
        assert_eq!(c("tool", json!({"tool_name": "Bash"})), Action::Tool { tool: json!("Bash"), model: json!("Claude 3.7 Sonnet") });
        assert_eq!(c("tool", json!({"tool": "Edit"})), Action::Tool { tool: json!("Edit"), model: json!("Claude 3.7 Sonnet") });
        assert_eq!(c("tool", json!({})), Action::Tool { tool: json!("Tool"), model: json!("Claude 3.7 Sonnet") });
        assert_eq!(c("x", json!({"hook_event_name": "PreToolUse", "tool_name": "Read"})), Action::Tool { tool: json!("Read"), model: json!("Claude 3.7 Sonnet") });
    }

    #[test]
    fn stop_notify_end() {
        assert_eq!(c("stop", json!({})), Action::Stop { model: json!("Claude 3.7 Sonnet") });
        assert_eq!(c("x", json!({"hook_event_name": "Stop", "model": "m"})), Action::Stop { model: json!("m") });
        assert_eq!(c("end", json!({})), Action::End);
        assert_eq!(c("x", json!({"hook_event_name": "SessionEnd"})), Action::End);
        assert_eq!(
            c("notify", json!({"message": "Claude needs your permission to use Bash"})),
            Action::WaitingForInput { prompt: "Claude needs your permission to use Bash".into() }
        );
        assert_eq!(c("notify", json!({"message": "Claude is waiting for your input"})), Action::Ignore);
        assert_eq!(c("x", json!({"hook_event_name": "Notification", "message": "please approve"})), Action::WaitingForInput { prompt: "please approve".into() });
        assert_eq!(c("notify", json!({})), Action::Ignore);
        assert_eq!(c("bogus", json!({})), Action::Ignore);
        assert_eq!(c("unknown", json!({})), Action::Ignore);
    }

    #[test]
    fn start_arg_wins_over_hook_name() {
        // the `start` argument is tested before the hook name for the later branches, as in the Node script
        assert!(matches!(c("start", json!({"hook_event_name": "Stop"})), Action::Start { .. }));
    }

    #[test]
    fn stdin_parsing_is_forgiving() {
        assert_eq!(parse_stdin(""), json!({}));
        assert_eq!(parse_stdin("{not json"), json!({}));
        assert_eq!(parse_stdin("null"), json!({}));
        assert_eq!(parse_stdin(r#"{"prompt":"hi"}"#), json!({"prompt": "hi"}));
    }

    #[test]
    fn each_conversation_gets_its_own_session() {
        let a = session_for(&json!({"session_id": "9f1c2e7a-4b3d-4a55-8f00-123456789abc"}));
        let b = session_for(&json!({"session_id": "0a0b0c0d-1111-2222-3333-444455556666"}));
        assert_eq!(a, "claude-code-9f1c2e7a4b3d");
        assert_ne!(a, b);
        // the agent name is still recognisable from the id, and the id stays a safe file name
        assert!(a.contains("claude"));
        assert_eq!(waitplay_core::signals::signal_file_id(&a), a);
        // no id, or one with nothing usable: the shared session of old
        assert_eq!(session_for(&json!({})), SESSION);
        assert_eq!(session_for(&json!({"session_id": "../../etc"})), "claude-code-etc");
        assert_eq!(session_for(&json!({"session_id": "---"})), SESSION);
        assert_eq!(session_for(&json!({"session_id": 7})), SESSION);
    }

    #[test]
    fn notification_types_decide_whether_the_user_is_needed() {
        let w = |ty: &str, msg: &str| c("notify", json!({"notification_type": ty, "message": msg}));
        assert_eq!(w("permission_prompt", "Claude needs your permission to use Bash"), Action::WaitingForInput { prompt: "Claude needs your permission to use Bash".into() });
        assert!(matches!(w("elicitation_dialog", "Pick one"), Action::WaitingForInput { .. }));
        assert_eq!(w("idle_prompt", "Claude is waiting for your input"), Action::Ignore, "after the turn: not an approval");
        assert_eq!(w("auth_success", "Signed in, please allow"), Action::Ignore);
        // older Claude Code without a type: the words decide, in any case
        assert!(matches!(c("notify", json!({"message": "Claude needs your PERMISSION"})), Action::WaitingForInput { .. }));
    }

    #[test]
    fn a_finished_tool_means_working_again() {
        assert_eq!(c("tool_done", json!({"tool_name": "Bash"})), Action::ToolDone { tool: json!("Bash"), model: json!("Claude 3.7 Sonnet") });
        assert!(matches!(c("x", json!({"hook_event_name": "PostToolUse", "tool_name": "Edit"})), Action::ToolDone { .. }));
    }

    #[test]
    fn the_prompt_carries_the_transcript_path_for_the_heartbeat() {
        assert!(matches!(c("start", json!({"transcript_path": "C:/t.jsonl"})), Action::Start { transcript, .. } if transcript == "C:/t.jsonl"));
    }

    #[test]
    fn interruption_is_read_only_from_what_the_turn_added() {
        let dir = std::env::temp_dir().join(format!("wp-beat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let t = dir.join("t.jsonl");
        std::fs::write(&t, "{\"text\":\"[Request interrupted by user]\"}\n").unwrap(); // an old interruption
        let offset = std::fs::metadata(&t).unwrap().len();
        assert!(!interrupted_since(&t, offset), "history before the prompt does not count");
        std::fs::OpenOptions::new().append(true).open(&t).unwrap().write_all(b"{\"text\":\"working\"}\n").unwrap();
        assert!(!interrupted_since(&t, offset));
        std::fs::OpenOptions::new().append(true).open(&t).unwrap().write_all(b"{\"text\":\"[Request interrupted by user for tool use]\"}\n").unwrap();
        assert!(interrupted_since(&t, offset));
        assert!(!interrupted_since(&dir.join("missing.jsonl"), 0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
