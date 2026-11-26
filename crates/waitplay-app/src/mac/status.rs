//! What the menu bar, the widget and the settings window say about the agents: derived from the bridge's `GET /sessions` body plus
//! three flags. Pure, so every rule is unit tested.

use crate::island::Act;
use serde_json::Value;

/// A working session counts only while it is this fresh (hooks and watchers repeat events well inside it).
const FRESH_WORKING_MS: i64 = 15_000;
/// A session that waits for the user stays waiting until the next event: far longer.
const FRESH_WAITING_MS: i64 = 30 * 60_000;
/// A finished or failed session stays in the island's list this long.
const FRESH_DONE_MS: i64 = 90_000;
/// Rows the island's list can hold.
const MAX_ROWS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Working,
    Waiting,
}

impl Activity {
    pub fn as_str(self) -> &'static str {
        match self {
            Activity::Idle => "idle",
            Activity::Working => "working",
            Activity::Waiting => "waiting",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRow {
    /// `claude-code`, `opencode`, ...
    pub id: String,
    pub name: String,
    pub activity: Activity,
    /// Terminal label reported by the hook, when there is one.
    pub terminal: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SessionsView {
    /// One row per agent that has a session on the bridge (the strongest activity of its sessions).
    pub agents: Vec<AgentRow>,
    /// Sessions (terminals / conversations) that are working right now, and working or waiting for the user.
    pub working_sessions: usize,
    pub active_sessions: usize,
    /// When the longest-running working session started its current task (ms since the epoch), straight from the bridge's
    /// `currentTaskStartedAt`; `None` when nothing is working or the bridge did not say.
    pub working_since: Option<i64>,
    /// Every live session, one row each: working and waiting first, then those that just finished or failed.
    pub sessions: Vec<SessionRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRow {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub act: Act,
    /// Working or waiting: when the current task started. Finished: nothing.
    pub since: Option<i64>,
    /// Finished: how long the last task took.
    pub duration_ms: Option<i64>,
    pub terminal: String,
    /// Time of the last event, for ordering only.
    pub at: i64,
}

impl SessionsView {
    pub fn working(&self) -> Vec<&AgentRow> {
        self.agents.iter().filter(|a| a.activity == Activity::Working).collect()
    }
    pub fn waiting(&self) -> Vec<&AgentRow> {
        self.agents.iter().filter(|a| a.activity == Activity::Waiting).collect()
    }
    pub fn activity_of(&self, id: &str) -> Activity {
        self.agents.iter().find(|a| a.id == id).map_or(Activity::Idle, |a| a.activity)
    }
}

pub fn agent_name(id: &str) -> String {
    match id {
        "claude-code" => "Claude Code",
        "opencode" => "OpenCode",
        "codex" => "Codex",
        "omp" => "OMP",
        "cline" => "Cline",
        "cursor" => "Cursor",
        "antigravity" => "Antigravity",
        other => other,
    }
    .to_string()
}

/// `None` when the body is not a session list.
pub fn parse_sessions(json: &str, now_ms: i64) -> Option<SessionsView> {
    let root: Value = serde_json::from_str(json).ok()?;
    let sessions = root.get("sessions")?.as_array()?;
    let mut agents: Vec<AgentRow> = Vec::new();
    let (mut working_sessions, mut active_sessions) = (0usize, 0usize);
    let mut working_since: Option<i64> = None;
    let mut rows: Vec<SessionRow> = Vec::new();
    for s in sessions.iter().filter_map(Value::as_object) {
        let id = s.get("agent").and_then(Value::as_str).unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let status = s.get("status").and_then(Value::as_str).unwrap_or("");
        let at = s.get("lastEventAt").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(0);
        let age = now_ms - at;
        let activity = match status {
            "WORKING" | "GAME_ACTIVE" if age <= FRESH_WORKING_MS => Activity::Working,
            "WAITING_FOR_INPUT" if age <= FRESH_WAITING_MS => Activity::Waiting,
            _ => Activity::Idle,
        };
        let num = |k: &str| s.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).filter(|t| *t > 0);
        let row_act = match status {
            "WORKING" | "GAME_ACTIVE" if age <= FRESH_WORKING_MS => Some(Act::Working),
            "WAITING_FOR_INPUT" if age <= FRESH_WAITING_MS => Some(Act::Waiting),
            "COMPLETED" if age <= FRESH_DONE_MS => Some(Act::Done),
            "FAILED" if age <= FRESH_DONE_MS => Some(Act::Failed),
            _ => None,
        };
        if let Some(act) = row_act {
            let sid = s.get("sessionId").and_then(Value::as_str).unwrap_or("");
            let shown = s.get("agentDisplayName").and_then(Value::as_str).filter(|n| !n.is_empty() && n.len() <= 40).map(str::to_string).unwrap_or_else(|| agent_name(&id));
            rows.push(SessionRow {
                id: if sid.is_empty() { id.clone() } else { sid.to_string() },
                agent_id: id.clone(),
                name: shown,
                act,
                since: if act.active() { num("currentTaskStartedAt").filter(|t| *t <= now_ms) } else { None },
                duration_ms: if act.active() { None } else { num("lastTaskDurationMs") },
                terminal: s.get("terminal").and_then(Value::as_str).unwrap_or("").to_string(),
                at,
            });
        }
        let terminal = s.get("terminal").and_then(Value::as_str).unwrap_or("").to_string();
        if activity == Activity::Working {
            working_sessions += 1;
            let began = s.get("currentTaskStartedAt").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).filter(|t| *t > 0 && *t <= now_ms);
            working_since = match (working_since, began) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        if activity != Activity::Idle {
            active_sessions += 1;
        }
        match agents.iter_mut().find(|a| a.id == id) {
            Some(row) => {
                // working beats waiting beats idle
                if rank(activity) > rank(row.activity) {
                    row.activity = activity;
                }
                if row.terminal.is_empty() {
                    row.terminal = terminal;
                }
            }
            None => agents.push(AgentRow { name: agent_name(&id), id, activity, terminal }),
        }
    }
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    rows.sort_by(|a, b| (!a.act.active()).cmp(&!b.act.active()).then(b.at.cmp(&a.at)).then(a.id.cmp(&b.id)));
    rows.truncate(MAX_ROWS);
    Some(SessionsView { agents, working_sessions, active_sessions, working_since, sessions: rows })
}

fn rank(a: Activity) -> u8 {
    match a {
        Activity::Idle => 0,
        Activity::Waiting => 1,
        Activity::Working => 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Working,
    Entertainment,
    Paused,
    Error,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Idle => "idle",
            Status::Working => "working",
            Status::Entertainment => "entertainment",
            Status::Paused => "paused",
            Status::Error => "error",
        }
    }
}

/// Paused beats everything (the user turned automation off); a dead bridge is an error; an open entertainment window beats
/// "working"; then working/waiting; else idle.
pub fn overall(enabled: bool, bridge_ok: bool, view: &SessionsView, entertainment_open: bool) -> Status {
    if !enabled {
        Status::Paused
    } else if !bridge_ok {
        Status::Error
    } else if entertainment_open {
        Status::Entertainment
    } else if view.agents.iter().any(|a| a.activity != Activity::Idle) {
        Status::Working
    } else {
        Status::Idle
    }
}

fn join_names(rows: &[&AgentRow]) -> String {
    match rows {
        [] => String::new(),
        [a] => a.name.clone(),
        [a, b] => format!("{} and {}", a.name, b.name),
        [a, rest @ ..] => format!("{} and {} more", a.name, rest.len()),
    }
}

/// (title, subtitle) for the widget and the menu.
pub fn describe(status: Status, view: &SessionsView, platform_label: &str) -> (String, String) {
    let working = view.working();
    let waiting = view.waiting();
    match status {
        Status::Paused => ("Paused".into(), "Automation is off".into()),
        Status::Error => ("Needs attention".into(), "Local bridge is not running".into()),
        Status::Idle => ("Idle".into(), "Waiting for an agent".into()),
        Status::Working | Status::Entertainment => {
            let (title, detail) = if view.working_sessions >= 2 {
                (format!("{} agents working", view.working_sessions), join_names(&working))
            } else if !working.is_empty() {
                (format!("{} is working", join_names(&working)), join_names(&working))
            } else if !waiting.is_empty() {
                (format!("{} needs you", join_names(&waiting)), join_names(&waiting))
            } else {
                ("Entertainment open".to_string(), String::new())
            };
            let sub = if detail.is_empty() { platform_label.to_string() } else { format!("{} \u{b7} {}", detail, platform_label) };
            (title, sub)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_100_000;

    fn body(items: &[(&str, &str, i64)]) -> String {
        let list: Vec<String> =
            items.iter().map(|(agent, st, t)| format!(r#"{{"sessionId":"{}-session","agent":"{}","status":"{}","lastEventAt":{}}}"#, agent, agent, st, t)).collect();
        format!(r#"{{"sessions":[{}]}}"#, list.join(","))
    }

    #[test]
    fn strongest_activity_per_agent_and_stale_working_is_idle() {
        let v = parse_sessions(
            &body(&[("claude-code", "WORKING", NOW - 1000), ("claude-code", "WAITING_FOR_INPUT", NOW - 1000), ("opencode", "WORKING", NOW - 20_000)]),
            NOW,
        )
        .unwrap();
        assert_eq!(v.activity_of("claude-code"), Activity::Working);
        assert_eq!(v.activity_of("opencode"), Activity::Idle, "20 s of silence: the bridge watchdog is about to complete it");
        assert_eq!(v.agents.len(), 2);
    }

    #[test]
    fn running_time_starts_at_the_oldest_current_task_of_a_working_session() {
        let json = format!(
            r#"{{"sessions":[
                {{"sessionId":"a","agent":"claude-code","status":"WORKING","lastEventAt":{},"currentTaskStartedAt":{}}},
                {{"sessionId":"b","agent":"codex","status":"WORKING","lastEventAt":{},"currentTaskStartedAt":{}}},
                {{"sessionId":"c","agent":"omp","status":"WAITING_FOR_INPUT","lastEventAt":{},"currentTaskStartedAt":{}}},
                {{"sessionId":"d","agent":"cline","status":"WORKING","lastEventAt":{}}}]}}"#,
            NOW - 500,
            NOW - 90_000,
            NOW - 500,
            NOW - 30_000,
            NOW - 500,
            NOW - 600_000,
            NOW - 500
        );
        let v = parse_sessions(&json, NOW).unwrap();
        assert_eq!(v.working_since, Some(NOW - 90_000), "the waiting session and the one without a start do not count");
        assert_eq!(parse_sessions(&body(&[("codex", "WORKING", NOW - 500)]), NOW).unwrap().working_since, None);
        assert_eq!(parse_sessions(&body(&[("codex", "WAITING_FOR_INPUT", NOW - 500)]), NOW).unwrap().working_since, None);
    }

    #[test]
    fn waiting_outlasts_the_working_freshness() {
        let v = parse_sessions(&body(&[("codex", "WAITING_FOR_INPUT", NOW - 5 * 60_000)]), NOW).unwrap();
        assert_eq!(v.activity_of("codex"), Activity::Waiting);
    }

    #[test]
    fn garbage_is_not_a_session_list() {
        assert!(parse_sessions("nope", NOW).is_none());
        assert!(parse_sessions(r#"{"sessions":3}"#, NOW).is_none());
        assert_eq!(parse_sessions(r#"{"sessions":[]}"#, NOW).unwrap(), SessionsView::default());
    }

    #[test]
    fn overall_priorities() {
        let idle = SessionsView::default();
        let busy = parse_sessions(&body(&[("omp", "WORKING", NOW)]), NOW).unwrap();
        assert_eq!(overall(false, false, &busy, true), Status::Paused);
        assert_eq!(overall(true, false, &busy, true), Status::Error);
        assert_eq!(overall(true, true, &busy, true), Status::Entertainment);
        assert_eq!(overall(true, true, &busy, false), Status::Working);
        assert_eq!(overall(true, true, &idle, false), Status::Idle);
    }

    #[test]
    fn descriptions_name_the_agents() {
        let one = parse_sessions(&body(&[("claude-code", "WORKING", NOW)]), NOW).unwrap();
        assert_eq!(describe(Status::Working, &one, "Instagram"), ("Claude Code is working".into(), "Claude Code \u{b7} Instagram".into()));
        let two = parse_sessions(&body(&[("claude-code", "WORKING", NOW), ("omp", "WORKING", NOW)]), NOW).unwrap();
        assert_eq!(describe(Status::Entertainment, &two, "X").0, "2 agents working");
        assert_eq!(two.working_sessions, 2);
        // two conversations of the same agent count as two
        let same = parse_sessions(&body(&[("claude-code", "WORKING", NOW), ("claude-code", "WORKING", NOW)]), NOW).unwrap();
        assert_eq!((same.working_sessions, same.active_sessions, same.agents.len()), (2, 2, 1));
        assert_eq!(describe(Status::Working, &same, "X").0, "2 agents working");
        let waiting = parse_sessions(&body(&[("codex", "WAITING_FOR_INPUT", NOW)]), NOW).unwrap();
        assert_eq!(describe(Status::Working, &waiting, "X").0, "Codex needs you");
        assert_eq!(describe(Status::Idle, &SessionsView::default(), "X").0, "Idle");
        assert_eq!(describe(Status::Entertainment, &SessionsView::default(), "YouTube"), ("Entertainment open".into(), "YouTube".into()));
    }

    #[test]
    fn every_live_session_is_a_row_with_its_own_state() {
        let json = format!(
            r#"{{"sessions":[
              {{"sessionId":"c1","agent":"claude-code","status":"WORKING","lastEventAt":{n},"currentTaskStartedAt":{s}}},
              {{"sessionId":"c2","agent":"claude-code","status":"WAITING_FOR_INPUT","lastEventAt":{n}}},
              {{"sessionId":"x1","agent":"cursor","status":"COMPLETED","lastEventAt":{d},"lastTaskDurationMs":42000,"agentDisplayName":"Cursor IDE"}},
              {{"sessionId":"x2","agent":"codex","status":"FAILED","lastEventAt":{n}}},
              {{"sessionId":"old","agent":"omp","status":"COMPLETED","lastEventAt":{o}}},
              {{"sessionId":"stale","agent":"omp","status":"WORKING","lastEventAt":{o}}}
            ]}}"#,
            n = NOW - 1_000,
            s = NOW - 65_000,
            d = NOW - 20_000,
            o = NOW - 600_000
        );
        let v = parse_sessions(&json, NOW).unwrap();
        let rows: Vec<(&str, Act)> = v.sessions.iter().map(|r| (r.id.as_str(), r.act)).collect();
        assert_eq!(rows, [("c1", Act::Working), ("c2", Act::Waiting), ("x2", Act::Failed), ("x1", Act::Done)], "active first, then the newest finished; old and stale ones are gone");
        assert_eq!(v.sessions[0].since, Some(NOW - 65_000));
        assert_eq!(v.sessions[3].duration_ms, Some(42_000));
        assert_eq!(v.sessions[3].name, "Cursor IDE");
        assert_eq!(v.sessions[1].name, "Claude Code");
        // two conversations of one agent stay two rows: one finishing never touches the other
        assert_eq!(v.sessions.iter().filter(|r| r.agent_id == "claude-code").count(), 2);
    }
}
