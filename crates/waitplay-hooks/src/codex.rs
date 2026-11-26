//! Codex: `codex-notify` (turn-end notify hook), `codex-watch` (log database watcher), `codex-test-turn` (simulated turn),
//! and `popup` (what popup.ps1 did).
//!
//! `codex-watch` follows `~/.codex/logs_2.sqlite` (override: `WAITPLAY_CODEX_DB`).

use crate::common::{self, PopRequest};
use crate::env::{run_loop, Env, RealEnv, Watcher};
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use waitplay_core::{paths, process, signals};

pub const SESSION: &str = "codex-session";
const POLL_MS: u64 = 75;
const SETTLE_MS: i64 = 800;

// ---------------------------------------------------------------------------------------------------------------------
// codex-notify

/// Configured in `~/.codex/config.toml` as `notify = ["waitplay-hook", "codex-notify"]`; Codex appends the turn JSON.
/// Optionally forwards to the computer-use notifier named by `WAITPLAY_CODEX_ORIGINAL_NOTIFY` (the old script hardcoded
/// one version-specific WindowsApps path that no longer exists on current installs).
pub fn run_notify(args: &[String]) {
    if let Ok(original) = std::env::var("WAITPLAY_CODEX_ORIGINAL_NOTIFY") {
        let original = PathBuf::from(original.trim());
        if original.is_file() {
            let mut cmd = std::process::Command::new(&original);
            cmd.arg("turn-ended").args(args);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            let _ = cmd.spawn();
        }
    }
    common::post_event_quiet(&common::event_payload("codex", SESSION, "task_completed", None, None, None));
    // Widget-safe close: signal the companion rather than killing it
    signals::write_close_signal(SESSION);
}

// ---------------------------------------------------------------------------------------------------------------------
// codex-watch

pub fn is_start_marker(body: &str) -> bool {
    body.contains("operation=\"turn_input\"")
        || body.contains("TurnInput")
        || body.contains("otel.name=\"op.dispatch.turn_input\"")
        || body.contains("codex.turn_input")
}

pub fn is_progress_marker(body: &str) -> bool {
    body.contains("invoke_tool")
        || body.contains("code_mode.broker")
        || body.contains("item/commandExecution")
        || body.contains("item/started")
        || body.contains("run_sampling_request")
}

pub fn is_end_marker(body: &str) -> bool {
    body.contains("app-server event: turn/completed targeted_connections=1") || body.contains("turn/completed targeted_connections=1")
}

#[derive(Default)]
pub struct CodexState {
    pub active: bool,
    /// (due at epoch ms, log id that ended the turn)
    completion: Option<(i64, i64)>,
}

impl CodexState {
    pub fn on_row(&mut self, id: i64, body: &str, env: &mut dyn Env) {
        if is_start_marker(body) {
            // A new step cancels any completion pending from an intermediate tool step
            self.completion = None;
            if !self.active {
                self.active = true;
                env.clear_dismissal(SESSION);
                env.log(&format!("[Vibe Play Watcher] ChatGPT/Codex prompt started! Thinking/reasoning... (log {})", id));
                env.post("task_started", None, None);
                self.pop(env);
            }
            return;
        }

        if self.active && is_progress_marker(body) {
            self.completion = None;
            env.post("task_running", None, None);
        }

        // Debounce: the turn only ends if no new step arrives within SETTLE_MS
        if self.active && is_end_marker(body) {
            self.completion = Some((env.now_ms() + SETTLE_MS, id));
        }
    }

    fn pop(&self, env: &mut dyn Env) {
        if !env.is_enabled() {
            env.log("[Vibe Play Watcher] Vibe Play is turned OFF, skipping popup.");
            return;
        }
        if env.is_prompt_dismissed(SESSION) {
            env.log("[Vibe Play Watcher] Prompt was dismissed by user, skipping popup.");
            return;
        }
        env.log("[Vibe Play Watcher] Launching native Vibe Play desktop application...");
        env.clear_close_signal(SESSION);
        env.pop_app(&PopRequest { session: SESSION.into(), terminal: None, prompt_time: None, parent_pid: std::process::id() });
    }

    pub fn deadline(&self) -> Option<i64> {
        self.completion.map(|c| c.0)
    }

    pub fn fire(&mut self, env: &mut dyn Env) {
        if let Some((due, end_id)) = self.completion {
            if env.now_ms() >= due {
                self.active = false;
                self.completion = None;
                env.log(&format!("[Vibe Play Watcher] ChatGPT/Codex finished turn! (log {})", end_id));
                env.clear_dismissal(SESSION);
                env.post("task_completed", None, None);
                env.write_close_signal(SESSION);
            }
        }
    }
}

pub struct CodexWatcher {
    db_path: PathBuf,
    conn: Option<Connection>,
    last_log_id: i64,
    pub state: CodexState,
}

fn open_readonly(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    // The node:sqlite default: fail at once on a lock and retry on the next tick rather than stalling the loop
    conn.busy_timeout(Duration::ZERO)?;
    Ok(conn)
}

impl CodexWatcher {
    pub fn new(db_path: PathBuf) -> CodexWatcher {
        CodexWatcher { db_path, conn: None, last_log_id: 0, state: CodexState::default() }
    }

    /// Opens the database and records the newest log id as the baseline, so history is never replayed.
    fn connect(&mut self, env: &mut dyn Env) -> bool {
        let Ok(conn) = open_readonly(&self.db_path) else { return false };
        let max: Option<i64> = conn.query_row("SELECT MAX(id) FROM logs", [], |r| r.get(0)).ok().flatten();
        if conn.prepare_cached("SELECT 1 FROM logs LIMIT 1").is_err() {
            return false;
        }
        self.last_log_id = max.unwrap_or(0);
        self.conn = Some(conn);
        env.log(&format!("[Vibe Play Watcher] Connected to Codex logs DB. Baseline log ID: {}", self.last_log_id));
        true
    }

    fn fetch(&self) -> rusqlite::Result<Vec<(i64, String)>> {
        let Some(conn) = &self.conn else { return Ok(Vec::new()) };
        let mut stmt = conn.prepare_cached("SELECT id, feedback_log_body FROM logs WHERE id > ?1 ORDER BY id ASC LIMIT 100")?;
        let mut rows = stmt.query([self.last_log_id])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let body = match row.get_ref(1)? {
                ValueRef::Text(t) | ValueRef::Blob(t) => String::from_utf8_lossy(t).into_owned(),
                _ => String::new(),
            };
            out.push((id, body));
        }
        Ok(out)
    }
}

impl Watcher for CodexWatcher {
    fn poll(&mut self, env: &mut dyn Env) {
        if self.conn.is_none() {
            if self.db_path.is_file() {
                self.connect(env);
            }
            return;
        }
        // Transient locks on the database are ignored; the next tick retries from the same id
        let Ok(rows) = self.fetch() else { return };
        for (id, body) in rows {
            self.last_log_id = self.last_log_id.max(id);
            self.state.on_row(id, &body, env);
        }
    }

    fn deadline(&self) -> Option<i64> {
        self.state.deadline()
    }

    fn is_idle(&self) -> bool {
        !self.state.active && self.state.completion.is_none()
    }

    fn fire(&mut self, env: &mut dyn Env) {
        self.state.fire(env);
    }
}

pub fn db_path() -> PathBuf {
    common::env_path_or("WAITPLAY_CODEX_DB", || common::home_dir().join(".codex").join("logs_2.sqlite"))
}

pub fn run_watch() -> ! {
    let path = db_path();
    println!("=======================================================");
    println!("  Vibe Play Companion Watcher for Codex & ChatGPT");
    println!("  Codex Logs DB: {}", path.display());
    println!("  Bridge Target: http://127.0.0.1:{}", paths::bridge_port());
    println!("=======================================================");
    let mut env = RealEnv::new("codex", SESSION);
    let mut watcher = CodexWatcher::new(path.clone());
    if path.is_file() {
        if !watcher.connect(&mut env) {
            eprintln!("[Vibe Play Watcher] Failed to open Codex DB initially: {}", path.display());
        }
    } else {
        eprintln!("[Vibe Play Watcher] Codex DB not found at {}. Waiting for it to appear...", path.display());
    }
    run_loop(POLL_MS, &mut watcher, &mut env)
}

// ---------------------------------------------------------------------------------------------------------------------
// codex-test-turn

/// The three-step simulated Codex turn: started, running a tool at +3 s, completed at +7 s.
pub fn run_test_turn() {
    fn post(event: &str, data: Option<serde_json::Map<String, Value>>) {
        common::post_event_quiet(&common::event_payload("codex", SESSION, event, None, None, data));
    }
    println!(">>> [1/3] Simulating: User gives prompt to ChatGPT / Codex...");
    println!(">>> ChatGPT starts reasoning and searching...");
    post("task_started", None);

    println!(">>> Popping Native Vibe Play game window and forcing to top-most...");
    // No arguments: exactly what the old script did (the app opens its default window)
    common::spawn_app_raw(&[]);
    std::thread::sleep(Duration::from_millis(3000));

    println!(">>> [2/3] Agent is still thinking and running tools (game stays open)...");
    post("task_running", Some(common::obj(vec![("tool", json!("WebSearch"))])));
    std::thread::sleep(Duration::from_millis(4000));

    println!(">>> [3/3] ChatGPT completed response! Quitting game & closing window...");
    post("task_completed", None);
    std::thread::sleep(Duration::from_millis(1000));
    println!(">>> Test completed cleanly.");
}

// ---------------------------------------------------------------------------------------------------------------------
// popup

/// Replacement for popup.ps1 (`show` / `close`), which opened the companion in an Edge/Chrome `--app` window and found it
/// again by title. The native app is that companion now: `show` launches it (or wakes the running one), `close` writes the
/// quit signal the app watches. Nothing in the repo called popup.ps1; this keeps the two verbs available.
pub fn run_popup(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("show") => {
            if let Some(exe) = paths::app_exe() {
                let _ = process::spawn_detached(&exe, &["--open".to_string()]);
            }
            0
        }
        Some("close") => {
            signals::write_quit_signal();
            0
        }
        _ => {
            eprintln!("Usage: waitplay-hook popup <show|close>");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::mock::MockEnv;

    #[test]
    fn markers() {
        assert!(is_start_marker("foo operation=\"turn_input\" bar"));
        assert!(is_start_marker("TurnInput{..}"));
        assert!(is_start_marker("otel.name=\"op.dispatch.turn_input\""));
        assert!(is_start_marker("x codex.turn_input y"));
        assert!(!is_start_marker("turn_input"));
        assert!(is_progress_marker("run_sampling_request"));
        assert!(is_progress_marker("item/started"));
        assert!(!is_progress_marker("hello"));
        assert!(is_end_marker("app-server event: turn/completed targeted_connections=1"));
        assert!(!is_end_marker("turn/completed targeted_connections=2"));
    }

    #[test]
    fn turn_lifecycle_with_debounce() {
        let mut env = MockEnv::new();
        let mut s = CodexState::default();
        s.on_row(1, "noise", &mut env);
        s.on_row(2, "tool invoke_tool x", &mut env); // ignored while idle
        assert!(env.posts.is_empty());

        s.on_row(3, "operation=\"turn_input\"", &mut env);
        assert_eq!(env.events(), ["task_started"]);
        assert_eq!(env.pops.len(), 1);
        assert_eq!(env.pops[0].session, "codex-session");
        assert!(env.calls.contains(&"unclose:codex-session".to_string()));

        // a second start marker inside the same turn does nothing
        s.on_row(4, "TurnInput", &mut env);
        assert_eq!(env.events(), ["task_started"]);

        s.on_row(5, "invoke_tool", &mut env);
        assert_eq!(env.events(), ["task_started", "task_running"]);

        // end marker schedules, a new step cancels
        s.on_row(6, "turn/completed targeted_connections=1", &mut env);
        assert_eq!(s.deadline(), Some(env.now + 800));
        env.advance(500);
        s.on_row(7, "run_sampling_request", &mut env);
        assert_eq!(s.deadline(), None);
        assert_eq!(env.events().last(), Some(&"task_running"));

        s.on_row(8, "turn/completed targeted_connections=1", &mut env);
        env.advance(799);
        s.fire(&mut env);
        assert_eq!(env.events().last(), Some(&"task_running"));
        env.advance(1);
        s.fire(&mut env);
        assert_eq!(env.events().last(), Some(&"task_completed"));
        assert!(env.calls.contains(&"close:codex-session".to_string()));
        assert!(!s.active);
        assert_eq!(s.deadline(), None);
        // order: completed event, then the close signal
        let n = env.calls.len();
        assert_eq!(&env.calls[n - 2..], ["post:task_completed", "close:codex-session"]);

        // the next prompt starts a fresh turn
        s.on_row(9, "codex.turn_input", &mut env);
        assert_eq!(env.events().last(), Some(&"task_started"));
    }

    #[test]
    fn disabled_or_dismissed_skips_the_popup_but_still_reports() {
        let mut env = MockEnv::new();
        env.enabled = false;
        let mut s = CodexState::default();
        s.on_row(1, "TurnInput", &mut env);
        assert_eq!(env.events(), ["task_started"]);
        assert!(env.pops.is_empty());

        let mut env = MockEnv::new();
        env.dismissed.insert("codex-session".into());
        let mut s = CodexState::default();
        // starting a turn clears the dismissal first (like the watcher), so the popup is not skipped
        env.calls.clear();
        s.on_row(1, "TurnInput", &mut env);
        assert!(env.calls.iter().any(|c| c == "clear_dismissal:codex-session"));
        assert_eq!(env.pops.len(), 1);
    }

    #[test]
    fn end_marker_ignored_when_idle() {
        let mut env = MockEnv::new();
        let mut s = CodexState::default();
        s.on_row(1, "turn/completed targeted_connections=1", &mut env);
        assert_eq!(s.deadline(), None);
    }

    #[test]
    fn reads_a_real_sqlite_log_database() {
        let dir = std::env::temp_dir().join(format!("waitplay-codex-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("logs_2.sqlite");
        let _ = std::fs::remove_file(&path);
        let rw = Connection::open(&path).unwrap();
        rw.execute_batch("CREATE TABLE logs (id INTEGER PRIMARY KEY, ts INTEGER, feedback_log_body TEXT);").unwrap();
        rw.execute("INSERT INTO logs (id, ts, feedback_log_body) VALUES (1, 0, 'old TurnInput')", []).unwrap();

        let mut env = MockEnv::new();
        let mut w = CodexWatcher::new(path.clone());
        w.poll(&mut env); // connects, baseline = 1: history is not replayed
        assert!(env.posts.is_empty());
        w.poll(&mut env);
        assert!(env.posts.is_empty());

        rw.execute("INSERT INTO logs (id, ts, feedback_log_body) VALUES (2, 0, 'operation=\"turn_input\"')", []).unwrap();
        rw.execute("INSERT INTO logs (id, ts, feedback_log_body) VALUES (3, 0, NULL)", []).unwrap();
        rw.execute("INSERT INTO logs (id, ts, feedback_log_body) VALUES (4, 0, 'invoke_tool')", []).unwrap();
        w.poll(&mut env);
        assert_eq!(env.events(), ["task_started", "task_running"]);
        w.poll(&mut env);
        assert_eq!(env.events().len(), 2); // nothing replayed
        drop(w);
        drop(rw);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
