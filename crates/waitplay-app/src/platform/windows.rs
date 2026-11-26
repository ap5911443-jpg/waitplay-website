//! Windows implementation of the platform contract (see `mod.rs`): Win32 window queries, ToolHelp process snapshots, RAM trimming,
//! UI Automation tab tracking for Windows Terminal and low-level input hooks.

use super::{Rect, WindowId, NO_WINDOW};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use windows::core::{Interface, BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, SetProcessInformation,
    MEMORY_PRIORITY_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
    PROCESS_SET_QUOTA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SetForegroundWindow, ShowWindow, ShowWindowAsync,
    SystemParametersInfoW, SPI_GETWORKAREA, SW_HIDE, SW_SHOW, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

fn hwnd(id: WindowId) -> HWND {
    HWND(id as usize as *mut c_void)
}
fn id_of(h: HWND) -> WindowId {
    h.0 as usize as u64
}

pub fn current_pid() -> u32 {
    unsafe { GetCurrentProcessId() }
}

pub fn foreground_window() -> WindowId {
    id_of(unsafe { GetForegroundWindow() })
}

pub fn is_window(id: WindowId) -> bool {
    id != NO_WINDOW && unsafe { IsWindow(Some(hwnd(id))) }.as_bool()
}

pub fn is_minimized(id: WindowId) -> bool {
    id != NO_WINDOW && unsafe { IsIconic(hwnd(id)) }.as_bool()
}

pub fn window_rect(id: WindowId) -> Option<Rect> {
    if id == NO_WINDOW {
        return None;
    }
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd(id), &mut r) }.ok()?;
    Some(Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom })
}

pub fn window_pid(id: WindowId) -> u32 {
    if id == NO_WINDOW {
        return 0;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd(id), Some(&mut pid)) };
    pid
}

pub fn focus_window(id: WindowId) {
    if !is_window(id) {
        return;
    }
    unsafe {
        if IsIconic(hwnd(id)).as_bool() {
            let _ = ShowWindowAsync(hwnd(id), windows::Win32::UI::WindowsAndMessaging::SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd(id));
    }
}

pub fn show_window(id: WindowId, activate: bool) {
    if id == NO_WINDOW {
        return;
    }
    unsafe {
        let _ = ShowWindow(hwnd(id), if activate { SW_SHOW } else { SW_SHOWNOACTIVATE });
    }
}

/// Hides a window at the OS level. tao's `set_visible(false)` only acts when its own VISIBLE flag is set, and a window shown with
/// `show_window` (raw ShowWindow) never sets it: the hide would be a no-op and the window would stay on screen, blank.
pub fn hide_window(id: WindowId) {
    if id == NO_WINDOW {
        return;
    }
    unsafe {
        let _ = ShowWindow(hwnd(id), SW_HIDE);
    }
}

/// Lower-cased executable file stem of a process (`waitplay` for `C:\x\WaitPlay.exe`).
fn process_stem(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let file = path.rsplit(['\\', '/']).next()?;
        Some(strip_exe(file).to_lowercase())
    }
}

fn strip_exe(file: &str) -> &str {
    match file.len().checked_sub(4) {
        Some(n) if file.is_char_boundary(n) && file[n..].eq_ignore_ascii_case(".exe") => &file[..n],
        _ => file,
    }
}

fn own_stem() -> &'static str {
    static STEM: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| process_stem(current_pid()).unwrap_or_default());
    &STEM
}

pub fn is_waitplay_window(id: WindowId) -> bool {
    if id == NO_WINDOW {
        return false;
    }
    let pid = window_pid(id);
    if pid == 0 {
        return false;
    }
    if pid == current_pid() {
        return true;
    }
    let own = own_stem();
    !own.is_empty() && process_stem(pid).as_deref() == Some(own)
}

fn window_title(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h);
        if n <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        let got = GetWindowTextW(h, &mut buf);
        String::from_utf16_lossy(&buf[..got.max(0) as usize])
    }
}

/// Calls `f` for every top-level window until it returns false.
fn each_window(mut f: impl FnMut(HWND) -> bool) {
    unsafe extern "system" fn cb(h: HWND, lparam: LPARAM) -> BOOL {
        let f = unsafe { &mut *(lparam.0 as *mut &mut dyn FnMut(HWND) -> bool) };
        BOOL::from(f(h))
    }
    let mut dynf: &mut dyn FnMut(HWND) -> bool = &mut f;
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut dynf as *mut _ as isize));
    }
}

pub fn other_companion_visible() -> bool {
    let me = current_pid();
    let own = own_stem();
    let mut found = false;
    each_window(|h| {
        if unsafe { IsWindowVisible(h) }.as_bool() && window_title(h) == "Vibe Play" {
            let pid = window_pid(id_of(h));
            if pid != me && pid != 0 && !own.is_empty() && process_stem(pid).as_deref() == Some(own) {
                found = true;
                return false;
            }
        }
        true
    });
    found
}

// ----------------------------------------------------------------------------------------------------------------- processes

/// (pid, parent pid, lower-cased exe file stem) for every process.
fn snapshot() -> Vec<(u32, u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut pe = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                let n = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
                let name = String::from_utf16_lossy(&pe.szExeFile[..n]);
                out.push((pe.th32ProcessID, pe.th32ParentProcessID, strip_exe(&name).to_lowercase()));
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

pub fn parent_pid(pid: u32) -> u32 {
    snapshot().into_iter().find(|p| p.0 == pid).map_or(0, |p| p.1)
}

/// Root plus every descendant, given (pid, parent pid) pairs. A recycled PID can't loop the walk.
pub(crate) fn tree_from(pairs: impl IntoIterator<Item = (u32, u32)>, root: u32) -> Vec<u32> {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pid, ppid) in pairs {
        kids.entry(ppid).or_default().push(pid);
    }
    let mut seen = HashSet::from([root]);
    let mut all = vec![root];
    let mut i = 0;
    while i < all.len() {
        if let Some(l) = kids.get(&all[i]) {
            for &k in l {
                if seen.insert(k) {
                    all.push(k);
                }
            }
        }
        i += 1;
    }
    all
}

pub fn process_tree(root: u32) -> Vec<u32> {
    tree_from(snapshot().into_iter().map(|p| (p.0, p.1)), root)
}

fn top_window_of_exact_pid(target: u32) -> WindowId {
    let mut found = NO_WINDOW;
    each_window(|h| {
        let id = id_of(h);
        if unsafe { IsWindow(Some(h)) }.as_bool()
            && (unsafe { IsWindowVisible(h) }.as_bool() || unsafe { IsIconic(h) }.as_bool())
            && window_pid(id) == target
            && unsafe { GetWindowTextLengthW(h) } > 0
        {
            found = id;
            return false;
        }
        true
    });
    found
}

pub fn find_top_window_for_pid(pid: u32) -> WindowId {
    if pid == 0 {
        return NO_WINDOW;
    }
    let mut curr = pid;
    for _ in 0..8 {
        if curr == 0 {
            break;
        }
        let w = top_window_of_exact_pid(curr);
        if w != NO_WINDOW {
            return w;
        }
        curr = parent_pid(curr);
    }
    NO_WINDOW
}

/// Which process names host a given agent (port of the `FindWindowForAgent` mapping).
pub(crate) fn agent_process_names(lower_agent: &str) -> Option<&'static [&'static str]> {
    let a = lower_agent;
    if a.contains("omp") {
        Some(&["omp"])
    } else if a.contains("opencode") {
        Some(&["opencode"])
    } else if a.contains("cline") || a.contains("kilo") {
        Some(&["cursor", "code"]) // Cursor, or Code when Cursor isn't running
    } else if a.contains("cursor") {
        Some(&["cursor"])
    } else if a.contains("chatgpt") || a.contains("codex") {
        Some(&["codex"])
    } else {
        None
    }
}

pub fn find_window_for_agent(agent_name: &str) -> WindowId {
    if agent_name.is_empty() {
        return NO_WINDOW;
    }
    let Some(names) = agent_process_names(&agent_name.to_lowercase()) else { return NO_WINDOW };
    let procs = snapshot();
    let mut pids: Vec<u32> = Vec::new();
    for name in names {
        pids = procs.iter().filter(|p| p.2 == *name).map(|p| p.0).collect();
        if !pids.is_empty() {
            break;
        }
    }
    for pid in pids {
        let w = find_top_window_for_pid(pid);
        if w != NO_WINDOW {
            return w;
        }
    }
    NO_WINDOW
}

pub fn work_area() -> Rect {
    let mut r = RECT::default();
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut _ as *mut c_void), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
    };
    if ok.is_err() || r.right <= r.left {
        return Rect { left: 0, top: 0, right: 1920, bottom: 1080 };
    }
    Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

pub fn cursor_position() -> (i32, i32) {
    let mut p = POINT::default();
    if unsafe { GetCursorPos(&mut p) }.is_ok() {
        (p.x, p.y)
    } else {
        (0, 0)
    }
}

/// `waitplay --toggle` is a GUI-subsystem binary: attach to the invoking shell's console and, unless stdout/stderr were
/// redirected, point them at it (Rust's std reads the standard handles lazily, so setting them here is enough).
pub fn attach_parent_console() {
    use windows::core::w;
    use windows::Win32::Foundation::GENERIC_WRITE;
    use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL};
    use windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            return;
        }
        for which in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let cur = GetStdHandle(which).unwrap_or_default();
            if !cur.is_invalid() {
                continue; // redirected to a file or pipe: leave it
            }
            if let Ok(h) = CreateFileW(
                w!("CONOUT$"),
                GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                let _ = SetStdHandle(which, h);
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------- RAM

/// Port of `RamTrim`: the moved pages go to the standby list and the very-low memory priority makes Windows reclaim them first.
pub mod memory {
    use super::*;
    use windows::Win32::System::Threading::PROCESS_INFORMATION_CLASS;
    use windows::Win32::System::ProcessStatus::EmptyWorkingSet;

    const PRIORITY_VERY_LOW: u32 = 1;
    const PRIORITY_NORMAL: u32 = 5;

    fn each(browser_pid: u32, priority: u32, trim: bool) {
        let mut pids = HashSet::from([current_pid()]);
        if browser_pid != 0 {
            pids.extend(process_tree(browser_pid));
        }
        for pid in pids {
            unsafe {
                let Ok(h) = OpenProcess(PROCESS_SET_INFORMATION | PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                else {
                    continue;
                };
                let info = MEMORY_PRIORITY_INFORMATION { MemoryPriority: windows::Win32::System::Threading::MEMORY_PRIORITY(priority) };
                let _ = SetProcessInformation(
                    h,
                    PROCESS_INFORMATION_CLASS(0), // ProcessMemoryPriority
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
                );
                if trim {
                    let _ = EmptyWorkingSet(h);
                }
                let _ = CloseHandle(h);
            }
        }
    }

    /// This process plus, when `browser_pid` is given, the whole WebView2 tree under it: very-low priority, working set emptied.
    pub fn park(browser_pid: u32) {
        each(browser_pid, PRIORITY_VERY_LOW, true);
    }
    /// Normal memory priority again, before the page asks for its memory back.
    pub fn wake(browser_pid: u32) {
        each(browser_pid, PRIORITY_NORMAL, false);
    }
    /// For a process that is only ever idle (the widget): empty the working set, nothing else changes.
    pub fn trim_self() {
        unsafe {
            let h: HANDLE = GetCurrentProcess();
            let _ = EmptyWorkingSet(h);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------- tabs

/// What the bookkeeping needs from a tab strip; UI Automation implements it, tests use a fake.
pub(crate) trait TabSource {
    fn count(&self) -> usize;
    fn is_selected(&self, i: usize) -> bool;
    /// Raw tab name (cleaned by `stable_title`).
    fn name(&self, i: usize) -> String;
}

/// Title with spinner/emoji/status glyphs removed: ASCII only, leading punctuation trimmed.
pub(crate) fn stable_title(name: &str) -> String {
    let ascii: String = name.chars().filter(|c| (' '..'\u{7f}').contains(c)).collect();
    ascii.trim_start_matches(|c: char| !c.is_ascii_alphanumeric()).trim().to_string()
}

/// Selected-tab bookkeeping (port of the logic in `TerminalTabTracker`). Tabs are tracked by position because UIA RuntimeIds
/// are not stable and titles churn; when tabs are added/closed the tab is re-located by its stable title.
#[derive(Debug)]
pub(crate) struct TabBook {
    own_index: usize,
    own_title: String,
    last_count: usize,
    pending_count: Option<usize>,
    pending_polls: u32,
    gone: bool,
}

impl TabBook {
    /// The book for the currently selected tab, None when none is selected.
    pub(crate) fn capture(tabs: &dyn TabSource) -> Option<TabBook> {
        (0..tabs.count()).find(|&i| tabs.is_selected(i)).map(|i| TabBook {
            own_index: i,
            own_title: stable_title(&tabs.name(i)),
            last_count: tabs.count(),
            pending_count: None,
            pending_polls: 0,
            gone: false,
        })
    }

    pub(crate) fn gone(&self) -> bool {
        self.gone
    }

    /// The window is minimized: a minimized terminal exposes a partial tab list, read nothing into it.
    pub(crate) fn minimized(&mut self) {
        self.pending_polls = 0;
    }

    /// `tabs` None or empty: unknown. A new tab count is trusted only once it has held for 3 polls (right after a restore the
    /// terminal briefly reports a partial list, which must not look like closed tabs).
    pub(crate) fn poll(&mut self, tabs: Option<&dyn TabSource>) -> Option<bool> {
        if self.gone {
            return None;
        }
        let tabs = tabs?;
        let count = tabs.count();
        if count == 0 {
            return None;
        }
        if count != self.last_count {
            if self.pending_count != Some(count) {
                self.pending_count = Some(count);
                self.pending_polls = 1;
                return None;
            }
            self.pending_polls += 1;
            if self.pending_polls < 3 {
                return None;
            }
            let grew = count > self.last_count;
            self.last_count = count;
            self.pending_count = None;
            if !self.relocate(tabs, grew) {
                self.gone = true;
                return None;
            }
        } else {
            self.pending_count = None;
        }
        if self.own_index >= count {
            return None;
        }
        let selected = tabs.is_selected(self.own_index);
        if selected {
            self.own_title = stable_title(&tabs.name(self.own_index)); // follow renames while we're looking at it
        }
        Some(selected)
    }

    /// Find our tab again by title, preferring the slot nearest its old position. If the title changed while unselected and
    /// tabs were only added (they are appended), keep our slot.
    fn relocate(&mut self, tabs: &dyn TabSource, grew: bool) -> bool {
        let mut best: Option<usize> = None;
        for i in 0..tabs.count() {
            if stable_title(&tabs.name(i)) != self.own_title {
                continue;
            }
            if best.map_or(true, |b| i.abs_diff(self.own_index) < b.abs_diff(self.own_index)) {
                best = Some(i);
            }
        }
        match best {
            Some(b) => {
                self.own_index = b;
                true
            }
            None => grew && self.own_index < tabs.count(),
        }
    }
}

mod uia {
    use super::*;
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationCondition, IUIAutomationElement, IUIAutomationElementArray,
        IUIAutomationSelectionItemPattern, TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_SelectionItemPatternId,
        UIA_TabControlTypeId, UIA_TabItemControlTypeId,
    };

    thread_local! {
        /// COM is initialised once per thread that touches UIA (MTA: the tracker is polled from a worker thread).
        static COM: () = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        };
    }

    pub(super) struct Items(IUIAutomationElementArray, usize);

    impl TabSource for Items {
        fn count(&self) -> usize {
            self.1
        }
        fn is_selected(&self, i: usize) -> bool {
            unsafe {
                let Ok(e) = self.0.GetElement(i as i32) else { return false };
                let Ok(p) = e.GetCurrentPattern(UIA_SelectionItemPatternId) else { return false };
                let Ok(p) = p.cast::<IUIAutomationSelectionItemPattern>() else { return false };
                p.CurrentIsSelected().map(|b| b.as_bool()).unwrap_or(false)
            }
        }
        fn name(&self, i: usize) -> String {
            unsafe {
                self.0
                    .GetElement(i as i32)
                    .and_then(|e| e.CurrentName())
                    .map(|b| b.to_string())
                    .unwrap_or_default()
            }
        }
    }

    pub(super) struct Strip {
        hwnd: HWND,
        auto: IUIAutomation,
        tab_cond: IUIAutomationCondition,
        item_cond: IUIAutomationCondition,
        strip: Option<IUIAutomationElement>,
    }

    impl Strip {
        pub(super) fn new(h: WindowId) -> Option<Strip> {
            COM.with(|_| {});
            unsafe {
                let auto: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
                let cond = |t: i32| auto.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(t)).ok();
                let tab_cond = cond(UIA_TabControlTypeId.0)?;
                let item_cond = cond(UIA_TabItemControlTypeId.0)?;
                Some(Strip { hwnd: hwnd(h), auto, tab_cond, item_cond, strip: None })
            }
        }

        /// The tab items right now; None when the window has no tab strip or UIA failed (the strip is looked up again next time).
        pub(super) fn items(&mut self) -> Option<Items> {
            COM.with(|_| {});
            unsafe {
                if self.strip.is_none() {
                    let root = self.auto.ElementFromHandle(self.hwnd).ok()?;
                    self.strip = Some(root.FindFirst(TreeScope_Descendants, &self.tab_cond).ok()?);
                }
                let arr = self.strip.as_ref()?.FindAll(TreeScope_Descendants, &self.item_cond);
                match arr.and_then(|a| a.Length().map(|n| (a, n))) {
                    Ok((a, n)) => Some(Items(a, n.max(0) as usize)),
                    Err(_) => {
                        self.strip = None;
                        None
                    }
                }
            }
        }
    }
}

/// Port of `TerminalTabTracker`: which tab of a (Windows Terminal) window a session belongs to.
pub struct TabTracker {
    hwnd: WindowId,
    strip: uia::Strip,
    book: TabBook,
}

// SAFETY: the UIA objects are created on an MTA thread (free-threaded COM proxies, usable from any MTA thread) and the tracker
// is only ever used by one thread at a time (all methods take &mut self / are called from the one polling thread).
unsafe impl Send for TabTracker {}

impl TabTracker {
    /// Selected tab of that terminal window right now; None: no tab strip (or no selected tab).
    pub fn capture(window: WindowId) -> Option<TabTracker> {
        let mut strip = uia::Strip::new(window)?;
        let items = strip.items()?;
        let book = TabBook::capture(&items)?;
        Some(TabTracker { hwnd: window, strip, book })
    }

    /// Some(true): own tab in front; Some(false): another tab is; None: unknown (minimized, UIA error, tab list in flux, gone).
    pub fn own_tab_selected(&mut self) -> Option<bool> {
        if self.book.gone() {
            return None;
        }
        if is_minimized(self.hwnd) {
            self.book.minimized();
            return None;
        }
        let items = self.strip.items();
        self.book.poll(items.as_ref().map(|i| i as &dyn TabSource))
    }

    /// The tab (or its window) was closed.
    pub fn gone(&self) -> bool {
        self.book.gone() || !is_window(self.hwnd)
    }
}

// ----------------------------------------------------------------------------------------------------------------- input

/// Shift/Ctrl/Alt (incl. left/right), Win and CapsLock on their own don't count.
pub(crate) fn is_modifier_vk(vk: u32) -> bool {
    (0x10..=0x12).contains(&vk) || (0xA0..=0xA5).contains(&vk) || vk == 0x5B || vk == 0x5C || vk == 0x14
}

/// Ctrl/Win chords are terminal or OS shortcuts (Ctrl+Tab switches tabs), not reading the output.
pub(crate) fn is_chord(ctrl: bool, lwin: bool, rwin: bool) -> bool {
    ctrl || lwin || rwin
}

mod input {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use windows::Win32::Foundation::{HINSTANCE, LRESULT};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetAncestor, GetMessageW, PeekMessageW, PostThreadMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, WindowFromPoint, GA_ROOT, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
        PM_NOREMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APP, WM_KEYDOWN, WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_QUIT,
    };

    const MSG_SYNC: u32 = WM_APP; // target changed: install/remove the hooks
    const MSG_FIRE: u32 = WM_APP + 1; // a hook saw the interaction: run the callback outside the hook

    struct HookState {
        target: Arc<AtomicU64>,
        thread_id: u32,
        mouse: Option<HHOOK>,
        key: Option<HHOOK>,
    }
    thread_local! {
        static STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
    }

    /// One-shot until the target is set again; the callback runs from the message loop, never inside the hook.
    fn fire() {
        STATE.with(|s| {
            if let Some(st) = s.borrow().as_ref() {
                st.target.store(0, Ordering::SeqCst);
                unsafe {
                    let _ = PostThreadMessageW(st.thread_id, MSG_FIRE, WPARAM(0), LPARAM(0));
                }
            }
        });
    }

    fn target() -> WindowId {
        STATE.with(|s| s.borrow().as_ref().map_or(NO_WINDOW, |st| st.target.load(Ordering::SeqCst)))
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let t = target();
        if code >= 0 && t != NO_WINDOW {
            let msg = wparam.0 as u32;
            if msg == WM_MOUSEWHEEL || msg == WM_MOUSEHWHEEL {
                let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
                let root = unsafe { GetAncestor(WindowFromPoint(info.pt), GA_ROOT) };
                if id_of(root) == t {
                    fire();
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    // WM_KEYDOWN only: Alt combinations arrive as WM_SYSKEYDOWN and are ignored by design.
    unsafe extern "system" fn key_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let t = target();
        if code >= 0 && t != NO_WINDOW && wparam.0 as u32 == WM_KEYDOWN && foreground_window() == t {
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let down = |vk: i32| (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0;
            if !is_modifier_vk(info.vkCode) && !is_chord(down(0x11), down(0x5B), down(0x5C)) {
                fire();
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    /// Hooks exist only while a target is set, so nothing sits in the system input path while the companion is hidden.
    fn sync() {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let Some(st) = s.as_mut() else { return };
            if st.target.load(Ordering::SeqCst) != NO_WINDOW {
                if st.mouse.is_none() {
                    unsafe {
                        let module: HINSTANCE = GetModuleHandleW(None).map(Into::into).unwrap_or_default();
                        st.mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), Some(module), 0).ok();
                        st.key = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), Some(module), 0).ok();
                    }
                }
            } else {
                unhook(st);
            }
        });
    }

    fn unhook(st: &mut HookState) {
        unsafe {
            if let Some(h) = st.mouse.take() {
                let _ = UnhookWindowsHookEx(h);
            }
            if let Some(h) = st.key.take() {
                let _ = UnhookWindowsHookEx(h);
            }
        }
    }

    pub(super) fn run(target: Arc<AtomicU64>, on_interaction: Arc<dyn Fn() + Send + Sync>, ready: std::sync::mpsc::Sender<u32>) {
        unsafe {
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE); // creates the thread's message queue
            let thread_id = GetCurrentThreadId();
            STATE.with(|s| *s.borrow_mut() = Some(HookState { target, thread_id, mouse: None, key: None }));
            let _ = ready.send(thread_id);
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                match msg.message {
                    MSG_SYNC => sync(),
                    MSG_FIRE => {
                        sync();
                        on_interaction();
                    }
                    WM_QUIT => break,
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            STATE.with(|s| {
                if let Some(st) = s.borrow_mut().as_mut() {
                    unhook(st);
                }
            });
        }
    }

    pub(super) fn post_sync(thread_id: u32) {
        unsafe {
            let _ = PostThreadMessageW(thread_id, MSG_SYNC, WPARAM(0), LPARAM(0));
        }
    }
    pub(super) fn post_quit(thread_id: u32) {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// Port of `TerminalInputWatcher`: fires once when the user scrolls over, or types into, the target window; then disarms
/// until `set_target` is called again. The hooks live on the watcher's own thread, which also runs the callback.
pub struct InputWatcher {
    target: std::sync::Arc<std::sync::atomic::AtomicU64>,
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl InputWatcher {
    pub fn new(on_interaction: impl Fn() + Send + Sync + 'static) -> InputWatcher {
        let target = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(NO_WINDOW));
        let (tx, rx) = std::sync::mpsc::channel();
        let t = target.clone();
        let cb: std::sync::Arc<dyn Fn() + Send + Sync> = std::sync::Arc::new(on_interaction);
        let thread = std::thread::Builder::new()
            .name("waitplay-input".into())
            .spawn(move || input::run(t, cb, tx))
            .ok();
        let thread_id = rx.recv().unwrap_or(0);
        InputWatcher { target, thread_id, thread }
    }

    pub fn set_target(&self, window: WindowId) {
        if self.target.swap(window, std::sync::atomic::Ordering::SeqCst) != window {
            input::post_sync(self.thread_id);
        }
    }
}

impl Drop for InputWatcher {
    fn drop(&mut self) {
        self.target.store(NO_WINDOW, std::sync::atomic::Ordering::SeqCst);
        if self.thread_id != 0 {
            input::post_quit(self.thread_id);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Calls back the moment the foreground window changes or any window is minimized or restored (`SetWinEventHook`, out of context),
/// so the companion reacts in about a millisecond instead of waiting for its next poll. The hook lives on its own thread with a
/// message loop; the callback only has to wake the main loop.
pub struct ForegroundWatcher {
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

static FOREGROUND_CALLBACK: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

unsafe extern "system" fn foreground_event(_hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK, _event: u32, _hwnd: HWND, _object: i32, _child: i32, _thread: u32, _time: u32) {
    if let Some(cb) = FOREGROUND_CALLBACK.get() {
        cb();
    }
}

impl ForegroundWatcher {
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> ForegroundWatcher {
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetMessageW, PeekMessageW, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, MSG, PM_NOREMOVE, WINEVENT_OUTOFCONTEXT,
        };
        let _ = FOREGROUND_CALLBACK.set(Box::new(on_change));
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("waitplay-foreground".into())
            .spawn(move || unsafe {
                let mut msg = MSG::default();
                let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE); // creates the thread's message queue
                let _ = tx.send(GetCurrentThreadId());
                let fg = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(foreground_event), 0, 0, WINEVENT_OUTOFCONTEXT);
                let min = SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(foreground_event), 0, 0, WINEVENT_OUTOFCONTEXT);
                while GetMessageW(&mut msg, None, 0, 0).0 > 0 {}
                let _ = UnhookWinEvent(fg);
                let _ = UnhookWinEvent(min);
            })
            .ok();
        let thread_id = rx.recv().unwrap_or(0);
        ForegroundWatcher { thread_id, thread }
    }
}

impl Drop for ForegroundWatcher {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
        if self.thread_id != 0 {
            // SAFETY: plain thread message to our own thread; it ends the message loop
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ------------------------------------------------------------------------------------------------------------------- displays

pub use self::displays::{cursor_logical, screens, to_physical};

/// The display list in the units `placement` works in, for the tray app (same contract as `macos.rs`: primary display first).
///
/// Windows has no single logical space across displays with different scale factors, so a [`Screen`] is expressed per display:
/// its origin (`frame.x`, `frame.y`) is the display's PHYSICAL top-left corner, and every size and every offset from that origin
/// (`work.x - frame.x`, `work.w`, ...) is in that display's logical units (physical px / `scale`). `placement` only does arithmetic
/// inside one screen, so the result converts back to physical pixels exactly with [`to_physical`].
mod displays {
    use super::*;
    use crate::placement::{LRect, Screen};
    use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Monitor {
        frame: Rect,
        work: Rect,
        scale: f64,
    }

    fn rect_of(r: RECT) -> Rect {
        Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
    }

    fn monitors() -> Vec<Monitor> {
        unsafe extern "system" fn cb(h: HMONITOR, _dc: HDC, _r: *mut RECT, lparam: LPARAM) -> BOOL {
            let list = unsafe { &mut *(lparam.0 as *mut Vec<(bool, Monitor)>) };
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if unsafe { GetMonitorInfoW(h, &mut info) }.as_bool() {
                let (mut dx, mut dy) = (96u32, 96u32);
                let _ = unsafe { GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
                let scale = if dx == 0 { 1.0 } else { dx as f64 / 96.0 };
                list.push((info.dwFlags & 1 != 0, Monitor { frame: rect_of(info.rcMonitor), work: rect_of(info.rcWork), scale }));
            }
            BOOL(1)
        }
        let mut list: Vec<(bool, Monitor)> = Vec::new();
        // SAFETY: the callback only runs during this call and `list` outlives it
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut list as *mut _ as isize));
        }
        list.sort_by_key(|(primary, _)| !*primary);
        list.into_iter().map(|(_, m)| m).collect()
    }

    fn logical_screen(m: &Monitor) -> Screen {
        let (ox, oy) = (m.frame.left as f64, m.frame.top as f64);
        let s = m.scale;
        Screen {
            frame: LRect::new(ox, oy, m.frame.width() as f64 / s, m.frame.height() as f64 / s),
            work: LRect::new(
                ox + (m.work.left - m.frame.left) as f64 / s,
                oy + (m.work.top - m.frame.top) as f64 / s,
                m.work.width() as f64 / s,
                m.work.height() as f64 / s,
            ),
            scale: s,
        }
    }

    /// A logical point inside `screen` (the convention above) in physical pixels.
    pub fn to_physical(screen: &Screen, x: f64, y: f64) -> (i32, i32) {
        ((screen.frame.x + (x - screen.frame.x) * screen.scale).round() as i32, (screen.frame.y + (y - screen.frame.y) * screen.scale).round() as i32)
    }

    fn point_to_logical(m: &Monitor, px: i32, py: i32) -> (f64, f64) {
        (m.frame.left as f64 + (px - m.frame.left) as f64 / m.scale, m.frame.top as f64 + (py - m.frame.top) as f64 / m.scale)
    }

    /// Every display, primary first. Empty only if Windows reports none.
    pub fn screens() -> Vec<Screen> {
        monitors().iter().map(logical_screen).collect()
    }

    /// The mouse pointer in the same convention as [`screens`] (so `placement::screen_at` finds its display).
    pub fn cursor_logical() -> Option<(f64, f64)> {
        let (px, py) = cursor_position();
        let list = monitors();
        let m = list.iter().find(|m| px >= m.frame.left && px < m.frame.right && py >= m.frame.top && py < m.frame.bottom).or(list.first())?;
        Some(point_to_logical(m, px, py))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn mon(frame: (i32, i32, i32, i32), work: (i32, i32, i32, i32), scale: f64) -> Monitor {
            Monitor { frame: Rect { left: frame.0, top: frame.1, right: frame.2, bottom: frame.3 }, work: Rect { left: work.0, top: work.1, right: work.2, bottom: work.3 }, scale }
        }

        #[test]
        fn a_150_percent_display_reports_logical_sizes_from_its_physical_origin() {
            // 3840x2160 at 150%, taskbar 80 px tall at the bottom
            let s = logical_screen(&mon((0, 0, 3840, 2160), (0, 0, 3840, 2080), 1.5));
            assert_eq!((s.frame.w, s.frame.h), (2560.0, 1440.0));
            assert_eq!((s.work.x, s.work.y, s.work.w, s.work.h), (0.0, 0.0, 2560.0, 2080.0 / 1.5));
            assert_eq!(s.scale, 1.5);
        }

        #[test]
        fn secondary_display_round_trips_exactly_to_physical_pixels() {
            // a 1920x1080 display at 125% to the right of a 2560x1440 one; the taskbar sits on its left (60 px)
            let s = logical_screen(&mon((2560, 0, 4480, 1080), (2620, 0, 4480, 1080), 1.25));
            assert_eq!(s.frame.x, 2560.0);
            assert_eq!(s.work.x, 2560.0 + 60.0 / 1.25);
            // the widget in the bottom-right corner of the work area with an 8 px margin
            let size = (340.0, 96.0);
            let (x, y) = crate::placement::corner_origin(s.work, size, waitplay_core::settings::WidgetPosition::BottomRight, 8.0);
            let (px, py) = to_physical(&s, x, y);
            let (w, h) = ((size.0 * s.scale).round() as i32, (size.1 * s.scale).round() as i32);
            assert_eq!(px + w + (8.0 * 1.25) as i32, 4480, "right edge sits 8 logical px inside the work area");
            assert_eq!(py + h + (8.0 * 1.25) as i32, 1080, "bottom edge too");
        }

        #[test]
        fn pointer_conversion_stays_on_its_display() {
            let m = mon((-1920, 0, 0, 1080), (-1920, 0, 0, 1040), 1.0);
            assert_eq!(point_to_logical(&m, -100, 500), (-100.0, 500.0));
            let hi = mon((0, 0, 3000, 2000), (0, 0, 3000, 1960), 2.0);
            assert_eq!(point_to_logical(&hi, 1000, 600), (500.0, 300.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Vec<(bool, &'static str)>);
    impl TabSource for Fake {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn is_selected(&self, i: usize) -> bool {
            self.0[i].0
        }
        fn name(&self, i: usize) -> String {
            self.0[i].1.to_string()
        }
    }
    fn tabs(sel: usize, names: &[&'static str]) -> Fake {
        Fake(names.iter().enumerate().map(|(i, n)| (i == sel, *n)).collect())
    }
    fn poll(b: &mut TabBook, f: &Fake) -> Option<bool> {
        b.poll(Some(f as &dyn TabSource))
    }

    #[test]
    fn tree_walk() {
        let pairs = [(1, 0), (2, 1), (3, 1), (4, 2), (5, 4), (9, 8)];
        let mut t = tree_from(pairs, 1);
        t.sort();
        assert_eq!(t, vec![1, 2, 3, 4, 5]);
        assert_eq!(tree_from(pairs, 7), vec![7]);
        // recycled pid: 1 is also a child of 5 -> no loop, no duplicates
        let cyc = [(2, 1), (1, 2)];
        let mut t = tree_from(cyc, 1);
        t.sort();
        assert_eq!(t, vec![1, 2]);
    }

    #[test]
    fn titles() {
        assert_eq!(stable_title("⠋ ✳ Claude Code"), "Claude Code");
        assert_eq!(stable_title("  - pwsh  "), "pwsh");
        assert_eq!(stable_title("★★"), "");
    }

    #[test]
    fn agents() {
        assert_eq!(agent_process_names("cline"), Some(&["cursor", "code"][..]));
        assert_eq!(agent_process_names("codex"), Some(&["codex"][..]));
        assert_eq!(agent_process_names("zzz"), None);
    }

    #[test]
    fn selection_follows_tab() {
        let f = tabs(1, &["a", "b", "c"]);
        let mut b = TabBook::capture(&f).unwrap();
        assert_eq!(poll(&mut b, &f), Some(true));
        assert_eq!(poll(&mut b, &tabs(2, &["a", "b", "c"])), Some(false));
        assert_eq!(poll(&mut b, &tabs(1, &["a", "b ⠋", "c"])), Some(true));
        assert!(TabBook::capture(&Fake(vec![(false, "a")])).is_none());
    }

    #[test]
    fn closed_tab_before_ours_relocates_after_three_polls() {
        let mut b = TabBook::capture(&tabs(2, &["a", "b", "term"])).unwrap();
        let now = tabs(1, &["b", "term"]);
        assert_eq!(poll(&mut b, &now), None);
        assert_eq!(poll(&mut b, &now), None);
        assert_eq!(poll(&mut b, &now), Some(true));
        assert!(!b.gone());
    }

    #[test]
    fn transient_partial_list_is_ignored() {
        let mut b = TabBook::capture(&tabs(2, &["a", "b", "term"])).unwrap();
        assert_eq!(poll(&mut b, &tabs(0, &["a"])), None);
        assert_eq!(poll(&mut b, &tabs(2, &["a", "b", "term"])), Some(true));
        assert!(!b.gone());
    }

    #[test]
    fn closed_own_tab_is_gone() {
        let mut b = TabBook::capture(&tabs(1, &["a", "term"])).unwrap();
        let now = tabs(0, &["a"]);
        for _ in 0..3 {
            poll(&mut b, &now);
        }
        assert!(b.gone());
        assert_eq!(poll(&mut b, &tabs(1, &["a", "term"])), None);
    }

    #[test]
    fn added_tab_keeps_slot_even_if_renamed() {
        let mut b = TabBook::capture(&tabs(0, &["term", "x"])).unwrap();
        let now = tabs(2, &["renamed", "x", "new"]);
        for _ in 0..3 {
            poll(&mut b, &now);
        }
        assert!(!b.gone());
        assert_eq!(poll(&mut b, &now), Some(false));
    }

    #[test]
    fn no_tabs_is_unknown() {
        let mut b = TabBook::capture(&tabs(0, &["a"])).unwrap();
        assert_eq!(b.poll(None), None);
        assert_eq!(poll(&mut b, &Fake(vec![])), None);
    }

    #[test]
    fn modifiers_and_chords() {
        for vk in [0x10, 0x11, 0x12, 0xA0, 0xA5, 0x5B, 0x5C, 0x14] {
            assert!(is_modifier_vk(vk));
        }
        assert!(!is_modifier_vk(0x41) && !is_modifier_vk(0x0D) && !is_modifier_vk(0x20));
        assert!(is_chord(true, false, false) && is_chord(false, true, false) && is_chord(false, false, true));
        assert!(!is_chord(false, false, false));
    }
}
