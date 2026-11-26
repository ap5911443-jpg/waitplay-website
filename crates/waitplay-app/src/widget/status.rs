//! The colour of the secondary button's status dot, from the bridge's session list. Port of `ParseSessions`.

use serde_json::Value;

/// A session counts only while it is this fresh.
const FRESH_MS: i64 = 15_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dot {
    /// Vibe Play is switched off.
    Paused,
    /// On, nothing to report: teal.
    Active,
    /// An agent is working.
    Working,
    /// An agent is waiting for the user.
    Waiting,
}

impl Dot {
    pub fn hex(self) -> &'static str {
        match self {
            Dot::Paused => "#777777",
            Dot::Active => "#639085",
            Dot::Working => "#60A5FA",
            Dot::Waiting => "#FBBF24",
        }
    }
}

/// The dot for a `GET /sessions` body: working wins over waiting. `None` when the body is not a session list (the dot then
/// stays as it was, as in the C# widget).
pub fn dot_for_sessions(json: &str, now_ms: i64) -> Option<Dot> {
    let root: Value = serde_json::from_str(json).ok()?;
    let sessions = root.get("sessions")?.as_array()?;
    let (mut working, mut waiting) = (false, false);
    for s in sessions {
        let Some(s) = s.as_object() else { continue };
        let status = s.get("status").and_then(Value::as_str).unwrap_or("");
        let at = s.get("lastEventAt").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(0);
        if now_ms - at > FRESH_MS {
            continue;
        }
        match status {
            "WORKING" | "GAME_ACTIVE" => working = true,
            "WAITING_FOR_INPUT" => waiting = true,
            _ => {}
        }
    }
    Some(if working {
        Dot::Working
    } else if waiting {
        Dot::Waiting
    } else {
        Dot::Active
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_100_000;

    fn body(items: &[(&str, i64)]) -> String {
        let list: Vec<String> = items.iter().map(|(s, t)| format!(r#"{{"sessionId":"a","status":"{}","lastEventAt":{}}}"#, s, t)).collect();
        format!(r#"{{"sessions":[{}]}}"#, list.join(","))
    }

    #[test]
    fn colours_are_the_design_tokens() {
        assert_eq!(Dot::Active.hex(), "#639085");
        assert_eq!(Dot::Working.hex(), "#60A5FA");
        assert_eq!(Dot::Waiting.hex(), "#FBBF24");
        assert_eq!(Dot::Paused.hex(), "#777777");
    }

    #[test]
    fn maps_status_to_dot() {
        assert_eq!(dot_for_sessions(&body(&[]), NOW), Some(Dot::Active));
        assert_eq!(dot_for_sessions(&body(&[("WORKING", NOW - 1000)]), NOW), Some(Dot::Working));
        assert_eq!(dot_for_sessions(&body(&[("GAME_ACTIVE", NOW - 1000)]), NOW), Some(Dot::Working));
        assert_eq!(dot_for_sessions(&body(&[("WAITING_FOR_INPUT", NOW - 1000)]), NOW), Some(Dot::Waiting));
        assert_eq!(dot_for_sessions(&body(&[("IDLE", NOW), ("COMPLETED", NOW), ("FAILED", NOW)]), NOW), Some(Dot::Active));
        // working beats waiting whatever the order
        assert_eq!(dot_for_sessions(&body(&[("WAITING_FOR_INPUT", NOW), ("WORKING", NOW)]), NOW), Some(Dot::Working));
        assert_eq!(dot_for_sessions(&body(&[("WORKING", NOW), ("WAITING_FOR_INPUT", NOW)]), NOW), Some(Dot::Working));
    }

    #[test]
    fn stale_sessions_do_not_count() {
        assert_eq!(dot_for_sessions(&body(&[("WORKING", NOW - 15_001)]), NOW), Some(Dot::Active));
        assert_eq!(dot_for_sessions(&body(&[("WORKING", NOW - 15_000)]), NOW), Some(Dot::Working));
        assert_eq!(dot_for_sessions(r#"{"sessions":[{"status":"WORKING","lastEventAt":null}]}"#, NOW), Some(Dot::Active));
        assert_eq!(dot_for_sessions(r#"{"sessions":[{"status":"WORKING"}]}"#, NOW), Some(Dot::Active));
        assert_eq!(dot_for_sessions(r#"{"sessions":[{"status":"WORKING","lastEventAt":1700000099000.0}]}"#, NOW), Some(Dot::Working));
    }

    #[test]
    fn not_a_session_list_changes_nothing() {
        assert_eq!(dot_for_sessions("", NOW), None);
        assert_eq!(dot_for_sessions("<html>", NOW), None);
        assert_eq!(dot_for_sessions(r#"{"status":"ok"}"#, NOW), None);
        assert_eq!(dot_for_sessions(r#"{"sessions":"x"}"#, NOW), None);
        assert_eq!(dot_for_sessions(r#"{"sessions":[1,"a",null]}"#, NOW), Some(Dot::Active));
    }
}
