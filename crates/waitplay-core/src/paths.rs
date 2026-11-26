//! Where Vibe Play keeps its small cross-process files, and where its executables live.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `<temp>/waitplay`: hand-off files between hooks, the bridge and the companion app. Every process derives it the same
/// way (`std::env::temp_dir`), so an isolated `TEMP`/`TMPDIR` isolates a whole Vibe Play instance.
pub fn signal_dir() -> PathBuf {
    std::env::temp_dir().join("waitplay")
}

pub fn ensure_signal_dir() -> PathBuf {
    let d = signal_dir();
    let _ = fs::create_dir_all(&d);
    d
}

/// Port of the bridge (`BRIDGE_PORT`, default 3150).
pub fn bridge_port() -> u16 {
    std::env::var("BRIDGE_PORT")
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(3150)
}

fn looks_like_repo(dir: &Path) -> bool {
    dir.join("package.json").is_file() && dir.join("integrations").is_dir()
}

/// The Vibe Play checkout: `WAITPLAY_HOME`, else the nearest ancestor of the running executable that looks like the repo.
pub fn repo_root() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("WAITPLAY_HOME") {
        let p = PathBuf::from(h.trim());
        if p.is_dir() {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let mut cur = exe.parent();
    while let Some(dir) = cur {
        if looks_like_repo(dir) {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}

/// An executable that ships next to the running one (`waitplay`, `waitplay-bridge`, `waitplay-hook`).
pub fn sibling_exe(name: &str) -> PathBuf {
    let file = format!("{}{}", name, std::env::consts::EXE_SUFFIX);
    match std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        Some(dir) => dir.join(file),
        None => PathBuf::from(file),
    }
}

/// Path of the companion app binary, if it exists.
pub fn app_exe() -> Option<PathBuf> {
    let p = sibling_exe("waitplay");
    if p.is_file() {
        return Some(p);
    }
    let root = repo_root()?;
    for profile in ["release", "debug"] {
        let c = root.join("target").join(profile).join(format!("waitplay{}", std::env::consts::EXE_SUFFIX));
        if c.is_file() {
            return Some(c);
        }
    }
    None
}

/// Writes through a temp file and renames, so a reader never sees half a file.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&tmp, contents)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Per-user data folder: `%LOCALAPPDATA%\WaitPlay`, `~/Library/Application Support/WaitPlay`, else `$XDG_DATA_HOME/WaitPlay`.
/// `WAITPLAY_CONFIG_DIR` overrides it (tests, portable installs).
pub fn app_support_dir() -> PathBuf {
    if let Some(over) = std::env::var_os("WAITPLAY_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(over);
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").filter(|h| !h.is_empty()).map(|h| PathBuf::from(h).join("Library").join("Application Support"));
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").filter(|h| !h.is_empty()).map(|h| PathBuf::from(h).join(".local").join("share")));
    base.unwrap_or_else(std::env::temp_dir).join("WaitPlay")
}

/// Where log files go: `~/Library/Logs/WaitPlay` on macOS, `<app support>/logs` elsewhere.
pub fn log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(h) = std::env::var_os("HOME").filter(|h| !h.is_empty()) {
            if std::env::var_os("WAITPLAY_CONFIG_DIR").map_or(true, |v| v.is_empty()) {
                return PathBuf::from(h).join("Library").join("Logs").join("WaitPlay");
            }
        }
    }
    app_support_dir().join("logs")
}
