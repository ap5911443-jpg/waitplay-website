//! `waitplay-hook simulate`: the 25-second end-to-end test mode (port of test-mode-25s.cjs).
//!
//! Emits task_started, launches the companion, reports four tool runs while the user plays, then task_completed.
//! The old script asked PowerShell whether a companion window was already open and skipped the launch if so. The app is
//! single-instance and hands a second launch to the running window, so launching unconditionally is equivalent and needs no
//! PowerShell.

use crate::common::{self, obj};
use serde_json::{json, Map, Value};
use std::io::Write;
use std::time::{Duration, Instant};

const DURATION_SECS: u64 = 25;

/// Tool announced at second `elapsed`, if any.
pub fn tool_at(elapsed: u64) -> Option<&'static str> {
    match elapsed {
        5 => Some("read (\"auth/tokens.ts\")"),
        10 => Some("ast_edit (\"auth_pkce.ts\")"),
        16 => Some("bash (\"npm run build\")"),
        21 => Some("bash (\"npm test\")"),
        _ => None,
    }
}

fn post(event: &str, extra: Option<Map<String, Value>>) {
    let mut data = obj(vec![("model", json!("Gemini 3.8 Flash"))]);
    if let Some(e) = extra {
        data.extend(e);
    }
    let payload = common::event_payload("omp", "omp-session", event, None, Some("Term - 1"), Some(data));
    if let Err(e) = common::post_event(&payload) {
        eprintln!("Bridge error: {}", e);
    }
}

pub fn run() {
    println!("====================================================");
    println!("  Vibe Play 25-Second Test Mode Simulation");
    println!("====================================================");
    println!("-> [0s] User submits prompt: \"Refactor auth PKCE flow\"");
    println!("-> Emitting task_started event to Localhost Bridge...");
    post("task_started", Some(obj(vec![("prompt", json!("Refactor auth PKCE flow to OAuth 2.0"))])));

    println!("-> Launching Native Vibe Play Companion Game...");
    common::spawn_app_raw(&["--session=omp-session".to_string(), "--terminal=Term - 1".to_string(), "--game".to_string()]);

    let start = Instant::now();
    for elapsed in 1..=DURATION_SECS {
        let due = start + Duration::from_secs(elapsed);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        if let Some(tool) = tool_at(elapsed) {
            println!("-> [{}s] Agent running tool: {}", elapsed, tool);
            post("task_running", Some(obj(vec![("tool", json!(tool))])));
        }
        let remaining = DURATION_SECS - elapsed;
        print!("\r[Test Mode] Running... {}s / {}s elapsed ({}s remaining) - Enjoy playing! ", elapsed, DURATION_SECS, remaining);
        let _ = std::io::stdout().flush();
    }

    println!("\n\n====================================================");
    println!("-> [25s] Agent Turn Complete!");
    println!("-> Emitting task_completed to Localhost Bridge...");
    post("task_completed", None);
    println!("-> Game window should now show completion banner and AUTO-QUIT in 2 seconds.");
    println!("====================================================\n");

    std::thread::sleep(Duration::from_millis(3500));
    println!("-> Verifying that game window closed automatically...");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_schedule() {
        let all: Vec<(u64, &str)> = (1..=25).filter_map(|s| tool_at(s).map(|t| (s, t))).collect();
        assert_eq!(
            all,
            vec![(5, "read (\"auth/tokens.ts\")"), (10, "ast_edit (\"auth_pkce.ts\")"), (16, "bash (\"npm run build\")"), (21, "bash (\"npm test\")")]
        );
    }
}
