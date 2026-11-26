//! The floating widget: a capsule (power) with a hover pill, a secondary circle (status dot + app picker), an edge handle when
//! idle, drag-to-dock and the Alt+W hotkey. Port of the C# `FloatingFlowWidget` + `DockDragOverlay`.
//!
//! One transparent, undecorated, always-on-top tao window with one wry webview (`assets/widget.html`). A transparent
//! WebView2 window does not click through its transparent pixels, so the window is always exactly the size of the UI that is
//! visible: the page measures its own box and reports it over IPC, and `dock::window_rect` places the window so the capsule
//! stays on its anchor whatever expands or collapses. While a drag is running the window covers the whole work area and the
//! page draws the dim overlay, the three drop zones and the ball itself (no second window).

mod dock;
mod quit;
mod status;

use crate::platform::{self, Rect};
use crate::sites;
use dock::{Content, DockSide};
use serde::Deserialize;
use serde_json::json;
use status::Dot;
use std::path::PathBuf;
use std::time::Duration;
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::{Window, WindowBuilder};
use waitplay_core::{now_ms, process, state, BridgeClient};
use wry::{WebContext, WebView, WebViewBuilder};

/// The title the platform layer uses to tell the widget from companion windows (which are titled "Vibe Play").
const TITLE: &str = "Vibe Play Widget";

/// Runs the widget until it exits. Blocks.
pub fn run() -> Result<(), String> {
    // one widget per TEMP: a second one could not even register Alt+W
    let Some(_single) = crate::single_instance::acquire("widget") else { return Ok(()) };

    let mut event_loop: EventLoop<Ev> = EventLoopBuilder::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut widget = Widget::new(&event_loop, proxy.clone())?;

    // one second heartbeat: display refits every tick, status poll every 3 s, RAM trim every 30 s
    let ticker = proxy.clone();
    std::thread::Builder::new()
        .name("wp-widget-tick".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            if ticker.send_event(Ev::Tick).is_err() {
                break;
            }
        })
        .map_err(|e| e.to_string())?;

    event_loop.run_return(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(ev) => {
                if widget.handle(ev) {
                    *flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent { event: WindowEvent::ScaleFactorChanged { .. }, .. } => widget.refit(),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *flow = ControlFlow::Exit,
            _ => {}
        }
    });
    Ok(())
}

/// `--dock=<left|right|bottom|free>`: persist the dock side the widget reads on start.
pub fn save_dock(side: &str) {
    dock::save_cli(&dock::dock_dir(), waitplay_core::paths::repo_root().as_deref(), side);
}

// ---------------------------------------------------------------------------------------------------------------------

enum Ev {
    /// A message from the page.
    Ipc(String),
    Hotkey,
    Tick,
    /// `GET /sessions` finished (`None`: the bridge did not answer).
    Sessions(Option<String>),
}

#[derive(Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
enum Msg {
    Ready,
    Size { dock: String, w: f64, h: f64, cx: f64, cy: f64, dpr: f64 },
    Toggle,
    Launch { site: String },
    Dock { side: String },
    QuitCompanions,
    Exit,
    DragStart,
    DragEnd { x: f64, y: f64, w: f64, h: f64 },
    DragDone { side: String },
}

struct Widget {
    window: Window,
    webview: WebView,
    // kept alive for the life of the widget: the hotkey registration and the WebView2 profile folder
    _hotkey: Option<global_hotkey::GlobalHotKeyManager>,
    _context: WebContext,
    proxy: EventLoopProxy<Ev>,
    dock: DockSide,
    enabled: bool,
    dot: Dot,
    wa: Rect,
    scale_factor: f64,
    zoom: f64,
    /// What the page last measured; `None` until the first report.
    content: Option<Content>,
    shown: bool,
    dragging: bool,
    ticks: u64,
    polling: bool,
}

impl Widget {
    fn new(event_loop: &EventLoop<Ev>, proxy: EventLoopProxy<Ev>) -> Result<Widget, String> {
        let wa = platform::work_area();
        let scale_factor = event_loop.primary_monitor().map_or(1.0, |m| m.scale_factor());
        let zoom = dock::zoom_for(wa.height(), scale_factor);

        let builder = WindowBuilder::new()
            .with_title(TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(true)
            .with_resizable(false)
            .with_visible(false)
            .with_focusable(false)
            .with_inner_size(PhysicalSize::new(64u32, 64u32))
            .with_position(PhysicalPosition::new(wa.left, wa.top));
        #[cfg(windows)]
        let builder = {
            use tao::platform::windows::WindowBuilderExtWindows;
            builder.with_skip_taskbar(true).with_undecorated_shadow(false)
        };
        let window = builder.build(event_loop).map_err(|e| format!("window: {}", e))?;

        let data_dir = webview_data_dir();
        let _ = std::fs::create_dir_all(&data_dir);
        let mut context = WebContext::new(Some(data_dir));
        let ipc_proxy = proxy.clone();
        let builder = WebViewBuilder::new_with_web_context(&mut context)
            .with_transparent(true)
            .with_background_color((0, 0, 0, 0))
            .with_focused(false)
            .with_devtools(cfg!(debug_assertions))
            .with_hotkeys_zoom(false)
            .with_html(page_html())
            .with_ipc_handler(move |req| {
                let _ = ipc_proxy.send_event(Ev::Ipc(req.into_body()));
            });
        #[cfg(windows)]
        let builder = {
            use wry::WebViewBuilderExtWindows;
            builder
                .with_browser_accelerator_keys(false)
                .with_default_context_menus(false)
                // One tiny local page: one renderer, no background services (its own data dir, so its own flags are fine)
                .with_additional_browser_args(
                    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate,OptimizationHints,MediaRouter \
                     --renderer-process-limit=1 --disable-background-networking --disable-component-update --disable-breakpad --no-pings \
                     --disable-extensions --disable-sync",
                )
        };
        let webview = builder.build(&window).map_err(|e| format!("webview: {}", e))?;
        let _ = webview.zoom(zoom);

        let enabled = state::is_enabled();
        let w = Widget {
            window,
            webview,
            _hotkey: register_hotkey(&proxy),
            _context: context,
            proxy,
            dock: dock::load(&dock::dock_dir()).unwrap_or(DockSide::Right),
            enabled,
            dot: if enabled { Dot::Active } else { Dot::Paused },
            wa,
            scale_factor,
            zoom,
            content: None,
            shown: false,
            dragging: false,
            ticks: 0,
            polling: false,
        };
        w.push(true);
        Ok(w)
    }

    /// Returns true when the widget should exit.
    fn handle(&mut self, ev: Ev) -> bool {
        match ev {
            Ev::Ipc(text) => match serde_json::from_str::<Msg>(&text) {
                Ok(msg) => return self.on_msg(msg),
                Err(e) => eprintln!("waitplay widget: bad message {:?}: {}", text, e),
            },
            Ev::Hotkey => self.toggle_enabled(),
            Ev::Tick => self.on_tick(),
            Ev::Sessions(body) => {
                self.polling = false;
                if self.enabled {
                    self.dot = body.and_then(|b| status::dot_for_sessions(&b, now_ms())).unwrap_or(Dot::Active);
                    self.push(false);
                }
            }
        }
        false
    }

    fn on_msg(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Ready => {
                self.push(true);
            }
            Msg::Size { dock, w, h, cx, cy, dpr } => {
                if self.dragging || dock != self.dock.css() || !(w > 0.0 && h > 0.0 && dpr > 0.0) {
                    return false;
                }
                let c = Content { w, h, cx, cy, dpr };
                self.content = Some(c);
                self.place_content();
            }
            Msg::Toggle => self.toggle_enabled(),
            Msg::Launch { site } => launch_site(&site),
            Msg::Dock { side } => {
                if let Some(side) = DockSide::parse(&side) {
                    self.set_dock(side);
                }
            }
            Msg::QuitCompanions => quit::quit_companions(),
            Msg::Exit => return true,
            Msg::DragStart => self.begin_drag(),
            Msg::DragEnd { x, y, w, h } => {
                let side = dock::nearest((x, y), w, h);
                self.eval(&format!("wp.snap('{}')", side.css()));
            }
            Msg::DragDone { side } => {
                self.dragging = false;
                if let Some(side) = DockSide::parse(&side) {
                    self.dock = side;
                    dock::save(&dock::dock_dir(), side);
                }
                self.content = None;
                self.push(true);
            }
        }
        false
    }

    // ---- state ------------------------------------------------------------------------------------------------------

    fn set_dock(&mut self, side: DockSide) {
        self.dock = side;
        dock::save(&dock::dock_dir(), side);
        self.content = None;
        self.push(true);
    }

    fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
        state::set_enabled(self.enabled);
        let on = self.enabled;
        let _ = std::thread::Builder::new().name("wp-post-state".into()).spawn(move || BridgeClient::new().post_state(on));
        self.dot = if on { Dot::Active } else { Dot::Paused };
        self.push(false);
        if !on {
            quit::quit_companions();
        }
    }

    /// Sends the whole visible state to the page. `reset`: also bring the page back to idle at the (new) dock.
    fn push(&self, reset: bool) {
        let dot = if self.enabled { self.dot } else { Dot::Paused };
        let mut st = json!({ "dock": self.dock.css(), "enabled": self.enabled, "dot": dot.hex() });
        if reset {
            st["reset"] = json!(true);
        }
        self.eval(&format!("wp.apply({})", st));
    }

    fn eval(&self, js: &str) {
        let _ = self.webview.evaluate_script(js);
    }

    // ---- ticks ------------------------------------------------------------------------------------------------------

    fn on_tick(&mut self) {
        self.ticks += 1;
        self.refit();
        if self.ticks % 3 == 0 {
            // `waitplay --toggle` and `--dock=` run in other processes: follow them
            let on = state::is_enabled();
            if on != self.enabled {
                self.enabled = on;
                self.dot = if on { Dot::Active } else { Dot::Paused };
                self.push(false);
            }
            if !self.dragging {
                if let Some(side) = dock::load(&dock::dock_dir()) {
                    if side != self.dock {
                        self.set_dock(side);
                    }
                }
            }
            if self.enabled && !self.polling {
                self.polling = true;
                let proxy = self.proxy.clone();
                let spawned = std::thread::Builder::new().name("wp-poll".into()).spawn(move || {
                    let body = BridgeClient::new().get_sessions(Duration::from_millis(400));
                    let _ = proxy.send_event(Ev::Sessions(body));
                });
                if spawned.is_err() {
                    self.polling = false;
                }
            }
        }
        // the widget sits idle all day: give its working set back every half minute (it is read back on demand)
        if self.ticks % 30 == 0 {
            platform::memory::trim_self();
        }
    }

    // ---- geometry ---------------------------------------------------------------------------------------------------

    /// Re-fits after a resolution, scaling or taskbar change.
    fn refit(&mut self) {
        let wa = platform::work_area();
        let sf = self.window.primary_monitor().map_or(self.scale_factor, |m| m.scale_factor());
        if wa == self.wa && (sf - self.scale_factor).abs() < 1e-9 {
            return;
        }
        self.wa = wa;
        self.scale_factor = sf;
        let zoom = dock::zoom_for(wa.height(), sf);
        if (zoom - self.zoom).abs() > 1e-9 {
            self.zoom = zoom;
            let _ = self.webview.zoom(zoom);
        }
        if self.dragging {
            self.place(wa);
        } else {
            self.content = None;
        }
        // the page measures itself again (its dpr changed) and the new size lands in `place_content`
        self.push(false);
    }

    fn place_content(&mut self) {
        if let Some(c) = self.content {
            let r = dock::window_rect(self.dock, self.wa, &c);
            self.place(r);
        }
    }

    fn place(&mut self, r: Rect) {
        place_window(&self.window, r, !self.shown);
        self.shown = true;
    }

    fn dpr(&self) -> f64 {
        self.content.map_or(self.scale_factor * self.zoom, |c| c.dpr)
    }

    fn begin_drag(&mut self) {
        self.dragging = true;
        let wa = self.wa;
        self.place(wa);
        let dpr = self.dpr();
        let (w, h) = (wa.width() as f64 / dpr, wa.height() as f64 / dpr);
        let (cx, cy) = platform::cursor_position();
        let centres: Vec<_> = dock::zone_centres(w, h).iter().map(|(s, (x, y))| json!({ "side": s.css(), "x": x, "y": y })).collect();
        let info = json!({ "centres": centres, "x": (cx - wa.left) as f64 / dpr, "y": (cy - wa.top) as f64 / dpr });
        self.eval(&format!("wp.dragReady({})", info));
    }
}

// ---------------------------------------------------------------------------------------------------------------------

/// The page with the site list (the real logos) filled in.
fn page_html() -> String {
    let items: String = sites::SITES
        .iter()
        .map(|s| {
            format!(
                "<div class=\"item\" data-site=\"{}\"><span class=\"ico\">{}</span><span>{}</span></div>",
                s.key,
                sites::logo_svg(s.key, 16, false),
                s.label
            )
        })
        .collect();
    include_str!("../../assets/widget.html").replace("<!--ITEMS-->", &items)
}

/// The widget's own WebView2 profile: separate from the companions' (their quit sequence ends their own browser trees and
/// must never touch the widget's).
fn webview_data_dir() -> PathBuf {
    if let Ok(over) = std::env::var("WAITPLAY_PROFILE_DIR") {
        if !over.trim().is_empty() {
            return PathBuf::from(over.trim()).join("widget-ui");
        }
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support")).unwrap_or_else(std::env::temp_dir);
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("WaitPlay").join("widget-ui")
}

/// Alt+W. A hotkey another program already owns is not fatal: the widget still works with the mouse.
fn register_hotkey(proxy: &EventLoopProxy<Ev>) -> Option<global_hotkey::GlobalHotKeyManager> {
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

    #[allow(unused_mut)]
    let mut hotkey = HotKey::new(Some(Modifiers::ALT), Code::KeyW);
    // debug builds only: lets an isolated test run use a free key while a real Vibe Play owns Alt+W
    #[cfg(debug_assertions)]
    if let Some(h) = std::env::var("WAITPLAY_TEST_HOTKEY").ok().and_then(|s| s.parse::<HotKey>().ok()) {
        hotkey = h;
    }

    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("waitplay widget: hotkey unavailable: {}", e);
            return None;
        }
    };
    if let Err(e) = manager.register(hotkey) {
        eprintln!("waitplay widget: could not register the hotkey: {}", e);
        return None;
    }
    let (id, proxy) = (hotkey.id(), proxy.clone());
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        if e.id() == id && e.state() == HotKeyState::Pressed {
            let _ = proxy.send_event(Ev::Hotkey);
        }
    }));
    Some(manager)
}

/// `current_exe --game --site=<key>`, detached: what the app picker does.
fn launch_site(key: &str) {
    let Some(site) = sites::by_name(key) else { return };
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = process::spawn_detached(&exe, &["--game".to_string(), format!("--site={}", site.key)]);
}

/// Moves and resizes in one step (two steps would flash a frame at the wrong size); `show` makes the window visible without
/// taking focus from whatever the user is typing in.
#[cfg(windows)]
fn place_window(window: &Window, r: Rect, show: bool) {
    use std::ffi::c_void;
    use tao::platform::windows::WindowExtWindows;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW};

    let hwnd = HWND(window.hwnd() as *mut c_void);
    let mut flags = SWP_NOACTIVATE;
    if show {
        flags |= SWP_SHOWWINDOW;
    }
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), r.left, r.top, r.width(), r.height(), flags);
    }
}

#[cfg(not(windows))]
fn place_window(window: &Window, r: Rect, show: bool) {
    window.set_outer_position(PhysicalPosition::new(r.left, r.top));
    window.set_inner_size(PhysicalSize::new(r.width().max(1) as u32, r.height().max(1) as u32));
    if show {
        window.set_visible(true);
    }
}
