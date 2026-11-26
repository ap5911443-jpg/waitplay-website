//! `waitplay-hook opencode-watch`: follows OpenCode's database, `~/.local/share/opencode/opencode.db` (override:
//! `WAITPLAY_OPENCODE_DB`). Port of opencode-watcher.cjs.
//!
//! User rows of `message` start a turn, rows of `part` report tool/reasoning/step progress and the end of the turn.

use crate::common::{self, js_string, obj, truthy, PopRequest};
use crate::env::{run_loop, Env, RealEnv, Watcher};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

pub const SESSION: &str = "opencode-session";
const POLL_MS: u64 = 75;
const SETTLE_MS: i64 = 1200;
const DEFAULT_MODEL: &str = "GLM 5.3 Flash";

pub fn format_model_name(raw: &Value) -> String {
    if !truthy(raw) {
        return DEFAULT_MODEL.to_string();
    }
    let s = js_string(raw);
    let lower = s.to_lowercase();
    if lower.contains("glm-5.3") || lower.contains("glm") {
        return "GLM 5.3 Flash".into();
    }
    if lower.contains("muse-spark") || lower.contains("spark") {
        return "Muse Spark 1.3".into();
    }
    if lower.contains("mimo") {
        return "Mimo v2.6 Flash".into();
    }
    if lower.contains("claude-3-7-sonnet") || lower.contains("sonnet") {
        return "Claude 3.7 Sonnet".into();
    }
    if lower.contains("deepseek-r1") || lower.contains("deepseek") {
        return "DeepSeek R1".into();
    }
    if lower.contains("gpt-4o") {
        return "GPT-4o".into();
    }
    crate::cline::last_segment(&s)
}

/// A user message that OpenCode generated itself (compaction, "continue" nudges) rather than a person typing.
/// `Err` means the part data was unusable, which stops the scan of the remaining parts (the old code threw there).
pub fn part_is_synthetic(pd: &Value) -> Result<bool, ()> {
    if pd.get("synthetic").and_then(Value::as_bool) == Some(true) || pd.get("type").and_then(Value::as_str) == Some("compaction") {
        return Ok(true);
    }
    if pd.get("metadata").and_then(|m| m.get("compaction_continue")).and_then(Value::as_bool) == Some(true) {
        return Ok(true);
    }
    match pd.get("text") {
        Some(Value::String(t)) => Ok(t.contains("Continue if you have next steps")),
        Some(v) if truthy(v) => Err(()),
        _ => Ok(false),
    }
}

pub struct OpencodeState {
    pub is_working: bool,
    finish_at: Option<i64>,
    pub current_model: String,
}

impl Default for OpencodeState {
    fn default() -> Self {
        OpencodeState { is_working: false, finish_at: None, current_model: DEFAULT_MODEL.into() }
    }
}

impl OpencodeState {
    fn model(&self) -> Value {
        Value::String(self.current_model.clone())
    }

    fn send(&self, env: &mut dyn Env, event: &str, extra: serde_json::Map<String, Value>) {
        let model = extra.get("model").filter(|m| truthy(m)).cloned().unwrap_or_else(|| self.model());
        env.post(event, Some(model), Some(extra));
    }

    fn running(&mut self, env: &mut dyn Env, extra: serde_json::Map<String, Value>) {
        self.finish_at = None;
        self.is_working = true;
        self.send(env, "task_running", extra);
    }

    /// One new `message` row whose JSON is `data`. `synthetic` says whether any of its parts marks it as internal.
    pub fn on_message(&mut self, data: &Value, time_created: i64, synthetic: bool, env: &mut dyn Env) {
        if data.get("role").and_then(Value::as_str) != Some("user") {
            return;
        }
        if synthetic {
            // Internal continuation/compaction: keep the current turn running, never pop a new window
            if self.is_working {
                self.finish_at = None;
                self.send(env, "task_running", obj(vec![("model", self.model())]));
            }
            return;
        }

        // A genuine human prompt
        self.finish_at = None;
        if let Some(m) = data.get("model").filter(|m| truthy(m)) {
            let id = match m.get("modelID") {
                Some(v) if truthy(v) => Some(v),
                _ => m.get("id").filter(|v| truthy(v)),
            };
            if let Some(id) = id {
                self.current_model = format_model_name(id);
            }
        }
        let prompt_time = if time_created != 0 { time_created } else { env.now_ms() };
        env.log(&format!("[OpenCode Watcher] Genuine user prompt started ({})!", self.current_model));
        self.send(env, "task_started", obj(vec![("model", self.model()), ("promptTime", Value::from(prompt_time))]));

        // Pop the window once, at the start of the prompt
        if !self.is_working {
            self.is_working = true;
            env.clear_dismissal(SESSION);
            if !env.is_prompt_dismissed(SESSION) && env.is_enabled() {
                env.pop_app(&PopRequest { session: SESSION.into(), terminal: None, prompt_time: None, parent_pid: std::process::id() });
            }
        }
    }

    /// One new `part` row.
    pub fn on_part(&mut self, part: &Value, env: &mut dyn Env) {
        match part.get("type").and_then(Value::as_str) {
            Some("tool") | Some("patch") => {
                let tool = match part.get("tool") {
                    Some(t) if truthy(t) => t.clone(),
                    _ => Value::String("tool".into()),
                };
                env.log(&format!("[OpenCode Watcher] Tool active: {}", js_string(&tool)));
                self.running(env, obj(vec![("tool", tool), ("model", self.model())]));
            }
            Some("reasoning") | Some("step-start") => self.running(env, obj(vec![("model", self.model())])),
            Some("step-finish") => match part.get("reason").and_then(Value::as_str) {
                Some("stop") => {
                    // Debounce: OpenCode may still start another step
                    self.finish_at = Some(env.now_ms() + SETTLE_MS);
                }
                Some("tool-calls") => self.running(env, obj(vec![("model", self.model())])),
                _ => {}
            },
            _ => {}
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
                env.log(&format!("[OpenCode Watcher] OpenCode turn fully settled and complete ({})!", self.current_model));
                env.clear_dismissal(SESSION);
                self.send(env, "task_completed", obj(vec![("model", self.model())]));
            }
        }
    }
}

pub struct OpencodeWatcher {
    db_path: PathBuf,
    conn: Option<Connection>,
    last_message_time: i64,
    last_part_time: i64,
    pub state: OpencodeState,
}

fn json_or_empty(text: Option<String>) -> Option<Value> {
    let t = text.filter(|t| !t.is_empty()).unwrap_or_else(|| "{}".to_string());
    serde_json::from_str(&t).ok()
}

impl OpencodeWatcher {
    pub fn new(db_path: PathBuf) -> OpencodeWatcher {
        OpencodeWatcher { db_path, conn: None, last_message_time: 0, last_part_time: 0, state: OpencodeState::default() }
    }

    fn connect(&mut self, env: &mut dyn Env) -> bool {
        let Ok(conn) = Connection::open_with_flags(&self.db_path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX) else {
            return false;
        };
        let _ = conn.busy_timeout(Duration::ZERO);
        let max = |table: &str| -> Option<Option<i64>> {
            conn.query_row(&format!("SELECT MAX(time_created) FROM {}", table), [], |r| r.get::<_, Option<i64>>(0)).ok()
        };
        let (Some(m), Some(p)) = (max("message"), max("part")) else { return false };
        let now = env.now_ms();
        self.last_message_time = m.filter(|v| *v != 0).unwrap_or(now);
        self.last_part_time = p.filter(|v| *v != 0).unwrap_or(now);

        let model_json: Option<String> =
            conn.query_row("SELECT model FROM session ORDER BY time_updated DESC LIMIT 1", [], |r| r.get(0)).ok().flatten();
        if let Some(sm) = model_json.and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
            if let Some(id) = sm.get("id").filter(|v| truthy(v)) {
                self.state.current_model = format_model_name(id);
            }
        }
        env.log(&format!(
            "[OpenCode Watcher] Connected to opencode.db. Baseline msg: {}, part: {}, model: {}",
            self.last_message_time, self.last_part_time, self.state.current_model
        ));
        self.conn = Some(conn);
        true
    }

    fn poll_messages(&mut self, env: &mut dyn Env) -> rusqlite::Result<()> {
        let rows: Vec<(String, i64, Option<String>)> = {
            let conn = self.conn.as_ref().expect("connected");
            let mut stmt =
                conn.prepare_cached("SELECT id, time_created, data FROM message WHERE time_created > ?1 ORDER BY time_created ASC LIMIT 20")?;
            let mapped = stmt.query_map([self.last_message_time], |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0), r.get(2)?)))?;
            mapped.collect::<rusqlite::Result<_>>()?
        };
        for (id, time_created, data) in rows {
            self.last_message_time = self.last_message_time.max(time_created);
            // Unparseable JSON abandons the rest of the batch; the next tick resumes after this row's time
            let Some(data) = json_or_empty(data) else { break };
            let mut synthetic = false;
            if data.get("role").and_then(Value::as_str) == Some("user") {
                synthetic = self.message_is_synthetic(&id);
            }
            self.state.on_message(&data, time_created, synthetic, env);
        }
        Ok(())
    }

    fn message_is_synthetic(&self, message_id: &str) -> bool {
        let conn = self.conn.as_ref().expect("connected");
        let Ok(mut stmt) = conn.prepare_cached("SELECT data FROM part WHERE message_id = ?1") else { return false };
        let Ok(rows) = stmt.query_map([message_id], |r| r.get::<_, Option<String>>(0)) else { return false };
        for data in rows {
            let Ok(data) = data else { return false };
            let Some(pd) = json_or_empty(data) else { return false };
            match part_is_synthetic(&pd) {
                Ok(true) => return true,
                Ok(false) => {}
                Err(()) => return false,
            }
        }
        false
    }

    fn poll_parts(&mut self, env: &mut dyn Env) -> rusqlite::Result<()> {
        let rows: Vec<(i64, Option<String>)> = {
            let conn = self.conn.as_ref().expect("connected");
            let mut stmt = conn.prepare_cached("SELECT time_created, data FROM part WHERE time_created > ?1 ORDER BY time_created ASC LIMIT 50")?;
            let mapped = stmt.query_map([self.last_part_time], |r| Ok((r.get::<_, Option<i64>>(0)?.unwrap_or(0), r.get(1)?)))?;
            mapped.collect::<rusqlite::Result<_>>()?
        };
        for (time_created, data) in rows {
            self.last_part_time = self.last_part_time.max(time_created);
            let Some(part) = json_or_empty(data) else { break };
            self.state.on_part(&part, env);
        }
        Ok(())
    }
}

impl Watcher for OpencodeWatcher {
    fn poll(&mut self, env: &mut dyn Env) {
        if self.conn.is_none() {
            if self.db_path.is_file() {
                self.connect(env);
            }
            return;
        }
        let _ = self.poll_messages(env);
        let _ = self.poll_parts(env);
    }

    fn deadline(&self) -> Option<i64> {
        self.state.deadline()
    }

    fn is_idle(&self) -> bool {
        !self.state.is_working && self.state.finish_at.is_none()
    }

    fn wants_heartbeat(&self) -> bool {
        true
    }

    fn fire(&mut self, env: &mut dyn Env) {
        self.state.fire(env);
    }
}

pub fn db_path() -> PathBuf {
    common::env_path_or("WAITPLAY_OPENCODE_DB", || common::home_dir().join(".local").join("share").join("opencode").join("opencode.db"))
}

pub fn run_watch() -> ! {
    let path = db_path();
    let mut env = RealEnv::new("opencode", SESSION);
    let mut watcher = OpencodeWatcher::new(path.clone());
    if path.is_file() {
        if !watcher.connect(&mut env) {
            println!("[OpenCode Watcher] opencode.db waiting...");
        }
    }
    println!("[OpenCode Watcher] Service active and monitoring OpenCode...");
    run_loop(POLL_MS, &mut watcher, &mut env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::mock::MockEnv;
    use serde_json::json;

    #[test]
    fn model_names() {
        assert_eq!(format_model_name(&json!("glm-5.3-flash-free")), "GLM 5.3 Flash");
        assert_eq!(format_model_name(&json!("anthropic/claude-3-7-sonnet")), "Claude 3.7 Sonnet");
        assert_eq!(format_model_name(&json!("deepseek-chat")), "DeepSeek R1");
        assert_eq!(format_model_name(&json!("acme/foo")), "foo");
        assert_eq!(format_model_name(&json!(null)), "GLM 5.3 Flash");
    }

    #[test]
    fn synthetic_detection() {
        assert_eq!(part_is_synthetic(&json!({"synthetic": true})), Ok(true));
        assert_eq!(part_is_synthetic(&json!({"type": "compaction"})), Ok(true));
        assert_eq!(part_is_synthetic(&json!({"metadata": {"compaction_continue": true}})), Ok(true));
        assert_eq!(part_is_synthetic(&json!({"text": "Continue if you have next steps, or stop"})), Ok(true));
        assert_eq!(part_is_synthetic(&json!({"text": "hello"})), Ok(false));
        assert_eq!(part_is_synthetic(&json!({"type": "text"})), Ok(false));
        assert_eq!(part_is_synthetic(&json!({"text": {"x": 1}})), Err(()));
    }

    #[test]
    fn prompt_then_tools_then_debounced_completion() {
        let mut env = MockEnv::new();
        let mut s = OpencodeState::default();
        s.on_message(&json!({"role": "user", "model": {"modelID": "claude-sonnet-4"}}), 5555, false, &mut env);
        assert_eq!(env.events(), ["task_started"]);
        assert_eq!(env.data(0), json!({"model": "Claude 3.7 Sonnet", "promptTime": 5555}));
        assert_eq!(env.pops.len(), 1);
        assert!(s.is_working);

        // a second human prompt in the same turn re-sends task_started but never re-pops
        s.on_message(&json!({"role": "user"}), 5600, false, &mut env);
        assert_eq!(env.events(), ["task_started", "task_started"]);
        assert_eq!(env.pops.len(), 1);

        s.on_part(&json!({"type": "step-start"}), &mut env);
        s.on_part(&json!({"type": "tool", "tool": "bash"}), &mut env);
        s.on_part(&json!({"type": "step-finish", "reason": "tool-calls"}), &mut env);
        assert_eq!(env.events(), ["task_started", "task_started", "task_running", "task_running", "task_running"]);
        assert_eq!(env.data(3), json!({"tool": "bash", "model": "Claude 3.7 Sonnet"}));

        s.on_part(&json!({"type": "step-finish", "reason": "stop"}), &mut env);
        assert_eq!(s.deadline(), Some(env.now + 1200));
        env.advance(1000);
        s.on_part(&json!({"type": "reasoning"}), &mut env); // more work cancels completion
        assert_eq!(s.deadline(), None);
        s.on_part(&json!({"type": "step-finish", "reason": "stop"}), &mut env);
        env.advance(1200);
        s.fire(&mut env);
        assert_eq!(env.events().last(), Some(&"task_completed"));
        assert!(!s.is_working);
        // OpenCode never writes a close signal; the app follows the bridge
        assert!(!env.calls.iter().any(|c| c.starts_with("close:")));
    }

    #[test]
    fn synthetic_messages_only_keep_the_turn_alive() {
        let mut env = MockEnv::new();
        let mut s = OpencodeState::default();
        s.on_message(&json!({"role": "user"}), 1, true, &mut env); // idle: nothing at all
        assert!(env.posts.is_empty());
        s.on_message(&json!({"role": "user"}), 2, false, &mut env);
        s.on_part(&json!({"type": "step-finish", "reason": "stop"}), &mut env);
        assert!(s.deadline().is_some());
        s.on_message(&json!({"role": "user"}), 3, true, &mut env);
        assert_eq!(s.deadline(), None);
        assert_eq!(env.events().last(), Some(&"task_running"));
        assert_eq!(env.pops.len(), 1);
    }

    #[test]
    fn assistant_rows_and_unknown_parts_are_ignored() {
        let mut env = MockEnv::new();
        let mut s = OpencodeState::default();
        s.on_message(&json!({"role": "assistant"}), 1, false, &mut env);
        s.on_part(&json!({"type": "text", "text": "hi"}), &mut env);
        s.on_part(&json!({"type": "step-finish", "reason": "length"}), &mut env);
        assert!(env.posts.is_empty());
    }

    #[test]
    fn reads_a_real_sqlite_database() {
        let dir = std::env::temp_dir().join(format!("waitplay-opencode-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("opencode.db");
        let _ = std::fs::remove_file(&path);
        let rw = Connection::open(&path).unwrap();
        rw.execute_batch(
            "CREATE TABLE message (id TEXT, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);
             CREATE TABLE part (id TEXT, message_id TEXT, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);
             CREATE TABLE session (id TEXT, model TEXT, time_updated INTEGER);
             INSERT INTO session VALUES ('s', '{\"id\":\"glm-5.3\"}', 1);
             INSERT INTO message VALUES ('m0', 's', 100, 100, '{\"role\":\"user\"}');",
        )
        .unwrap();

        let mut env = MockEnv::new();
        let mut w = OpencodeWatcher::new(path.clone());
        w.poll(&mut env); // baseline
        assert_eq!(w.last_message_time, 100);
        assert!(env.posts.is_empty());

        rw.execute_batch(
            "INSERT INTO message VALUES ('m1', 's', 200, 200, '{\"role\":\"user\",\"model\":{\"modelID\":\"gpt-4o\"}}');
             INSERT INTO part VALUES ('p1', 'm1', 's', 201, 201, '{\"type\":\"text\",\"text\":\"hello\"}');
             INSERT INTO message VALUES ('m2', 's', 300, 300, '{\"role\":\"user\"}');
             INSERT INTO part VALUES ('p2', 'm2', 's', 301, 301, '{\"type\":\"text\",\"text\":\"Continue if you have next steps\",\"synthetic\":true}');
             INSERT INTO part VALUES ('p3', 'm2', 's', 1000302, 1000302, '{\"type\":\"tool\",\"tool\":\"read\"}');",
        )
        .unwrap();
        w.poll(&mut env);
        // m1 is a human prompt, m2 synthetic (turn is working so it re-asserts running), then the parts
        assert_eq!(env.events(), ["task_started", "task_running", "task_running"]);
        assert_eq!(env.data(0)["model"], "GPT-4o");
        w.poll(&mut env);
        assert_eq!(env.events().len(), 3);
        drop(w);
        drop(rw);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
