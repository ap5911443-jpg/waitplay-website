//! Vibe Play bridge: tracks coding-agent sessions over HTTP + WebSocket on 127.0.0.1, and serves the companion web UI.

pub mod bridge;
pub mod event_validator;
pub mod js;
pub mod limits;
pub mod notifier;
pub mod security;
pub mod server;
pub mod session_manager;
pub mod state_machine;
pub mod ws;
pub mod web;
