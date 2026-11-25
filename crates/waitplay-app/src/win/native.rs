//! The Win32 pieces of the tray app that have no cross-platform counterpart: the glass of the entertainment window, the notch's shape, the settings window's
//! frame colours, opening the few whitelisted system pages, revealing the log file, and the job object that ends the helper
//! processes with the app.

use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::Path;
use windows::core::{w, BOOL, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, COLORREF, HANDLE, HWND};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_DONOTROUND, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, HGDIOBJ};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNORMAL};

fn hwnd_of(raw: isize) -> HWND {
    HWND(raw as *mut std::ffi::c_void)
}

// ── glass ────────────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The real behind-window blur of the whole entertainment window (header strip and the frame around the site; the site covers the
/// middle): DWM blur-behind over a region larger than any window (a blur without a region is only transparent, not blurred). Black
/// GDI pixels of the window show the blur through, which is why the companion paints its bare window black. Unlike the Windows 11
/// backdrop materials this keeps working while the window is inactive; Windows turns it off with "Transparency effects".
pub fn window_glass(hwnd: isize) {
    // SAFETY: the region is created here, copied by DWM during the call and deleted right after
    unsafe {
        let rgn = CreateRectRgn(0, 0, 16384, 16384);
        let bb = DWM_BLURBEHIND { dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION, fEnable: BOOL(1), hRgnBlur: rgn, fTransitionOnMaximized: BOOL(0) };
        let _ = DwmEnableBlurBehindWindow(hwnd_of(hwnd), &bb);
        let _ = DeleteObject(HGDIOBJ(rgn.0));
    }
}

/// The window shape of the notch. The page draws an opaque black pill on a transparent window; this clips the window itself to
/// that pill (a hair larger, so the page's antialiased edge is never cut) which is what makes everything around the pill
/// click-through: a browser tab or a button under the empty corners of the widget window still gets the mouse. The page reports
/// its live rectangle every frame while it grows or shrinks, so the clip follows the animation.
pub struct NotchShape {
    hwnd: HWND,
    last: Option<(i32, i32, i32, i32, i32)>,
}

impl NotchShape {
    pub fn attach(hwnd: isize) -> NotchShape {
        let h = hwnd_of(hwnd);
        // The pill has its own shape: no system rounding or frame for this window
        let pref = DWMWCP_DONOTROUND;
        // SAFETY: the pointer is to a live local of the size the attribute expects
        unsafe {
            let _ = DwmSetWindowAttribute(h, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as *const _, std::mem::size_of_val(&pref) as u32);
        }
        NotchShape { hwnd: h, last: None }
    }

    /// `x`,`y`,`w`,`h`: the pill in CSS px relative to the page; `radius`: its corner radius; `scale`: device pixels per CSS px.
    pub fn set_shape(&mut self, x: f64, y: f64, w: f64, h: f64, radius: f64, scale: f64) {
        const GROW: f64 = 1.5; // CSS px
        let (l, t) = (((x - GROW) * scale).floor() as i32, ((y - GROW) * scale).floor() as i32);
        let (r, b) = (((x + w + GROW) * scale).ceil() as i32, ((y + h + GROW) * scale).ceil() as i32);
        let d = (((radius + GROW).min((w + 2.0 * GROW) / 2.0).min((h + 2.0 * GROW) / 2.0) * 2.0) * scale).round() as i32;
        let key = (l, t, r, b, d);
        if self.last == Some(key) || r <= l || b <= t {
            return;
        }
        // SAFETY: after a successful call the window owns the region (it must not be deleted here)
        unsafe {
            let rgn = CreateRoundRectRgn(l, t, r, b, d, d);
            if SetWindowRgn(self.hwnd, Some(rgn), true) != 0 {
                self.last = Some(key);
            } else {
                let _ = DeleteObject(HGDIOBJ(rgn.0));
            }
        }
    }
}

/// Raises the window to the top of the always-on-top band without activating it (the widget stays above the entertainment window).
pub fn bring_to_top(hwnd: isize) {
    // SAFETY: plain window-position call on a live window
    unsafe {
        let _ = SetWindowPos(hwnd_of(hwnd), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// True while a fullscreen game, a fullscreen video or a borderless fullscreen window is in front: the foreground window covers its
/// whole monitor (taskbar included) and has no title bar. A normal maximized window stops at the taskbar, or keeps its title bar
/// when the taskbar hides itself, so it never counts; neither do the desktop, the taskbar or WaitPlay's own windows. The notch hides
/// for that time instead of floating over it.
pub fn fullscreen_app_active() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GWL_STYLE};
    // SAFETY: plain queries about the foreground window; every out-pointer is to a live local of the right type
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool() {
            return false;
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(fg, &mut class).max(0) as usize;
        if matches!(String::from_utf16_lossy(&class[..n]).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() {
            return false;
        }
        let m = mi.rcMonitor;
        covers_screen_without_title_bar((r.left, r.top, r.right, r.bottom), (m.left, m.top, m.right, m.bottom), GetWindowLongW(fg, GWL_STYLE) as u32)
    }
}

/// The rule behind [`fullscreen_app_active`], on plain numbers: the window rectangle contains the monitor rectangle and the window
/// style lacks the title bar (`WS_CAPTION` = `WS_BORDER | WS_DLGFRAME`).
fn covers_screen_without_title_bar(window: (i32, i32, i32, i32), monitor: (i32, i32, i32, i32), style: u32) -> bool {
    const WS_CAPTION: u32 = 0x00C0_0000;
    window.0 <= monitor.0 && window.1 <= monitor.1 && window.2 >= monitor.2 && window.3 >= monitor.3 && style & WS_CAPTION != WS_CAPTION
}

// ── settings window frame ────────────────────────────────────────────────────────────────────────────────────────────────

/// Caption bar, border and caption text in the settings page's own colours (Windows 11; older Windows keeps its default frame).
pub fn style_frame(hwnd: isize, dark: bool) {
    let colorref = |r: u32, g: u32, b: u32| COLORREF(r | (g << 8) | (b << 16));
    // the page's `--bg` and `--label`
    let (bg, border, text) = if dark { (colorref(0x1e, 0x1e, 0x1e), colorref(0x28, 0x28, 0x2a), colorref(0xf5, 0xf5, 0xf7)) } else { (colorref(0xf5, 0xf5, 0xf7), colorref(0xeb, 0xeb, 0xed), colorref(0x1d, 0x1d, 0x1f)) };
    let h = hwnd_of(hwnd);
    for (attr, value) in [(DWMWA_CAPTION_COLOR, bg), (DWMWA_BORDER_COLOR, border), (DWMWA_TEXT_COLOR, text)] {
        // SAFETY: the pointer is to a live COLORREF
        unsafe {
            let _ = DwmSetWindowAttribute(h, attr, &value as *const _ as *const _, std::mem::size_of::<COLORREF>() as u32);
        }
    }
}

// ── opening things ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// Opens `target` with its registered handler. Callers pass only compile-time constants (the two project URLs and the
/// `ms-settings:` pages of `permissions::pane_url`), never text from a page.
pub fn open_constant(target: &'static str) -> Result<(), String> {
    let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: both strings are NUL terminated and outlive the call
    let r = unsafe { ShellExecuteW(None, w!("open"), PCWSTR(wide.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    // ShellExecuteW reports success with a value greater than 32
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Windows could not open {} (code {})", target, r.0 as isize))
    }
}

/// Shows `path` selected in Explorer; when the file does not exist yet, opens its folder.
pub fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    let mut cmd = std::process::Command::new("explorer.exe");
    if path.is_file() {
        // explorer wants `/select,"path"` as ONE argument with the quotes kept
        cmd.raw_arg(format!("/select,\"{}\"", path.display()));
    } else {
        let dir = path.parent().unwrap_or(path);
        let _ = std::fs::create_dir_all(dir);
        cmd.arg(dir);
    }
    cmd.spawn().map(|mut c| {
        std::thread::spawn(move || {
            let _ = c.wait();
        });
    })
    .map_err(|e| format!("cannot start explorer.exe: {}", e))
}

// ── helper processes ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A job object that kills everything assigned to it when the app's last handle to it closes (the app quits OR crashes): the bridge
/// and the agent watchers the app started never outlive it, so a crashed app cannot leave duplicate watchers behind.
pub struct KillOnClose(HANDLE);

// SAFETY: a job handle is just a kernel object reference, usable from any thread
unsafe impl Send for KillOnClose {}

impl KillOnClose {
    pub fn new() -> Option<KillOnClose> {
        // SAFETY: plain job object creation and configuration with a correctly sized, initialised struct
        unsafe {
            let job = CreateJobObjectW(None, PCWSTR::null()).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(job, JobObjectExtendedLimitInformation, &info as *const _ as *const _, std::mem::size_of_val(&info) as u32).is_ok();
            if !ok {
                let _ = CloseHandle(job);
                return None;
            }
            Some(KillOnClose(job))
        }
    }

    pub fn assign(&self, child: &std::process::Child) -> bool {
        // SAFETY: both handles are live; the child handle is owned by `child`
        unsafe { AssignProcessToJobObject(self.0, HANDLE(child.as_raw_handle())).is_ok() }
    }
}

impl Drop for KillOnClose {
    fn drop(&mut self) {
        // SAFETY: closing the handle we created; this is what ends the assigned processes
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_borderless_window_covering_the_monitor_counts_as_fullscreen() {
        let monitor = (0, 0, 1920, 1080);
        let borderless = 0x9000_0000u32; // WS_POPUP | WS_VISIBLE
        let titled = 0x16CF_0000u32; // a normal overlapped window
        assert!(covers_screen_without_title_bar((0, 0, 1920, 1080), monitor, borderless), "a game or F11 video");
        assert!(covers_screen_without_title_bar((-8, -8, 1928, 1088), monitor, borderless), "a few pixels over the edge still covers it");
        assert!(!covers_screen_without_title_bar((-8, -8, 1928, 1048), monitor, titled), "maximized: stops at the taskbar");
        assert!(!covers_screen_without_title_bar((-8, -8, 1928, 1088), monitor, titled), "maximized with a hidden taskbar keeps its title bar");
        assert!(!covers_screen_without_title_bar((100, 100, 900, 700), monitor, borderless), "a small window");
    }

    #[test]
    fn a_job_object_can_be_created_and_takes_a_child() {
        let job = KillOnClose::new().expect("job object");
        let started = std::time::Instant::now();
        let mut child = std::process::Command::new("cmd.exe").args(["/c", "ping -n 30 127.0.0.1 >nul"]).creation_flags(0x0800_0000).spawn().unwrap();
        assert!(job.assign(&child));
        drop(job); // closing the job ends the child
        let _ = child.wait().unwrap();
        // the ping would run for ~30 s; the exit code of a process that the OS terminated is not specified, the time is
        assert!(started.elapsed() < std::time::Duration::from_secs(15), "the child outlived the job it was assigned to");
    }
}
