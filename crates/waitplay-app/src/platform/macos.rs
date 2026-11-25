//! macOS implementation of the platform contract (see `platform/mod.rs`).
//!
//! A `WindowId` on macOS is the PID of an application. macOS exposes no per-window id to other apps without the Screen Recording
//! permission, and WaitPlay does not need one: the companion shows while the application that hosts the agent's terminal is in
//! front. Nothing here needs Accessibility or Screen Recording: the frontmost application comes from `NSWorkspace`.
//!
//! Not provided on macOS (neutral answers, by design): terminal tab tracking and "typing in the terminal steps the window aside"
//! (both need the Accessibility permission, which WaitPlay does not ask for), window rectangles of other apps.

use super::{Rect, WindowId, NO_WINDOW};
use crate::placement::{LRect, Screen};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication, NSScreen, NSWorkspace};
use std::process::Command;

pub fn current_pid() -> u32 {
    std::process::id()
}

fn running_app(pid: u32) -> Option<Retained<NSRunningApplication>> {
    if pid == 0 || pid > i32::MAX as u32 {
        return None;
    }
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid as libc::pid_t)
}

/// The application in front. `NO_WINDOW` when there is none.
pub fn foreground_window() -> WindowId {
    match NSWorkspace::sharedWorkspace().frontmostApplication() {
        Some(app) => {
            let pid = app.processIdentifier();
            if pid > 0 {
                pid as WindowId
            } else {
                NO_WINDOW
            }
        }
        None => NO_WINDOW,
    }
}

pub fn is_window(id: WindowId) -> bool {
    if id == NO_WINDOW || id > i32::MAX as u64 {
        return false;
    }
    // SAFETY: signal 0 only tests for existence; EPERM still means the process exists.
    let r = unsafe { libc::kill(id as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

pub fn is_minimized(_id: WindowId) -> bool {
    false
}

pub fn window_rect(_id: WindowId) -> Option<Rect> {
    None
}

pub fn window_pid(id: WindowId) -> u32 {
    if id <= u32::MAX as u64 {
        id as u32
    } else {
        0
    }
}

/// Brings that application to the front (the keyboard goes back to the user's terminal).
pub fn focus_window(id: WindowId) {
    if let Some(app) = running_app(window_pid(id)) {
        #[allow(deprecated)]
        let _ = app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
    }
}

/// Own windows are shown and hidden through tao on macOS.
pub fn show_window(_id: WindowId, _activate: bool) {}
pub fn hide_window(_id: WindowId) {}

fn ps_field(pid: u32, field: &str) -> Option<String> {
    let out = Command::new("/bin/ps").args(["-o", &format!("{}=", field), "-p", &pid.to_string()]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn exe_name_of(pid: u32) -> Option<String> {
    // `comm` is the full executable path on macOS
    let path = ps_field(pid, "comm")?;
    path.rsplit('/').next().map(str::to_string)
}

/// Our own process, or another WaitPlay process (the widget or a companion of another site).
pub fn is_waitplay_window(id: WindowId) -> bool {
    let pid = window_pid(id);
    if pid == 0 {
        return false;
    }
    if pid == std::process::id() {
        return true;
    }
    let me = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    match (me, exe_name_of(pid)) {
        (Some(me), Some(other)) => me == other,
        _ => false,
    }
}

/// Another WaitPlay process has its companion on screen (so a shared browser must not be trimmed).
pub fn other_companion_visible() -> bool {
    let me = std::process::id();
    waitplay_core::companion_state::live().iter().any(|s| s.visible && s.pid != me)
}

pub fn parent_pid(pid: u32) -> u32 {
    ps_field(pid, "ppid").and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// `root` plus every descendant, from one `ps` snapshot.
pub fn process_tree(root: u32) -> Vec<u32> {
    let Ok(out) = Command::new("/bin/ps").args(["-axo", "pid=,ppid="]).output() else { return vec![root] };
    descendants(&String::from_utf8_lossy(&out.stdout), root)
}

pub(crate) fn descendants(ps_output: &str, root: u32) -> Vec<u32> {
    let pairs: Vec<(u32, u32)> = ps_output
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
        })
        .collect();
    let mut tree = vec![root];
    let mut i = 0;
    while i < tree.len() {
        let parent = tree[i];
        for (pid, ppid) in &pairs {
            if *ppid == parent && !tree.contains(pid) {
                tree.push(*pid);
            }
        }
        i += 1;
    }
    tree
}

/// The application that hosts the agent: `pid` or its nearest ancestor that is a regular (Dock-capable) application, so a hook
/// run by `zsh` inside Terminal.app, iTerm2 or VS Code resolves to that app. `NO_WINDOW` when there is none.
pub fn find_top_window_for_pid(pid: u32) -> WindowId {
    let mut cur = pid;
    for _ in 0..32 {
        if cur <= 1 {
            break;
        }
        if let Some(app) = running_app(cur) {
            if app.activationPolicy() == NSApplicationActivationPolicy::Regular {
                return cur as WindowId;
            }
        }
        let parent = parent_pid(cur);
        if parent == 0 || parent == cur {
            break;
        }
        cur = parent;
    }
    NO_WINDOW
}

pub fn find_window_for_agent(_agent_name: &str) -> WindowId {
    NO_WINDOW
}

fn lrect(r: objc2_foundation::NSRect) -> LRect {
    LRect::new(r.origin.x, r.origin.y, r.size.width, r.size.height)
}

/// Every display in logical points with a top-left origin (primary display first). Main thread only; empty elsewhere.
pub fn screens() -> Vec<Screen> {
    let Some(mtm) = MainThreadMarker::new() else { return Vec::new() };
    let list = NSScreen::screens(mtm).to_vec();
    let primary_height = list.first().map(|s| s.frame().size.height).unwrap_or(0.0);
    // Cocoa measures y upwards from the bottom of the primary display; everything else here measures downwards from its top
    let flip = |r: LRect| LRect::new(r.x, primary_height - (r.y + r.h), r.w, r.h);
    list.iter().map(|s| Screen { frame: flip(lrect(s.frame())), work: flip(lrect(s.visibleFrame())), scale: s.backingScaleFactor() }).collect()
}

/// Primary display's usable area in physical pixels (the contract's unit).
pub fn work_area() -> Rect {
    match screens().first() {
        Some(s) => Rect {
            left: (s.work.x * s.scale).round() as i32,
            top: (s.work.y * s.scale).round() as i32,
            right: (s.work.right() * s.scale).round() as i32,
            bottom: (s.work.bottom() * s.scale).round() as i32,
        },
        None => Rect { left: 0, top: 0, right: 1440, bottom: 900 },
    }
}

pub fn cursor_position() -> (i32, i32) {
    (0, 0)
}

/// The mouse pointer in logical points with a top-left origin (the same space as [`screens`]). Main thread only.
pub fn cursor_logical() -> Option<(f64, f64)> {
    let primary_height = screens().first()?.frame.h;
    let p = objc2_app_kit::NSEvent::mouseLocation();
    Some((p.x, primary_height - p.y))
}

pub fn attach_parent_console() {}

/// The native glass behind the widget: an `NSVisualEffectView` (real behind-window blur and vibrancy that follows light/dark,
/// Reduce Transparency and Increase Contrast by itself) placed under the transparent web page and shaped to the page's pill.
pub struct Glass {
    fx: Retained<objc2_app_kit::NSVisualEffectView>,
}

impl Glass {
    /// `content_view` is the window's `NSView` (tao's `ns_view()`). Main thread only.
    pub fn attach(content_view: *mut std::ffi::c_void) -> Option<Glass> {
        use objc2_app_kit::{NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSView, NSWindowOrderingMode};
        use objc2_foundation::{NSPoint, NSRect, NSSize};
        let mtm = MainThreadMarker::new()?;
        if content_view.is_null() {
            return None;
        }
        // SAFETY: tao hands out the window's live content view
        let content: &NSView = unsafe { &*(content_view as *const NSView) };
        let fx = NSVisualEffectView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)));
        fx.setMaterial(NSVisualEffectMaterial::Popover);
        fx.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        fx.setState(NSVisualEffectState::Active);
        fx.setWantsLayer(true);
        content.addSubview_positioned_relativeTo(&fx, NSWindowOrderingMode::Below, None);
        Some(Glass { fx })
    }

    /// `x`,`y`,`w`,`h`: the pill inside the page (CSS px, top-left origin); `wh`: the page's height; `radius`: corner radius.
    pub fn set_shape(&self, x: f64, y: f64, w: f64, h: f64, wh: f64, radius: f64) {
        use objc2::runtime::{AnyObject, Bool};
        use objc2_foundation::{NSPoint, NSRect, NSSize};
        // AppKit measures y upwards from the bottom of the content view
        self.fx.setFrame(NSRect::new(NSPoint::new(x, wh - (y + h)), NSSize::new(w.max(1.0), h.max(1.0))));
        // SAFETY: `layer` is a CALayer once wantsLayer is set; both setters take plain scalars
        unsafe {
            let layer: *mut AnyObject = objc2::msg_send![&*self.fx, layer];
            if !layer.is_null() {
                let _: () = objc2::msg_send![layer, setCornerRadius: radius];
                let _: () = objc2::msg_send![layer, setMasksToBounds: Bool::YES];
            }
        }
    }
}

/// Liquid glass behind a whole window: an `NSVisualEffectView` (real behind-window blur and vibrancy, follows light/dark, Reduce
/// Transparency and Increase Contrast by itself) filling the content view under the transparent header page; the opaque site view
/// covers the middle. The window itself must be non-opaque (tao's `with_transparent`). Main thread only.
pub fn fill_with_glass(content_view: *mut std::ffi::c_void) {
    use objc2_app_kit::{NSAutoresizingMaskOptions, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSView, NSWindowOrderingMode};
    let Some(mtm) = MainThreadMarker::new() else { return };
    if content_view.is_null() {
        return;
    }
    // SAFETY: tao hands out the window's live content view
    let content: &NSView = unsafe { &*(content_view as *const NSView) };
    let fx = NSVisualEffectView::initWithFrame(mtm.alloc(), content.bounds());
    fx.setMaterial(NSVisualEffectMaterial::Popover);
    fx.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    fx.setState(NSVisualEffectState::Active);
    fx.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
    content.addSubview_positioned_relativeTo(&fx, NSWindowOrderingMode::Below, None);
}

/// Sets an `NSWindow`'s level (tao's always-on-top is the floating level; the widget sits above it so its expanded panel is never
/// covered by the entertainment window).
pub fn set_window_level(ns_window: *mut std::ffi::c_void, level: isize) {
    if ns_window.is_null() {
        return;
    }
    // SAFETY: tao hands out the window's live NSWindow; setLevel: takes an NSInteger
    unsafe {
        let w = ns_window as *mut objc2::runtime::AnyObject;
        let _: () = objc2::msg_send![w, setLevel: level];
    }
}

pub mod memory {
    pub fn park(_browser_pid: u32) {}
    pub fn wake(_browser_pid: u32) {}
    pub fn trim_self() {}
}

/// Terminal tab tracking needs the Accessibility permission, which WaitPlay does not request.
pub struct TabTracker;

impl TabTracker {
    pub fn capture(_hwnd: WindowId) -> Option<TabTracker> {
        None
    }
    pub fn own_tab_selected(&mut self) -> Option<bool> {
        None
    }
    pub fn gone(&self) -> bool {
        false
    }
}

/// Input observation needs Accessibility / Input Monitoring: not requested, so the callback never fires.
pub struct InputWatcher;

impl InputWatcher {
    pub fn new(_on_interaction: impl Fn() + Send + Sync + 'static) -> InputWatcher {
        InputWatcher
    }
    pub fn set_target(&self, _hwnd: WindowId) {}
}

/// Standard application menu (WaitPlay, Edit, Window) so the usual shortcuts work inside the web pages: without an Edit menu
/// WKWebView ignores Cmd+C / Cmd+V / Cmd+A, and signing in to a site needs paste. Call once on the main thread.
pub fn install_main_menu(app_name: &str) {
    use tray_icon::menu::{Menu, PredefinedMenuItem, Submenu};
    let build = || -> Result<(Menu, Submenu), tray_icon::menu::Error> {
        let menu = Menu::new();
        let app = Submenu::with_items(
            app_name,
            true,
            &[
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ],
        )?;
        let edit = Submenu::with_items(
            "Edit",
            true,
            &[
                &PredefinedMenuItem::undo(None),
                &PredefinedMenuItem::redo(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::cut(None),
                &PredefinedMenuItem::copy(None),
                &PredefinedMenuItem::paste(None),
                &PredefinedMenuItem::select_all(None),
            ],
        )?;
        let window = Submenu::with_items("Window", true, &[&PredefinedMenuItem::minimize(None), &PredefinedMenuItem::close_window(None)])?;
        menu.append_items(&[&app, &edit, &window])?;
        Ok((menu, window))
    };
    match build() {
        Ok((menu, window)) => {
            menu.init_for_nsapp();
            window.set_as_windows_menu_for_nsapp();
            // both must outlive the app
            std::mem::forget(window);
            std::mem::forget(menu);
        }
        Err(e) => eprintln!("waitplay: could not build the main menu: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendants_walks_the_whole_subtree_once() {
        let ps = "  1     0\n 10     1\n 11    10\n 12    10\n 13    11\n 99     1\ngarbage\n";
        let mut t = descendants(ps, 10);
        t.sort();
        assert_eq!(t, vec![10, 11, 12, 13]);
        assert_eq!(descendants(ps, 77), vec![77]);
    }

    #[test]
    fn own_process_is_a_waitplay_window_and_exists() {
        let me = std::process::id() as WindowId;
        assert!(is_window(me));
        assert!(is_waitplay_window(me));
        assert!(!is_window(NO_WINDOW));
        assert_eq!(window_pid(me), std::process::id());
    }

    #[test]
    fn parent_of_this_process_is_known() {
        assert!(parent_pid(std::process::id()) > 0);
    }
}
