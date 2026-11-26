//! `waitplay-hook mcp`: the Vibe Play Model Context Protocol server, JSON-RPC 2.0 over stdio, newline-delimited.
//! Port of integrations/mcp/waitplay-mcp.cjs with the same methods, tool list, schemas, texts and errors.

use crate::common::{self, js_string, truthy, PopRequest};
use serde_json::{Map, Value};
use std::io::{BufRead, Write};
use std::sync::mpsc;
use waitplay_core::state;

pub const TOOLS_LIST_RESULT: &str = concat!(
    r#"{"tools":["#,
    r#"{"name":"waitplay_start_game","description":"Pops up the native Vibe Play companion game window on the user screen while the coding agent is thinking, researching, or running long tasks.","inputSchema":{"type":"object","properties":{"agent":{"type":"string","description":"Name of the coding agent (e.g. cursor, cline, opencode, antigravity, claude-code)"},"model":{"type":"string","description":"Name of the AI model currently generating the response"},"task":{"type":"string","description":"Brief summary of the active task"}}}},"#,
    r#"{"name":"waitplay_update_status","description":"Updates live telemetry and tool execution state in the Vibe Play companion game window.","inputSchema":{"type":"object","properties":{"agent":{"type":"string"},"tool":{"type":"string","description":"Name of the tool currently executing (e.g. read, edit, bash)"},"step":{"type":"string","description":"Description of the current operation"}}}},"#,
    r#"{"name":"waitplay_finish_game","description":"Notifies Vibe Play that the coding agent has completed its task, showing the completion summary and closing the game.","inputSchema":{"type":"object","properties":{"agent":{"type":"string"},"summary":{"type":"string","description":"Brief summary of task results"}}}}"#,
    r#"]}"#
);

const INITIALIZE_RESULT: &str =
    r#"{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"waitplay-mcp","version":"0.1.0"}}"#;

/// Side effects of a tool call, separated out so the request handling is testable.
pub trait Host {
    fn pop_app(&mut self, session: &str);
    fn post_event(&mut self, payload: Value);
}

fn response(id: Option<&Value>, key: &str, body: &str) -> String {
    match id {
        Some(id) => format!(r#"{{"jsonrpc":"2.0","id":{},"{}":{}}}"#, id, key, body),
        None => format!(r#"{{"jsonrpc":"2.0","{}":{}}}"#, key, body),
    }
}

fn text_result(text: &str) -> String {
    let t = Value::String(text.to_string());
    format!(r#"{{"content":[{{"type":"text","text":{}}}]}}"#, t)
}

/// `args.<key>` where `args` may be anything the client sent; only objects have fields.
fn arg<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key)
}

/// `value || fallback` as text, the way the original's template literals rendered it.
fn text_or(v: Option<&Value>, fallback: &str) -> String {
    match v {
        Some(v) if truthy(v) => js_string(v),
        _ => fallback.to_string(),
    }
}

fn event(agent: &Value, session_id: &str, event: &str, model: Option<&Value>, data: Vec<(&str, Option<&Value>)>) -> Value {
    let mut m = Map::new();
    m.insert("agent".into(), agent.clone());
    m.insert("sessionId".into(), Value::String(session_id.to_string()));
    m.insert("event".into(), Value::String(event.to_string()));
    m.insert("timestamp".into(), Value::from(waitplay_core::protocol::now_ms()));
    if let Some(model) = model {
        m.insert("model".into(), model.clone());
    }
    let mut d = Map::new();
    for (k, v) in data {
        if let Some(v) = v {
            d.insert(k.to_string(), v.clone());
        }
    }
    m.insert("data".into(), Value::Object(d));
    Value::Object(m)
}

/// Handles one input line. The result is the line to write back, if any (notifications and junk get none).
pub fn handle_line(line: &str, host: &mut dyn Host) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let msg: Value = serde_json::from_str(line.trim()).ok()?; // malformed frames are ignored
    let obj = msg.as_object()?;
    let id = obj.get("id");
    let method = obj.get("method").and_then(Value::as_str);

    match method {
        Some("initialize") => return Some(response(id, "result", INITIALIZE_RESULT)),
        Some("notifications/initialized") => return None,
        Some("tools/list") => return Some(response(id, "result", TOOLS_LIST_RESULT)),
        Some("tools/call") => {
            let params = obj.get("params");
            let tool_name = params.and_then(|p| p.get("name"));
            let args = match params.and_then(|p| p.get("arguments")) {
                Some(a) if truthy(a) => a.clone(),
                _ => Value::Object(Map::new()),
            };
            let agent = match arg(&args, "agent") {
                Some(a) if truthy(a) => a.clone(),
                _ => Value::String("coding-agent".into()),
            };
            let session_id = format!("{}-session", js_string(&agent));

            match tool_name.and_then(Value::as_str) {
                Some("waitplay_start_game") => {
                    host.pop_app(&session_id);
                    host.post_event(event(
                        &agent,
                        &session_id,
                        "task_started",
                        arg(&args, "model"),
                        vec![("model", arg(&args, "model")), ("task", arg(&args, "task"))],
                    ));
                    let text = format!(
                        "Vibe Play game window popped up on user's screen for {} ({}).",
                        js_string(&agent),
                        text_or(arg(&args, "model"), "active model")
                    );
                    return Some(response(id, "result", &text_result(&text)));
                }
                Some("waitplay_update_status") => {
                    host.post_event(event(
                        &agent,
                        &session_id,
                        "task_running",
                        None,
                        vec![("tool", arg(&args, "tool")), ("step", arg(&args, "step"))],
                    ));
                    let text = format!("Vibe Play status updated with tool: {}.", text_or(arg(&args, "tool"), "running"));
                    return Some(response(id, "result", &text_result(&text)));
                }
                Some("waitplay_finish_game") => {
                    host.post_event(event(&agent, &session_id, "task_completed", None, vec![("summary", arg(&args, "summary"))]));
                    let text = format!("Vibe Play notified that {} finished. Game completion banner displayed.", js_string(&agent));
                    return Some(response(id, "result", &text_result(&text)));
                }
                _ => {
                    let name = match tool_name {
                        None => "undefined".to_string(),
                        Some(v) => js_string(v),
                    };
                    let err = Value::Object(common::obj(vec![("code", Value::from(-32601)), ("message", Value::String(format!("Unknown tool: {}", name)))]));
                    return Some(response(id, "error", &err.to_string()));
                }
            }
        }
        _ => {}
    }

    // Default response for other notifications and pings
    id.map(|id| response(Some(id), "result", "{}"))
}

struct RealHost {
    tx: mpsc::Sender<Value>,
}

impl Host for RealHost {
    fn pop_app(&mut self, session: &str) {
        if state::is_enabled() {
            common::spawn_app(&PopRequest { session: session.to_string(), terminal: None, prompt_time: None, parent_pid: std::process::id() });
        }
    }

    fn post_event(&mut self, payload: Value) {
        let _ = self.tx.send(payload);
    }
}

/// Serves stdin until it closes. Events are posted from a worker so a slow bridge never delays a JSON-RPC reply.
pub fn run() {
    let (tx, rx) = mpsc::channel::<Value>();
    let worker = std::thread::spawn(move || {
        for payload in rx {
            common::post_event_quiet(&payload);
        }
    });
    let mut host = RealHost { tx };
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match input.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = String::from_utf8_lossy(&buf);
        if let Some(out) = handle_line(line.trim_end_matches(['\n', '\r']), &mut host) {
            let mut stdout = std::io::stdout().lock();
            if writeln!(stdout, "{}", out).and_then(|_| stdout.flush()).is_err() {
                break;
            }
        }
    }
    drop(host);
    let _ = worker.join();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Default)]
    struct Rec {
        pops: Vec<String>,
        posts: Vec<Value>,
    }
    impl Host for Rec {
        fn pop_app(&mut self, s: &str) {
            self.pops.push(s.to_string());
        }
        fn post_event(&mut self, p: Value) {
            self.posts.push(p);
        }
    }

    fn call(line: &str) -> (Option<Value>, Rec) {
        let mut h = Rec::default();
        let out = handle_line(line, &mut h).map(|s| serde_json::from_str::<Value>(&s).expect("valid JSON out"));
        (out, h)
    }

    #[test]
    fn initialize() {
        let (r, _) = call(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        assert_eq!(
            r.unwrap(),
            json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"waitplay-mcp","version":"0.1.0"}}})
        );
    }

    #[test]
    fn raw_output_key_order_matches_node() {
        let mut h = Rec::default();
        let out = handle_line(r#"{"jsonrpc":"2.0","id":7,"method":"initialize"}"#, &mut h).unwrap();
        assert!(out.starts_with(r#"{"jsonrpc":"2.0","id":7,"result":{"protocolVersion":"2024-11-05""#));
        let out = handle_line(r#"{"jsonrpc":"2.0","id":"a","method":"ping"}"#, &mut h).unwrap();
        assert_eq!(out, r#"{"jsonrpc":"2.0","id":"a","result":{}}"#);
    }

    #[test]
    fn tool_list_is_the_three_original_tools() {
        let (r, _) = call(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let r = r.unwrap();
        let tools = r["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["waitplay_start_game", "waitplay_update_status", "waitplay_finish_game"]);
        assert_eq!(tools[0]["inputSchema"]["properties"]["model"]["description"], "Name of the AI model currently generating the response");
        assert_eq!(tools[1]["inputSchema"]["properties"]["tool"]["description"], "Name of the tool currently executing (e.g. read, edit, bash)");
        assert_eq!(tools[1]["inputSchema"]["properties"]["agent"], json!({"type": "string"}));
        assert_eq!(tools[2]["inputSchema"]["properties"]["summary"]["description"], "Brief summary of task results");
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn start_game_pops_and_posts() {
        let (r, h) = call(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"waitplay_start_game","arguments":{"agent":"cursor","model":"gpt-5","task":"refactor"}}}"#,
        );
        assert_eq!(h.pops, ["cursor-session"]);
        assert_eq!(h.posts.len(), 1);
        let p = &h.posts[0];
        assert_eq!((p["agent"].as_str(), p["sessionId"].as_str(), p["event"].as_str(), p["model"].as_str()), (Some("cursor"), Some("cursor-session"), Some("task_started"), Some("gpt-5")));
        assert_eq!(p["data"], json!({"model": "gpt-5", "task": "refactor"}));
        assert_eq!(
            r.unwrap(),
            json!({"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"Vibe Play game window popped up on user's screen for cursor (gpt-5)."}]}})
        );
    }

    #[test]
    fn defaults_when_arguments_are_missing() {
        let (r, h) = call(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"waitplay_start_game"}}"#);
        assert_eq!(h.pops, ["coding-agent-session"]);
        assert_eq!(h.posts[0]["agent"], "coding-agent");
        assert!(h.posts[0].get("model").is_none());
        assert_eq!(h.posts[0]["data"], json!({}));
        assert_eq!(r.unwrap()["result"]["content"][0]["text"], "Vibe Play game window popped up on user's screen for coding-agent (active model).");
    }

    #[test]
    fn update_and_finish() {
        let (r, h) = call(r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"waitplay_update_status","arguments":{"agent":"cline","tool":"bash","step":"npm test"}}}"#);
        assert!(h.pops.is_empty());
        assert_eq!(h.posts[0]["event"], "task_running");
        assert_eq!(h.posts[0]["sessionId"], "cline-session");
        assert_eq!(h.posts[0]["data"], json!({"tool": "bash", "step": "npm test"}));
        assert!(h.posts[0].get("model").is_none());
        assert_eq!(r.unwrap()["result"]["content"][0]["text"], "Vibe Play status updated with tool: bash.");

        let (r, _) = call(r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"waitplay_update_status","arguments":{}}}"#);
        assert_eq!(r.unwrap()["result"]["content"][0]["text"], "Vibe Play status updated with tool: running.");

        let (r, h) = call(r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"waitplay_finish_game","arguments":{"agent":"omp","summary":"done"}}}"#);
        assert_eq!(h.posts[0]["event"], "task_completed");
        assert_eq!(h.posts[0]["data"], json!({"summary": "done"}));
        assert_eq!(r.unwrap()["result"]["content"][0]["text"], "Vibe Play notified that omp finished. Game completion banner displayed.");
    }

    #[test]
    fn unknown_tool_is_an_error() {
        let (r, h) = call(r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"nope"}}"#);
        assert!(h.posts.is_empty());
        assert_eq!(r.unwrap(), json!({"jsonrpc":"2.0","id":9,"error":{"code":-32601,"message":"Unknown tool: nope"}}));
        let (r, _) = call(r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{}}"#);
        assert_eq!(r.unwrap()["error"]["message"], "Unknown tool: undefined");
    }

    #[test]
    fn notifications_pings_and_junk() {
        assert!(call(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).0.is_none());
        assert!(call(r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{}}"#).0.is_none());
        assert_eq!(call(r#"{"jsonrpc":"2.0","id":11,"method":"ping"}"#).0.unwrap(), json!({"jsonrpc":"2.0","id":11,"result":{}}));
        assert!(call("").0.is_none());
        assert!(call("   ").0.is_none());
        assert!(call("{garbage").0.is_none());
        assert!(call("null").0.is_none());
        assert!(call("[1,2]").0.is_none());
        // a request without an id still gets an answer, id omitted (JSON.stringify drops undefined)
        let mut h = Rec::default();
        let out = handle_line(r#"{"method":"tools/list"}"#, &mut h).unwrap();
        assert!(out.starts_with(r#"{"jsonrpc":"2.0","result":{"tools":["#));
    }
}
