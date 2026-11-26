//! Launch at login through the per-user Run key (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `Vibe Play`): the
//! documented mechanism that needs no helper and no elevation. One value named after the app means enabling twice can never create
//! two entries; it is rewritten on every start when the app was moved, so the login item always points at the installed copy.
//! Windows' own "Startup apps" page lists the entry and can switch it off (that is stored by Windows in `StartupApproved`, which
//! this module never touches).

use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
    KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const VALUE_NAME: &str = "WaitPlay";

/// The command line stored in the Run value: the executable path in double quotes (no arguments: the tray app is the default).
pub fn run_command(exe: &Path) -> String {
    format!("\"{}\"", exe.to_string_lossy())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct Key(HKEY);

impl Key {
    fn open(subkey: &str) -> Result<Key, String> {
        let name = wide(subkey);
        let mut h = HKEY::default();
        // SAFETY: `name` is NUL terminated and outlives the call; `h` receives the opened key
        let r = unsafe { RegCreateKeyExW(HKEY_CURRENT_USER, PCWSTR(name.as_ptr()), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_QUERY_VALUE | KEY_SET_VALUE, None, &mut h, None) };
        if r == ERROR_SUCCESS {
            Ok(Key(h))
        } else {
            Err(format!("cannot open HKCU\\{} (error {})", subkey, r.0))
        }
    }

    fn get(&self, value: &str) -> Option<String> {
        let name = wide(value);
        let mut size = 0u32;
        // SAFETY: first call only asks for the size of the value
        let r = unsafe { RegQueryValueExW(self.0, PCWSTR(name.as_ptr()), None, None, None, Some(&mut size)) };
        if r != ERROR_SUCCESS || size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        // SAFETY: `buf` is `size` bytes
        let r = unsafe { RegQueryValueExW(self.0, PCWSTR(name.as_ptr()), None, None, Some(buf.as_mut_ptr()), Some(&mut size)) };
        if r != ERROR_SUCCESS {
            return None;
        }
        let units: Vec<u16> = buf[..size as usize].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        Some(String::from_utf16_lossy(&units))
    }

    fn set(&self, value: &str, data: &str) -> Result<(), String> {
        let name = wide(value);
        let bytes: Vec<u8> = wide(data).iter().flat_map(|u| u.to_le_bytes()).collect();
        // SAFETY: `name` is NUL terminated; `bytes` is the UTF-16 data including its terminator
        let r = unsafe { RegSetValueExW(self.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(&bytes)) };
        if r == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("cannot write the {} value (error {})", value, r.0))
        }
    }

    fn delete(&self, value: &str) -> Result<bool, String> {
        let name = wide(value);
        // SAFETY: `name` is NUL terminated
        let r = unsafe { RegDeleteValueW(self.0, PCWSTR(name.as_ptr())) };
        if r == ERROR_SUCCESS {
            Ok(true)
        } else if r == ERROR_FILE_NOT_FOUND {
            Ok(false)
        } else {
            Err(format!("cannot remove the {} value (error {})", value, r.0))
        }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from RegCreateKeyExW and is closed once
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

/// Makes the login item at `subkey` match `enabled`. `Ok(changed)`.
fn sync_at(subkey: &str, enabled: bool, exe: &Path) -> Result<bool, String> {
    let key = Key::open(subkey)?;
    if enabled {
        let want = run_command(exe);
        if key.get(VALUE_NAME).as_deref() == Some(want.as_str()) {
            return Ok(false);
        }
        key.set(VALUE_NAME, &want)?;
        Ok(true)
    } else {
        key.delete(VALUE_NAME)
    }
}

pub fn is_enabled() -> bool {
    Key::open(RUN_KEY).map(|k| k.get(VALUE_NAME).is_some()).unwrap_or(false)
}

/// Makes the login item match `enabled`. `Ok(changed)`.
pub fn sync(enabled: bool, exe: &Path) -> Result<bool, String> {
    sync_at(RUN_KEY, enabled, exe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Registry::RegDeleteKeyW;

    #[test]
    fn the_run_value_quotes_the_path_and_has_no_arguments() {
        assert_eq!(run_command(Path::new(r"C:\Program Files\WaitPlay\waitplay.exe")), r#""C:\Program Files\WaitPlay\waitplay.exe""#);
        assert_eq!(RUN_KEY, r"Software\Microsoft\Windows\CurrentVersion\Run");
    }

    #[test]
    fn enable_is_idempotent_follows_a_move_and_disable_removes_the_value() {
        // a scratch key, never the real Run key
        let sub = format!(r"Software\WaitPlayTest\Run-{}", std::process::id());
        let exe = Path::new(r"C:\Apps\WaitPlay\waitplay.exe");
        assert_eq!(sync_at(&sub, true, exe), Ok(true));
        assert_eq!(Key::open(&sub).unwrap().get(VALUE_NAME).as_deref(), Some(r#""C:\Apps\WaitPlay\waitplay.exe""#));
        assert_eq!(sync_at(&sub, true, exe), Ok(false), "no duplicate, no rewrite");
        let moved = Path::new(r"D:\Tools\WaitPlay\waitplay.exe");
        assert_eq!(sync_at(&sub, true, moved), Ok(true), "the entry follows the app");
        assert_eq!(Key::open(&sub).unwrap().get(VALUE_NAME).as_deref(), Some(r#""D:\Tools\WaitPlay\waitplay.exe""#));
        assert_eq!(sync_at(&sub, false, moved), Ok(true));
        assert!(Key::open(&sub).unwrap().get(VALUE_NAME).is_none());
        assert_eq!(sync_at(&sub, false, moved), Ok(false));
        let name = wide(&sub);
        // SAFETY: deletes the scratch key created above
        unsafe {
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(name.as_ptr()));
        }
    }
}
