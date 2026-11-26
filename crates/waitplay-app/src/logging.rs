#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
//! Structured log lines in a macOS-appropriate place (`~/Library/Logs/WaitPlay/waitplay.log`, `<app support>/logs` elsewhere).
//!
//! `2026-11-23T10:15:02.123Z INFO  app: widget placed top-right`. Every Vibe Play process appends to the same file (single
//! `write` calls in append mode are not interleaved mid-line). The file rotates to `waitplay.log.1` at 1 MiB, so it never grows
//! without bound. Logging never fails the caller.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 1024 * 1024;
const FILE_NAME: &str = "waitplay.log";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn tag(self) -> &'static str {
        match self {
            Level::Info => "INFO ",
            Level::Warn => "WARN ",
            Level::Error => "ERROR",
        }
    }
}

static COMPONENT: OnceLock<String> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Names this process in every line (`app`, `companion`, ...) and turns logging on. Call once at start.
pub fn init(component: &str) {
    let _ = COMPONENT.set(component.to_string());
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn log_path() -> PathBuf {
    waitplay_core::paths::log_dir().join(FILE_NAME)
}

/// `1700000000123` -> `2023-11-14T22:13:20.123Z` (civil-from-days, proleptic Gregorian, UTC).
pub fn format_ts(unix_ms: i64) -> String {
    let secs = unix_ms.div_euclid(1000);
    let ms = unix_ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z", y, m, d, rem / 3600, (rem % 3600) / 60, rem % 60, ms)
}

pub fn format_line(unix_ms: i64, level: Level, component: &str, msg: &str) -> String {
    // one line per record: a newline in the message would forge a second record
    let msg: String = msg.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
    format!("{} {} {}: {}\n", format_ts(unix_ms), level.tag(), component, msg)
}

pub fn log(level: Level, msg: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
        return; // processes that never called `init` (the Windows build) stay silent
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let line = format_line(now, level, COMPONENT.get().map(String::as_str).unwrap_or("waitplay"), msg);
    let _ = std::io::stderr().write_all(line.as_bytes());
    let _guard = WRITE_LOCK.lock();
    let path = log_path();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if fs::metadata(&path).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
        let _ = fs::rename(&path, path.with_extension("log.1"));
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

#[macro_export]
macro_rules! wp_info {
    ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Info, &format!($($arg)*)) };
}
#[macro_export]
macro_rules! wp_warn {
    ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Warn, &format!($($arg)*)) };
}
#[macro_export]
macro_rules! wp_error {
    ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Error, &format!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_iso_utc() {
        assert_eq!(format_ts(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(format_ts(1_700_000_000_123), "2023-11-14T22:13:20.123Z");
        assert_eq!(format_ts(951_782_400_000), "2000-02-29T00:00:00.000Z", "leap day");
        assert_eq!(format_ts(-1), "1969-12-31T23:59:59.999Z");
    }

    #[test]
    fn a_record_is_exactly_one_line() {
        let l = format_line(0, Level::Warn, "app", "a\nb\r\nc");
        assert_eq!(l, "1970-01-01T00:00:00.000Z WARN  app: a b  c\n");
        assert_eq!(l.matches('\n').count(), 1);
    }
}
