//! Vibe Play account sign-in on Supabase Auth.
//!
//! Layers, bottom to top: [`pkce`] / [`callback`] (browser flow primitives), [`gotrue`] (REST client), [`store`] (OS secret store),
//! [`session`] (state machine: offline-safe refresh, sign-out), [`cloud`] (RLS-protected data), [`service`] (what the app calls).
//! Nothing here touches the bridge, the hooks or any local feature: the app works with this crate absent or unreachable.

pub mod callback;
pub mod cloud;
pub mod config;
pub mod error;
pub mod gotrue;
pub mod pkce;
pub mod service;
pub mod session;
pub mod store;

pub use config::AuthConfig;
pub use error::{AuthError, Result};
pub use service::AccountService;
