//! The session manager: owns the signed-in state and keeps it honest without ever blocking local features.
//!
//! - Start-up reads the refresh token from the secret store and does **no network call**: the user is "signed in, offline" until
//!   the first refresh succeeds. Vibe Play's local features never wait for this.
//! - Refreshing before expiry rotates the refresh token; the new one is written to the secret store *before* the old session is
//!   dropped from memory. Only one process (the tray app) refreshes, so rotation cannot race.
//! - A network failure or a 5xx keeps the session (backoff is the caller's timer). Only a definitive "refresh token invalid"
//!   answer signs the user out, with a reason the UI can show.
//! - Sign-out always clears the local secret, even when the server cannot be reached.

use crate::error::{AuthError, Result};
use crate::gotrue::{now_secs, Client, Session, User};
use crate::store::SecretStore;
use serde_json::{json, Value};
use std::sync::Mutex;

/// Refresh this many seconds before the access token expires.
pub const REFRESH_MARGIN: i64 = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    SignedOut { reason: Option<String> },
    /// `offline`: the tokens have not been refreshed yet in this run (no network, or the app just started).
    SignedIn { user_id: String, offline: bool },
}

struct Inner {
    /// Live session (with access token) once a refresh or sign-in succeeded in this run.
    session: Option<Session>,
    /// What the secret store holds: user id + refresh token. Present whenever the user is signed in.
    stored: Option<(String, String)>,
    signed_out_reason: Option<String>,
}

pub struct SessionManager {
    client: Client,
    store: Box<dyn SecretStore>,
    inner: Mutex<Inner>,
    now: Box<dyn Fn() -> i64 + Send + Sync>,
}

fn encode(user_id: &str, refresh_token: &str) -> String {
    json!({"v": 1, "user_id": user_id, "refresh_token": refresh_token}).to_string()
}

fn decode(s: &str) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(s).ok()?;
    if v.get("v")?.as_i64()? != 1 {
        return None;
    }
    let (u, r) = (v.get("user_id")?.as_str()?, v.get("refresh_token")?.as_str()?);
    (!u.is_empty() && !r.is_empty()).then(|| (u.to_string(), r.to_string()))
}

impl SessionManager {
    pub fn new(client: Client, store: Box<dyn SecretStore>) -> SessionManager {
        Self::with_clock(client, store, Box::new(now_secs))
    }

    pub fn with_clock(client: Client, store: Box<dyn SecretStore>, now: Box<dyn Fn() -> i64 + Send + Sync>) -> SessionManager {
        let mut inner = Inner { session: None, stored: None, signed_out_reason: None };
        match store.load() {
            Ok(Some(s)) => match decode(&s) {
                Some(pair) => inner.stored = Some(pair),
                None => {
                    // unreadable or foreign content: never trust it, never keep it
                    let _ = store.delete();
                }
            },
            Ok(None) => {}
            Err(_) => inner.signed_out_reason = Some("Vibe Play could not read your saved sign-in from the system keychain.".into()),
        }
        SessionManager { client, store, inner: Mutex::new(inner), now }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state(&self) -> AuthState {
        let g = self.lock();
        match (&g.stored, &g.session) {
            (Some((uid, _)), session) => AuthState::SignedIn { user_id: uid.clone(), offline: session.is_none() },
            (None, _) => AuthState::SignedOut { reason: g.signed_out_reason.clone() },
        }
    }

    pub fn user(&self) -> Option<User> {
        self.lock().session.as_ref().map(|s| s.user.clone())
    }

    /// Adopts a session obtained by a sign-in flow: stored first, then kept in memory.
    pub fn adopt(&self, session: Session) -> Result<()> {
        self.store.save(&encode(&session.user.id, &session.refresh_token))?;
        let mut g = self.lock();
        g.stored = Some((session.user.id.clone(), session.refresh_token.clone()));
        g.signed_out_reason = None;
        g.session = Some(session);
        Ok(())
    }

    /// A valid access token, refreshing first when it is missing or about to expire. Errors: transient ones leave the state as it was.
    pub fn access_token(&self) -> Result<String> {
        {
            let g = self.lock();
            if let Some(s) = &g.session {
                if s.expires_at - (self.now)() > REFRESH_MARGIN {
                    return Ok(s.access_token.clone());
                }
            }
        }
        self.refresh()
    }

    /// Refreshes now and returns the new access token.
    pub fn refresh(&self) -> Result<String> {
        // hold the lock across the request: one refresh at a time, and nobody sees a half-rotated pair
        let mut g = self.lock();
        let Some((_, refresh_token)) = g.stored.clone() else { return Err(AuthError::State("Not signed in.")) };
        match self.client.refresh(&refresh_token) {
            Ok(session) => {
                // the rotated token is persisted before the old one is forgotten
                self.store.save(&encode(&session.user.id, &session.refresh_token))?;
                g.stored = Some((session.user.id.clone(), session.refresh_token.clone()));
                let token = session.access_token.clone();
                g.session = Some(session);
                Ok(token)
            }
            Err(AuthError::SessionExpired) => {
                let _ = self.store.delete();
                g.stored = None;
                g.session = None;
                g.signed_out_reason = Some(AuthError::SessionExpired.user_message());
                Err(AuthError::SessionExpired)
            }
            Err(e) => Err(e),
        }
    }

    /// Seconds until the next refresh is due (0 when due now or when there is no live session yet); `None` when signed out.
    pub fn next_refresh_in(&self) -> Option<i64> {
        let g = self.lock();
        g.stored.as_ref()?;
        Some(g.session.as_ref().map_or(0, |s| (s.expires_at - (self.now)() - REFRESH_MARGIN).max(0)))
    }

    /// Signs out locally no matter what; `scope` is sent to the server on a best-effort basis (`local`, `others`, `global`).
    pub fn sign_out(&self, scope: &str) -> Result<()> {
        let access = {
            let g = self.lock();
            g.session.as_ref().map(|s| s.access_token.clone())
        };
        if let Some(token) = access {
            let _ = self.client.logout(&token, scope); // offline or already invalid: the local sign-out below is what matters
        }
        let result = self.store.delete();
        let mut g = self.lock();
        g.stored = None;
        g.session = None;
        g.signed_out_reason = None;
        result
    }

    /// Signs the other devices out and keeps this one.
    pub fn sign_out_others(&self) -> Result<()> {
        let token = self.access_token()?;
        self.client.logout(&token, "others")
    }

    /// Signs out locally (no server call) and remembers why, for the sign-in screen.
    pub fn force_sign_out(&self, reason: &str) {
        let _ = self.store.delete();
        let mut g = self.lock();
        g.stored = None;
        g.session = None;
        g.signed_out_reason = Some(reason.to_string());
    }

    /// Forgets the local session without telling the server (the account was deleted server-side).
    pub fn forget_local(&self) -> Result<()> {
        let result = self.store.delete();
        let mut g = self.lock();
        g.stored = None;
        g.session = None;
        g.signed_out_reason = None;
        result
    }
}
