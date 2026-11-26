//! Wires the pieces together (port of index.ts): token, session manager, HTTP + WebSocket server, notifications,
//! startup banner and clean shutdown.

use crate::limits::Limits;
use crate::notifier::{self, CompletedTaskNotification};
use crate::security;
use crate::server::{self, BridgeState};
use crate::session_manager::{SessionCallback, SessionManager, SessionManagerOptions};
use crate::ws::WsHub;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use waitplay_core::{AgentSessionInfo, BridgeHealth, HealthStatus};

pub const DEFAULT_PORT: u16 = 3150;
/// The release version (the workspace version, so /health, the DMG and the zip always agree).
pub const BRIDGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A running bridge on an already-bound listener.
pub struct Bridge {
    pub state: Arc<BridgeState>,
    pub hub: Arc<WsHub>,
    pub addr: SocketAddr,
    accept_loop: JoinHandle<()>,
}

/// Starts serving on `listener`. Session changes (including background transitions such as the launch timer) are
/// broadcast to WebSocket clients. Needs a Tokio runtime.
pub fn start(listener: TcpListener, token: String, on_task_completed: Option<SessionCallback>) -> Bridge {
    start_with_limits(listener, token, on_task_completed, Limits::default())
}

/// [`start`] with explicit request limits (tests use small, long-window ones so they do not depend on timing).
pub fn start_with_limits(listener: TcpListener, token: String, on_task_completed: Option<SessionCallback>, limits: Limits) -> Bridge {
    let addr = listener.local_addr().expect("bound listener has an address");
    let start_time = Instant::now();
    let sessions = Arc::new(SessionManager::new(SessionManagerOptions { on_task_completed, ..Default::default() }));
    let hub = WsHub::new();
    hub.start_heartbeat();

    let health_sessions = sessions.clone();
    let health_hub = hub.clone();
    let port = addr.port();
    let get_health = Box::new(move || BridgeHealth {
        status: HealthStatus::Ok,
        uptime_seconds: start_time.elapsed().as_secs(),
        version: BRIDGE_VERSION.to_string(),
        active_sessions: health_sessions.get_active_count(),
        clients_connected: health_hub.connected_count(),
        port,
    });

    let broadcast_hub = hub.clone();
    sessions.on_session_change(Arc::new(move |session: &AgentSessionInfo| broadcast_hub.broadcast_session_update(session)));

    let state = Arc::new(BridgeState::with_limits(sessions, Some(hub.clone()), get_health, token, limits));
    let accept_loop = tokio::spawn(server::serve(listener, state.clone()));
    Bridge { state, hub, addr, accept_loop }
}

impl Bridge {
    /// Closes WebSocket clients, drops all sessions and stops accepting connections.
    pub fn shutdown(&self) {
        self.hub.close();
        self.state.sessions.clear_all();
        self.accept_loop.abort();
    }
}

/// `parseInt`-style: leading digits only.
fn parse_int_prefix(s: &str) -> Option<u16> {
    let s = s.trim_start();
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u32>().ok().and_then(|n| u16::try_from(n).ok())
}

/// `BRIDGE_PORT || PORT || 3150`.
pub fn port_from_env() -> Result<u16, String> {
    let raw = ["BRIDGE_PORT", "PORT"].iter().filter_map(|k| std::env::var(k).ok()).find(|v| !v.is_empty());
    match raw {
        None => Ok(DEFAULT_PORT),
        Some(v) => parse_int_prefix(&v).ok_or_else(|| format!("Invalid bridge port \"{}\"", v)),
    }
}

fn task_completed_hook() -> SessionCallback {
    // SMS costs money and reaches a person: cap it, so a runaway or forged stream of completions cannot turn into a bill
    let sms_limit = Arc::new(crate::limits::RateLimiter::new(crate::limits::SMS_PER_HOUR, std::time::Duration::from_secs(3600)));
    Arc::new(move |session: &AgentSessionInfo| {
        let task = CompletedTaskNotification {
            agent: session.agent.as_str().to_string(),
            session_id: session.session_id.clone(),
            duration_ms: session.last_task_duration_ms,
        };
        if !notifier::is_phone_notification_configured() {
            return;
        }
        if !sms_limit.allow() {
            eprintln!("[Vibe Play Bridge] SMS limit reached ({} per hour): completion notice not sent", crate::limits::SMS_PER_HOUR);
            return;
        }
        // Fire and forget: SMS problems are logged and never block event handling.
        tokio::spawn(async move {
            if let Err(e) = notifier::notify_task_completed(&task).await {
                eprintln!("Error in task completion callback: {}", e);
            }
        });
    })
}

/// The `waitplay-bridge` entry point; returns the process exit code.
pub async fn run() -> i32 {
    let port = match port_from_env() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[Vibe Play Bridge] {}", e);
            return 1;
        }
    };

    // The token is written to ONE per-user file (owner-only permissions) and is never printed: stdout is often captured by
    // log files, terminals and CI output.
    let token_file = waitplay_core::paths::signal_dir().join(".bridge-token");
    let token = match security::initialize_bridge_token(&token_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[Vibe Play Bridge] {}", e);
            return 1;
        }
    };
    println!("[Vibe Play Bridge] Security token initialized");
    println!(
        "[Vibe Play Bridge] Phone completion notifications: {}",
        if notifier::is_phone_notification_configured() { "enabled" } else { "disabled" }
    );

    let listener = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[Vibe Play Bridge] Could not listen on 127.0.0.1:{}: {}", port, e);
            return 1;
        }
    };
    let bridge = start(listener, token, Some(task_completed_hook()));
    println!("=======================================================");
    println!("  Vibe Play Local Companion Bridge running");
    println!("  HTTP:      http://127.0.0.1:{}", port);
    println!("  WebSocket: ws://127.0.0.1:{}/ws", port);
    println!("  Token:     {} (readable by this user only)", token_file.display());
    println!("=======================================================");

    let signal = wait_for_shutdown_signal().await;
    println!("[Vibe Play Bridge] Received {}. Shutting down cleanly...", signal);
    bridge.shutdown();
    // Give the close frames a moment to leave the socket buffers.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    println!("[Vibe Play Bridge] Server closed.");
    0
}

#[cfg(unix)]
async fn wait_for_shutdown_signal() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => "SIGINT",
        _ = term.recv() => "SIGTERM",
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() -> &'static str {
    let _ = tokio::signal::ctrl_c().await;
    "SIGINT"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_parsing_follows_parse_int() {
        assert_eq!(parse_int_prefix("3150"), Some(3150));
        assert_eq!(parse_int_prefix(" 4000abc"), Some(4000));
        assert_eq!(parse_int_prefix("abc"), None);
        assert_eq!(parse_int_prefix("70000"), None);
    }
}
