//! The AI agents Vibe Play can listen to, whether they are present on this PC, and how each one is connected (Windows copy of
//! `mac/integrations.rs`: same logic, Windows folder names and Windows-safe hook command quoting).
//!
//! * Claude Code: hooks in `%USERPROFILE%\.claude\settings.json` (installed and removed here, idempotently, preserving everything
//!   else). Hooks written by the earlier Node-based integration (`waitplay-hook.cjs`) are recognised as Vibe Play's own and replaced.
//! * OpenCode, Codex, OMP, Cline: a watcher (`waitplay-hook <agent>-watch`) that Vibe Play runs while the agent is present.
//! * Cursor, Antigravity: IDE hooks the user wires up by hand (instructions shown in Settings).

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Hooks written into the agent's own settings file by Vibe Play.
    ClaudeHooks,
    /// A long-running `waitplay-hook <command>` that Vibe Play supervises.
    Watcher(&'static str),
    /// The user adds the hook themselves.
    Manual(&'static str),
}

pub struct AgentSpec {
    pub id: &'static str,
    pub name: &'static str,
    /// Folders (relative to the home directory) whose presence means the agent is installed. Any one is enough.
    pub detect: &'static [&'static str],
    pub kind: Kind,
}

pub const AGENTS: [AgentSpec; 7] = [
    AgentSpec { id: "claude-code", name: "Claude Code", detect: &[".claude"], kind: Kind::ClaudeHooks },
    AgentSpec { id: "opencode", name: "OpenCode", detect: &[".local/share/opencode", ".config/opencode"], kind: Kind::Watcher("opencode-watch") },
    AgentSpec { id: "codex", name: "Codex", detect: &[".codex"], kind: Kind::Watcher("codex-watch") },
    AgentSpec { id: "omp", name: "OMP", detect: &[".omp"], kind: Kind::Watcher("omp-watch") },
    AgentSpec { id: "cline", name: "Cline", detect: &[".cline"], kind: Kind::Watcher("cline-watch") },
    AgentSpec { id: "cursor", name: "Cursor", detect: &["AppData/Roaming/Cursor", ".cursor"], kind: Kind::Manual("cursor") },
    AgentSpec { id: "antigravity", name: "Antigravity", detect: &["AppData/Roaming/Antigravity", ".antigravity"], kind: Kind::Manual("antigravity") },
];

pub fn spec(id: &str) -> Option<&'static AgentSpec> {
    AGENTS.iter().find(|a| a.id == id)
}

pub fn is_detected(spec: &AgentSpec, home: &Path) -> bool {
    spec.detect.iter().any(|d| home.join(d).exists())
}

/// Agent ids whose watcher is on by default (the user can remove any in Settings).
pub fn default_watchers() -> Vec<String> {
    AGENTS.iter().filter(|a| matches!(a.kind, Kind::Watcher(_))).map(|a| a.id.to_string()).collect()
}

// ── Claude Code hooks ────────────────────────────────────────────────────────────────────────────────────────────────────

/// (hook event, `waitplay-hook claude-code` argument, tool matcher)
const CLAUDE_EVENTS: [(&str, &str, Option<&str>); 6] = [
    ("UserPromptSubmit", "start", None),
    ("PreToolUse", "tool", Some("*")),
    ("PostToolUse", "tool_done", Some("*")),
    ("Stop", "stop", None),
    ("Notification", "notify", None),
    ("SessionEnd", "end", None),
];

pub fn claude_settings_path(home: &Path) -> PathBuf {
    home.join(".claude").join("settings.json")
}

/// A command line that runs the hook executable from Claude Code's hook shell (Git Bash on Windows) and from cmd: double quotes and
/// forward slashes, which both accept.
fn shell_quote(p: &Path) -> String {
    format!("\"{}\"", p.to_string_lossy().replace('\\', "/"))
}

/// Characters a double-quoted path cannot carry safely through every shell. Windows paths practically never contain them; refusing
/// is better than writing a hook command that means something else.
fn quotable(p: &Path) -> Result<(), String> {
    let text = p.to_string_lossy();
    match text.chars().find(|c| matches!(c, '"' | '$' | '`' | '%' | '!')) {
        Some(c) => Err(format!("{} contains the character {} that hook commands cannot quote; move Vibe Play to another folder", text, c)),
        None => Ok(()),
    }
}

fn claude_command(hook_exe: &Path, arg: &str) -> String {
    format!("{} claude-code {}", shell_quote(hook_exe), arg)
}

/// A hook command that Vibe Play wrote (whichever location it pointed at, including the earlier Node-based hook script).
fn is_ours(command: &str) -> bool {
    (command.contains("waitplay-hook") && command.contains(" claude-code ")) || command.contains("waitplay-hook.cjs")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaudeState {
    NotConfigured,
    Configured,
    /// Hooks exist but point at another Vibe Play location (the app moved): installing again fixes it.
    Outdated,
    /// The settings file cannot be read as JSON: Vibe Play leaves it alone.
    Error(String),
}

fn read_settings(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str::<Value>(&text).map_err(|e| format!("{} is not valid JSON ({})", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("cannot read {}: {}", path.display(), e)),
    }
}

fn commands_in(entry: &Value) -> Vec<&str> {
    entry.get("hooks").and_then(Value::as_array).map(|h| h.iter().filter_map(|c| c.get("command").and_then(Value::as_str)).collect()).unwrap_or_default()
}

pub fn claude_state(settings: &Path, hook_exe: &Path) -> ClaudeState {
    let root = match read_settings(settings) {
        Ok(v) => v,
        Err(e) => return ClaudeState::Error(e),
    };
    let mut found = 0;
    let mut current = 0;
    for (event, arg, _) in CLAUDE_EVENTS {
        let wanted = claude_command(hook_exe, arg);
        let entries = root.get("hooks").and_then(|h| h.get(event)).and_then(Value::as_array).cloned().unwrap_or_default();
        let cmds: Vec<&str> = entries.iter().flat_map(commands_in).collect();
        if cmds.iter().any(|c| is_ours(c)) {
            found += 1;
        }
        if cmds.contains(&wanted.as_str()) {
            current += 1;
        }
    }
    match (found, current) {
        (0, _) => ClaudeState::NotConfigured,
        (_, c) if c == CLAUDE_EVENTS.len() => ClaudeState::Configured,
        _ => ClaudeState::Outdated,
    }
}

fn strip_ours(entries: &mut Vec<Value>) {
    for e in entries.iter_mut() {
        if let Some(h) = e.get_mut("hooks").and_then(Value::as_array_mut) {
            h.retain(|c| !c.get("command").and_then(Value::as_str).map_or(false, is_ours));
        }
    }
    entries.retain(|e| e.get("hooks").and_then(Value::as_array).map_or(true, |h| !h.is_empty()));
}

fn write_settings(path: &Path, root: &Value) -> Result<(), String> {
    if path.exists() {
        let backup = path.with_extension("json.waitplay-backup");
        if !backup.exists() {
            // never touch the user's file without a copy of it
            std::fs::copy(path, &backup).map_err(|e| format!("cannot back up {} to {}: {}", path.display(), backup.display(), e))?;
        }
    }
    let body = serde_json::to_vec_pretty(root).map_err(|e| e.to_string())?;
    waitplay_core::paths::write_atomic(path, &body).map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

/// Adds (or refreshes) the five hooks. Everything else in the file is kept. An unreadable file is never overwritten.
pub fn claude_install(settings: &Path, hook_exe: &Path) -> Result<(), String> {
    quotable(hook_exe)?;
    let mut root = read_settings(settings)?;
    let obj = root.as_object_mut().ok_or_else(|| format!("{} is not a JSON object", settings.display()))?;
    let hooks = obj.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in the settings file is not an object")?;
    for (event, arg, matcher) in CLAUDE_EVENTS {
        let list = hooks.entry(event).or_insert_with(|| Value::Array(Vec::new()));
        let list = list.as_array_mut().ok_or_else(|| format!("\"hooks.{}\" is not a list", event))?;
        strip_ours(list);
        let mut entry = Map::new();
        if let Some(m) = matcher {
            entry.insert("matcher".into(), json!(m));
        }
        entry.insert("hooks".into(), json!([{ "type": "command", "command": claude_command(hook_exe, arg) }]));
        list.push(Value::Object(entry));
    }
    write_settings(settings, &root)
}

/// Removes Vibe Play's hooks and nothing else.
pub fn claude_remove(settings: &Path) -> Result<(), String> {
    if !settings.exists() {
        return Ok(());
    }
    let mut root = read_settings(settings)?;
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else { return Ok(()) };
    for (event, _, _) in CLAUDE_EVENTS {
        if let Some(list) = hooks.get_mut(event).and_then(Value::as_array_mut) {
            strip_ours(list);
        }
        if hooks.get(event).and_then(Value::as_array).map_or(false, Vec::is_empty) {
            hooks.remove(event);
        }
    }
    if hooks.is_empty() {
        root.as_object_mut().map(|o| o.remove("hooks"));
    }
    write_settings(settings, &root)
}

/// Instructions for agents that are wired by hand.
pub fn how_to(spec: &AgentSpec, hook_exe: &Path) -> String {
    match spec.kind {
        Kind::Manual(cmd) => format!(
            "Add these commands to {}'s hooks:\n{exe} {cmd} start\n{exe} {cmd} tool\n{exe} {cmd} stop",
            spec.name,
            exe = shell_quote(hook_exe),
            cmd = cmd
        ),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // next to the test binary, not in TMPDIR: another test in this process rewrites TMPDIR and deletes that folder
        let base = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let p = base.join(format!("wp-integ-{}-{}-{}", tag, std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    const EXE: &str = "C:/Program Files/WaitPlay/waitplay-hook.exe";

    #[test]
    fn install_is_idempotent_and_keeps_the_users_settings() {
        let dir = scratch("a");
        let f = dir.join("settings.json");
        std::fs::write(
            &f,
            r#"{"model":"opus","hooks":{"Stop":[{"hooks":[{"type":"command","command":"say done"}]}],"PostToolUse":[{"matcher":"Edit","hooks":[{"type":"command","command":"fmt"}]}]}}"#,
        )
        .unwrap();
        let exe = Path::new(EXE);
        assert_eq!(claude_state(&f, exe), ClaudeState::NotConfigured);
        claude_install(&f, exe).unwrap();
        claude_install(&f, exe).unwrap();
        assert_eq!(claude_state(&f, exe), ClaudeState::Configured);

        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["model"], "opus");
        assert_eq!(v["hooks"]["PostToolUse"][0]["hooks"][0]["command"], "fmt");
        let stop = v["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2, "the user's own Stop hook plus exactly one of ours: {:?}", stop);
        assert!(stop.iter().any(|e| e["hooks"][0]["command"] == "say done"));
        assert_eq!(v["hooks"]["PreToolUse"][0]["matcher"], "*");
        assert!(dir.join("settings.json.waitplay-backup").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_takes_only_ours_and_prunes_empty_containers() {
        let dir = scratch("b");
        let f = dir.join("settings.json");
        let exe = Path::new(EXE);
        claude_install(&f, exe).unwrap();
        claude_remove(&f).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert!(v.get("hooks").is_none(), "nothing of ours or anyone's left: {}", v);
        assert_eq!(claude_state(&f, exe), ClaudeState::NotConfigured);

        std::fs::write(&f, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"say done"}]}]}}"#).unwrap();
        claude_install(&f, exe).unwrap();
        claude_remove(&f).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["hooks"]["Stop"].as_array().unwrap().len(), 1);
        assert_eq!(v["hooks"]["Stop"][0]["hooks"][0]["command"], "say done");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn moved_app_is_outdated_until_reinstalled() {
        let dir = scratch("c");
        let f = dir.join("settings.json");
        claude_install(&f, Path::new("/old/WaitPlay.app/Contents/MacOS/waitplay-hook")).unwrap();
        let new = Path::new(EXE);
        assert_eq!(claude_state(&f, new), ClaudeState::Outdated);
        claude_install(&f, new).unwrap();
        assert_eq!(claude_state(&f, new), ClaudeState::Configured);
        let text = std::fs::read_to_string(&f).unwrap();
        assert!(!text.contains("/old/"), "the stale commands are replaced, not duplicated");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_settings_are_never_overwritten() {
        let dir = scratch("d");
        let f = dir.join("settings.json");
        std::fs::write(&f, "{ broken").unwrap();
        let exe = Path::new(EXE);
        let state = claude_state(&f, exe);
        assert!(matches!(state, ClaudeState::Error(_)), "state was {:?}, file is {:?}", state, std::fs::read_to_string(&f));
        assert!(claude_install(&f, exe).is_err());
        assert!(claude_remove(&f).is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "{ broken");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_with_spaces_are_quoted_for_bash_and_cmd() {
        let c = claude_command(Path::new(r"C:\Users\o'neil\My Apps\waitplay-hook.exe"), "start");
        assert_eq!(c, r#""C:/Users/o'neil/My Apps/waitplay-hook.exe" claude-code start"#);
        assert!(is_ours(&c));
        assert!(quotable(Path::new(r"C:\Users\o'neil\a b\waitplay-hook.exe")).is_ok());
        assert!(quotable(Path::new(r"C:\a$b\waitplay-hook.exe")).is_err());
        assert!(quotable(Path::new("C:\\a%b\\waitplay-hook.exe")).is_err());
    }

    #[test]
    fn earlier_node_hooks_are_replaced_not_duplicated() {
        let dir = scratch("f");
        let f = dir.join("settings.json");
        std::fs::write(
            &f,
            r#"{"hooks":{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"node \"C:\\Temp\\waitplay\\integrations\\claude-code\\waitplay-hook.cjs\" stop"}]}]}}"#,
        )
        .unwrap();
        let exe = Path::new(EXE);
        assert_eq!(claude_state(&f, exe), ClaudeState::Outdated);
        claude_install(&f, exe).unwrap();
        assert_eq!(claude_state(&f, exe), ClaudeState::Configured);
        assert!(!std::fs::read_to_string(&f).unwrap().contains("waitplay-hook.cjs"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hook_path_that_cannot_be_quoted_is_refused_and_the_file_untouched() {
        let dir = scratch("g");
        let f = dir.join("settings.json");
        std::fs::write(&f, r#"{"model":"opus"}"#).unwrap();
        assert!(claude_install(&f, Path::new(r"C:\a$b\waitplay-hook.exe")).is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), r#"{"model":"opus"}"#);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detection_looks_for_the_agents_folders() {
        let home = scratch("e");
        std::fs::create_dir_all(home.join(".local/share/opencode")).unwrap();
        assert!(is_detected(spec("opencode").unwrap(), &home));
        assert!(!is_detected(spec("claude-code").unwrap(), &home));
        assert_eq!(default_watchers(), ["opencode", "codex", "omp", "cline"]);
        let _ = std::fs::remove_dir_all(&home);
    }
}
