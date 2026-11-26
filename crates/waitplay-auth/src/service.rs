//! What the app calls. Blocking methods meant for worker threads; `view()` is local and instant (it never touches the network),
//! so the UI can always render, offline included.

use crate::callback::CallbackServer;
use crate::cloud::{self, Device, DeviceRegistration, Entitlement};
use crate::config::AuthConfig;
use crate::error::{AuthError, Result};
use crate::gotrue::{now_secs, Client, Session, SignUp};
use crate::pkce;
use crate::session::{AuthState, SessionManager};
use crate::store::{os_store, SecretStore};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Keychain / Credential Manager item name of the production app.
pub const STORE_NAME: &str = "com.waitplay.app";
/// How long the browser flow may take.
pub const OAUTH_TIMEOUT: Duration = Duration::from_secs(300);
/// Cloud data is re-fetched at most this often by the background tick.
pub const SYNC_EVERY_SECS: i64 = 15 * 60;

#[derive(Default, Clone)]
struct Cache {
    user_id: String,
    email: String,
    name: String,
    provider: String,
    verified: bool,
    plan: String,
    plan_status: String,
    account_status: String,
    device_limit: i64,
    devices: Vec<Device>,
    synced_at: i64,
    device_limit_reached: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SignUpOutcome {
    /// A confirmation code was emailed; call `verify_code(email, code, "signup")`.
    NeedsCode,
    SignedIn,
}

pub struct AccountService {
    mgr: SessionManager,
    dir: PathBuf,
    cache: Mutex<Cache>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
    last_sync_attempt: Mutex<i64>,
}

pub fn valid_email(e: &str) -> bool {
    let e = e.trim();
    e.len() <= 254 && e.split_once('@').map_or(false, |(l, d)| !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !e.contains(char::is_whitespace))
}

fn clean_email(e: &str) -> Result<String> {
    let e = e.trim().to_lowercase();
    if valid_email(&e) {
        Ok(e)
    } else {
        Err(AuthError::Rejected { code: "validation_failed".into(), message: "Enter a valid email address.".into() })
    }
}

fn clean_password(p: &str) -> Result<()> {
    if p.chars().count() >= 8 && p.len() <= 256 {
        Ok(())
    } else {
        Err(AuthError::Rejected { code: "weak_password".into(), message: "".into() })
    }
}

fn clean_code(c: &str) -> Result<String> {
    let c: String = c.chars().filter(|ch| !ch.is_whitespace()).collect();
    if (6..=10).contains(&c.len()) && c.chars().all(|ch| ch.is_ascii_digit()) {
        Ok(c)
    } else {
        Err(AuthError::Rejected { code: "otp_expired".into(), message: "".into() })
    }
}

impl AccountService {
    /// The production service (OS secret store, build configuration); `None` when this build has no Supabase configuration.
    pub fn from_build(dir: PathBuf) -> Option<Arc<AccountService>> {
        let cfg = AuthConfig::from_build()?;
        Some(Arc::new(AccountService::new(cfg, os_store(STORE_NAME), dir)))
    }

    pub fn new(cfg: AuthConfig, store: Box<dyn SecretStore>, dir: PathBuf) -> AccountService {
        let svc = AccountService { mgr: SessionManager::new(Client::new(cfg), store), dir, cache: Mutex::new(Cache::default()), cancel: Mutex::new(None), last_sync_attempt: Mutex::new(0) };
        svc.load_cache();
        svc
    }

    pub fn manager(&self) -> &SessionManager {
        &self.mgr
    }

    fn cache_path(&self) -> PathBuf {
        self.dir.join("account.json")
    }

    fn lock_cache(&self) -> std::sync::MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ── local state (no network) ─────────────────────────────────────────────────────────────────────────────────────────────

    fn load_cache(&self) {
        let AuthState::SignedIn { user_id, .. } = self.mgr.state() else {
            let _ = std::fs::remove_file(self.cache_path()); // signed out: no account data stays behind
            return;
        };
        let Some(v) = std::fs::read_to_string(self.cache_path()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return };
        if v.get("userId").and_then(Value::as_str) != Some(user_id.as_str()) {
            return; // a cache of another account is never shown
        }
        let g = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let devices = v
            .get("devices")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|d| Device { id: g_in(d, "id"), device_id: g_in(d, "deviceId"), name: g_in(d, "name"), platform: g_in(d, "platform"), last_seen: g_in(d, "lastSeen"), revoked: d.get("revoked").and_then(Value::as_bool).unwrap_or(false) }).collect())
            .unwrap_or_default();
        *self.lock_cache() = Cache {
            user_id,
            email: g("email"),
            name: g("name"),
            provider: g("provider"),
            verified: v.get("verified").and_then(Value::as_bool).unwrap_or(false),
            plan: g("plan"),
            plan_status: g("planStatus"),
            account_status: g("accountStatus"),
            device_limit: v.get("deviceLimit").and_then(Value::as_i64).unwrap_or(2),
            devices,
            synced_at: v.get("syncedAt").and_then(Value::as_i64).unwrap_or(0),
            device_limit_reached: false,
        };
    }

    fn save_cache(&self) {
        let c = self.lock_cache().clone();
        let doc = json!({
            "userId": c.user_id, "email": c.email, "name": c.name, "provider": c.provider, "verified": c.verified,
            "plan": c.plan, "planStatus": c.plan_status, "accountStatus": c.account_status, "deviceLimit": c.device_limit, "syncedAt": c.synced_at,
            "devices": c.devices.iter().map(|d| json!({"id": d.id, "deviceId": d.device_id, "name": d.name, "platform": d.platform, "lastSeen": d.last_seen, "revoked": d.revoked})).collect::<Vec<_>>(),
        });
        let _ = waitplay_core::paths::write_atomic(&self.cache_path(), doc.to_string().as_bytes());
    }

    fn clear_cache(&self) {
        *self.lock_cache() = Cache::default();
        let _ = std::fs::remove_file(self.cache_path());
    }

    /// Everything the pages render. Contains no token and no secret.
    pub fn view(&self) -> Value {
        let c = self.lock_cache().clone();
        match self.mgr.state() {
            AuthState::SignedOut { reason } => json!({"configured": true, "state": "signedOut", "reason": reason, "appleEnabled": false}),
            AuthState::SignedIn { offline, .. } => {
                let device_id = self.device_id(); // only while signed in: a signed-out view must not create files
                json!({
                "configured": true, "state": "signedIn", "offline": offline, "appleEnabled": false,
                "user": {"email": c.email, "name": c.name, "provider": c.provider, "verified": c.verified},
                "plan": if c.plan.is_empty() { "free" } else { &c.plan },
                "planStatus": if c.plan_status.is_empty() { "active" } else { &c.plan_status },
                "accountStatus": if c.account_status.is_empty() { "active" } else { &c.account_status },
                "deviceLimit": if c.device_limit == 0 { 2 } else { c.device_limit },
                "deviceLimitReached": c.device_limit_reached,
                "devices": c.devices.iter().map(|d| json!({"id": d.id, "name": d.name, "platform": d.platform, "lastSeen": d.last_seen, "revoked": d.revoked, "current": d.device_id == device_id})).collect::<Vec<_>>(),
                "syncedAt": c.synced_at,
                })
            }
        }
    }

    pub fn is_signed_in(&self) -> bool {
        matches!(self.mgr.state(), AuthState::SignedIn { .. })
    }

    /// Cheap local check for the app's timer: is a token refresh or a cloud sync due? (No network.)
    pub fn upkeep_due(&self) -> bool {
        match self.mgr.next_refresh_in() {
            None => false,
            Some(0) => true,
            Some(_) => now_secs() - *self.last_sync_attempt.lock().unwrap_or_else(|e| e.into_inner()) >= SYNC_EVERY_SECS,
        }
    }

    /// A stable random id for this installation (not a hardware fingerprint), used for the device list.
    pub fn device_id(&self) -> String {
        let path = self.dir.join("device.json");
        if let Some(id) = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.get("deviceId").and_then(Value::as_str).map(String::from)) {
            if id.len() >= 16 {
                return id;
            }
        }
        let id = pkce::random_b64url(16);
        let _ = waitplay_core::paths::write_atomic(&path, json!({"deviceId": id}).to_string().as_bytes());
        id
    }

    // ── sign-in flows ────────────────────────────────────────────────────────────────────────────────────────────────────────

    pub fn sign_in_password(&self, email: &str, password: &str) -> Result<()> {
        let session = self.mgr.client().sign_in_password(&clean_email(email)?, password)?;
        self.finish_sign_in(session)
    }

    pub fn sign_up(&self, email: &str, password: &str) -> Result<SignUpOutcome> {
        let email = clean_email(email)?;
        clean_password(password)?;
        match self.mgr.client().sign_up(&email, password)? {
            SignUp::ConfirmationRequired(_) => Ok(SignUpOutcome::NeedsCode),
            SignUp::SignedIn(session) => self.finish_sign_in(session).map(|_| SignUpOutcome::SignedIn),
        }
    }

    pub fn resend_confirmation(&self, email: &str) -> Result<()> {
        self.mgr.client().resend_signup(&clean_email(email)?)
    }

    /// Emails a sign-in code. `create_user = false` for the sign-in screen (a typo must not create an account).
    pub fn send_code(&self, email: &str, create_user: bool) -> Result<()> {
        self.mgr.client().send_otp(&clean_email(email)?, create_user)
    }

    /// `kind`: `email` (sign-in), `signup` (confirm a new account), `recovery` (password reset).
    pub fn verify_code(&self, email: &str, code: &str, kind: &str) -> Result<()> {
        if !matches!(kind, "email" | "signup" | "recovery") {
            return Err(AuthError::State("Unknown code type."));
        }
        let session = self.mgr.client().verify_otp(&clean_email(email)?, &clean_code(code)?, kind)?;
        self.finish_sign_in(session)
    }

    pub fn send_password_reset(&self, email: &str) -> Result<()> {
        self.mgr.client().recover(&clean_email(email)?)
    }

    /// Verifies the reset code (which signs the user in) and sets the new password.
    pub fn reset_password(&self, email: &str, code: &str, new_password: &str) -> Result<()> {
        clean_password(new_password)?;
        self.verify_code(email, code, "recovery")?;
        let token = self.mgr.access_token()?;
        self.mgr.client().update_password(&token, new_password)
    }

    /// Browser sign-in (PKCE) with `provider` (`google`). `open` receives the authorize URL and must open the system browser;
    /// returning `false` aborts. Blocks until the callback, a timeout, or `cancel_flow()`.
    pub fn sign_in_oauth(&self, provider: &str, open: &dyn Fn(&str) -> bool) -> Result<()> {
        if provider != "google" {
            return Err(AuthError::Callback("That sign-in method is not available yet.".into()));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut slot = self.cancel.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(old) = slot.replace(cancel.clone()) {
                old.store(true, Ordering::SeqCst); // a second start replaces (and cancels) the first
            }
        }
        let result = (|| {
            let server = CallbackServer::start()?;
            let pair = pkce::new_pair();
            let url = self.mgr.client().authorize_url(provider, &server.redirect_url(), &pair.challenge);
            if !open(&url) {
                return Err(AuthError::Callback("Vibe Play could not open your browser.".into()));
            }
            let code = server.wait(OAUTH_TIMEOUT, &|| cancel.load(Ordering::SeqCst))?;
            self.mgr.client().exchange_code(&code, &pair.verifier)
        })();
        {
            let mut slot = self.cancel.lock().unwrap_or_else(|e| e.into_inner());
            if slot.as_ref().map_or(false, |c| Arc::ptr_eq(c, &cancel)) {
                *slot = None;
            }
        }
        self.finish_sign_in(result?)
    }

    pub fn cancel_flow(&self) {
        if let Some(c) = self.cancel.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            c.store(true, Ordering::SeqCst);
        }
    }

    fn finish_sign_in(&self, session: Session) -> Result<()> {
        let user = session.user.clone();
        {
            let mut c = self.lock_cache();
            *c = Cache { user_id: user.id.clone(), email: user.email.clone(), name: user.display_name.clone(), provider: user.provider.clone(), verified: user.email_confirmed, ..Cache::default() };
        }
        self.mgr.adopt(session)?;
        self.save_cache();
        let _ = self.sync(true); // best effort: a failure leaves the profile shown from the sign-in answer
        Ok(())
    }

    // ── background upkeep ────────────────────────────────────────────────────────────────────────────────────────────────────

    /// Called by the app's timer (every few seconds is fine; it only acts when something is due). Never blocks local features for
    /// longer than the HTTP timeout, never signs out on a network error. Returns true when the view changed.
    pub fn tick(&self) -> bool {
        let Some(due) = self.mgr.next_refresh_in() else { return false };
        let before = self.mgr.state();
        let mut changed = false;
        if due == 0 {
            match self.mgr.refresh() {
                Ok(_) => {}
                Err(AuthError::SessionExpired) => {
                    self.clear_cache();
                    return true;
                }
                Err(_) => return before != self.mgr.state(), // offline: try again on a later tick
            }
            changed = true;
        }
        let now = now_secs();
        let attempt = *self.last_sync_attempt.lock().unwrap_or_else(|e| e.into_inner());
        if now - attempt >= SYNC_EVERY_SECS && self.sync_cloud().is_ok() {
            changed = true;
        }
        changed || before != self.mgr.state()
    }

    /// Fetches profile, plan and devices (and registers this device). Stores the result in the local cache.
    pub fn sync_cloud(&self) -> Result<()> {
        self.sync(false)
    }

    /// `explicit`: the user just signed in on this device (may re-add a device that was removed); otherwise a background heartbeat.
    fn sync(&self, explicit: bool) -> Result<()> {
        *self.last_sync_attempt.lock().unwrap_or_else(|e| e.into_inner()) = now_secs();
        let token = self.mgr.access_token()?;
        let client = self.mgr.client();
        let AuthState::SignedIn { user_id, .. } = self.mgr.state() else { return Err(AuthError::State("Not signed in.")) };

        let registration = cloud::register_device(client, &token, &self.device_id(), &device_name(), platform(), env!("CARGO_PKG_VERSION"), explicit)?;
        if registration == DeviceRegistration::Revoked {
            // this installation was removed from the account (from another device): sign out here, keep local Vibe Play features
            self.mgr.force_sign_out("This device was removed from your account. Sign in again to add it back.");
            self.clear_cache();
            return Err(AuthError::SessionExpired);
        }
        let (name, status) = cloud::profile(client, &token, &user_id)?;
        let ent: Entitlement = cloud::entitlement(client, &token)?;
        let devices = cloud::devices(client, &token)?;
        {
            let mut c = self.lock_cache();
            if !name.is_empty() {
                c.name = name;
            }
            c.account_status = status;
            c.plan = ent.plan;
            c.plan_status = ent.status;
            c.device_limit = ent.device_limit;
            c.devices = devices;
            c.device_limit_reached = registration == DeviceRegistration::LimitReached;
            c.synced_at = now_secs();
        }
        self.save_cache();
        Ok(())
    }

    // ── account actions ──────────────────────────────────────────────────────────────────────────────────────────────────────

    pub fn revoke_device(&self, id: &str) -> Result<()> {
        let token = self.mgr.access_token()?;
        cloud::revoke_device(self.mgr.client(), &token, id)?;
        self.sync_cloud()
    }

    pub fn set_display_name(&self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(AuthError::Rejected { code: "validation_failed".into(), message: "Use a name of 1 to 80 characters.".into() });
        }
        let token = self.mgr.access_token()?;
        let AuthState::SignedIn { user_id, .. } = self.mgr.state() else { return Err(AuthError::State("Not signed in.")) };
        cloud::set_display_name(self.mgr.client(), &token, &user_id, name)?;
        self.lock_cache().name = name.to_string();
        self.save_cache();
        Ok(())
    }

    pub fn sign_out(&self) -> Result<()> {
        let result = self.mgr.sign_out("local");
        self.clear_cache();
        result
    }

    pub fn sign_out_others(&self) -> Result<()> {
        self.mgr.sign_out_others()
    }

    /// Deletes the account on the server (Edge Function), then everything local. The caller must have asked the user to re-confirm.
    pub fn delete_account(&self) -> Result<()> {
        let token = self.mgr.access_token()?;
        cloud::delete_account(self.mgr.client(), &token).map_err(delete_failed)?; // nothing local is touched unless the server confirmed
        let result = self.mgr.forget_local();
        self.clear_cache();
        let _ = std::fs::remove_file(self.dir.join("device.json"));
        result
    }
}

/// Every refusal of the deletion function becomes an explicit "not deleted" message: a 4xx/5xx must never read like a success.
fn delete_failed(e: AuthError) -> AuthError {
    let say = |code: &str, message: &str| AuthError::Rejected { code: code.into(), message: message.into() };
    match e {
        AuthError::Rejected { code, .. } if code == "NOT_FOUND" => say("delete_unavailable", "Account deletion is not available right now, so your account was not deleted. Nothing was changed."),
        AuthError::Rejected { .. } | AuthError::Unavailable => say("delete_failed", "Your account was not deleted. Nothing was changed. Try again in a moment."),
        other => other, // offline, session expired, rate limited: their own messages already say nothing happened
    }
}

fn g_in(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "other"
    }
}

fn device_name() -> String {
    let n = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    let n: String = n.chars().filter(|c| !c.is_control()).take(60).collect();
    if n.is_empty() { format!("{} device", platform()) } else { n }
}

/// Opens `url` in the user's default browser. Only the project's own https `authorize` URL is ever opened.
pub fn open_in_browser(cfg: &AuthConfig, url: &str) -> bool {
    if !url.starts_with(&format!("{}/auth/v1/authorize?", cfg.url)) || url.chars().any(|c| c.is_control() || c == ' ') {
        return false;
    }
    #[cfg(windows)]
    let status = std::process::Command::new("rundll32.exe").arg("url.dll,FileProtocolHandler").arg(url).status();
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("/usr/bin/open").arg(url).status();
    #[cfg(not(any(windows, target_os = "macos")))]
    let status: std::io::Result<std::process::ExitStatus> = Err(std::io::Error::other("unsupported"));
    status.map(|s| s.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_validation() {
        for ok in ["a@b.co", "first.last+tag@example.com"] {
            assert!(valid_email(ok), "{ok}");
        }
        for bad in ["", "a", "a@b", "@b.co", "a b@c.de", "a@.co", "a@co.", &format!("{}@x.co", "a".repeat(260))] {
            assert!(!valid_email(bad), "{bad}");
        }
        assert!(clean_password("12345678").is_ok() && clean_password("1234567").is_err());
        assert_eq!(clean_code(" 123 456 ").unwrap(), "123456");
        for bad in ["12345", "12345a", "", "12345678901"] {
            assert!(clean_code(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_the_projects_own_authorize_url_is_ever_opened() {
        let cfg = AuthConfig { url: "https://x.supabase.co".into(), anon_key: "sb_publishable_k".into() };
        for bad in ["https://evil.example/auth/v1/authorize?provider=google", "file:///C:/Windows/win.ini", "https://x.supabase.co/other", "https://x.supabase.co/auth/v1/authorize?a=1 && calc", "ms-settings:privacy"] {
            assert!(!open_in_browser(&cfg, bad), "{bad}");
        }
    }
}
