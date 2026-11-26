//! Neutral implementation of the platform contract for non-Windows targets.
//!
//! Window tracking is NOT implemented on this platform: every query answers "no window" / `None` / `false` and every action is
//! a no-op, so the companion simply stays where the user put it. Nothing here pretends to track terminals or input.
//! Real macOS support means replacing these bodies (or adding a `macos.rs`) behind the same surface.

use super::{Rect, WindowId, NO_WINDOW};

pub fn current_pid() -> u32 {
    std::process::id()
}
pub fn foreground_window() -> WindowId {
    NO_WINDOW
}
pub fn is_window(_id: WindowId) -> bool {
    false
}
pub fn is_minimized(_id: WindowId) -> bool {
    false
}
pub fn window_rect(_id: WindowId) -> Option<Rect> {
    None
}
pub fn window_pid(_id: WindowId) -> u32 {
    0
}
pub fn focus_window(_id: WindowId) {}
pub fn show_window(_id: WindowId, _activate: bool) {}
pub fn hide_window(_id: WindowId) {}
pub fn is_waitplay_window(_id: WindowId) -> bool {
    false
}
pub fn other_companion_visible() -> bool {
    false
}
pub fn parent_pid(_pid: u32) -> u32 {
    0
}
/// Only the root: descendants are not enumerated on this platform.
pub fn process_tree(root: u32) -> Vec<u32> {
    vec![root]
}
pub fn find_top_window_for_pid(_pid: u32) -> WindowId {
    NO_WINDOW
}
pub fn find_window_for_agent(_agent_name: &str) -> WindowId {
    NO_WINDOW
}
/// A fixed 1920x1080 area: the real work area is not queried on this platform.
pub fn work_area() -> Rect {
    Rect { left: 0, top: 0, right: 1920, bottom: 1080 }
}
pub fn cursor_position() -> (i32, i32) {
    (0, 0)
}
pub fn attach_parent_console() {}

pub mod memory {
    pub fn park(_browser_pid: u32) {}
    pub fn wake(_browser_pid: u32) {}
    pub fn trim_self() {}
}

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

pub struct InputWatcher;

impl InputWatcher {
    /// The callback never fires: input is not observed on this platform.
    pub fn new(_on_interaction: impl Fn() + Send + Sync + 'static) -> InputWatcher {
        InputWatcher
    }
    pub fn set_target(&self, _hwnd: WindowId) {}
}

pub struct ForegroundWatcher;

impl ForegroundWatcher {
    /// The callback never fires on this platform.
    pub fn new(_on_change: impl Fn() + Send + Sync + 'static) -> ForegroundWatcher {
        ForegroundWatcher
    }
}
