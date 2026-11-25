//! The companion window: a header (site switcher, agent status, window buttons) above the live site, shown while a coding agent
//! works and parked (hidden, frozen, RAM handed back) while it does not. One process per site; every terminal's sessions are
//! served by it. The rules for when to show or hide live in `logic`; background file/HTTP polling in `poller`.

mod logic;
mod navigation;
mod poller;
mod webview2;

use crate::platform::{self, InputWatcher, WindowId, NO_WINDOW};
use crate::single_instance::Guard;
use crate::sites::{self, Site};
use crate::profile;
use logic::{Action, Core, Env, LaunchRequest, Status};
use poller::{PollResult, Poller};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::{Window, WindowBuilder};
use waitplay_core::signals;
use waitplay_core::now_ms;
use wry::{NewWindowResponse, WebContext, WebView, WebViewBuilder};

pub struct Options {
    /// Identity of this window: its single-instance lock and the signal files it owns.
    pub site: &'static Site,
    /// The page it opens on (the user's preferred platform for agent launches).
    pub start: &'static Site,
    pub session: String,
    pub terminal: String,
    pub parent_hwnd: u64,
}

const HEADER_HEIGHT_DIP: f64 = 44.0;
/// Windows draws no frame, so the site sits inside a thin border that keeps the resize edges (and shows the glass around the
/// site); the macOS title bar needs none.
const BORDER_DIP: f64 = if cfg!(target_os = "macos") { 0.0 } else { 6.0 };
/// The header page paints on glass (acrylic on Windows, the window's visual-effect view on macOS), so its own background is clear.
const HEADER_BG: (u8, u8, u8, u8) = (0, 0, 0, 0);
const MIN_WIDTH_DIP: f64 = 360.0;
const MIN_HEIGHT_DIP: f64 = 320.0;
const ACTIVE_TICK: Duration = Duration::from_millis(100);
const PARKED_TICK: Duration = Duration::from_millis(500);
/// A companion parked this long with no prompt running quits for real, handing back all its RAM (the next prompt reopens it,
/// logins are kept on disk). Parking alone still leaves a few hundred MB committed.
const IDLE_QUIT: Duration = Duration::from_secs(600);

/// Pause what is playing and remember it (window.__wpResume) so showing the window continues from the same frame.
const PAUSE_MEDIA_JS: &str = "(function(){try{var a=window.__wpResume||[];document.querySelectorAll('video,audio').forEach(function(m){if(!m.paused&&!m.ended){a.push(m);m.pause();}});window.__wpResume=a;}catch(e){}})();";
const RESUME_MEDIA_JS: &str = "(function(){try{var a=window.__wpResume||[];window.__wpResume=null;a.forEach(function(m){if(m.isConnected){var p=m.play();if(p&&p.catch)p.catch(function(){});}});}catch(e){}})();";

enum UserEvent {
    Tick,
    Poll(PollResult),
    Interaction,
    Ipc(String),
    Navigate(String),
    Suspended(bool),
}

// ── pure helpers (unit tested) ───────────────────────────────────────────────────────────────────────────────────────────

/// Window content layout in physical pixels: (x, y, w, h) of the header and of the site.
#[derive(Debug, PartialEq, Eq)]
struct Layout {
    header: (i32, i32, u32, u32),
    content: (i32, i32, u32, u32),
}

/// The site sits inside a thin border so the undecorated window keeps its resize edges (child webviews swallow the mouse).
fn layout(width: u32, height: u32, scale: f64) -> Layout {
    let b = (BORDER_DIP * scale).round() as i32;
    let hh = (HEADER_HEIGHT_DIP * scale).round() as i32;
    let inner_w = (width as i32 - 2 * b).max(1);
    let content_h = (height as i32 - 2 * b - hh).max(1);
    Layout { header: (b, b, inner_w as u32, hh as u32), content: (b, b + hh, inner_w as u32, content_h as u32) }
}

/// "Term - 3" for ids like `term3-session` when the hook gave no label.
fn terminal_label(session_id: &str, terminal: &str) -> String {
    if !terminal.is_empty() && terminal != "Desktop" {
        return terminal.to_string();
    }
    if session_id.contains("term") {
        let digits: String = session_id.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return format!("Term - {}", digits);
        }
    }
    String::new()
}

#[derive(Default, Clone, PartialEq, Debug)]
struct HeaderState {
    agent: String,
    terminal: String,
    model: String,
    tool: String,
    waiting_prompt: Option<String>,
    shown: bool,
}

impl HeaderState {
    /// The `wp.setStatus(...)` argument.
    fn to_json(&self) -> serde_json::Value {
        use serde_json::json;
        if !self.shown {
            return json!({ "kind": "none" });
        }
        let join = |parts: &[&str]| parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(" \u{b7} ");
        match &self.waiting_prompt {
            Some(prompt) => json!({
                "kind": "waiting",
                "text": format!("{} needs your input", self.agent),
                "where": join(&[&self.terminal]),
                "title": prompt,
            }),
            None => json!({
                "kind": "working",
                "text": format!("{} is working", self.agent),
                "where": join(&[&self.terminal, &self.model, &self.tool]),
                "title": "",
            }),
        }
    }
}

// ── environment for the rules ────────────────────────────────────────────────────────────────────────────────────────────

struct UiEnv {
    cache: (WindowId, bool),
}

impl Env for UiEnv {
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn foreground(&self) -> WindowId {
        platform::foreground_window()
    }
    /// Cached per window: this runs every tick and process lookups aren't free.
    fn is_waitplay_window(&mut self, id: WindowId) -> bool {
        if id != self.cache.0 {
            self.cache = (id, platform::is_waitplay_window(id));
        }
        self.cache.1
    }
}

// ── the app ──────────────────────────────────────────────────────────────────────────────────────────────────────────────

struct App {
    // Webviews drop before the window that hosts them
    header: WebView,
    content: WebView,
    window: Window,
    _context: WebContext,

    core: Core,
    env: UiEnv,
    poller: Poller,
    input: InputWatcher,
    proxy: EventLoopProxy<UserEvent>,
    parked_flag: Arc<AtomicBool>,

    header_state: HeaderState,
    last_site: Option<&'static str>,
    last_can_go_back: bool,
    url_check_ticks: u32,

    suspended: bool,
    visible: bool,
    quit_requested: bool,
    exit: bool,

    arm_at: Option<Instant>,
    site_key: &'static str,
    close_on_finish: bool,
    hide_at: Option<Instant>,
    deep_park_at: Option<Instant>,
    retrim_at: Option<Instant>,
    retrim_fast: u32,
    idle_quit_at: Option<Instant>,
}

impl App {
    fn hwnd(&self) -> WindowId {
        #[cfg(windows)]
        {
            use tao::platform::windows::WindowExtWindows;
            self.window.hwnd() as WindowId
        }
        #[cfg(not(windows))]
        {
            NO_WINDOW
        }
    }

    fn do_layout(&self) {
        let size = self.window.inner_size();
        let l = layout(size.width, size.height, self.window.scale_factor());
        let rect = |(x, y, w, h): (i32, i32, u32, u32)| wry::Rect { position: wry::dpi::PhysicalPosition::new(x, y).into(), size: wry::dpi::PhysicalSize::new(w, h).into() };
        let _ = self.header.set_bounds(rect(l.header));
        let _ = self.content.set_bounds(rect(l.content));
    }

    fn set_header_status(&self) {
        let js = format!("wp.setStatus({});", self.header_state.to_json());
        let _ = self.header.evaluate_script(&js);
    }

    fn run_actions(&mut self, actions: Vec<Action>) {
        for a in actions {
            match a {
                Action::HideWhileAway => {
                    self.input.set_target(NO_WINDOW);
                    webview2::set_muted(&self.content, true); // a playing reel shouldn't keep talking from an invisible window
                    let _ = self.content.evaluate_script(PAUSE_MEDIA_JS); // queued before the hide, so the page can't pause itself first
                    self.hide_at = Some(Instant::now() + Duration::from_millis(80));
                    self.begin_deep_park();
                    self.set_poll_rate();
                }
                Action::DeepenPark => self.begin_deep_park(),
                Action::Show { activate } => self.show(activate),
                Action::Raise => {
                    self.window.set_always_on_top(true);
                    self.window.set_focus();
                }
                Action::Restore => self.window.set_minimized(false),
                Action::FocusTerminal(h) => platform::focus_window(h),
                Action::SetInputTarget(h) => self.input.set_target(h),
                Action::ShowSession { id, terminal, agent, hwnd } => {
                    self.header_state = HeaderState { agent, terminal: terminal_label(&id, &terminal), shown: true, ..Default::default() };
                    self.set_header_status();
                    self.keep_clear_of_tab_row(hwnd);
                }
                Action::SetPollRate => self.set_poll_rate(),
            }
        }
    }

    fn set_poll_rate(&self) {
        let parked = self.core.is_parked();
        self.parked_flag.store(parked, Ordering::SeqCst);
        self.poller.set_parked(parked);
    }

    /// Every route that shows the window ends here.
    fn show(&mut self, activate: bool) {
        self.hide_at = None;
        if activate {
            self.window.set_visible(true);
            platform::show_window(self.hwnd(), true); // tao's set_visible is a no-op once its flag says visible
            self.window.set_always_on_top(true);
            self.window.set_focus();
        } else {
            // reappear beside the terminal without stealing its keyboard focus
            #[cfg(not(windows))]
            self.window.set_visible(true);
            platform::show_window(self.hwnd(), false);
            self.window.set_always_on_top(true);
        }
        self.on_became_visible();
    }

    fn on_became_visible(&mut self) {
        self.visible = true;
        waitplay_core::companion_state::write(self.site_key, true);
        crate::wp_info!("companion visible");
        self.core.became_visible();
        let _ = self.content.set_visible(true);
        let _ = self.header.set_visible(true);
        webview2::set_muted(&self.content, false);
        self.wake_page();
        let _ = self.content.evaluate_script(RESUME_MEDIA_JS);
        self.set_poll_rate();
    }

    // ── deep park: a hidden page is frozen and its RAM handed back ─────────────────────────────────────────────────────
    //
    // After the page has settled (a quick alt-tab peek never gets here), freeze it like an Edge "sleeping tab", let it drop its
    // caches, then give the physical RAM back to Windows. Nothing is discarded: the pages sit in the standby list or the pagefile
    // and are read back on demand, so the reel position and scroll come back exactly as they were.
    fn begin_deep_park(&mut self) {
        if self.suspended {
            return;
        }
        // mid-turn absences are usually short; a parked window deepens sooner
        let delay = if self.core.is_parked() { Duration::from_millis(1200) } else { Duration::from_millis(6000) };
        self.deep_park_at = Some(Instant::now() + delay);
    }

    fn cancel_deep_park(&mut self) {
        self.deep_park_at = None;
        self.retrim_at = None;
        self.idle_quit_at = None;
    }

    fn start_suspend(&mut self) {
        if !self.core.hidden_away {
            return;
        }
        webview2::set_low_memory(&self.content, true);
        let _ = self.content.set_visible(false); // WebView2 refuses to suspend a visible page
        let _ = self.header.set_visible(false);
        let proxy = self.proxy.clone();
        webview2::try_suspend(&self.content, move |ok| {
            let _ = proxy.send_event(UserEvent::Suspended(ok));
        });
    }

    fn on_suspended(&mut self, ok: bool) {
        if !self.core.hidden_away {
            // shown while the freeze was in flight: thaw it again
            if ok {
                self.wake_page();
            }
            return;
        }
        // WKWebView cannot be frozen like WebView2, so on macOS an idle parked window simply quits after the same grace period
        if (ok || cfg!(target_os = "macos")) && self.core.is_parked() {
            self.idle_quit_at = Some(Instant::now() + IDLE_QUIT);
        }
        self.suspended = ok;
        self.trim_now();
        if self.retrim_at.is_none() && !cfg!(target_os = "macos") {
            // A frozen page keeps doing memory-reduction work for ~30s (each burst re-faults ~70MB), so trim again every 4s
            // through that, then only now and then.
            self.retrim_fast = 8;
            self.retrim_at = Some(Instant::now() + Duration::from_secs(4));
        }
    }

    fn trim_now(&self) {
        // browser 0 = only our own process: a visible sibling shares the browser and must not stutter
        let browser = if platform::other_companion_visible() { 0 } else { webview2::browser_pid(&self.content) };
        platform::memory::park(browser);
    }

    /// Shown again by any route: normal memory priority first (the page is about to ask for its pages back), then thaw.
    fn wake_page(&mut self) {
        self.cancel_deep_park();
        platform::memory::wake(webview2::browser_pid(&self.content));
        if self.suspended {
            webview2::resume(&self.content);
        }
        webview2::set_low_memory(&self.content, false);
        self.suspended = false;
    }

    // ── placement ───────────────────────────────────────────────────────────────────────────────────────────────────────

    /// Never cover the owning terminal's tab row, or the user can't click over to another tab. In Windows Terminal the tabs share
    /// the title row: the top ~48px of the window.
    fn keep_clear_of_tab_row(&self, terminal: WindowId) {
        let Some(r) = platform::window_rect(terminal) else { return };
        let scale = self.window.scale_factor();
        let Ok(pos) = self.window.outer_position() else { return };
        let size = self.window.outer_size();
        let row_bottom = r.top + (48.0 * scale).round() as i32;
        let overlaps_x = pos.x < r.right && pos.x + size.width as i32 > r.left;
        if !overlaps_x || pos.y >= row_bottom {
            return;
        }
        let gap = (8.0 * scale).round() as i32;
        let new_top = row_bottom + gap;
        let room = platform::work_area().bottom - new_top - gap;
        if (room as f64) < MIN_HEIGHT_DIP * scale {
            return; // terminal sits too low to clear it; leave the window as is
        }
        self.window.set_inner_size(PhysicalSize::new(size.width, (size.height as i32).min(room) as u32));
        self.window.set_outer_position(PhysicalPosition::new(pos.x, new_top));
    }

    // ── events ──────────────────────────────────────────────────────────────────────────────────────────────────────────

    /// X, Esc, Alt+F4, the task ending and leaving the window never destroy it: they hide it, so the page, the playback position and
    /// the login stay in RAM. Hiding pauses and mutes whatever was playing; showing by ANY route resumes it from the same spot.
    /// `quit` is the only real exit.
    fn close_requested(&mut self) {
        if self.quit_requested {
            self.exit = true;
            return;
        }
        self.dismiss_all();
        if !self.core.hidden_away {
            let mut out = Vec::new();
            self.core.park(&mut out);
            self.run_actions(out);
        }
    }

    fn dismiss_all(&mut self) {
        for id in self.core.dismiss_all() {
            self.poller.forget(&id);
            signals::record_dismissal(&id);
        }
    }

    fn quit(&mut self) {
        crate::wp_info!("companion quitting");
        self.quit_requested = true;
        self.exit = true;
    }

    fn on_tick(&mut self) {
        let acts = self.core.tick(&mut self.env, &logic::agent_name_of_session);
        self.run_actions(acts);
        // "Close when the agent finishes": nothing left to wait for after at least one session ran, so leave for real (cookies are
        // flushed by the clean exit, and the next prompt starts a fresh window)
        if self.close_on_finish && !self.core.sessions.is_empty() && self.core.is_parked() && !self.quit_requested {
            self.quit();
        }

        // the address changes without a page load on single-page sites: look at it now and then, only while someone can see it
        self.url_check_ticks += 1;
        if self.visible && self.url_check_ticks >= 5 {
            self.url_check_ticks = 0;
            self.refresh_site_state();
        }
    }

    fn refresh_site_state(&mut self) {
        let current = self.content.url().ok().and_then(|u| sites::for_url(&u)).map(|s| s.key);
        if current != self.last_site {
            self.last_site = current;
            let js = format!("wp.setActive({});", serde_json::to_string(current.unwrap_or("")).unwrap_or_else(|_| "\"\"".into()));
            let _ = self.header.evaluate_script(&js);
        }
        let back = self.content.can_go_back().unwrap_or(false);
        if back != self.last_can_go_back {
            self.last_can_go_back = back;
            let _ = self.header.evaluate_script(&format!("wp.setNav({});", back));
        }
    }

    fn on_poll(&mut self, r: PollResult) {
        self.core.enabled = r.enabled;
        if r.quit {
            self.quit();
            return;
        }
        for l in &r.launches {
            let minimized = self.window.is_minimized();
            let visible = self.visible;
            let acts = self.core.apply_launch(l, minimized, &mut |h| platform::is_waitplay_window(h), visible);
            self.run_actions(acts);
        }
        for (id, ts) in &r.closed {
            if self.core.apply_close(id, *ts) {
                self.poller.forget(id);
                signals::clear_dismissal(id);
            }
        }
        for (id, own) in &r.own_tab {
            self.core.apply_own_tab(id, *own);
        }
        if let Some(json) = &r.bridge_json {
            self.apply_bridge(json);
        }
        self.set_poll_rate();
    }

    /// The bridge (when running) adds model/tool/waiting detail and is a second completion source.
    fn apply_bridge(&mut self, json: &str) {
        let entries = logic::parse_bridge_sessions(json);
        let (finished, status) = self.core.apply_bridge(&entries, &logic::agent_name_of_session);
        for id in finished {
            self.poller.forget(&id);
            signals::clear_dismissal(&id);
        }
        let Some(status) = status else { return };
        let before = self.header_state.clone();
        match status {
            Status::Working { model, tool, .. } => {
                self.header_state.model = model;
                self.header_state.tool = tool;
                self.header_state.waiting_prompt = None;
            }
            Status::Waiting { prompt, .. } => self.header_state.waiting_prompt = Some(prompt),
        }
        if self.header_state.shown && self.header_state != before {
            self.set_header_status();
        }
    }

    fn on_ipc(&mut self, body: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else { return };
        match v.get("t").and_then(|t| t.as_str()).unwrap_or("") {
            "site" => {
                if let Some(site) = v.get("key").and_then(|k| k.as_str()).and_then(sites::by_name) {
                    // load_url (not a same-address no-op) so a click on the site you are already on goes back to its home page
                    let _ = self.content.load_url(site.url);
                }
            }
            "back" => {
                let _ = self.content.go_back();
            }
            "reload" => {
                let _ = self.content.reload();
            }
            "min" => self.window.set_minimized(true),
            "max" => {
                let on = !self.window.is_maximized();
                self.window.set_maximized(on);
                let _ = self.header.evaluate_script(&format!("wp.setMaximized({});", on));
            }
            "drag" => {
                let _ = self.window.drag_window();
            }
            "close" => self.close_requested(),
            _ => {}
        }
    }

    fn user_event(&mut self, e: UserEvent) {
        match e {
            UserEvent::Tick => self.on_tick(),
            UserEvent::Poll(r) => self.on_poll(r),
            UserEvent::Interaction => {
                let (acts, finished) = self.core.on_terminal_interaction();
                if let Some(id) = finished {
                    self.poller.forget(&id);
                    signals::clear_dismissal(&id);
                }
                self.run_actions(acts);
            }
            UserEvent::Ipc(body) => self.on_ipc(&body),
            UserEvent::Navigate(url) => {
                // belt and braces: the event is only ever sent for an approved pop-up, but the load is the sink that matters
                if navigation::popup_allowed(&url) {
                    let _ = self.content.load_url(&url);
                }
            }
            UserEvent::Suspended(ok) => self.on_suspended(ok),
        }
    }

    // ── deadlines ───────────────────────────────────────────────────────────────────────────────────────────────────────

    fn run_deadlines(&mut self) {
        let now = Instant::now();
        if self.arm_at.map_or(false, |t| now >= t) {
            self.arm_at = None;
            self.core.auto_close_armed = true;
        }
        if self.hide_at.map_or(false, |t| now >= t) {
            self.hide_at = None;
            if self.core.hidden_away {
                waitplay_core::companion_state::write(self.site_key, false);
                crate::wp_info!("companion hidden");
                self.window.set_visible(false);
                platform::hide_window(self.hwnd());
                self.visible = false;
            }
        }
        if self.deep_park_at.map_or(false, |t| now >= t) {
            self.deep_park_at = None;
            self.start_suspend();
        }
        if self.retrim_at.map_or(false, |t| now >= t) {
            if !self.core.hidden_away {
                self.retrim_at = None;
            } else {
                self.trim_now();
                if self.retrim_fast > 0 {
                    self.retrim_fast -= 1;
                }
                self.retrim_at = Some(now + if self.retrim_fast == 0 { Duration::from_secs(30) } else { Duration::from_secs(4) });
            }
        }
        if self.idle_quit_at.map_or(false, |t| now >= t) {
            self.idle_quit_at = None;
            if self.core.is_parked() {
                self.quit();
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        [self.arm_at, self.hide_at, self.deep_park_at, self.retrim_at, self.idle_quit_at].into_iter().flatten().min()
    }
}

// ── start-up ─────────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Compact portrait window (a phone-sized reel column) in the widget's corner of the display under the pointer.
#[cfg(target_os = "macos")]
fn place_on_macos(window: &Window, settings: &waitplay_core::settings::Settings) {
    use crate::placement;
    use tao::dpi::{LogicalPosition, LogicalSize};
    const TITLE_BAR: f64 = 28.0;
    const WIDGET_BLOCK: f64 = 68.0;
    let screens = platform::screens();
    let idx = platform::cursor_logical().and_then(|(x, y)| placement::screen_at(&screens, x, y)).unwrap_or(0);
    let Some(screen) = screens.get(idx) else { return };
    let widget = if settings.show_widget { WIDGET_BLOCK } else { 0.0 };
    let r = placement::companion_rect(screen.work, (440.0, 800.0), settings.widget_position, widget);
    window.set_inner_size(LogicalSize::new(r.w, (r.h - TITLE_BAR).max(MIN_HEIGHT_DIP)));
    window.set_outer_position(LogicalPosition::new(r.x, r.y));
}

fn header_html() -> String {
    // Only the platforms the user switched on (always the one this window opens on)
    let settings = waitplay_core::settings::Settings::load();
    let tabs: String = sites::SITES
        .iter()
        .filter(|s| settings.platforms.get(s.key) != Some(false) || s.key == sites::preferred_site().key)
        .map(|s| {
            format!(
                "<button class='tab' data-site='{key}' title='{label}' aria-label='{label}'>{logo}<span class='label'>{label}</span></button>",
                key = s.key,
                label = s.label,
                logo = if cfg!(target_os = "macos") { sites::logo_svg_adaptive(s.key, 18) } else { sites::logo_svg(s.key, 18, true) }
            )
        })
        .collect();
    let platform = if cfg!(target_os = "macos") { "mac" } else { "" };
    include_str!("../../assets/header.html").replace("{{SITES}}", &tabs).replace("{{PLATFORM}}", platform)
}

pub fn run(opts: Options, _guard: Guard) -> Result<(), String> {
    let site = opts.site;
    let mut desktop_parent = platform::foreground_window();
    if platform::is_waitplay_window(desktop_parent) {
        desktop_parent = NO_WINDOW;
    }

    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        // a utility without a Dock icon; the standard Edit menu makes paste work in the sites' sign-in forms
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
        platform::install_main_menu("WaitPlay");
    }
    let proxy = event_loop.create_proxy();
    let user_settings = waitplay_core::settings::Settings::load();

    #[cfg(target_os = "macos")]
    crate::logging::init("companion");
    let window = {
        let b = WindowBuilder::new().with_title("WaitPlay").with_resizable(true).with_always_on_top(true).with_visible(false);
        // Windows draws its own caption in the header page; macOS keeps the native title bar and traffic lights
        #[cfg(not(target_os = "macos"))]
        // black GDI pixels are what a transparent window shows through as glass, so the bare window (the frame around the site) is black
        let b = b.with_decorations(false).with_transparent(true).with_background_color((0, 0, 0, 255));
        #[cfg(windows)]
        let b = {
            use tao::platform::windows::WindowBuilderExtWindows;
            b.with_undecorated_shadow(true)
        };
        #[cfg(target_os = "macos")]
        let b = b.with_transparent(true).with_visible_on_all_workspaces(true).with_theme(match user_settings.appearance {
            waitplay_core::settings::Appearance::System => None,
            waitplay_core::settings::Appearance::Light => Some(tao::window::Theme::Light),
            waitplay_core::settings::Appearance::Dark => Some(tao::window::Theme::Dark),
        });
        b.build(&event_loop).map_err(|e| e.to_string())?
    };
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::WindowExtMacOS;
        platform::fill_with_glass(window.ns_view());
    }
    let scale = window.scale_factor();
    window.set_min_inner_size(Some(PhysicalSize::new((MIN_WIDTH_DIP * scale) as u32, (MIN_HEIGHT_DIP * scale) as u32)));
    #[cfg(target_os = "macos")]
    place_on_macos(&window, &user_settings);
    #[cfg(not(target_os = "macos"))]
    {
        // Centred, large enough that the site's desktop layout fits without scrolling sideways
        let wa = platform::work_area();
        let (wa_w, wa_h) = (wa.width() as f64, wa.height() as f64);
        let w = (wa_w * 0.70).clamp(480.0 * scale, 1400.0 * scale);
        let h = (wa_h * 0.82).clamp(420.0 * scale, 950.0 * scale);
        window.set_inner_size(PhysicalSize::new(w as u32, h as u32));
        window.set_outer_position(PhysicalPosition::new(wa.left + ((wa_w - w) / 2.0) as i32, wa.top + ((wa_h - h) / 2.0) as i32));
    }

    #[cfg(windows)]
    let scale = window.scale_factor();
    let size = window.inner_size();
    let l = layout(size.width, size.height, scale);
    let rect = |(x, y, w, h): (i32, i32, u32, u32)| wry::Rect { position: wry::dpi::PhysicalPosition::new(x, y).into(), size: wry::dpi::PhysicalSize::new(w, h).into() };

    // Every login lives in this profile; both webviews share it, and with it one browser process
    let mut context = WebContext::new(Some(profile::resolve()));

    let ipc_proxy = proxy.clone();
    let header = webview2::tune(WebViewBuilder::new_with_web_context(&mut context));
    #[cfg(windows)]
    let header = header.with_transparent(true);
    let header = header
        .with_html(header_html())
        .with_bounds(rect(l.header))
        .with_background_color(HEADER_BG)
        .with_navigation_handler(|url| !url.starts_with("http"))
        .with_ipc_handler(move |req| {
            let _ = ipc_proxy.send_event(UserEvent::Ipc(req.body().to_string()));
        })
        .build_as_child(&window)
        .map_err(|e| format!("header webview: {}", e))?;

    let nav_proxy = proxy.clone();
    let content = webview2::tune(WebViewBuilder::new_with_web_context(&mut context))
        .with_url(opts.start.url)
        .with_bounds(rect(l.content))
        .with_background_color((0, 0, 0, 255))
        // Only safe schemes (https, about, blob, data) in the site view, frames included: no file://, plain http or app schemes
        .with_navigation_handler(|url| navigation::navigation_allowed(&url))
        // Popups (Facebook login, 2FA / security checkpoint) open in place instead of being dropped, so login completes; but only
        // https on the sites and their login providers (navigation::popup_allowed). Anything else is dropped.
        .with_new_window_req_handler(move |url, _| {
            if navigation::popup_allowed(&url) {
                let _ = nav_proxy.send_event(UserEvent::Navigate(url));
            }
            NewWindowResponse::Deny
        })
        .build_as_child(&window)
        .map_err(|e| format!("site webview: {}", e))?;

    let parked_flag = Arc::new(AtomicBool::new(false));
    let tick_proxy = proxy.clone();
    let tick_parked = parked_flag.clone();
    std::thread::Builder::new()
        .name("waitplay-ticker".into())
        .spawn(move || loop {
            std::thread::sleep(if tick_parked.load(Ordering::SeqCst) { PARKED_TICK } else { ACTIVE_TICK });
            if tick_proxy.send_event(UserEvent::Tick).is_err() {
                break;
            }
        })
        .map_err(|e| e.to_string())?;

    let poll_proxy = proxy.clone();
    let poller = Poller::start(site.key, true, Box::new(move |r| {
        let _ = poll_proxy.send_event(UserEvent::Poll(r));
    }));
    let input_proxy = proxy.clone();
    let input = InputWatcher::new(move || {
        let _ = input_proxy.send_event(UserEvent::Interaction);
    });

    let desktop = desktop_parent;
    let mut app = App {
        header,
        content,
        window,
        _context: context,
        core: Core::new(desktop),
        env: UiEnv { cache: (NO_WINDOW, false) },
        poller,
        input,
        proxy,
        parked_flag,
        header_state: HeaderState::default(),
        last_site: None,
        last_can_go_back: false,
        url_check_ticks: 0,
        suspended: false,
        visible: false,
        quit_requested: false,
        exit: false,
        arm_at: Some(Instant::now() + Duration::from_secs(1)), // give the window a moment to appear before visibility rules apply
        site_key: site.key,
        close_on_finish: user_settings.on_finish == waitplay_core::settings::OnFinish::Close,
        hide_at: None,
        deep_park_at: None,
        retrim_at: None,
        retrim_fast: 0,
        idle_quit_at: None,
    };
    app.core.my_hwnd = app.hwnd();
    let _ = app.header.evaluate_script(&format!("wp.setActive({});", serde_json::to_string(opts.start.key).unwrap_or_default()));
    app.last_site = Some(opts.start.key);
    crate::wp_info!("companion started: owner={} opens={} session={:?} terminal={:?} parent={}", site.key, opts.start.key, opts.session, opts.terminal, opts.parent_hwnd);

    // A hook launch registers its session; a widget/CLI launch ("Desktop") has no owning terminal
    if !opts.session.is_empty() && opts.terminal != "Desktop" {
        let launch_hwnd = if opts.parent_hwnd != NO_WINDOW { opts.parent_hwnd } else { desktop_parent };
        app.poller.enqueue(LaunchRequest {
            session_id: opts.session.clone(),
            terminal: terminal_label(&opts.session, &opts.terminal),
            ts: now_ms(),
            hwnd: launch_hwnd,
        });
        app.keep_clear_of_tab_row(launch_hwnd); // before the first frame, so it doesn't jump later
    }

    app.window.set_visible(true);
    #[cfg(windows)]
    {
        use tao::platform::windows::WindowExtWindows;
        crate::win::native::window_glass(app.window.hwnd());
    }
    app.window.set_always_on_top(true);
    app.on_became_visible();

    event_loop.run_return(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => app.close_requested(),
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => app.do_layout(),
            Event::UserEvent(e) => app.user_event(e),
            _ => {}
        }
        app.run_deadlines();
        if app.exit {
            *flow = ControlFlow::Exit;
        } else if let Some(d) = app.next_deadline() {
            *flow = ControlFlow::WaitUntil(d);
        }
    });
    waitplay_core::companion_state::remove(site.key);
    // `app` (and with it both webviews) is dropped here: a clean browser shutdown is what flushes cookies to disk, so a sign-in
    // made moments ago survives.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_leaves_resize_borders_and_a_header() {
        // the border is 5 dip around the site on Windows (resize edges) and 0 on macOS (native frame)
        let b = BORDER_DIP.round() as i32;
        let l = layout(1400, 900, 1.0);
        assert_eq!(l.header, (b, b, (1400 - 2 * b) as u32, 44));
        assert_eq!(l.content, (b, b + 44, (1400 - 2 * b) as u32, (900 - 2 * b - 44) as u32));
        let l = layout(2100, 1350, 1.5);
        assert_eq!(l.header.3, 66);
        assert_eq!(l.content.1, (BORDER_DIP * 1.5).round() as i32 + 66);
        // tiny windows never produce zero or negative sizes
        let l = layout(10, 10, 2.0);
        assert!(l.header.2 >= 1 && l.content.3 >= 1);
    }

    #[test]
    fn terminal_labels() {
        assert_eq!(terminal_label("omp-session", "Windows Terminal"), "Windows Terminal");
        assert_eq!(terminal_label("term3-session", ""), "Term - 3");
        assert_eq!(terminal_label("omp-session", ""), "");
        assert_eq!(terminal_label("term7", "Desktop"), "Term - 7");
    }

    #[test]
    fn header_status_json() {
        let mut s = HeaderState::default();
        assert_eq!(s.to_json()["kind"], "none");
        s = HeaderState { agent: "Claude Code".into(), terminal: "WT".into(), model: "opus".into(), tool: "bash".into(), shown: true, ..Default::default() };
        let j = s.to_json();
        assert_eq!(j["kind"], "working");
        assert_eq!(j["text"], "Claude Code is working");
        assert_eq!(j["where"], "WT \u{b7} opus \u{b7} bash");
        s.waiting_prompt = Some("Allow rm?".into());
        let j = s.to_json();
        assert_eq!(j["kind"], "waiting");
        assert_eq!(j["title"], "Allow rm?");
    }

    #[test]
    fn header_page_has_a_button_per_site() {
        let html = header_html();
        for s in sites::SITES.iter() {
            assert!(html.contains(&format!("data-site='{}'", s.key)), "{}", s.key);
        }
        assert!(!html.contains("{{SITES}}"));
        assert!(!html.to_lowercase().contains("reddit"));
    }
}
