//! The normalized agent event protocol (see docs/EVENT_PROTOCOL.md).
//!
//! The JSON shapes here are the wire contract with the React web app (`apps/web`), which still speaks the original
//! TypeScript types: field names are camelCase, event names snake_case, statuses SCREAMING_SNAKE_CASE.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024;
pub const MAX_SESSION_ID_CHARS: usize = 128;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AgentType {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "omp")]
    Omp,
    #[serde(rename = "opencode")]
    Opencode,
    #[serde(rename = "cline")]
    Cline,
    #[serde(rename = "cursor")]
    Cursor,
    #[serde(rename = "antigravity")]
    Antigravity,
}

impl AgentType {
    pub const ALL: [AgentType; 7] = [
        AgentType::ClaudeCode,
        AgentType::Codex,
        AgentType::Omp,
        AgentType::Opencode,
        AgentType::Cline,
        AgentType::Cursor,
        AgentType::Antigravity,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AgentType::ClaudeCode => "claude-code",
            AgentType::Codex => "codex",
            AgentType::Omp => "omp",
            AgentType::Opencode => "opencode",
            AgentType::Cline => "cline",
            AgentType::Cursor => "cursor",
            AgentType::Antigravity => "antigravity",
        }
    }

    pub fn parse(s: &str) -> Option<AgentType> {
        AgentType::ALL.iter().copied().find(|a| a.as_str() == s)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentEvent {
    SessionStarted,
    TaskStarted,
    TaskRunning,
    WaitingForInput,
    TaskCompleted,
    TaskFailed,
    SessionEnded,
    /// "Still going": keeps a working session alive while the agent is silent (a long tool, a long answer). Changes no status.
    Heartbeat,
}

impl AgentEvent {
    pub const ALL: [AgentEvent; 8] = [
        AgentEvent::SessionStarted,
        AgentEvent::TaskStarted,
        AgentEvent::TaskRunning,
        AgentEvent::WaitingForInput,
        AgentEvent::TaskCompleted,
        AgentEvent::TaskFailed,
        AgentEvent::SessionEnded,
        AgentEvent::Heartbeat,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AgentEvent::SessionStarted => "session_started",
            AgentEvent::TaskStarted => "task_started",
            AgentEvent::TaskRunning => "task_running",
            AgentEvent::WaitingForInput => "waiting_for_input",
            AgentEvent::TaskCompleted => "task_completed",
            AgentEvent::TaskFailed => "task_failed",
            AgentEvent::SessionEnded => "session_ended",
            AgentEvent::Heartbeat => "heartbeat",
        }
    }

    pub fn parse(s: &str) -> Option<AgentEvent> {
        AgentEvent::ALL.iter().copied().find(|e| e.as_str() == s)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentStatus {
    Idle,
    Working,
    GameActive,
    WaitingForInput,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEventPayload {
    pub agent: AgentType,
    pub session_id: String,
    pub event: AgentEvent,
    pub timestamp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<String, Value>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionInfo {
    pub session_id: String,
    pub agent: AgentType,
    pub status: AgentStatus,
    pub started_at: i64,
    pub last_event_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_task_started_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_task_duration_ms: Option<i64>,
    pub total_tasks_completed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_for_input_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Degraded,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeHealth {
    pub status: HealthStatus,
    pub uptime_seconds: u64,
    pub version: String,
    pub active_sessions: usize,
    pub clients_connected: usize,
    pub port: u16,
}

/// Messages the bridge sends to WebSocket clients.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeClientMessage {
    AgentEvent { payload: AgentEventPayload },
    SessionUpdated { session: AgentSessionInfo },
    SessionList { sessions: Vec<AgentSessionInfo> },
    HealthCheck { health: BridgeHealth },
    StateChange { enabled: bool },
    Pong { timestamp: i64 },
}

/// Milliseconds since the Unix epoch (what `Date.now()` returns in the TypeScript code).
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `String(value)` as JavaScript would render it, so validation errors read the same as before.
fn js_string(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a.iter().map(|x| js_string(Some(x))).collect::<Vec<_>>().join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// Validates an inbound event. Error strings match the original TypeScript validator byte for byte.
pub fn validate_agent_event_payload(raw: &Value) -> Result<AgentEventPayload, String> {
    let obj = match raw {
        Value::Object(o) => o,
        _ => return Err("Payload must be a non-null JSON object".to_string()),
    };

    let agent = obj.get("agent").and_then(Value::as_str).and_then(AgentType::parse);
    let agent = match agent {
        Some(a) => a,
        None => {
            let names: Vec<&str> = AgentType::ALL.iter().map(|a| a.as_str()).collect();
            return Err(format!(
                "Invalid agent \"{}\". Expected one of: {}",
                js_string(obj.get("agent")),
                names.join(", ")
            ));
        }
    };

    let session_id = match obj.get("sessionId").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s,
        _ => return Err("sessionId must be a non-empty string".to_string()),
    };
    if session_id.encode_utf16().count() > MAX_SESSION_ID_CHARS {
        return Err("sessionId exceeds maximum length of 128 characters".to_string());
    }

    let event = obj.get("event").and_then(Value::as_str).and_then(AgentEvent::parse);
    let event = match event {
        Some(e) => e,
        None => {
            let names: Vec<&str> = AgentEvent::ALL.iter().map(|e| e.as_str()).collect();
            return Err(format!(
                "Invalid event \"{}\". Valid events: {}",
                js_string(obj.get("event")),
                names.join(", ")
            ));
        }
    };

    let timestamp = match obj.get("timestamp").and_then(Value::as_f64) {
        Some(t) if t.is_finite() && t > 0.0 => t as i64,
        _ => return Err("timestamp must be a positive finite unix timestamp in ms".to_string()),
    };

    let data = match obj.get("data") {
        None => None,
        Some(Value::Object(m)) => Some(m.clone()),
        Some(_) => return Err("data field if present must be an object".to_string()),
    };

    Ok(AgentEventPayload {
        agent,
        session_id: session_id.trim().to_string(),
        event,
        timestamp,
        model: obj.get("model").and_then(Value::as_str).map(str::to_string),
        terminal: None,
        data,
    })
}

/// Size check first, then the structural validation (the TypeScript `validateEventInput`).
pub fn validate_event_input(raw: &Value, raw_body_len: Option<usize>) -> Result<AgentEventPayload, String> {
    if let Some(len) = raw_body_len {
        if len > MAX_PAYLOAD_BYTES {
            return Err(format!(
                "Payload size ({} bytes) exceeds maximum limit of {} bytes.",
                len, MAX_PAYLOAD_BYTES
            ));
        }
    }
    validate_agent_event_payload(raw)
}

/// Human-readable agent name for UI labels.
pub fn agent_display_name(session_id_or_agent: &str) -> String {
    let lower = session_id_or_agent.to_lowercase();
    if lower.contains("claude") {
        "Claude Code".into()
    } else if lower.contains("opencode") {
        "OpenCode".into()
    } else if lower.contains("cline") || lower.contains("kilo") {
        "Cline".into()
    } else if lower.contains("cursor") {
        "Cursor".into()
    } else if lower.contains("antigravity") {
        "Antigravity".into()
    } else if lower.contains("codex") || lower.contains("chatgpt") {
        "ChatGPT".into()
    } else {
        session_id_or_agent.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ok() -> Value {
        json!({"agent":"omp","sessionId":" s1 ","event":"task_started","timestamp":1700000000000i64,"model":"m","data":{"a":1}})
    }

    #[test]
    fn accepts_and_trims() {
        let p = validate_agent_event_payload(&ok()).unwrap();
        assert_eq!(p.session_id, "s1");
        assert_eq!(p.agent, AgentType::Omp);
        assert_eq!(p.model.as_deref(), Some("m"));
        assert!(p.data.unwrap().contains_key("a"));
    }

    #[test]
    fn rejects_like_typescript() {
        assert_eq!(validate_agent_event_payload(&json!([1])).unwrap_err(), "Payload must be a non-null JSON object");
        assert_eq!(validate_agent_event_payload(&json!(null)).unwrap_err(), "Payload must be a non-null JSON object");
        let mut v = ok();
        v["agent"] = json!("nope");
        assert_eq!(
            validate_agent_event_payload(&v).unwrap_err(),
            "Invalid agent \"nope\". Expected one of: claude-code, codex, omp, opencode, cline, cursor, antigravity"
        );
        let mut v = ok();
        v.as_object_mut().unwrap().remove("agent");
        assert!(validate_agent_event_payload(&v).unwrap_err().starts_with("Invalid agent \"undefined\""));
        let mut v = ok();
        v["sessionId"] = json!("   ");
        assert_eq!(validate_agent_event_payload(&v).unwrap_err(), "sessionId must be a non-empty string");
        let mut v = ok();
        v["sessionId"] = json!("x".repeat(129));
        assert_eq!(validate_agent_event_payload(&v).unwrap_err(), "sessionId exceeds maximum length of 128 characters");
        let mut v = ok();
        v["event"] = json!("boom");
        assert!(validate_agent_event_payload(&v).unwrap_err().starts_with("Invalid event \"boom\". Valid events: session_started,"));
        for bad in [json!(0), json!(-5), json!("5"), json!(null)] {
            let mut v = ok();
            v["timestamp"] = bad;
            assert_eq!(validate_agent_event_payload(&v).unwrap_err(), "timestamp must be a positive finite unix timestamp in ms");
        }
        for bad in [json!(null), json!([1]), json!("x")] {
            let mut v = ok();
            v["data"] = bad;
            assert_eq!(validate_agent_event_payload(&v).unwrap_err(), "data field if present must be an object");
        }
    }

    #[test]
    fn size_limit() {
        let e = validate_event_input(&ok(), Some(MAX_PAYLOAD_BYTES + 1)).unwrap_err();
        assert_eq!(e, "Payload size (32769 bytes) exceeds maximum limit of 32768 bytes.");
        assert!(validate_event_input(&ok(), Some(MAX_PAYLOAD_BYTES)).is_ok());
    }

    #[test]
    fn wire_shapes() {
        let m = BridgeClientMessage::StateChange { enabled: true };
        assert_eq!(serde_json::to_string(&m).unwrap(), r#"{"type":"state_change","enabled":true}"#);
        let m = BridgeClientMessage::Pong { timestamp: 5 };
        assert_eq!(serde_json::to_string(&m).unwrap(), r#"{"type":"pong","timestamp":5}"#);
        let s = serde_json::to_value(AgentStatus::GameActive).unwrap();
        assert_eq!(s, json!("GAME_ACTIVE"));
        assert_eq!(serde_json::to_value(AgentStatus::WaitingForInput).unwrap(), json!("WAITING_FOR_INPUT"));
    }
}
