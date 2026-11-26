//! The Vibe Play menu-bar application (macOS): one process that owns the tray icon and its menu, the floating widget, the settings
//! window, the global shortcuts and the helpers (bridge, agent watchers). Entertainment windows stay in their own companion
//! processes (`companion`), started by the agent hooks or by "Open Now".
//!
//! Threads: everything AppKit runs on the main thread (this event loop). One ticker thread wakes the loop once per second; bridge
//! polls run on short-lived worker threads and report back through the event loop proxy, so the UI thread never waits on I/O.

use super::menu_model::{self, Entry};
use super::status::{self, SessionsView, Status};
use super::{icons, integrations, login_item, permissions};
use crate::account_ui::{AccountUi, Notify};
use crate::placement::{self, Screen};
use crate::{platform, wp_error, wp_info, wp_warn};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS, EventLoopWindowTargetExtMacOS, WindowBuilderExtMacOS, WindowExtMacOS};
use tao::window::{Theme, Window, WindowBuilder, WindowId};
use tray_icon::menu::{CheckMenuItem, ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use waitplay_core::settings::{Appearance, Settings, WidgetPosition};
use waitplay_core::{companion_state, now_ms, paths, process, signals, state, BridgeClient};
use waitplay_core::insights;

use crate::island::{self, Act};
use crate::sound;
use wry::{WebView, WebViewBuilder};

const WEBSITE: &str = "https://github.com/ap5911443-jpg/waitplay-website";
const SUPPORT: &str = "https://github.com/ap5911443-jpg/waitplay-website/issues";
const SETTINGS_HTML: &str = include_str!("../../assets/settings.html");
const WIDGET_HTML: &str = include_str!("../../assets/mac_widget.html");
/// The companion character, spliced into the page at its marker.
const MASCOT_JS: &str = include_str!("../../assets/mascot.js");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD: &str = match option_env!("WAITPLAY_BUILD") {
    Some(b) => b,
    None => "dev",
};

/// How long the widget waits for the page's first size report before it shows anyway.
const REFRESH_EVERY: u64 = 2;
const SUPERVISE_EVERY: u64 = 5;
/// The bridge must have been unreachable this many refreshes in a row before the app calls it an error (it is still starting).
const BRIDGE_GRACE: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    Widget,
    Settings,
    Account,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HotkeyAction {
    ToggleAutomation,
    ToggleWidget,
}

enum Ev {
    Tick,
    Sessions(Option<String>),
    Ipc(Target, String),
    Menu(String),
    Hotkey(u32),
    SoundChosen(Option<String>),
}

// ── helpers ──────────────────────────────────────────────────────────────────────────────────────────────────────────────

fn home() -> PathBuf {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

fn open_with_system(arg: &str) {
    match std::process::Command::new("/usr/bin/open").arg(arg).spawn() {
        Ok(mut c) => {
            std::thread::spawn(move || {
                let _ = c.wait();
            });
        }
        Err(e) => wp_warn!("could not run open {}: {}", arg, e),
    }
}

fn theme_for(a: Appearance) -> Option<Theme> {
    match a {
        Appearance::System => None,
        Appearance::Light => Some(Theme::Light),
        Appearance::Dark => Some(Theme::Dark),
    }
}

fn bridge_reachable() -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], paths::bridge_port()));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

fn process_running(pattern: &str) -> bool {
    std::process::Command::new("/usr/bin/pgrep").args(["-f", pattern]).output().map(|o| o.status.success()).unwrap_or(false)
}

fn parse_hotkey(text: &str) -> Result<global_hotkey::hotkey::HotKey, String> {
    let hk: global_hotkey::hotkey::HotKey = text.parse().map_err(|e| format!("{}", e))?;
    if hk.mods.is_empty() {
        return Err("needs at least one modifier key".into());
    }
    Ok(hk)
}

// ── helper processes ─────────────────────────────────────────────────────────────────────────────────────────────────────

struct Helper {
    child: Option<Child>,
    started: Instant,
    restarts: u32,
    next_try: Instant,
    last_error: Option<String>,
}

impl Helper {
    fn new() -> Helper {
        Helper { child: None, started: Instant::now(), restarts: 0, next_try: Instant::now(), last_error: None }
    }

    /// Reaps an exited child and schedules the restart with a growing delay (5 s .. 60 s).
    fn reap(&mut self, name: &str) {
        let Some(c) = self.child.as_mut() else { return };
        match c.try_wait() {
            Ok(Some(status)) => {
                let lived = self.started.elapsed();
                self.restarts = if lived > Duration::from_secs(60) { 1 } else { self.restarts + 1 };
                let delay = Duration::from_secs((5u64 << self.restarts.min(4)).min(60));
                self.next_try = Instant::now() + delay;
                self.child = None;
                self.last_error = Some(format!("{} stopped ({}); restarting in {} s", name, status, delay.as_secs()));
                wp_warn!("{}", self.last_error.as_deref().unwrap_or(""));
            }
            Ok(None) => {}
            Err(e) => wp_warn!("{}: cannot query the process: {}", name, e),
        }
    }

    fn start(&mut self, name: &str, exe: &std::path::Path, args: &[String]) {
        if Instant::now() < self.next_try {
            return;
        }
        if !exe.is_file() {
            self.last_error = Some(format!("{} not found at {}", name, exe.display()));
            self.next_try = Instant::now() + Duration::from_secs(60);
            return;
        }
        match process::spawn_detached(exe, args) {
            Ok(c) => {
                wp_info!("started {} (pid {})", name, c.id());
                self.child = Some(c);
                self.started = Instant::now();
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("cannot start {}: {}", name, e));
                wp_error!("{}", self.last_error.as_deref().unwrap_or(""));
                self.next_try = Instant::now() + Duration::from_secs(30);
            }
        }
    }

    fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

struct Supervisor {
    bridge: Helper,
    watchers: HashMap<&'static str, Helper>,
}

impl Supervisor {
    fn new() -> Supervisor {
        Supervisor { bridge: Helper::new(), watchers: HashMap::new() }
    }

    fn ensure_bridge(&mut self) {
        self.bridge.reap("waitplay-bridge");
        if self.bridge.child.is_none() && !bridge_reachable() {
            self.bridge.start("waitplay-bridge", &paths::sibling_exe("waitplay-bridge"), &[]);
        }
    }

    /// Runs exactly the wanted watchers (`(agent id, hook command)`), stops the others.
    fn sync_watchers(&mut self, wanted: &[(&'static str, &'static str)]) {
        let ids: Vec<&str> = wanted.iter().map(|(id, _)| *id).collect();
        let stale: Vec<&'static str> = self.watchers.keys().copied().filter(|k| !ids.contains(k)).collect();
        for k in stale {
            if let Some(mut h) = self.watchers.remove(k) {
                h.stop();
            }
        }
        for &(id, cmd) in wanted {
            let h = self.watchers.entry(id).or_insert_with(Helper::new);
            h.reap(cmd);
            if h.child.is_none() {
                // a watcher left over from an earlier run (the app crashed) already does the job
                if process_running(&format!("waitplay-hook {}", cmd)) {
                    continue;
                }
                h.start(cmd, &paths::sibling_exe("waitplay-hook"), &[cmd.to_string()]);
            }
        }
    }

    fn watcher_error(&self, id: &str) -> Option<String> {
        self.watchers.get(id).and_then(|h| h.last_error.clone())
    }

    fn shutdown(&mut self) {
        self.bridge.stop();
        for h in self.watchers.values_mut() {
            h.stop();
        }
    }
}

// ── windows ──────────────────────────────────────────────────────────────────────────────────────────────────────────────

struct WidgetWin {
    // the webview drops before the window that hosts it
    webview: WebView,
    glass: Option<platform::Glass>,
    window: Window,
    size: Option<(f64, f64)>,
    shown: bool,
    /// Frame origin of the display the widget lives on; follows display changes, falls back to the primary display.
    screen_origin: Option<(f64, f64)>,
}

struct SettingsWin {
    webview: WebView,
    window: Window,
}

#[derive(Default)]
struct Runtime {
    enabled: bool,
    bridge_ok: bool,
    bridge_misses: u32,
    view: SessionsView,
    entertainment: Option<String>,
}

struct App {
    settings: Settings,
    rt: Runtime,
    proxy: EventLoopProxy<Ev>,
    supervisor: Supervisor,
    widget: Option<WidgetWin>,
    settings_win: Option<SettingsWin>,
    account: AccountUi,
    tray: Option<TrayIcon>,
    menu: Option<Menu>,
    last_menu: Vec<Entry>,
    hotkeys: Option<global_hotkey::GlobalHotKeyManager>,
    registered: Vec<(global_hotkey::hotkey::HotKey, HotkeyAction)>,
    hotkey_map: Arc<Mutex<HashMap<u32, HotkeyAction>>>,
    shortcut_errors: [Option<String>; 2],
    screens: Vec<Screen>,
    ticks: u64,
    polling: bool,
    last_widget_state: Option<Value>,
    last_settings_state: Option<String>,
    /// (id, severity, message, hint)
    problems: Vec<(String, String, String, String)>,
    update_detail: &'static str,
    quitting: bool,
    /// Grouped notifications and sound cues, from the session list.
    tracker: island::Tracker,
    /// Timings of finished work for the Insights page (local only).
    recorder: insights::Recorder,
    last_sound_ms: i64,
}

impl App {
    fn new(proxy: EventLoopProxy<Ev>, settings: Settings) -> App {
        App {
            rt: Runtime { enabled: state::is_enabled(), ..Default::default() },
            settings,
            proxy,
            supervisor: Supervisor::new(),
            widget: None,
            settings_win: None,
            account: AccountUi::new(waitplay_core::paths::app_support_dir()),
            tray: None,
            menu: None,
            last_menu: Vec::new(),
            hotkeys: None,
            registered: Vec::new(),
            hotkey_map: Arc::new(Mutex::new(HashMap::new())),
            shortcut_errors: [None, None],
            screens: Vec::new(),
            ticks: 0,
            polling: false,
            last_widget_state: None,
            last_settings_state: None,
            problems: Vec::new(),
            update_detail: "Automatic update checks arrive in a later release. New versions are published on the website.",
            quitting: false,
            tracker: island::Tracker::new(),
            recorder: insights::Recorder::new(),
            last_sound_ms: 0,
        }
    }

    // ── start-up ────────────────────────────────────────────────────────────────────────────────────────────────────────

    fn init(&mut self, target: &EventLoopWindowTarget<Ev>) {
        self.screens = platform::screens();
        self.supervisor.ensure_bridge();
        self.create_tray();
        self.setup_hotkeys();
        self.refresh_widget_window(target);
        self.sync_login_item();
        self.refresh_runtime(true);
        wp_info!("Vibe Play {} ({}) started; {} display(s)", VERSION, BUILD, self.screens.len());
    }

    fn create_tray(&mut self) {
        let icon = match Icon::from_rgba(icons::menu_bar_rgba(36), 36, 36) {
            Ok(i) => i,
            Err(e) => {
                wp_error!("menu bar icon: {}", e);
                self.problem("tray", "error", "The menu bar icon could not be created.", &e.to_string());
                return;
            }
        };
        let built = TrayIconBuilder::new().with_icon(icon).with_icon_as_template(true).with_tooltip("Vibe Play").build();
        match built {
            Ok(t) => {
                wp_info!("menu bar item created");
                self.tray = Some(t);
            }
            Err(e) => {
                wp_error!("menu bar item: {}", e);
                self.problem("tray", "error", "The menu bar item could not be created.", &e.to_string());
            }
        }
    }

    fn problem(&mut self, id: &str, severity: &str, message: &str, hint: &str) {
        self.problems.retain(|p| p.0 != id);
        self.problems.push((id.to_string(), severity.to_string(), message.to_string(), hint.to_string()));
    }

    fn clear_problem(&mut self, id: &str) {
        self.problems.retain(|p| p.0 != id);
    }

    // ── hotkeys ─────────────────────────────────────────────────────────────────────────────────────────────────────────

    fn setup_hotkeys(&mut self) {
        match global_hotkey::GlobalHotKeyManager::new() {
            Ok(m) => self.hotkeys = Some(m),
            Err(e) => {
                wp_warn!("global shortcuts unavailable: {}", e);
                self.shortcut_errors = [Some(e.to_string()), Some(e.to_string())];
                return;
            }
        }
        let proxy = self.proxy.clone();
        let map = self.hotkey_map.clone();
        global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |e: global_hotkey::GlobalHotKeyEvent| {
            if e.state() == global_hotkey::HotKeyState::Pressed && map.lock().map(|m| m.contains_key(&e.id())).unwrap_or(false) {
                let _ = proxy.send_event(Ev::Hotkey(e.id()));
            }
        }));
        self.register_hotkeys();
    }

    fn register_hotkeys(&mut self) {
        let Some(manager) = self.hotkeys.as_ref() else { return };
        for (hk, _) in self.registered.drain(..) {
            let _ = manager.unregister(hk);
        }
        let mut map = HashMap::new();
        let wanted = [
            (HotkeyAction::ToggleAutomation, self.settings.shortcuts.toggle_automation.clone()),
            (HotkeyAction::ToggleWidget, self.settings.shortcuts.toggle_widget.clone()),
        ];
        let mut errors: [Option<String>; 2] = [None, None];
        let mut used: Vec<u32> = Vec::new();
        for (i, (action, text)) in wanted.iter().enumerate() {
            let hk = match parse_hotkey(text) {
                Ok(h) => h,
                Err(e) => {
                    errors[i] = Some(e);
                    continue;
                }
            };
            if used.contains(&hk.id()) {
                errors[i] = Some("already used by the other shortcut".into());
                continue;
            }
            match manager.register(hk) {
                Ok(()) => {
                    used.push(hk.id());
                    map.insert(hk.id(), *action);
                    self.registered.push((hk, *action));
                }
                Err(e) => errors[i] = Some(e.to_string()),
            }
        }
        for (i, e) in errors.iter().enumerate() {
            if let Some(e) = e {
                wp_warn!("shortcut {} could not be registered: {}", wanted[i].1, e);
            }
        }
        if let Ok(mut m) = self.hotkey_map.lock() {
            *m = map;
        }
        self.shortcut_errors = errors;
    }

    // ── state refresh ───────────────────────────────────────────────────────────────────────────────────────────────────

    fn on_tick(&mut self, target: &EventLoopWindowTarget<Ev>) {
        self.ticks += 1;
        self.account.on_tick(&self.account_notify());
        // a second launch of the app (run from a shell) asks this one to show its settings
        if open_request_pending() {
            self.open_settings(target);
        }
        if self.ticks % REFRESH_EVERY == 0 {
            self.refresh_runtime(false);
            let now = platform::screens();
            if now != self.screens {
                wp_info!("display configuration changed ({} -> {} displays)", self.screens.len(), now.len());
                self.screens = now;
                self.refresh_widget_window(target);
            }
        }
        if self.ticks % SUPERVISE_EVERY == 0 {
            self.supervise();
        }
        // a notification on screen is timed to the second; everything else refreshes every REFRESH_EVERY ticks
        if self.tracker.toast().is_some() && self.ticks % REFRESH_EVERY != 0 {
            self.render();
        }
    }

    fn supervise(&mut self) {
        self.supervisor.ensure_bridge();
        let wanted: Vec<(&'static str, &'static str)> = integrations::AGENTS
            .iter()
            .filter_map(|a| match a.kind {
                integrations::Kind::Watcher(cmd) if self.settings.watchers.iter().any(|w| w == a.id) && integrations::is_detected(a, &home()) => Some((a.id, cmd)),
                _ => None,
            })
            .collect();
        if self.rt.enabled {
            self.supervisor.sync_watchers(&wanted);
        } else {
            self.supervisor.sync_watchers(&[]);
        }
    }

    fn refresh_runtime(&mut self, first: bool) {
        self.rt.enabled = state::is_enabled();
        let live = companion_state::live();
        self.rt.entertainment = live.iter().find(|c| c.visible).map(|c| c.site.clone());
        if !self.polling {
            self.polling = true;
            let proxy = self.proxy.clone();
            let spawned = std::thread::Builder::new().name("wp-poll".into()).spawn(move || {
                let body = BridgeClient::new().get_sessions(Duration::from_millis(600));
                let _ = proxy.send_event(Ev::Sessions(body));
            });
            if spawned.is_err() {
                self.polling = false;
            }
        }
        if first {
            self.supervise();
        }
        self.render();
    }

    fn on_sessions(&mut self, body: Option<String>) {
        self.polling = false;
        match body.as_deref().and_then(|b| status::parse_sessions(b, now_ms())) {
            Some(view) => {
                self.rt.bridge_ok = true;
                self.rt.bridge_misses = 0;
                self.rt.view = view;
            }
            None => {
                self.rt.bridge_misses += 1;
                if self.rt.bridge_misses >= BRIDGE_GRACE {
                    self.rt.bridge_ok = false;
                }
                self.rt.view = SessionsView::default();
            }
        }
        self.observe_sessions();
        self.render();
    }

    /// Feeds the new session list to the notification tracker (and so to the sounds) and to the insights recorder.
    fn observe_sessions(&mut self) {
        let now = now_ms();
        let sessions: Vec<island::Sess> = self.rt.view.sessions.iter().map(|r| island::Sess { id: r.id.clone(), name: r.name.clone(), act: r.act }).collect();
        let key = self.rt.entertainment.clone().unwrap_or_else(|| self.settings.preferred_platform.clone());
        let label = menu_model::site_label(&key);
        let ctx = island::Context { platform: label, automation: self.rt.enabled, entertainment_open: self.rt.entertainment.is_some() };
        let cues = self.tracker.observe(now, &sessions, &ctx, self.settings.notification_secs, self.settings.notifications);
        if cues.any() {
            self.play_for(cues, now);
        }
        let obs: Vec<insights::Obs> = self
            .rt
            .view
            .sessions
            .iter()
            .map(|r| insights::Obs {
                id: r.id.clone(),
                agent: r.agent_id.clone(),
                phase: match r.act {
                    Act::Working => insights::Phase::Working,
                    Act::Waiting => insights::Phase::Waiting,
                    Act::Done => insights::Phase::Completed,
                    Act::Failed => insights::Phase::Failed,
                },
            })
            .collect();
        let done = self.recorder.observe(now, &obs, self.rt.entertainment.as_deref());
        if !done.is_empty() {
            let _ = std::thread::Builder::new().name("wp-insights".into()).spawn(move || insights::append(&done));
        }
    }

    /// One sound per group of events, chosen by priority, never closer than 1.5 s to the previous one.
    fn play_for(&mut self, c: island::Cues, now: i64) {
        let e = self.settings.sound_events;
        let cue = if c.error && e.error {
            Some(sound::Cue::Error)
        } else if c.all_done && e.all_done {
            Some(sound::Cue::AllDone)
        } else if (c.completed || c.all_done) && e.completed {
            Some(sound::Cue::Completed)
        } else if c.started && e.started {
            Some(sound::Cue::Started)
        } else if c.opened && e.opened {
            Some(sound::Cue::Opened)
        } else {
            None
        };
        if let Some(cue) = cue {
            if now - self.last_sound_ms >= 1_500 {
                self.last_sound_ms = now;
                sound::play(&self.settings, cue);
            }
        }
    }

    fn status(&self) -> Status {
        status::overall(self.rt.enabled, self.rt.bridge_ok || self.rt.bridge_misses < BRIDGE_GRACE, &self.rt.view, self.rt.entertainment.is_some())
    }

    // ── rendering the three surfaces ────────────────────────────────────────────────────────────────────────────────────

    fn render(&mut self) {
        self.tracker.tick(now_ms(), self.settings.notification_secs);
        let st = self.status();
        let platform_label = menu_model::site_label(&self.settings.preferred_platform);
        let (title, detail) = status::describe(st, &self.rt.view, platform_label);
        self.render_menu(st, &title);
        self.render_widget(st, &title, &detail);
        self.push_settings();
    }

    fn detected_agents(&self) -> Vec<(String, String)> {
        let h = home();
        integrations::AGENTS.iter().filter(|a| integrations::is_detected(a, &h)).map(|a| (a.id.to_string(), a.name.to_string())).collect()
    }

    fn render_menu(&mut self, st: Status, title: &str) {
        let entries = menu_model::build(st, title, self.rt.enabled, &self.rt.view, &self.detected_agents(), &self.settings);
        if entries == self.last_menu {
            return;
        }
        let Some(tray) = self.tray.as_ref() else { return };
        match build_menu(&entries) {
            Ok(menu) => {
                tray.set_menu(Some(Box::new(menu.clone())));
                let _ = tray.set_tooltip(Some(format!("Vibe Play \u{2014} {}", title)));
                self.menu = Some(menu);
                self.last_menu = entries;
            }
            Err(e) => wp_error!("could not build the menu: {}", e),
        }
    }

    /// One entry per live session for the island and the Sessions pane.
    fn session_rows_json(&self) -> Vec<Value> {
        self.rt
            .view
            .sessions
            .iter()
            .map(|r| {
                let act = match r.act {
                    Act::Working => "working",
                    Act::Waiting => "waiting",
                    Act::Done => "done",
                    Act::Failed => "failed",
                };
                json!({ "id": r.id, "name": r.name, "act": act, "since": r.since, "durMs": r.duration_ms, "terminal": r.terminal })
            })
            .collect()
    }

    fn widget_state(&self, st: Status, title: &str, detail: &str) -> Value {
        let key = self.settings.preferred_platform.as_str();
        let view = &self.rt.view;
        let lead = view.agents.iter().find(|a| a.activity == status::Activity::Working).or_else(|| view.agents.iter().find(|a| a.activity != status::Activity::Idle));
        let status_text = match lead.map(|a| a.activity) {
            Some(status::Activity::Working) => "Working",
            Some(status::Activity::Waiting) => "Needs you",
            _ => title,
        };
        let platforms: Vec<Value> = waitplay_core::settings::SITE_KEYS
            .iter()
            .map(|k| json!({ "key": k, "label": menu_model::site_label(k), "on": self.settings.platforms.get(k) == Some(true) }))
            .collect();
        let now = now_ms();
        let open_label = self.rt.entertainment.as_deref().map(menu_model::site_label);
        let opening = view.sessions.iter().any(|r| r.act.active() && r.since.is_some_and(|s| now - s < 10_000));
        let just_finished = view.sessions.iter().any(|r| !r.act.active() && now - r.at < 8_000);
        let ent = island::entertainment_line(self.rt.enabled, open_label, menu_model::site_label(key), view.active_sessions, opening, just_finished);
        let rows = self.session_rows_json();
        let toast = self.tracker.toast().map(|t| json!({ "id": t.id, "kind": t.kind, "title": t.title, "body": t.body, "secs": self.settings.notification_secs }));
        json!({
            "status": st.as_str(),
            "label": title,
            "detail": detail,
            "agent": lead.map(|a| a.name.as_str()).unwrap_or(""),
            "terminal": lead.map(|a| a.terminal.as_str()).unwrap_or(""),
            "sessions": view.active_sessions,
            "statusText": status_text,
            "platforms": platforms,
            "platform": key,
            "platformLabel": menu_model::site_label(key),
            "enabled": self.rt.enabled,
            "compact": self.settings.widget_compact,
            "showStatus": self.settings.widget_show_status,
            "corner": self.settings.widget_position.as_str(),
            // the camera notch of the display the widget is on (points), so the page can start exactly as big as the hardware
            "notch": self.widget_screen().and_then(|s| platform::notch_size(s.frame.x, s.frame.y)).map(|(w, h)| json!({ "w": w, "h": h })),
            "since": view.working_since,
            "sessionList": rows,
            "ent": { "state": ent.state, "text": ent.text, "hint": ent.hint },
            "entOpen": self.rt.entertainment.is_some(),
            "toast": toast,
            "muted": self.settings.sound_muted,
            "mascot": { "on": self.settings.mascot, "intensity": self.settings.mascot_intensity.as_str(), "idle": self.settings.mascot_idle, "pointer": self.settings.mascot_pointer, "reduce": self.settings.reduce_motion },
        })
    }

    fn render_widget(&mut self, st: Status, title: &str, detail: &str) {
        let state = self.widget_state(st, title, detail);
        if self.last_widget_state.as_ref() == Some(&state) {
            return;
        }
        if let Some(w) = self.widget.as_ref() {
            let _ = w.webview.evaluate_script(&format!("window.wpWidget&&wpWidget.apply({})", state));
        }
        self.last_widget_state = Some(state);
    }

    fn agents_json(&self) -> Value {
        let h = home();
        let hook_exe = paths::sibling_exe("waitplay-hook");
        let list: Vec<Value> = integrations::AGENTS
            .iter()
            .map(|a| {
                let detected = integrations::is_detected(a, &h);
                let activity = self.rt.view.activity_of(a.id).as_str();
                let integration = match a.kind {
                    integrations::Kind::ClaudeHooks => {
                        let path = integrations::claude_settings_path(&h);
                        match integrations::claude_state(&path, &hook_exe) {
                            integrations::ClaudeState::Configured => json!({"state": "configured", "detail": format!("Hooks installed in {}", path.display()), "canInstall": true, "howTo": ""}),
                            integrations::ClaudeState::NotConfigured => json!({"state": "notConfigured", "detail": if detected { "Hooks are not installed yet." } else { "Claude Code was not found on this Mac." }, "canInstall": detected, "howTo": ""}),
                            integrations::ClaudeState::Outdated => json!({"state": "notConfigured", "detail": "The hooks point to an old Vibe Play location. Install to update them.", "canInstall": true, "howTo": ""}),
                            integrations::ClaudeState::Error(e) => json!({"state": "error", "detail": e, "canInstall": false, "howTo": ""}),
                        }
                    }
                    integrations::Kind::Watcher(_) => {
                        let on = self.settings.watchers.iter().any(|w| w == a.id);
                        if !detected {
                            json!({"state": "notConfigured", "detail": format!("{} was not found on this Mac.", a.name), "canInstall": false, "howTo": ""})
                        } else if let Some(e) = self.supervisor.watcher_error(a.id) {
                            json!({"state": "error", "detail": e, "canInstall": true, "howTo": ""})
                        } else if on {
                            json!({"state": "configured", "detail": "Vibe Play watches this agent while automation is on.", "canInstall": true, "howTo": ""})
                        } else {
                            json!({"state": "notConfigured", "detail": "The watcher is off.", "canInstall": true, "howTo": ""})
                        }
                    }
                    integrations::Kind::Manual(_) => json!({"state": "manual", "detail": "Connect it by adding hook commands.", "canInstall": false, "howTo": integrations::how_to(a, &hook_exe)}),
                };
                json!({"id": a.id, "name": a.name, "detected": detected, "activity": activity, "integration": integration})
            })
            .collect();
        Value::Array(list)
    }

    fn errors_json(&self, agents: &Value) -> Value {
        let mut errors: Vec<Value> = Vec::new();
        if self.rt.enabled && !self.rt.bridge_ok && self.rt.bridge_misses >= BRIDGE_GRACE {
            let hint = self.supervisor.bridge.last_error.clone().unwrap_or_else(|| "Vibe Play starts it automatically; if this stays, restart Vibe Play.".into());
            errors.push(json!({"id": "bridge", "severity": "error", "message": "The local bridge is not running, so agent activity cannot be seen.", "hint": hint}));
        }
        for a in agents.as_array().into_iter().flatten() {
            let (id, detected) = (a["id"].as_str().unwrap_or(""), a["detected"].as_bool().unwrap_or(false));
            let integ = &a["integration"];
            if detected && a["id"] == "claude-code" && integ["state"] == "notConfigured" {
                errors.push(json!({"id": "claude-hooks", "severity": "warning", "message": "Claude Code is installed but Vibe Play's hooks are not set up.", "hint": "Open AI Agents and choose Install Hooks."}));
            }
            if integ["state"] == "error" {
                errors.push(json!({"id": format!("integration-{}", id), "severity": "error", "message": format!("{}: {}", a["name"].as_str().unwrap_or(id), integ["detail"].as_str().unwrap_or("")), "hint": "See the log file for details."}));
            }
        }
        for (i, name) in ["Enable/Disable automation", "Show/Hide widget"].iter().enumerate() {
            if let Some(e) = &self.shortcut_errors[i] {
                errors.push(json!({"id": format!("shortcut-{}", i), "severity": "warning", "message": format!("The shortcut for \"{}\" is not active.", name), "hint": e}));
            }
        }
        for (id, severity, message, hint) in &self.problems {
            errors.push(json!({"id": id, "severity": severity, "message": message, "hint": hint}));
        }
        Value::Array(errors)
    }

    fn settings_state(&self) -> Value {
        let st = self.status();
        let agents = self.agents_json();
        let errors = self.errors_json(&agents);
        let mut settings = self.settings.to_json();
        if let Some(o) = settings.as_object_mut() {
            o.remove("watchers"); // internal
        }
        json!({
            "settings": settings,
            "runtime": {
                "enabled": self.rt.enabled,
                "status": st.as_str(),
                "bridge": { "connected": self.rt.bridge_ok },
                "sessions": self.session_rows_json(),
                "entertainment": { "open": self.rt.entertainment.is_some() },
                "customSound": self.settings.sound_custom_file,
                "errors": errors,
                "agents": agents,
                "permissions": permissions::list(self.settings.launch_at_login),
                "shortcutErrors": { "toggleAutomation": self.shortcut_errors[0], "toggleWidget": self.shortcut_errors[1] },
                "version": VERSION,
                "build": BUILD,
                "logPath": crate::logging::log_path().display().to_string(),
                "website": WEBSITE,
                "supportUrl": SUPPORT,
                "update": { "state": "unknown", "detail": self.update_detail },
            }
        })
    }

    fn push_settings(&mut self) {
        if self.settings_win.is_none() {
            return;
        }
        let text = self.settings_state().to_string();
        if self.last_settings_state.as_deref() == Some(text.as_str()) {
            return;
        }
        if let Some(w) = self.settings_win.as_ref() {
            let _ = w.webview.evaluate_script(&format!("window.wpSettings&&wpSettings.update({})", text));
        }
        self.last_settings_state = Some(text);
    }

    // ── widget window ───────────────────────────────────────────────────────────────────────────────────────────────────

    /// Creates, replaces or removes the widget so it matches the settings and the current displays.
    fn refresh_widget_window(&mut self, target: &EventLoopWindowTarget<Ev>) {
        if !self.settings.show_widget {
            if self.widget.take().is_some() {
                wp_info!("widget hidden");
            }
            return;
        }
        if self.widget.is_none() {
            match self.create_widget(target) {
                Ok(w) => self.widget = Some(w),
                Err(e) => {
                    wp_error!("widget: {}", e);
                    self.problem("widget", "error", "The floating widget could not be created.", &e);
                    return;
                }
            }
            self.clear_problem("widget");
        }
        self.pick_widget_screen();
        self.place_widget();
        self.last_widget_state = None; // the page may be new
    }

    fn create_widget(&mut self, target: &EventLoopWindowTarget<Ev>) -> Result<WidgetWin, String> {
        let window = WindowBuilder::new()
            .with_title("Vibe Play Widget")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(true)
            .with_resizable(false)
            .with_focusable(false)
            .with_visible(false)
            .with_visible_on_all_workspaces(true)
            .with_has_shadow(false)
            .with_theme(theme_for(self.settings.appearance))
            .with_inner_size(LogicalSize::new(300.0, 68.0))
            .build(target)
            .map_err(|e| format!("window: {}", e))?;
        let proxy = self.proxy.clone();
        let webview = WebViewBuilder::new()
            .with_transparent(true)
            .with_html(WIDGET_HTML.replace("/*@MASCOT@*/", MASCOT_JS))
            .with_accept_first_mouse(true)
            .with_ipc_handler(move |req| {
                let _ = proxy.send_event(Ev::Ipc(Target::Widget, req.into_body()));
            })
            .build(&window)
            .map_err(|e| format!("webview: {}", e))?;
        platform::set_window_level(window.ns_window(), 8);
        let glass = platform::Glass::attach(window.ns_view());
        if glass.is_none() {
            wp_warn!("native glass could not be attached; the widget falls back to its flat tint");
        }
        Ok(WidgetWin { webview, glass, window, size: None, shown: false, screen_origin: None })
    }

    /// The widget stays on the display it was on; a first placement (or a vanished display) uses the display under the pointer.
    fn pick_widget_screen(&mut self) {
        let screens = self.screens.clone();
        let Some(w) = self.widget.as_mut() else { return };
        let known = w.screen_origin.and_then(|o| screens.iter().position(|s| (s.frame.x, s.frame.y) == o));
        if known.is_none() {
            let idx = platform::cursor_logical().and_then(|(x, y)| placement::screen_at(&screens, x, y)).unwrap_or(0);
            w.screen_origin = screens.get(idx).map(|s| (s.frame.x, s.frame.y));
        }
    }

    fn widget_screen(&self) -> Option<Screen> {
        let w = self.widget.as_ref()?;
        let o = w.screen_origin?;
        self.screens.iter().find(|s| (s.frame.x, s.frame.y) == o).copied().or_else(|| self.screens.first().copied())
    }

    fn place_widget(&mut self) {
        let Some(screen) = self.widget_screen() else { return };
        let pos = self.settings.widget_position;
        let Some(w) = self.widget.as_mut() else { return };
        let Some((cw, ch)) = w.size else { return }; // wait for the page's first size report
        w.window.set_inner_size(LogicalSize::new(cw, ch));
        // Top centre is the notch: flush with the very top of a display that has one (the window sits over the menu bar, above it),
        // a floating island just under the menu bar on a display without. Every other position is a corner as before.
        let notch = platform::notch_size(screen.frame.x, screen.frame.y);
        let (x, y) = if pos == WidgetPosition::TopCenter {
            (screen.frame.x + (screen.frame.w - cw) / 2.0, if notch.is_some() { screen.frame.y } else { screen.work.y })
        } else {
            placement::corner_origin(screen.work, (cw, ch), pos, 0.0)
        };
        platform::set_window_level(w.window.ns_window(), if pos == WidgetPosition::TopCenter { 25 } else { 8 }); // 25 = status window level, above the menu bar
        w.window.set_outer_position(LogicalPosition::new(x, y));
        if !w.shown {
            w.window.set_visible(true);
            w.shown = true;
            wp_info!("widget shown {} at ({:.0}, {:.0}), {:.0}x{:.0}", pos.as_str(), x, y, cw, ch);
        }
    }

    // ── settings window ─────────────────────────────────────────────────────────────────────────────────────────────────

    fn open_settings(&mut self, target: &EventLoopWindowTarget<Ev>) {
        if let Some(w) = self.settings_win.as_ref() {
            w.window.set_visible(true);
            w.window.set_minimized(false);
            w.window.set_focus();
            return;
        }
        match self.create_settings(target) {
            Ok(w) => {
                w.window.set_focus();
                wp_info!("settings window opened");
                self.settings_win = Some(w);
                self.last_settings_state = None;
                self.clear_problem("settings-window");
            }
            Err(e) => {
                wp_error!("settings window: {}", e);
                self.problem("settings-window", "error", "The settings window could not be opened.", &e);
            }
        }
    }


    fn account_notify(&self) -> Notify {
        let proxy = Mutex::new(self.proxy.clone());
        Arc::new(move |body: String| {
            if let Ok(p) = proxy.lock() {
                let _ = p.send_event(Ev::Ipc(Target::Account, body));
            }
        })
    }

    fn on_account_ipc(&mut self, body: &str) {
        let notify = self.account_notify();
        self.account.on_message(body, &notify);
    }

    fn open_account(&mut self, target: &EventLoopWindowTarget<Ev>) {
        if !self.account.available() {
            wp_warn!("account window requested but this build has no account service configured");
            return;
        }
        let notify = self.account_notify();
        let theme = theme_for(self.settings.appearance);
        let was_open = self.account.is_open();
        match self.account.open(target, None, notify, theme, |b| b.with_accept_first_mouse(true)) {
            Ok(()) => {
                if !was_open {
                    wp_info!("account window opened");
                }
            }
            Err(e) => wp_error!("account window: {}", e),
        }
    }

    fn create_settings(&mut self, target: &EventLoopWindowTarget<Ev>) -> Result<SettingsWin, String> {
        let window = WindowBuilder::new()
            .with_title("Vibe Play")
            .with_inner_size(LogicalSize::new(760.0, 540.0))
            .with_min_inner_size(LogicalSize::new(680.0, 460.0))
            .with_resizable(true)
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true)
            .with_theme(theme_for(self.settings.appearance))
            .build(target)
            .map_err(|e| format!("window: {}", e))?;
        let proxy = self.proxy.clone();
        let webview = WebViewBuilder::new()
            .with_html(SETTINGS_HTML)
            .with_accept_first_mouse(true)
            .with_ipc_handler(move |req| {
                let _ = proxy.send_event(Ev::Ipc(Target::Settings, req.into_body()));
            })
            .build(&window)
            .map_err(|e| format!("webview: {}", e))?;
        Ok(SettingsWin { webview, window })
    }

    // ── actions ─────────────────────────────────────────────────────────────────────────────────────────────────────────

    fn toggle_automation(&mut self) {
        let on = state::toggle();
        self.rt.enabled = on;
        wp_info!("automation {}", if on { "enabled" } else { "disabled" });
        let _ = std::thread::Builder::new().name("wp-post-state".into()).spawn(move || BridgeClient::new().post_state(on));
        if !on {
            signals::write_quit_signal(); // companions close for real, which flushes their cookies
        }
        self.render();
    }

    fn open_entertainment(&mut self) {
        let Ok(exe) = std::env::current_exe() else { return };
        if let Err(e) = process::spawn_detached(&exe, &["--open".to_string()]) {
            wp_error!("could not open the entertainment window: {}", e);
            self.problem("entertainment", "error", "The entertainment window could not be started.", &e.to_string());
        } else {
            self.clear_problem("entertainment");
        }
    }

    /// "Hide entertainment" from the island. With agents running, the window steps aside for these prompts only (the same path as
    /// closing it with its own button: monitoring and the sessions are untouched, and it does not pop straight back); with none
    /// running, it closes.
    fn hide_entertainment(&mut self) {
        let ids: Vec<String> = self.rt.view.sessions.iter().filter(|r| r.act.active()).map(|r| r.id.clone()).collect();
        if ids.is_empty() {
            signals::write_quit_signal();
        } else {
            for id in &ids {
                signals::record_dismissal(id);
                signals::write_close_signal(id);
            }
        }
    }

    fn choose_sound(&self) {
        let proxy = self.proxy.clone();
        let _ = std::thread::Builder::new().name("wp-choose-sound".into()).spawn(move || {
            let script = "POSIX path of (choose file with prompt \"Choose a completion sound\" of type {\"public.audio\"})";
            let Ok(out) = std::process::Command::new("/usr/bin/osascript").args(["-e", script]).output() else { return };
            if !out.status.success() {
                return; // cancelled
            }
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            match sound::import_custom(std::path::Path::new(&path), &sound::custom_dir()) {
                Ok(name) => {
                    let _ = proxy.send_event(Ev::SoundChosen(Some(name)));
                }
                Err(e) => {
                    wp_warn!("custom sound: {}", e);
                    let _ = proxy.send_event(Ev::SoundChosen(None));
                }
            }
        });
    }

    fn on_sound_chosen(&mut self, file: Option<String>, target: &EventLoopWindowTarget<Ev>) {
        let Some(name) = file else {
            self.problem("custom-sound", "error", "That sound could not be used.", "Choose a WAV, AIFF, CAF, MP3 or M4A file smaller than 5 MB.");
            return;
        };
        self.clear_problem("custom-sound");
        self.apply_settings_patch(&json!({ "soundCustomFile": name, "soundName": "custom" }), target);
        sound::preview(&self.settings);
    }

    fn send_insights(&self, range: &str, tz_min: i64) {
        let Some(range) = insights::Range::parse(range) else { return };
        let summary = insights::summarize(&insights::load(), now_ms(), range, tz_min.clamp(-840, 840) as i32);
        if let Some(w) = self.settings_win.as_ref() {
            let _ = w.webview.evaluate_script(&format!("window.wpInsights&&wpInsights({})", summary));
        }
    }

    fn sync_login_item(&mut self) {
        let Ok(exe) = std::env::current_exe() else { return };
        match login_item::sync(&home(), self.settings.launch_at_login, &exe) {
            Ok(true) => wp_info!("login item {}", if self.settings.launch_at_login { "installed" } else { "removed" }),
            Ok(false) => {}
            Err(e) => {
                wp_error!("login item: {}", e);
                self.problem("login-item", "error", "Launch at login could not be changed.", &e);
                return;
            }
        }
        self.clear_problem("login-item");
    }

    fn apply_settings_patch(&mut self, patch: &Value, target: &EventLoopWindowTarget<Ev>) {
        let old = self.settings.clone();
        let rejected = self.settings.apply_patch(patch);
        if !rejected.is_empty() {
            wp_warn!("settings: rejected {:?}", rejected);
        }
        if self.settings == old {
            self.last_settings_state = None;
            self.render();
            return;
        }
        match self.settings.save() {
            Ok(()) => self.clear_problem("settings-save"),
            Err(e) => {
                wp_error!("settings could not be saved: {}", e);
                self.problem("settings-save", "error", "Settings could not be saved, so they will be lost on quit.", &e.to_string());
            }
        }
        if self.settings.launch_at_login != old.launch_at_login {
            self.sync_login_item();
        }
        if self.settings.appearance != old.appearance {
            let theme = theme_for(self.settings.appearance);
            if let Some(w) = self.widget.as_ref() {
                w.window.set_theme(theme);
            }
            if let Some(w) = self.settings_win.as_ref() {
                w.window.set_theme(theme);
            }
        }
        if self.settings.show_dock_icon != old.show_dock_icon {
            target.set_activation_policy_at_runtime(if self.settings.show_dock_icon { ActivationPolicy::Regular } else { ActivationPolicy::Accessory });
        }
        if self.settings.shortcuts != old.shortcuts {
            self.register_hotkeys();
        }
        if self.settings.show_widget != old.show_widget || self.settings.widget_position != old.widget_position {
            self.refresh_widget_window(target);
        }
        if self.settings.watchers != old.watchers {
            self.supervise();
        }
        self.last_widget_state = None;
        self.last_menu.clear();
        self.render();
    }

    fn install_integration(&mut self, id: &str, install: bool) {
        let Some(spec) = integrations::spec(id) else { return };
        let result = match spec.kind {
            integrations::Kind::ClaudeHooks => {
                let path = integrations::claude_settings_path(&home());
                if install {
                    let exe = paths::sibling_exe("waitplay-hook");
                    if exe.is_file() {
                        integrations::claude_install(&path, &exe)
                    } else {
                        Err(format!("waitplay-hook was not found at {}", exe.display()))
                    }
                } else {
                    integrations::claude_remove(&path)
                }
            }
            integrations::Kind::Watcher(_) => {
                let mut list = self.settings.watchers.clone();
                list.retain(|w| w != id);
                if install {
                    list.push(id.to_string());
                }
                self.settings.watchers = list;
                self.settings.save().map_err(|e| e.to_string())
            }
            integrations::Kind::Manual(_) => Err("This agent is connected by hand.".into()),
        };
        let pid = format!("integration-action-{}", id);
        match result {
            Ok(()) => {
                wp_info!("{} {}", id, if install { "connected" } else { "disconnected" });
                self.clear_problem(&pid);
                self.supervise();
            }
            Err(e) => {
                wp_error!("{} {}: {}", if install { "installing" } else { "removing" }, id, e);
                self.problem(&pid, "error", &format!("Could not {} {}.", if install { "connect" } else { "disconnect" }, spec.name), &e);
            }
        }
        self.last_settings_state = None;
        self.render();
    }

    fn on_settings_ipc(&mut self, body: &str, target: &EventLoopWindowTarget<Ev>) {
        let Ok(v) = serde_json::from_str::<Value>(body) else { return };
        let str_of = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        match str_of("t").as_str() {
            "ready" => {
                self.last_settings_state = None;
                self.push_settings();
            }
            "set" => {
                if let Some(patch) = v.get("patch") {
                    self.apply_settings_patch(patch, target);
                }
            }
            "toggleAutomation" => self.toggle_automation(),
            "openUrl" => {
                let url = str_of("url");
                if url == WEBSITE || url == SUPPORT {
                    open_with_system(&url);
                } else {
                    wp_warn!("refused to open {:?}", url);
                }
            }
            "openSystemSettings" => match permissions::pane_url(&str_of("pane")) {
                Some(url) => open_with_system(url),
                None => wp_warn!("unknown settings pane {:?}", str_of("pane")),
            },
            "installIntegration" => self.install_integration(&str_of("agent"), true),
            "removeIntegration" => self.install_integration(&str_of("agent"), false),
            "revealLogs" => {
                let path = crate::logging::log_path();
                if !path.exists() {
                    wp_info!("log file requested");
                }
                let _ = std::process::Command::new("/usr/bin/open").arg("-R").arg(&path).spawn().map(|mut c| std::thread::spawn(move || c.wait()));
            }
            "checkUpdates" => {
                self.last_settings_state = None;
                self.push_settings();
            }
            "openEntertainment" => self.open_entertainment(),
            "openAccount" => self.open_account(target),
            "drag" => {
                if let Some(w) = self.settings_win.as_ref() {
                    let _ = w.window.drag_window();
                }
            }
            "zoom" => {
                if let Some(w) = self.settings_win.as_ref() {
                    w.window.set_maximized(!w.window.is_maximized());
                }
            }
            "insights" => self.send_insights(&str_of("range"), v.get("tz").and_then(Value::as_i64).unwrap_or(0)),
            "recent" => {
                let summary = insights::recent(&insights::load(), 10);
                if let Some(w) = self.settings_win.as_ref() {
                    let _ = w.webview.evaluate_script(&format!("window.wpRecent&&wpRecent({})", summary));
                }
            }
            "hideEntertainment" => self.hide_entertainment(),
            "previewSound" => sound::preview(&self.settings),
            "chooseSound" => self.choose_sound(),
            "removeSound" => {
                sound::remove_custom();
                self.clear_problem("custom-sound");
                let patch = if self.settings.sound_name == "custom" { json!({ "soundCustomFile": "", "soundName": "gentle" }) } else { json!({ "soundCustomFile": "" }) };
                self.apply_settings_patch(&patch, target);
            }
            "quit" => self.quitting = true,
            other => wp_warn!("settings page sent an unknown message {:?}", other),
        }
    }

    fn on_widget_ipc(&mut self, body: &str, target: &EventLoopWindowTarget<Ev>) {
        let Ok(v) = serde_json::from_str::<Value>(body) else { return };
        match v.get("t").and_then(Value::as_str).unwrap_or("") {
            "ready" => {
                self.last_widget_state = None;
                // test-only, and only in the separate `--features test-hooks` build the CI smoke test uses for its screenshots (it
                // cannot click): the page is asked to show its expanded panel. The shipped app does not contain this code.
                #[cfg(feature = "test-hooks")]
                if std::env::var_os("WAITPLAY_TEST_EXPAND_WIDGET").is_some() {
                    if let Some(w) = self.widget.as_ref() {
                        let _ = w.webview.evaluate_script("setTimeout(function(){window.wpWidget&&wpWidget.expand(true)},3500)");
                    }
                }
            }
            "size" => {
                let (w, h) = (v.get("w").and_then(Value::as_f64).unwrap_or(0.0), v.get("h").and_then(Value::as_f64).unwrap_or(0.0));
                if (40.0..=1000.0).contains(&w) && (20.0..=400.0).contains(&h) {
                    if let Some(win) = self.widget.as_mut() {
                        win.size = Some((w.ceil(), h.ceil()));
                    }
                    self.place_widget();
                }
            }
            "shape" => {
                // the page's live pill rectangle: the native glass follows it frame by frame during expand/collapse
                let g = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                if let Some(win) = self.widget.as_ref() {
                    if let Some(glass) = win.glass.as_ref() {
                        glass.set_shape(g("x"), g("y"), g("w"), g("h"), g("wh"), g("r"));
                    }
                }
            }
            "platform" => {
                if let Some(key) = v.get("key").and_then(Value::as_str) {
                    if self.settings.platforms.get(key) == Some(true) {
                        self.apply_settings_patch(&json!({ "preferredPlatform": key }), target);
                    }
                }
            }
            "toggle" => self.toggle_automation(),
            "openSettings" => self.open_settings(target),
            "menu" => {
                if let (Some(menu), Some(w)) = (self.menu.as_ref(), self.widget.as_ref()) {
                    // SAFETY: the pointer is the widget window's live NSView
                    unsafe {
                        menu.show_context_menu_for_nsview(w.window.ns_view() as *const c_void, None);
                    }
                }
            }
            "openEntertainment" => self.open_entertainment(),
            "hideEntertainment" => self.hide_entertainment(),
            "toggleMute" => {
                let muted = !self.settings.sound_muted;
                self.apply_settings_patch(&json!({ "soundMuted": muted }), target);
            }
            "dismissToast" => self.tracker.dismiss_toast(),
            other => wp_warn!("widget page sent an unknown message {:?}", other),
        }
        self.render();
    }

    fn on_menu(&mut self, id: &str, target: &EventLoopWindowTarget<Ev>) {
        match id {
            "toggle" => self.toggle_automation(),
            "settings" => self.open_settings(target),
            "account" => self.open_account(target),
            "quit" => self.quitting = true,
            "widget" => {
                let show = !self.settings.show_widget;
                self.apply_settings_patch(&json!({ "showWidget": show }), target);
            }
            "open-entertainment" => self.open_entertainment(),
            other => {
                if let Some(key) = other.strip_prefix("platform:") {
                    self.apply_settings_patch(&json!({ "preferredPlatform": key }), target);
                }
            }
        }
    }

    fn on_hotkey(&mut self, id: u32, target: &EventLoopWindowTarget<Ev>) {
        let action = self.registered.iter().find(|(hk, _)| hk.id() == id).map(|(_, a)| *a);
        match action {
            Some(HotkeyAction::ToggleAutomation) => self.toggle_automation(),
            Some(HotkeyAction::ToggleWidget) => {
                let show = !self.settings.show_widget;
                self.apply_settings_patch(&json!({ "showWidget": show }), target);
            }
            None => {}
        }
    }

    fn cleanup(&mut self) {
        wp_info!("quitting");
        let records = self.recorder.flush(now_ms());
        insights::append(&records);
        signals::write_quit_signal();
        self.supervisor.shutdown();
        self.account.close();
    }

    fn handle_window_event(&mut self, wid: WindowId, event: &WindowEvent) {
        if self.account.on_window_event(wid, event) {
            return;
        }
        if let WindowEvent::CloseRequested = event {
            if self.settings_win.as_ref().map_or(false, |w| w.window.id() == wid) {
                // closing frees the page; the next "Settings…" builds a fresh one
                self.settings_win = None;
                self.last_settings_state = None;
            }
        }
    }
}

fn build_menu(entries: &[Entry]) -> Result<Menu, tray_icon::menu::Error> {
    let menu = Menu::new();
    let mut counter = 0u32;
    for e in entries {
        match e {
            Entry::Separator => menu.append(&PredefinedMenuItem::separator())?,
            Entry::Item { id, text, enabled, checked } => {
                let id = if id.is_empty() {
                    counter += 1;
                    format!("label-{}", counter)
                } else {
                    id.clone()
                };
                match checked {
                    Some(on) => menu.append(&CheckMenuItem::with_id(id, text, *enabled, *on, None))?,
                    None => menu.append(&MenuItem::with_id(id, text, *enabled, None))?,
                }
            }
            Entry::Submenu { text, items } => {
                let sub = Submenu::new(text, true);
                for item in items {
                    match item {
                        Entry::Separator => sub.append(&PredefinedMenuItem::separator())?,
                        Entry::Item { id, text, enabled, checked } => match checked {
                            Some(on) => sub.append(&CheckMenuItem::with_id(id.clone(), text, *enabled, *on, None))?,
                            None => sub.append(&MenuItem::with_id(id.clone(), text, *enabled, None))?,
                        },
                        Entry::Submenu { .. } => {} // one level only
                    }
                }
                menu.append(&sub)?;
            }
        }
    }
    Ok(menu)
}

// ── "open settings" from a second launch ─────────────────────────────────────────────────────────────────────────────────

fn open_request_path() -> PathBuf {
    paths::signal_dir().join(".waitplay-open-settings")
}

fn open_request_pending() -> bool {
    let p = open_request_path();
    if p.exists() {
        let _ = std::fs::remove_file(&p);
        true
    } else {
        false
    }
}

fn request_open_settings() {
    let _ = paths::write_atomic(&open_request_path(), now_ms().to_string().as_bytes());
}

// ── entry point ──────────────────────────────────────────────────────────────────────────────────────────────────────────

pub fn run() -> Result<(), String> {
    crate::logging::init("app");
    let Some(_single) = crate::single_instance::acquire("app") else {
        // already running: ask it to show its settings, then leave
        request_open_settings();
        return Ok(());
    };

    let mut settings = Settings::load();
    // "Start automation automatically": Vibe Play opens switched on (or off) as the user chose
    state::set_enabled(settings.auto_start_automation);
    if settings.watchers.is_empty() && !Settings::path().exists() {
        settings.watchers = integrations::default_watchers();
        let _ = settings.save();
    }

    let mut event_loop = EventLoopBuilder::<Ev>::with_user_event().build();
    event_loop.set_activation_policy(if settings.show_dock_icon { ActivationPolicy::Regular } else { ActivationPolicy::Accessory });
    let proxy = event_loop.create_proxy();

    platform::install_main_menu("Vibe Play");

    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let _ = menu_proxy.send_event(Ev::Menu(e.id().0.clone()));
    }));

    let ticker = proxy.clone();
    std::thread::Builder::new()
        .name("wp-tick".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            if ticker.send_event(Ev::Tick).is_err() {
                break;
            }
        })
        .map_err(|e| e.to_string())?;

    let mut app = App::new(proxy, settings);
    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => app.init(target),
            Event::UserEvent(Ev::Tick) => app.on_tick(target),
            Event::UserEvent(Ev::Sessions(body)) => app.on_sessions(body),
            Event::UserEvent(Ev::SoundChosen(file)) => app.on_sound_chosen(file, target),
            Event::UserEvent(Ev::Ipc(Target::Settings, body)) => app.on_settings_ipc(&body, target),
            Event::UserEvent(Ev::Ipc(Target::Widget, body)) => app.on_widget_ipc(&body, target),
            Event::UserEvent(Ev::Ipc(Target::Account, body)) => app.on_account_ipc(&body),
            Event::UserEvent(Ev::Menu(id)) => app.on_menu(&id, target),
            Event::UserEvent(Ev::Hotkey(id)) => app.on_hotkey(id, target),
            Event::Reopen { .. } => app.open_settings(target),
            Event::WindowEvent { window_id, event, .. } => app.handle_window_event(window_id, &event),
            Event::LoopDestroyed => app.cleanup(),
            _ => {}
        }
        if app.quitting {
            app.quitting = false;
            *flow = ControlFlow::Exit;
        }
    });
}
