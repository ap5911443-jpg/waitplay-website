//! The background signal poller. All file, UI Automation and HTTP work happens here, off the UI thread, so the companion window
//! (and the low-level input hook) never stalls. Port of the C# `SignalPoller`.
//!
//! Hand-off files are per session so terminals never consume or delete each other's signals (see `waitplay_core::signals`).

use super::logic::LaunchRequest;
use crate::platform::{self, TabTracker, WindowId, NO_WINDOW};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};
use waitplay_core::paths;
use waitplay_core::signals::{self, LaunchSignal};
use waitplay_core::{now_ms, state, BridgeClient};

const ACTIVE_INTERVAL: Duration = Duration::from_millis(150);
const PARKED_INTERVAL: Duration = Duration::from_millis(400);
const STALE_LAUNCH_MS: i64 = 15_000;
const QUIT_FRESH_MS: i64 = 15_000;

#[derive(Default)]
pub struct PollResult {
    pub launches: Vec<LaunchRequest>,
    /// (session id, signal time); `i64::MAX` = the terminal window or tab is gone.
    pub closed: Vec<(String, i64)>,
    pub own_tab: Vec<(String, Option<bool>)>,
    pub bridge_json: Option<String>,
    pub quit: bool,
    pub enabled: bool,
}

struct Live {
    hwnd: WindowId,
    tab: Option<Arc<Mutex<TabTracker>>>,
}

struct Shared {
    pending: Mutex<Vec<LaunchRequest>>,
    live: Mutex<HashMap<String, Live>>,
    parked: AtomicBool,
    stop: AtomicBool,
    /// Sleeps the poll interval, but ends the sleep early when a hand-off or quit file appears or the rate changes.
    waker: waker::Waker,
}

pub struct Poller {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Poller {
    /// `owns_signal_files`: this process is the companion that consumes launch/close files; `site_key` picks which launch files.
    pub fn start(site_key: &'static str, owns_signal_files: bool, deliver: Box<dyn Fn(PollResult) + Send>) -> Poller {
        let shared = Arc::new(Shared {
            pending: Mutex::new(Vec::new()),
            live: Mutex::new(HashMap::new()),
            parked: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            waker: waker::Waker::new(&paths::signal_dir()),
        });
        let worker = shared.clone();
        let started_at = now_ms(); // a quit request only counts if it is newer than this
        let thread = thread::Builder::new()
            .name("waitplay-poller".into())
            .spawn(move || run(worker, site_key, owns_signal_files, started_at, deliver))
            .ok();
        Poller { shared, thread }
    }

    pub fn enqueue(&self, r: LaunchRequest) {
        if let Ok(mut p) = self.shared.pending.lock() {
            p.push(r);
        }
    }

    pub fn forget(&self, id: &str) {
        if let Ok(mut l) = self.shared.live.lock() {
            l.remove(&id.to_lowercase());
        }
    }

    /// A parked window polls slowly and skips the bridge; whatever can bring it back switches it to the fast rate.
    pub fn set_parked(&self, parked: bool) {
        if self.shared.parked.swap(parked, Ordering::SeqCst) != parked {
            self.shared.waker.wake();
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.shared.waker.wake();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// An interruptible sleep. On Windows it also watches the signal folder, so a prompt's hand-off file starts the work immediately
/// instead of up to a whole poll interval later; elsewhere it is a plain timed wait that `wake` can cut short.
#[cfg(windows)]
mod waker {
    use std::path::Path;
    use std::time::Duration;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE};
    use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};

    pub struct Waker {
        event: HANDLE,
        change: Option<HANDLE>,
    }
    // SAFETY: kernel handles may be used from any thread
    unsafe impl Send for Waker {}
    unsafe impl Sync for Waker {}

    impl Waker {
        pub fn new(dir: &Path) -> Waker {
            let _ = std::fs::create_dir_all(dir);
            // SAFETY: plain object creation; every handle is closed in Drop
            unsafe {
                let event = CreateEventW(None, false, false, None).unwrap_or_default();
                let change = FindFirstChangeNotificationW(&HSTRING::from(dir.as_os_str()), false, FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE).ok();
                Waker { event, change }
            }
        }
        pub fn wait(&self, timeout: Duration) {
            let ms = timeout.as_millis().min(u32::MAX as u128) as u32;
            // SAFETY: both handles live as long as `self`
            unsafe {
                match self.change {
                    Some(c) => {
                        let r = WaitForMultipleObjects(&[self.event, c], false, ms);
                        if r.0 == 1 {
                            let _ = FindNextChangeNotification(c); // re-arm for the next change
                        }
                    }
                    None => {
                        let _ = WaitForMultipleObjects(&[self.event], false, ms);
                    }
                }
            }
        }
        pub fn wake(&self) {
            // SAFETY: the event handle is valid until Drop
            unsafe {
                let _ = SetEvent(self.event);
            }
        }
    }
    impl Drop for Waker {
        fn drop(&mut self) {
            // SAFETY: closing the handles this struct owns
            unsafe {
                if let Some(c) = self.change.take() {
                    let _ = FindCloseChangeNotification(c);
                }
                let _ = CloseHandle(self.event);
            }
        }
    }
}

#[cfg(not(windows))]
mod waker {
    use std::path::Path;
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    pub struct Waker {
        woken: Mutex<bool>,
        cv: Condvar,
    }
    impl Waker {
        pub fn new(_dir: &Path) -> Waker {
            Waker { woken: Mutex::new(false), cv: Condvar::new() }
        }
        pub fn wait(&self, timeout: Duration) {
            if let Ok(mut w) = self.woken.lock() {
                if !*w {
                    w = self.cv.wait_timeout(w, timeout).map(|(g, _)| g).unwrap_or_else(|e| e.into_inner().0);
                }
                *w = false;
            }
        }
        pub fn wake(&self) {
            if let Ok(mut w) = self.woken.lock() {
                *w = true;
                self.cv.notify_all();
            }
        }
    }
}

fn run(shared: Arc<Shared>, site_key: &'static str, owns: bool, started_at: i64, deliver: Box<dyn Fn(PollResult) + Send>) {
    let client = BridgeClient::new();
    let mut next_bridge = Instant::now();
    while !shared.stop.load(Ordering::SeqCst) {
        let interval = if shared.parked.load(Ordering::SeqCst) { PARKED_INTERVAL } else { ACTIVE_INTERVAL };
        shared.waker.wait(interval);
        if shared.stop.load(Ordering::SeqCst) {
            break;
        }
        let mut r = PollResult { enabled: state::is_enabled(), quit: quit_requested(started_at), ..Default::default() };

        let mut batch: Vec<LaunchRequest> = match shared.pending.lock() {
            Ok(mut p) => std::mem::take(&mut *p),
            Err(_) => Vec::new(),
        };
        if owns {
            batch.extend(read_launch_files(site_key));
        }
        for l in batch {
            if !l.session_id.is_empty() {
                // The prompt was just submitted, so the selected tab right now is the session's tab
                let tab = if l.hwnd != NO_WINDOW { TabTracker::capture(l.hwnd).map(|t| Arc::new(Mutex::new(t))) } else { None };
                if let Ok(mut live) = shared.live.lock() {
                    live.insert(l.session_id.to_lowercase(), Live { hwnd: l.hwnd, tab });
                }
            }
            r.launches.push(l);
        }
        if owns && site_key == crate::sites::DEFAULT_KEY {
            r.closed.extend(read_close_files()); // agent sessions belong to the default site
        }

        let watched: Vec<(String, WindowId, Option<Arc<Mutex<TabTracker>>>)> = match shared.live.lock() {
            Ok(live) => live.iter().map(|(id, l)| (id.clone(), l.hwnd, l.tab.clone())).collect(),
            Err(_) => Vec::new(),
        };
        for (id, hwnd, tab) in watched {
            let (own, gone) = match &tab {
                Some(t) => match t.lock() {
                    Ok(mut t) => (t.own_tab_selected(), t.gone()),
                    Err(_) => (None, false),
                },
                None => (None, false),
            };
            // Terminal window or tab closed: that session can never finish normally
            if (hwnd != NO_WINDOW && !platform::is_window(hwnd)) || gone {
                r.closed.push((id, i64::MAX));
                continue;
            }
            r.own_tab.push((id, own));
        }

        if !shared.parked.load(Ordering::SeqCst) && Instant::now() >= next_bridge {
            // Short timeout and a 5s back-off while the bridge is down (a refused localhost connect is slow on Windows)
            match client.get_sessions(Duration::from_millis(300)) {
                Some(body) => {
                    next_bridge = Instant::now() + Duration::from_secs(1);
                    r.bridge_json = Some(body);
                }
                None => next_bridge = Instant::now() + Duration::from_secs(5),
            }
        }
        deliver(r);
    }
}

/// The widget's "Quit companion" / WaitPlay-off writes this file. A normal close only hides us, so quitting has to be asked for; a
/// clean exit is what lets the browser flush cookies. A stale file can't end a companion that started afterwards.
fn quit_requested(started_at: i64) -> bool {
    let Ok(text) = fs::read_to_string(signals::quit_file_path()) else { return false };
    let Ok(ts) = text.trim().parse::<i64>() else { return false };
    ts > started_at && now_ms() - ts < QUIT_FRESH_MS
}

fn delete_if_old(p: &Path) {
    if let Ok(modified) = fs::metadata(p).and_then(|m| m.modified()) {
        if SystemTime::now().duration_since(modified).map(|d| d.as_secs() > 30).unwrap_or(false) {
            let _ = fs::remove_file(p);
        }
    }
}

fn read_launch_files(site_key: &str) -> Vec<LaunchRequest> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(paths::signal_dir()) else { return out };
    let default = site_key == crate::sites::DEFAULT_KEY;
    let summon = format!(".waitplay-summon-{}.txt", site_key);
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let matches = if default { name.starts_with(".waitplay-launch-") && name.ends_with(".txt") } else { name == summon };
        if !matches {
            continue;
        }
        let path = e.path();
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let Some(sig) = LaunchSignal::parse(&text) else {
            delete_if_old(&path); // mid-write: retry next tick
            continue;
        };
        let _ = fs::remove_file(&path);
        if now_ms() - sig.ts > STALE_LAUNCH_MS {
            continue; // left over from a companion that died
        }
        out.push(LaunchRequest { session_id: sig.session_id, terminal: sig.terminal, ts: sig.ts, hwnd: sig.hwnd as u64 });
    }
    out
}

fn read_close_files() -> Vec<(String, i64)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(paths::signal_dir()) else { return out };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !(name.starts_with(".waitplay-close-") && name.ends_with(".json")) {
            continue;
        }
        let path = e.path();
        let parsed = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).and_then(|v| {
            let id = v.get("sessionId")?.as_str()?.to_string();
            let ts = v.get("ts")?.as_i64()?;
            Some((id, ts))
        });
        match parsed {
            Some(entry) => {
                let _ = fs::remove_file(&path);
                out.push(entry);
            }
            None => delete_if_old(&path),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_temp(tag: &str) -> std::path::PathBuf {
        // The signal dir derives from TEMP/TMPDIR; tests that touch it run serially behind this lock
        let p = std::env::temp_dir().join(format!("wp-poller-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn launch_and_close_files_are_consumed_once() {
        let _g = LOCK.lock().unwrap();
        let root = isolated_temp("files");
        std::env::set_var("TEMP", &root);
        std::env::set_var("TMP", &root);
        std::env::set_var("TMPDIR", &root);
        let dir = paths::ensure_signal_dir();

        LaunchSignal { session_id: "omp-session".into(), terminal: "WT".into(), ts: now_ms(), hwnd: 42 }.write("instagram");
        LaunchSignal { session_id: "".into(), terminal: "Desktop".into(), ts: now_ms(), hwnd: 7 }.write("youtube");
        // a stale one and a half-written one
        fs::write(dir.join(".waitplay-launch-old.txt"), format!("old|WT|{}|1", now_ms() - 60_000)).unwrap();
        fs::write(dir.join(".waitplay-launch-half.txt"), "half|WT|12").unwrap();

        let got = read_launch_files("instagram");
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].session_id.as_str(), got[0].hwnd), ("omp-session", 42));
        assert!(read_launch_files("instagram").is_empty(), "consumed");
        assert!(dir.join(".waitplay-launch-half.txt").exists(), "half-written file is kept for the next tick");
        let yt = read_launch_files("youtube");
        assert_eq!(yt.len(), 1);
        assert_eq!(yt[0].terminal, "Desktop");

        signals::write_close_signal("omp-session");
        let closed = read_close_files();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].0, "omp-session");
        assert!(read_close_files().is_empty());

        assert!(!quit_requested(now_ms() + 1000));
        signals::write_quit_signal();
        assert!(quit_requested(now_ms() - 5000));
        assert!(!quit_requested(now_ms() + 5000), "a quit request older than this companion is ignored");
        let _ = fs::remove_dir_all(&root);
    }
}
