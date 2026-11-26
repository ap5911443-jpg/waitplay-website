//! Operating-system services the companion window and the widget rely on.
//!
//! This file is the CONTRACT: every implementation (`windows.rs`, `macos.rs`, `other.rs`) must provide every item below with exactly
//! these signatures, and callers use only `crate::platform::*`. `macos.rs` maps a WindowId to an application PID
//! (see its header). `other.rs` keeps the app compiling on other systems with neutral answers.

/// A top-level window of any application: the HWND as an integer on Windows. `NO_WINDOW` (0) means none.
pub type WindowId = u64;
pub const NO_WINDOW: WindowId = 0;

/// Screen rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(not(any(windows, target_os = "macos")))]
mod other;
#[cfg(not(any(windows, target_os = "macos")))]
pub use self::other::*;

// ---------------------------------------------------------------------------------------------------------------------
// Functions every implementation provides (signatures only; see windows.rs for the bodies):
//
//   pub fn current_pid() -> u32
//   pub fn foreground_window() -> WindowId                    // GetForegroundWindow; NO_WINDOW when none (locked screen)
//   pub fn is_window(id: WindowId) -> bool                    // the window still exists
//   pub fn is_minimized(id: WindowId) -> bool
//   pub fn window_rect(id: WindowId) -> Option<Rect>          // physical px
//   pub fn window_pid(id: WindowId) -> u32                    // owning process, 0 if unknown
//   pub fn focus_window(id: WindowId)                         // restore if minimized, then bring to the foreground
//   pub fn show_window(id: WindowId, activate: bool)          // show an own/other window; activate=false: SW_SHOWNOACTIVATE
//   pub fn hide_window(id: WindowId)                          // SW_HIDE at the OS level (tao's set_visible(false) is a no-op for a raw-shown window)
//   pub fn is_waitplay_window(id: WindowId) -> bool           // same process, or a process with our executable name
//   pub fn other_companion_visible() -> bool                  // another waitplay process shows a companion window (title "Vibe Play"; the widget's title is "Vibe Play Widget")
//   pub fn parent_pid(pid: u32) -> u32                        // 0 if unknown
//   pub fn process_tree(root: u32) -> Vec<u32>                // root plus every descendant (parent-pid walk over a snapshot)
//   pub fn find_top_window_for_pid(pid: u32) -> WindowId      // port of ParentWindowTracker.FindTopWindowForPid
//   pub fn find_window_for_agent(agent_name: &str) -> WindowId// port of ParentWindowTracker.FindWindowForAgent
//   pub fn work_area() -> Rect                                // primary monitor work area (excludes the taskbar), physical px
//   pub fn cursor_position() -> (i32, i32)                    // physical px
//   pub fn attach_parent_console()                            // so `waitplay --toggle` can print into the invoking shell
//
//   pub mod memory {                                          // port of RamTrim (the managed-heap GC part is gone with C#)
//       pub fn park(browser_pid: u32)                         // this process + the whole WebView2 tree under browser_pid (0 = only this process): very-low memory priority + EmptyWorkingSet
//       pub fn wake(browser_pid: u32)                         // normal memory priority again
//       pub fn trim_self()                                    // EmptyWorkingSet on this process only
//   }
//
//   pub struct TabTracker;                                    // port of TerminalTabTracker (UI Automation); Send
//   impl TabTracker {
//       pub fn capture(hwnd: WindowId) -> Option<TabTracker>  // selected tab of that terminal window right now; None: no tab strip
//       pub fn own_tab_selected(&mut self) -> Option<bool>    // Some(true) own tab in front, Some(false) another tab, None unknown
//       pub fn gone(&self) -> bool                            // the tab (or window) was closed
//   }
//
//   pub struct InputWatcher;                                  // port of TerminalInputWatcher (low-level mouse-wheel/keyboard hooks on its own thread)
//   impl InputWatcher {
//       pub fn new(on_interaction: impl Fn() + Send + Sync + 'static) -> InputWatcher
//       pub fn set_target(&self, hwnd: WindowId)              // NO_WINDOW disarms; the callback fires (on the watcher's thread) when the user scrolls/types while `hwnd` is the foreground window
//   }                                                          // dropping it unhooks and stops the thread
// ---------------------------------------------------------------------------------------------------------------------
