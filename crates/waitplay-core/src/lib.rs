//! Shared building blocks for every Vibe Play binary: the event protocol, the small files processes hand to each other,
//! and a loopback client for the bridge.

pub mod client;
pub mod companion_state;
pub mod paths;
pub mod process;
pub mod settings;
pub mod protocol;
pub mod signals;
pub mod state;
pub mod token;

pub use client::BridgeClient;
pub use protocol::{
    now_ms, AgentEvent, AgentEventPayload, AgentSessionInfo, AgentStatus, AgentType, BridgeClientMessage, BridgeHealth, HealthStatus,
};
