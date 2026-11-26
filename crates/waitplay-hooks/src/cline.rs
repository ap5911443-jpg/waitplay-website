//! `waitplay-hook cline-watch`: follows the newest Cline CLI session, `~/.cline/data/sessions/<id>/<id>.messages.json`
//! (override the root with `WAITPLAY_CLINE_SESSIONS_DIR`). Port of cline-watcher.cjs.

use crate::common::{self, js_len, js_slice, js_string, obj, truthy, PopRequest};
use crate::env::{run_loop, Env, RealEnv, Watcher};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use waitplay_core::paths;

pub const SESSION: &str = "cline-session";
const POLL_MS: u64 = 75;
const SETTLE_MS: i64 = 800;
const DEFAULT_MODEL: &str = "Claude 3.7 Sonnet";

pub fn format_model_name(raw: &Value) -> String {
    if !truthy(raw) {
        return DEFAULT_MODEL.to_string();
    }
    let s = js_string(raw);
    let lower = s.to_lowercase();
    if lower.contains("claude-sonnet") || lower.contains("sonnet") {
        return "Claude 3.7 Sonnet".into();
    }
    if lower.contains("gpt-5.6") || lower.contains("luna") {
        return "GPT-5.6 Luna".into();
    }
    if lower.contains("gemini-4") || lower.contains("argon") {
        return "Gemini 4 Argon".into();
    }
    if lower.contains("gpt-4o") {
        return "GPT-4o".into();
    }
    if lower.contains("o3") {
        return "o3-mini".into();
    }
    if lower.contains("best-coding") {
        return "Best Coding".into();
    }
    last_segment(&s)
}

/// `String(raw).split('/').pop() || raw`
pub fn last_segment(s: &str) -> String {
    match s.rsplit('/').next() {
        Some(seg) if !seg.is_empty() => seg.to_string(),
        _ => s.to_string(),
    }
}

/// Removes `<user_input ...>` (first opening tag) and the first `</user_input>`, then trims; the text stays untouched
/// unless it mentions `<user_input`.
pub fn strip_user_input(text: &str) -> String {
    if !text.contains("<user_input") {
        return text.to_string();
    }
    let mut t = text.to_string();
    if let Some(start) = t.find("<user_input") {
        if let Some(rel) = t[start..].find('>') {
            t.replace_range(start..start + rel + 1, "");
        }
    }
    if let Some(start) = t.find("</user_input>") {
        t.replace_range(start..start + "</user_input>".len(), "");
    }
    t.trim().to_string()
}

pub struct ClineState {
    pub is_working: bool,
    finish_at: Option<i64>,
    pub current_model: String,
    pub has_popped: bool,
}

impl Default for ClineState {
    fn default() -> Self {
        ClineState { is_working: false, finish_at: None, current_model: DEFAULT_MODEL.into(), has_popped: false }
    }
}

impl ClineState {
    fn send(&self, env: &mut dyn Env, event: &str, extra: Map<String, Value>) {
        let model = extra.get("model").filter(|m| truthy(m)).cloned().unwrap_or_else(|| Value::String(self.current_model.clone()));
        env.post(event, Some(model), Some(extra));
    }

    fn model(&self) -> Value {
        Value::String(self.current_model.clone())
    }

    fn pop(&mut self, env: &mut dyn Env) {
        if !env.is_enabled() || self.has_popped {
            return;
        }
        if env.is_prompt_dismissed(SESSION) {
            env.log("[Cline Watcher] Prompt was dismissed by user, skipping popup.");
            return;
        }
        self.has_popped = true;
        env.pop_app(&PopRequest { session: SESSION.into(), terminal: None, prompt_time: None, parent_pid: std::process::id() });
    }

    /// Processes `messages[from..]`.
    pub fn on_messages(&mut self, messages: &[Value], from: usize, env: &mut dyn Env) {
        for m in &messages[from.min(messages.len())..] {
            let role = m.get("role").and_then(Value::as_str).unwrap_or("");
            let content = match m.get("content") {
                Some(c) if truthy(c) => c.clone(),
                _ => Value::Array(vec![]),
            };
            if role == "user" {
                let has_tool_result =
                    content.as_array().map_or(false, |a| a.iter().any(|c| c.get("type").and_then(Value::as_str) == Some("tool_result")));
                self.finish_at = None;
                self.is_working = true;
                if has_tool_result {
                    // Tool output returned: Cline is working
                    self.send(env, "task_running", obj(vec![("model", self.model())]));
                } else {
                    // A genuine human prompt
                    self.has_popped = false;
                    env.clear_dismissal(SESSION);
                    let mut prompt = String::new();
                    match &content {
                        Value::Array(a) => {
                            for c in a {
                                if let Some(t) = c.get("text").filter(|t| truthy(t)) {
                                    prompt.push_str(&js_string(t));
                                }
                            }
                        }
                        Value::String(s) => prompt = s.clone(),
                        _ => {}
                    }
                    let mut prompt = strip_user_input(&prompt);
                    if js_len(&prompt) > 80 {
                        prompt = js_slice(&prompt, 80) + "...";
                    }
                    env.log(&format!("[Cline Watcher] User prompt submitted ({})!", prompt));
                    self.send(env, "task_started", obj(vec![("model", self.model()), ("prompt", Value::String(prompt))]));
                    self.pop(env);
                }
            } else if role == "assistant" {
                let mut has_tool_use = false;
                let mut tool_name = Value::String("tool".into());
                let mut has_text = false;
                if let Value::Array(a) = &content {
                    for c in a {
                        match c.get("type").and_then(Value::as_str) {
                            Some("tool_use") => {
                                has_tool_use = true;
                                if let Some(n) = c.get("name").filter(|n| truthy(n)) {
                                    tool_name = n.clone();
                                }
                            }
                            Some("text") => {
                                if c.get("text").and_then(Value::as_str).map_or(false, |t| !t.trim().is_empty()) {
                                    has_text = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if has_tool_use {
                    self.finish_at = None;
                    self.is_working = true;
                    env.log(&format!("[Cline Watcher] Tool active: {}", js_string(&tool_name)));
                    self.send(env, "task_running", obj(vec![("tool", tool_name), ("model", self.model())]));
                } else if has_text {
                    // Final assistant text: a candidate for completion, confirmed after SETTLE_MS of quiet
                    self.finish_at = Some(env.now_ms() + SETTLE_MS);
                }
            }
        }
    }

    pub fn deadline(&self) -> Option<i64> {
        self.finish_at
    }

    pub fn fire(&mut self, env: &mut dyn Env) {
        if let Some(due) = self.finish_at {
            if env.now_ms() >= due {
                self.finish_at = None;
                self.is_working = false;
                self.has_popped = false;
                env.clear_dismissal(SESSION);
                env.log(&format!("[Cline Watcher] Cline task completed ({})!", self.current_model));
                self.send(env, "task_completed", obj(vec![("model", self.model())]));
            }
        }
    }
}

fn mtime_ms(p: &Path) -> Option<f64> {
    let m = fs::metadata(p).ok()?.modified().ok()?;
    let d = m.duration_since(UNIX_EPOCH).ok()?;
    Some(d.as_secs_f64() * 1000.0)
}

/// The most recently modified session directory (first wins on a tie).
pub fn latest_session_dir(root: &Path) -> Option<PathBuf> {
    let mut best: Option<(PathBuf, f64)> = None;
    for e in fs::read_dir(root).ok()?.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let t = mtime_ms(&p).unwrap_or(0.0);
        if best.as_ref().map_or(true, |(_, bt)| t > *bt) {
            best = Some((p, t));
        }
    }
    best.map(|b| b.0)
}

fn read_json(p: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(p).ok()?).ok()
}

pub struct ClineWatcher {
    root: PathBuf,
    pub state: ClineState,
    active_session_id: String,
    last_messages_count: usize,
    last_file_mtime: f64,
}

impl ClineWatcher {
    pub fn new(root: PathBuf) -> ClineWatcher {
        ClineWatcher { root, state: ClineState::default(), active_session_id: String::new(), last_messages_count: 0, last_file_mtime: 0.0 }
    }

    fn load_model(&mut self, dir: &Path) {
        let meta = dir.join(format!("{}.json", self.active_session_id));
        if let Some(model) = read_json(&meta).and_then(|m| m.get("model").cloned()).filter(truthy) {
            self.state.current_model = format_model_name(&model);
        }
    }

    /// Baseline: the newest session as it is now, so earlier messages are never replayed.
    pub fn init(&mut self, env: &mut dyn Env) {
        let Some(dir) = latest_session_dir(&self.root) else { return };
        self.active_session_id = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.load_model(&dir);
        let msgs = dir.join(format!("{}.messages.json", self.active_session_id));
        if msgs.is_file() {
            self.last_file_mtime = mtime_ms(&msgs).unwrap_or(0.0);
            if let Some(v) = read_json(&msgs) {
                self.last_messages_count = v.get("messages").and_then(Value::as_array).map_or(0, Vec::len);
            }
        }
        env.log(&format!(
            "[Cline Watcher] Connected to Cline sessions. Baseline: {} ({}), msgs: {}",
            self.active_session_id, self.state.current_model, self.last_messages_count
        ));
    }
}

impl Watcher for ClineWatcher {
    fn poll(&mut self, env: &mut dyn Env) {
        let Some(latest) = latest_session_dir(&self.root) else { return };
        let id = latest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if id != self.active_session_id {
            self.active_session_id = id;
            self.last_messages_count = 0;
            self.last_file_mtime = 0.0;
            self.state.has_popped = false;
            self.load_model(&latest);
            env.log(&format!("[Cline Watcher] Detected new Cline session: {} ({})", self.active_session_id, self.state.current_model));
        }

        let msgs = latest.join(format!("{}.messages.json", self.active_session_id));
        if !msgs.is_file() {
            return;
        }
        let Some(mtime) = mtime_ms(&msgs) else { return };
        if mtime <= self.last_file_mtime {
            return;
        }
        self.last_file_mtime = mtime;

        // A file caught mid-write fails to parse and is retried on its next modification
        let Some(data) = read_json(&msgs) else { return };
        let empty = Vec::new();
        let messages = data.get("messages").and_then(Value::as_array).unwrap_or(&empty);
        if messages.is_empty() || messages.len() < self.last_messages_count {
            self.last_messages_count = messages.len();
            return;
        }
        self.state.on_messages(messages, self.last_messages_count, env);
        self.last_messages_count = messages.len();
    }

    fn deadline(&self) -> Option<i64> {
        self.state.deadline()
    }

    fn is_idle(&self) -> bool {
        !self.state.is_working && self.state.finish_at.is_none()
    }

    fn fire(&mut self, env: &mut dyn Env) {
        self.state.fire(env);
    }
}

pub fn sessions_dir() -> PathBuf {
    common::env_path_or("WAITPLAY_CLINE_SESSIONS_DIR", || common::home_dir().join(".cline").join("data").join("sessions"))
}

pub fn run_watch() -> ! {
    let root = sessions_dir();
    println!("=======================================================");
    println!("  Vibe Play Dedicated Watcher for Cline");
    println!("  Cline Sessions Dir: {}", root.display());
    println!("  Bridge Target: http://127.0.0.1:{}", paths::bridge_port());
    println!("=======================================================");
    let mut env = RealEnv::new("cline", SESSION);
    let mut watcher = ClineWatcher::new(root);
    watcher.init(&mut env);
    println!("[Cline Watcher] Service active and monitoring Cline CLI...");
    run_loop(POLL_MS, &mut watcher, &mut env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::mock::MockEnv;
    use serde_json::json;

    #[test]
    fn model_names() {
        assert_eq!(format_model_name(&json!("anthropic/claude-sonnet-4")), "Claude 3.7 Sonnet");
        assert_eq!(format_model_name(&json!("openai/gpt-4o-mini")), "GPT-4o");
        assert_eq!(format_model_name(&json!("deepseek/o3")), "o3-mini");
        assert_eq!(format_model_name(&json!("acme/some-model")), "some-model");
        assert_eq!(format_model_name(&json!("acme/")), "acme/");
        assert_eq!(format_model_name(&json!(null)), "Claude 3.7 Sonnet");
        assert_eq!(format_model_name(&json!("")), "Claude 3.7 Sonnet");
    }

    #[test]
    fn user_input_stripping() {
        assert_eq!(strip_user_input("plain"), "plain");
        assert_eq!(strip_user_input("<user_input mode=\"x\">  fix it </user_input>"), "fix it");
        assert_eq!(strip_user_input("  keep spaces  "), "  keep spaces  ");
    }

    #[test]
    fn prompt_tool_text_flow() {
        let mut env = MockEnv::new();
        let mut s = ClineState::default();
        let msgs = vec![
            json!({"role": "user", "content": [{"type": "text", "text": "<user_input>Refactor the parser</user_input>"}]}),
            json!({"role": "assistant", "content": [{"type": "tool_use", "name": "read_file"}]}),
            json!({"role": "user", "content": [{"type": "tool_result", "content": "ok"}]}),
            json!({"role": "assistant", "content": [{"type": "text", "text": "Done."}]}),
        ];
        s.on_messages(&msgs, 0, &mut env);
        assert_eq!(env.events(), ["task_started", "task_running", "task_running"]);
        assert_eq!(env.data(0), json!({"model": "Claude 3.7 Sonnet", "prompt": "Refactor the parser"}));
        assert_eq!(env.data(1), json!({"tool": "read_file", "model": "Claude 3.7 Sonnet"}));
        assert_eq!(env.data(2), json!({"model": "Claude 3.7 Sonnet"}));
        assert_eq!(env.pops.len(), 1);
        assert_eq!(s.deadline(), Some(env.now + 800));

        env.advance(800);
        s.fire(&mut env);
        assert_eq!(env.events().last(), Some(&"task_completed"));
        assert!(!s.is_working);
        // Cline never wrote a close signal
        assert!(!env.calls.iter().any(|c| c.starts_with("close:")));
    }

    #[test]
    fn long_prompts_are_cut_to_80() {
        let mut env = MockEnv::new();
        let mut s = ClineState::default();
        let msgs = vec![json!({"role": "user", "content": "y".repeat(120)})];
        s.on_messages(&msgs, 0, &mut env);
        let prompt = env.data(0)["prompt"].as_str().unwrap().to_string();
        assert_eq!(prompt, format!("{}...", "y".repeat(80)));
    }

    #[test]
    fn popup_once_per_prompt_and_respects_switches() {
        let mut env = MockEnv::new();
        let mut s = ClineState::default();
        s.pop(&mut env);
        s.pop(&mut env);
        assert_eq!(env.pops.len(), 1);

        let mut env = MockEnv::new();
        env.enabled = false;
        let mut s = ClineState::default();
        s.pop(&mut env);
        assert!(env.pops.is_empty());
        assert!(!s.has_popped);

        let mut env = MockEnv::new();
        env.dismissed.insert(SESSION.into());
        let mut s = ClineState::default();
        s.pop(&mut env);
        assert!(env.pops.is_empty());
    }

    #[test]
    fn tool_use_cancels_pending_completion() {
        let mut env = MockEnv::new();
        let mut s = ClineState::default();
        s.on_messages(&[json!({"role": "assistant", "content": [{"type": "text", "text": "thinking aloud"}]})], 0, &mut env);
        assert!(s.deadline().is_some());
        s.on_messages(&[json!({"role": "assistant", "content": [{"type": "tool_use", "name": "bash"}]})], 0, &mut env);
        assert_eq!(s.deadline(), None);
        assert_eq!(env.events(), ["task_running"]);
        // whitespace-only text does not schedule a completion
        s.on_messages(&[json!({"role": "assistant", "content": [{"type": "text", "text": "  "}]})], 0, &mut env);
        assert_eq!(s.deadline(), None);
    }

    #[test]
    fn watches_a_session_directory_on_disk() {
        let root = std::env::temp_dir().join(format!("waitplay-cline-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let old = root.join("sess-old");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("sess-old.json"), r#"{"model":"anthropic/claude-sonnet"}"#).unwrap();
        fs::write(old.join("sess-old.messages.json"), r#"{"messages":[{"role":"user","content":"earlier"}]}"#).unwrap();

        let mut env = MockEnv::new();
        let mut w = ClineWatcher::new(root.clone());
        w.init(&mut env);
        assert_eq!(w.last_messages_count, 1);
        w.poll(&mut env);
        assert!(env.posts.is_empty(), "baseline must not replay history");

        std::thread::sleep(std::time::Duration::from_millis(30));
        fs::write(
            old.join("sess-old.messages.json"),
            r#"{"messages":[{"role":"user","content":"earlier"},{"role":"user","content":"new prompt"}]}"#,
        )
        .unwrap();
        w.poll(&mut env);
        assert_eq!(env.events(), ["task_started"]);
        assert_eq!(env.data(0)["prompt"], "new prompt");
        let _ = fs::remove_dir_all(&root);
    }
}
