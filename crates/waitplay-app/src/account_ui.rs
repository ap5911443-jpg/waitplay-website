//! The account window (sign in, create account, codes, password reset, account settings), shared by the macOS and Windows apps.
//!
//! Rust owns every secret and every network call (`waitplay-auth`); the page (`assets/account.html`) only renders the state that is
//! pushed to it and sends intents over `window.ipc`. Intents run on worker threads, so the tray, the widget and agent detection never
//! wait for the network. With no account configured or no connection, the rest of the app is unaffected.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tao::dpi::LogicalSize;
use tao::event::WindowEvent;
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Theme, Window, WindowBuilder, WindowId};
use waitplay_auth::service::open_in_browser;
use waitplay_auth::{AccountService, AuthError};
use wry::{NewWindowResponse, WebContext, WebView, WebViewBuilder};

pub const HTML: &str = include_str!("../assets/account.html");
const MAX_MESSAGE: usize = 8 * 1024;

/// Posts a JSON message back to the main thread (the apps turn it into an `Ipc(Account, ..)` event).
pub type Notify = Arc<dyn Fn(String) + Send + Sync>;

pub struct AccountUi {
    service: Option<Arc<AccountService>>,
    win: Option<(Window, WebView)>,
    /// A sign-in/account operation is running; the page disables its buttons, this is the guard against a second one.
    busy: Arc<AtomicBool>,
    upkeep_running: Arc<AtomicBool>,
    /// Earliest time of the next background refresh/sync attempt (backs off while offline).
    next_upkeep: Arc<Mutex<(Instant, u64)>>,
}

impl AccountUi {
    pub fn new(dir: std::path::PathBuf) -> AccountUi {
        AccountUi {
            service: AccountService::from_build(dir),
            win: None,
            busy: Arc::new(AtomicBool::new(false)),
            upkeep_running: Arc::new(AtomicBool::new(false)),
            next_upkeep: Arc::new(Mutex::new((Instant::now(), 5))),
        }
    }

    pub fn available(&self) -> bool {
        self.service.is_some()
    }

    /// Drops the window (and its page) now; used when the app quits.
    pub fn close(&mut self) {
        if let Some(s) = &self.service {
            s.cancel_flow();
        }
        self.win = None;
    }

    pub fn is_open(&self) -> bool {
        self.win.is_some()
    }

    /// Shows the window (creating it the first time). `configure` lets each platform add its own webview options.
    pub fn open<T: 'static>(
        &mut self,
        target: &EventLoopWindowTarget<T>,
        context: Option<&mut WebContext>,
        notify: Notify,
        theme: Option<Theme>,
        configure: impl FnOnce(WebViewBuilder) -> WebViewBuilder,
    ) -> Result<(), String> {
        if let Some((w, _)) = self.win.as_ref() {
            w.set_visible(true);
            w.set_minimized(false);
            w.set_focus();
            return Ok(());
        }
        let window = WindowBuilder::new()
            .with_title("Vibe Play Account")
            .with_inner_size(LogicalSize::new(440.0, 700.0))
            .with_min_inner_size(LogicalSize::new(380.0, 560.0))
            .with_resizable(true)
            .with_theme(theme)
            .with_visible(false)
            .build(target)
            .map_err(|e| format!("window: {e}"))?;
        let base = match context {
            Some(c) => WebViewBuilder::new_with_web_context(c),
            None => WebViewBuilder::new(),
        };
        let builder = configure(base)
            .with_html(HTML)
            .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
            .with_ipc_handler(move |req| notify(req.into_body()));
        let webview = builder.build(&window).map_err(|e| format!("webview: {e}"))?;
        center(&window);
        window.set_visible(true);
        window.set_focus();
        self.win = Some((window, webview));
        Ok(())
    }

    /// Returns true when the event closed the account window (the caller then has nothing more to do for it).
    pub fn on_window_event(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        if let WindowEvent::CloseRequested = event {
            if self.win.as_ref().map_or(false, |(w, _)| w.id() == id) {
                if let Some(s) = &self.service {
                    s.cancel_flow(); // closing the window abandons a browser sign-in in progress
                }
                self.win = None;
                return true;
            }
        }
        false
    }

    /// Handles a message from the page or from a worker thread. Runs on the main thread.
    pub fn on_message(&mut self, body: &str, notify: &Notify) {
        if body.len() > MAX_MESSAGE {
            return;
        }
        let Ok(msg) = serde_json::from_str::<Value>(body) else { return };
        let Some(t) = msg.get("t").and_then(Value::as_str) else { return };
        let Some(service) = self.service.clone() else { return self.push(&json!({"configured": false, "state": "signedOut"})) };
        let s = |k: &str| msg.get(k).and_then(Value::as_str).unwrap_or("").to_string();

        match t {
            "ready" | "__view" => self.push(&service.view()),
            "__done" => {
                self.busy.store(false, Ordering::SeqCst);
                self.push(&service.view());
                self.eval(&format!("window.wpAccount&&wpAccount.result({})", msg.get("result").cloned().unwrap_or(Value::Null)));
            }
            "cancelFlow" => service.cancel_flow(),
            "google" | "signInPassword" | "signUp" | "sendCode" | "verifyCode" | "forgot" | "reset" | "resendConfirm" | "signOut" | "signOutOthers" | "revokeDevice" | "setName" | "deleteAccount" => {
                if self.busy.swap(true, Ordering::SeqCst) {
                    return; // one operation at a time
                }
                let (op, notify, svc) = (t.to_string(), notify.clone(), service);
                let (email, password, code, kind, id, name) = (s("email"), s("password"), s("code"), s("kind"), s("id"), s("name"));
                let create = msg.get("create").and_then(Value::as_bool).unwrap_or(false);
                let _ = std::thread::Builder::new().name("wp-account".into()).spawn(move || {
                    let outcome: Result<&'static str, AuthError> = match op.as_str() {
                        "google" => {
                            let cfg = svc.manager().client().config().clone();
                            svc.sign_in_oauth("google", &|url| open_in_browser(&cfg, url)).map(|_| "")
                        }
                        "signInPassword" => svc.sign_in_password(&email, &password).map(|_| ""),
                        "signUp" => svc.sign_up(&email, &password).map(|o| if o == waitplay_auth::service::SignUpOutcome::NeedsCode { "needsCode" } else { "" }),
                        "sendCode" => svc.send_code(&email, create).map(|_| ""),
                        "verifyCode" => svc.verify_code(&email, &code, if kind.is_empty() { "email" } else { &kind }).map(|_| ""),
                        "forgot" => svc.send_password_reset(&email).map(|_| ""),
                        "reset" => svc.reset_password(&email, &code, &password).map(|_| ""),
                        "resendConfirm" => svc.resend_confirmation(&email).map(|_| ""),
                        "signOut" => svc.sign_out().map(|_| ""),
                        "signOutOthers" => svc.sign_out_others().map(|_| ""),
                        "revokeDevice" => svc.revoke_device(&id).map(|_| ""),
                        "setName" => svc.set_display_name(&name).map(|_| ""),
                        "deleteAccount" => svc.delete_account().map(|_| ""),
                        _ => Err(AuthError::State("Unknown request.")),
                    };
                    let result = match outcome {
                        Ok(o) => json!({"op": op, "ok": true, "outcome": o}),
                        Err(e) => {
                            let code = if let AuthError::Rejected { code, .. } = &e { code.clone() } else { String::new() };
                            json!({"op": op, "ok": false, "error": e.user_message(), "code": code})
                        }
                    };
                    notify(json!({"t": "__done", "result": result}).to_string());
                });
            }
            _ => {}
        }
    }

    /// Called about once a second by the app: starts a background token refresh / cloud sync when one is due. Never blocks.
    pub fn on_tick(&self, notify: &Notify) {
        let Some(service) = self.service.clone() else { return };
        if !service.upkeep_due() || self.upkeep_running.load(Ordering::SeqCst) {
            return;
        }
        {
            let g = self.next_upkeep.lock().unwrap_or_else(|e| e.into_inner());
            if Instant::now() < g.0 {
                return;
            }
        }
        self.upkeep_running.store(true, Ordering::SeqCst);
        let (running, next, notify) = (self.upkeep_running.clone(), self.next_upkeep.clone(), notify.clone());
        let _ = std::thread::Builder::new().name("wp-account-upkeep".into()).spawn(move || {
            let changed = service.tick();
            let healthy = matches!(service.manager().state(), waitplay_auth::session::AuthState::SignedIn { offline: false, .. }) || !service.is_signed_in();
            {
                let mut g = next.lock().unwrap_or_else(|e| e.into_inner());
                // success: look again in a minute; failure (offline): back off 5 s, 10 s, ... up to 5 minutes
                g.1 = if healthy { 5 } else { (g.1 * 2).min(300) };
                g.0 = Instant::now() + Duration::from_secs(if healthy { 60 } else { g.1 });
            }
            running.store(false, Ordering::SeqCst);
            if changed {
                notify("{\"t\":\"__view\"}".to_string());
            }
        });
    }

    fn push(&self, view: &Value) {
        self.eval(&format!("window.wpAccount&&wpAccount.set({view})"));
    }

    fn eval(&self, script: &str) {
        if let Some((_, wv)) = self.win.as_ref() {
            let _ = wv.evaluate_script(script);
        }
    }
}

fn center(window: &Window) {
    let Some(m) = window.current_monitor().or_else(|| window.primary_monitor()) else { return };
    let (ms, mp, ws) = (m.size(), m.position(), window.outer_size());
    let x = mp.x + (ms.width as i32 - ws.width as i32) / 2;
    let y = mp.y + (ms.height as i32 - ws.height as i32) / 2;
    window.set_outer_position(tao::dpi::PhysicalPosition::new(x.max(mp.x), y.max(mp.y)));
}
