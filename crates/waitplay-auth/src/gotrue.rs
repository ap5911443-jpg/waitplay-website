//! Supabase Auth (GoTrue) REST client. Blocking (`ureq`); the app calls it from worker threads.
//!
//! Endpoints used: `/authorize`, `/token` (pkce, password, refresh_token), `/signup`, `/otp`, `/verify`, `/recover`, `/resend`,
//! `/user`, `/logout`. Every request carries the project's publishable key in `apikey`; user requests also carry the user's access
//! token. No privileged key is used or accepted anywhere (see `AuthConfig::is_valid`).

use crate::config::AuthConfig;
use crate::error::{AuthError, Result};
use crate::pkce::urlencode;
use serde_json::{json, Value};
use std::fmt;
use std::time::Duration;

#[derive(Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub email_confirmed: bool,
    pub provider: String,
}

impl User {
    pub fn from_json(v: &Value) -> Option<User> {
        let meta = v.get("user_metadata");
        let name = ["full_name", "name", "display_name"].iter().find_map(|k| meta.and_then(|m| m.get(*k)).and_then(Value::as_str)).unwrap_or("");
        Some(User {
            id: v.get("id")?.as_str()?.to_string(),
            email: v.get("email").and_then(Value::as_str).unwrap_or("").to_string(),
            display_name: name.to_string(),
            email_confirmed: v.get("email_confirmed_at").map_or(false, |c| !c.is_null()) || v.get("confirmed_at").map_or(false, |c| !c.is_null()),
            provider: v.pointer("/app_metadata/provider").and_then(Value::as_str).unwrap_or("email").to_string(),
        })
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User({})", self.id)
    }
}

/// Access + refresh token pair. `Debug` is redacted on purpose: a token must not reach a log through `{:?}`.
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds.
    pub expires_at: i64,
    pub user: User,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Session(user {}, tokens redacted)", self.user.id)
    }
}

pub enum SignUp {
    /// Email confirmation is on (the default): the user must enter the code that was emailed.
    ConfirmationRequired(User),
    SignedIn(Session),
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[derive(Clone)]
pub struct Client {
    cfg: AuthConfig,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(cfg: AuthConfig) -> Client {
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(8)).timeout(Duration::from_secs(20)).user_agent("Vibe Play").build();
        Client { cfg, agent }
    }

    pub fn config(&self) -> &AuthConfig {
        &self.cfg
    }

    // ── browser (PKCE) flow ──────────────────────────────────────────────────────────────────────────────────────────────────

    /// The URL to open in the system browser. `provider` is a Supabase provider id (`google`, `apple`).
    pub fn authorize_url(&self, provider: &str, redirect_to: &str, code_challenge: &str) -> String {
        format!(
            "{}/auth/v1/authorize?provider={}&redirect_to={}&code_challenge={}&code_challenge_method=s256",
            self.cfg.url,
            urlencode(provider),
            urlencode(redirect_to),
            urlencode(code_challenge)
        )
    }

    pub fn exchange_code(&self, code: &str, verifier: &str) -> Result<Session> {
        let v = self.call("POST", "/auth/v1/token?grant_type=pkce", None, Some(json!({"auth_code": code, "code_verifier": verifier})))?;
        parse_session(&v)
    }

    // ── email flows ──────────────────────────────────────────────────────────────────────────────────────────────────────────

    pub fn sign_in_password(&self, email: &str, password: &str) -> Result<Session> {
        parse_session(&self.call("POST", "/auth/v1/token?grant_type=password", None, Some(json!({"email": email, "password": password})))?)
    }

    pub fn sign_up(&self, email: &str, password: &str) -> Result<SignUp> {
        let v = self.call("POST", "/auth/v1/signup", None, Some(json!({"email": email, "password": password})))?;
        if v.get("access_token").is_some() {
            return Ok(SignUp::SignedIn(parse_session(&v)?));
        }
        let user = User::from_json(v.get("user").unwrap_or(&v)).ok_or(AuthError::Unavailable)?;
        Ok(SignUp::ConfirmationRequired(user))
    }

    /// Emails a 6-digit code. `create_user = false` for "sign in" so a typo does not create an account.
    pub fn send_otp(&self, email: &str, create_user: bool) -> Result<()> {
        self.call("POST", "/auth/v1/otp", None, Some(json!({"email": email, "create_user": create_user}))).map(|_| ())
    }

    /// `kind`: `email` (sign-in code), `signup` (confirmation code), `recovery` (password reset code).
    pub fn verify_otp(&self, email: &str, token: &str, kind: &str) -> Result<Session> {
        parse_session(&self.call("POST", "/auth/v1/verify", None, Some(json!({"type": kind, "email": email, "token": token})))?)
    }

    pub fn resend_signup(&self, email: &str) -> Result<()> {
        self.call("POST", "/auth/v1/resend", None, Some(json!({"type": "signup", "email": email}))).map(|_| ())
    }

    pub fn recover(&self, email: &str) -> Result<()> {
        self.call("POST", "/auth/v1/recover", None, Some(json!({"email": email}))).map(|_| ())
    }

    pub fn update_password(&self, access_token: &str, new_password: &str) -> Result<()> {
        self.call("PUT", "/auth/v1/user", Some(access_token), Some(json!({"password": new_password}))).map(|_| ())
    }

    pub fn update_display_name(&self, access_token: &str, name: &str) -> Result<()> {
        self.call("PUT", "/auth/v1/user", Some(access_token), Some(json!({"data": {"full_name": name}}))).map(|_| ())
    }

    // ── session ──────────────────────────────────────────────────────────────────────────────────────────────────────────────

    /// A definitive "this refresh token is dead" answer is `SessionExpired`; offline or a 5xx is a transient error.
    pub fn refresh(&self, refresh_token: &str) -> Result<Session> {
        match self.call("POST", "/auth/v1/token?grant_type=refresh_token", None, Some(json!({"refresh_token": refresh_token}))) {
            Ok(v) => parse_session(&v),
            // any 4xx on the refresh endpoint (not found, already used, revoked, malformed) means this refresh token will never work again
            Err(AuthError::Rejected { .. }) => Err(AuthError::SessionExpired),
            Err(e) => Err(e),
        }
    }

    pub fn get_user(&self, access_token: &str) -> Result<User> {
        let v = self.call("GET", "/auth/v1/user", Some(access_token), None)?;
        User::from_json(&v).ok_or(AuthError::Unavailable)
    }

    /// `scope`: `local` (this device), `others` (every other device), `global`.
    pub fn logout(&self, access_token: &str, scope: &str) -> Result<()> {
        self.call("POST", &format!("/auth/v1/logout?scope={}", urlencode(scope)), Some(access_token), None).map(|_| ())
    }

    // ── transport ────────────────────────────────────────────────────────────────────────────────────────────────────────────

    /// One JSON request. `path` starts with `/`. Used by the cloud (PostgREST / function) calls too.
    pub fn call(&self, method: &str, path: &str, bearer: Option<&str>, body: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.cfg.url, path);
        let mut req = self.agent.request(method, &url).set("apikey", &self.cfg.anon_key).set("Accept", "application/json");
        req = req.set("Authorization", &format!("Bearer {}", bearer.unwrap_or(&self.cfg.anon_key)));
        let result = match body {
            Some(b) => req.set("Content-Type", "application/json").send_string(&b.to_string()),
            None => req.call(),
        };
        match result {
            Ok(resp) => {
                let text = resp.into_string().map_err(|e| AuthError::Network(e.to_string()))?;
                if text.trim().is_empty() {
                    Ok(Value::Null)
                } else {
                    serde_json::from_str(&text).map_err(|_| AuthError::Unavailable)
                }
            }
            Err(ureq::Error::Status(status, resp)) => Err(map_status(status, resp.into_string().unwrap_or_default())),
            Err(ureq::Error::Transport(t)) => Err(AuthError::Network(t.kind().to_string())),
        }
    }
}

fn map_status(status: u16, body: String) -> AuthError {
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let code = ["error_code", "code", "error"].iter().find_map(|k| v.get(*k).and_then(Value::as_str)).unwrap_or("").to_string();
    let message = ["msg", "message", "error_description"].iter().find_map(|k| v.get(*k).and_then(Value::as_str)).unwrap_or("").to_string();
    match status {
        // the email limit is a Rejected answer (its own message), not the generic "too many requests"
        _ if code == "over_email_send_rate_limit" => AuthError::Rejected { code, message },
        429 => AuthError::RateLimited,
        500..=599 => AuthError::Unavailable,
        _ if code == "over_request_rate_limit" => AuthError::RateLimited,
        _ => AuthError::Rejected { code, message },
    }
}

fn parse_session(v: &Value) -> Result<Session> {
    let access = v.get("access_token").and_then(Value::as_str).ok_or(AuthError::Unavailable)?;
    let refresh = v.get("refresh_token").and_then(Value::as_str).ok_or(AuthError::Unavailable)?;
    let expires_at = v.get("expires_at").and_then(Value::as_i64).unwrap_or_else(|| now_secs() + v.get("expires_in").and_then(Value::as_i64).unwrap_or(3600));
    let user = User::from_json(v.get("user").ok_or(AuthError::Unavailable)?).ok_or(AuthError::Unavailable)?;
    Ok(Session { access_token: access.to_string(), refresh_token: refresh.to_string(), expires_at, user })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_tokens() {
        let s = Session {
            access_token: "ACCESS-SECRET".into(),
            refresh_token: "REFRESH-SECRET".into(),
            expires_at: 1,
            user: User { id: "u1".into(), email: "a@b.c".into(), display_name: "A".into(), email_confirmed: true, provider: "email".into() },
        };
        let text = format!("{s:?} {:?}", s.user);
        assert!(!text.contains("SECRET") && !text.contains("a@b.c"), "{text}");
    }

    #[test]
    fn the_authorize_url_carries_pkce_and_the_encoded_redirect() {
        let c = Client::new(AuthConfig { url: "https://x.supabase.co".into(), anon_key: "sb_publishable_k".into() });
        let u = c.authorize_url("google", "http://127.0.0.1:5000/auth/callback/n", "CHAL");
        assert_eq!(
            u,
            "https://x.supabase.co/auth/v1/authorize?provider=google&redirect_to=http%3A%2F%2F127.0.0.1%3A5000%2Fauth%2Fcallback%2Fn&code_challenge=CHAL&code_challenge_method=s256"
        );
        assert!(!u.contains("sb_publishable"), "the key goes in a header, not in the browser URL");
    }

    #[test]
    fn user_metadata_names_and_confirmation_are_read() {
        let u = User::from_json(&json!({"id": "1", "email": "e@x.y", "email_confirmed_at": "2026-01-01", "user_metadata": {"full_name": "Ada L"}, "app_metadata": {"provider": "google"}})).unwrap();
        assert_eq!((u.display_name.as_str(), u.email_confirmed, u.provider.as_str()), ("Ada L", true, "google"));
        assert!(!User::from_json(&json!({"id": "1", "email_confirmed_at": null})).unwrap().email_confirmed);
        assert!(User::from_json(&json!({"email": "no-id"})).is_none());
    }
}
