//! "Quit companion (free RAM)" and "Vibe Play turned off": port of `CloseOtherGameWindows`.
//!
//! Companions park themselves on every close request, so ending them takes an explicit ask: the quit file, read by each
//! companion's poller. A clean exit lets the browser flush cookies (a sign-in made seconds ago survives) and takes the browser
//! with it. Whatever is still running after 5 s is killed, and so is any browser left behind by a killed companion.
//!
//! Which processes are companions: other processes running THIS executable (same full image path, taken from one process
//! snapshot, never the widget itself). An image name alone would also match another Vibe Play build living elsewhere.

use std::collections::{HashMap, HashSet};

/// One row of a process snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    /// Image file name, lower-cased (`waitplay.exe`).
    pub name: String,
}

/// Every descendant of any of `roots` (the roots themselves excluded), in the snapshot.
pub fn descendants(procs: &[Proc], roots: &[u32]) -> Vec<u32> {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in procs {
        kids.entry(p.ppid).or_default().push(p.pid);
    }
    let mut seen: HashSet<u32> = roots.iter().copied().collect();
    let mut out = Vec::new();
    let mut queue: Vec<u32> = roots.to_vec();
    while let Some(pid) = queue.pop() {
        for &k in kids.get(&pid).into_iter().flatten() {
            if seen.insert(k) {
                out.push(k);
                queue.push(k);
            }
        }
    }
    out
}

/// Runs the whole sequence on a background thread; the widget stays responsive.
pub fn quit_companions() {
    let _ = std::thread::Builder::new().name("wp-quit-companions".into()).spawn(imp::run);
}

#[cfg(windows)]
mod imp {
    use super::{descendants, Proc};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use waitplay_core::signals;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    const BROWSER: &str = "msedgewebview2.exe";

    fn snapshot() -> Vec<Proc> {
        let mut out = Vec::new();
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
            let mut pe = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            if Process32FirstW(snap, &mut pe).is_ok() {
                loop {
                    let n = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
                    out.push(Proc {
                        pid: pe.th32ProcessID,
                        ppid: pe.th32ParentProcessID,
                        name: String::from_utf16_lossy(&pe.szExeFile[..n]).to_lowercase(),
                    });
                    if Process32NextW(snap, &mut pe).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }
        out
    }

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    fn open(pid: u32, access: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS) -> Option<Handle> {
        unsafe { OpenProcess(access, false, pid) }.ok().map(Handle)
    }

    fn image_path(pid: u32) -> Option<String> {
        let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        unsafe { QueryFullProcessImageNameW(h.0, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len) }.ok()?;
        Some(norm(&String::from_utf16_lossy(&buf[..len as usize])))
    }

    fn norm(p: &str) -> String {
        p.trim_start_matches(r"\\?\").replace('/', "\\").to_lowercase()
    }

    fn start_time(pid: u32) -> Option<u64> {
        let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
        let (mut c, mut e, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe { GetProcessTimes(h.0, &mut c, &mut e, &mut k, &mut u) }.ok()?;
        Some(((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64)
    }

    /// A browser process recorded in the snapshot, still the same process (the PID might have been recycled since).
    fn is_same_browser(pid: u32, started: u64) -> bool {
        snapshot().iter().any(|p| p.pid == pid && p.name == BROWSER) && start_time(pid) == Some(started)
    }

    pub fn run() {
        let me = std::process::id();
        let Some(own_path) = std::env::current_exe().ok().map(PathBuf::into_os_string).and_then(|s| s.into_string().ok()).map(|s| norm(&s)) else {
            return;
        };
        let own_name = own_path.rsplit('\\').next().unwrap_or("").to_string();

        let procs = snapshot();
        let others: Vec<u32> = procs
            .iter()
            .filter(|p| p.pid != me && p.name == own_name && image_path(p.pid).as_deref() == Some(own_path.as_str()))
            .map(|p| p.pid)
            .collect();
        if others.is_empty() {
            return;
        }
        // their browsers (and anything else they started), remembered now: after a kill the parent links are gone
        let trees: Vec<(u32, u64)> = descendants(&procs, &others)
            .into_iter()
            .filter(|pid| procs.iter().any(|p| p.pid == *pid && p.name == BROWSER))
            .filter_map(|pid| start_time(pid).map(|t| (pid, t)))
            .collect();

        signals::write_quit_signal();
        let deadline = Instant::now() + Duration::from_secs(5);
        for pid in &others {
            let Some(h) = open(*pid, PROCESS_SYNCHRONIZE | PROCESS_TERMINATE) else { continue };
            let left = deadline.saturating_duration_since(Instant::now()).as_millis() as u32;
            if unsafe { WaitForSingleObject(h.0, left) } != WAIT_OBJECT_0 {
                unsafe {
                    let _ = TerminateProcess(h.0, 1);
                }
            }
        }
        let _ = std::fs::remove_file(signals::quit_file_path());

        // a clean exit takes the browser along within a moment; give it up to 10 s to finish writing before ending leftovers
        for _ in 0..40 {
            if !trees.iter().any(|(pid, t)| is_same_browser(*pid, *t)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        for (pid, t) in &trees {
            if is_same_browser(*pid, *t) {
                if let Some(h) = open(*pid, PROCESS_TERMINATE) {
                    unsafe {
                        let _ = TerminateProcess(h.0, 1);
                    }
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    /// Without a process enumerator here the quit file is the whole mechanism: every companion exits cleanly on it. Stuck
    /// ones are not force-killed (known gap on this platform).
    pub fn run() {
        waitplay_core::signals::write_quit_signal();
        std::thread::sleep(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_file(waitplay_core::signals::quit_file_path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, ppid: u32, name: &str) -> Proc {
        Proc { pid, ppid, name: name.into() }
    }

    #[test]
    fn descendants_walk_every_level_and_skip_the_roots() {
        let procs = vec![
            p(10, 1, "waitplay.exe"),
            p(11, 10, "msedgewebview2.exe"),
            p(12, 11, "msedgewebview2.exe"),
            p(13, 11, "msedgewebview2.exe"),
            p(20, 1, "waitplay.exe"),
            p(21, 20, "msedgewebview2.exe"),
            p(30, 1, "explorer.exe"),
            p(31, 30, "msedgewebview2.exe"),
        ];
        let mut d = descendants(&procs, &[10, 20]);
        d.sort();
        assert_eq!(d, vec![11, 12, 13, 21]);
        assert!(descendants(&procs, &[31]).is_empty());
    }

    #[test]
    fn recycled_parent_links_cannot_loop() {
        // 5 -> 6 -> 5 (a reused PID closing a cycle)
        let procs = vec![p(5, 6, "a.exe"), p(6, 5, "b.exe")];
        let mut d = descendants(&procs, &[5]);
        d.sort();
        assert_eq!(d, vec![6]);
    }
}
