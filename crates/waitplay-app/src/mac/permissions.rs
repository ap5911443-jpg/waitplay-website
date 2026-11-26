//! The permission screen's content. Vibe Play's design needs no privacy permission: the active application comes from NSWorkspace,
//! agent events travel over loopback, and nothing reads the screen or the keyboard. This list says so explicitly (and links to the
//! pane where the one user-controllable item, Login Items, lives) instead of asking for anything it does not use.

use serde_json::{json, Value};

pub fn pane_url(pane: &str) -> Option<&'static str> {
    match pane {
        "loginItems" => Some("x-apple.systempreferences:com.apple.LoginItems-Settings.extension"),
        "accessibility" => Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"),
        "screenRecording" => Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"),
        _ => None,
    }
}

pub fn list(launch_at_login: bool) -> Value {
    json!([
        {
            "id": "loginItems",
            "title": "Login Items",
            "why": "Lets Vibe Play start by itself when you sign in to your Mac.",
            "affects": "Launch at login. If you switch Vibe Play off under Login Items in System Settings, it will not start automatically.",
            "state": if launch_at_login { "unknown" } else { "notRequired" },
            "canOpen": true,
            "pane": "loginItems"
        },
        {
            "id": "accessibility",
            "title": "Accessibility",
            "why": "Vibe Play does not read your keystrokes or control other apps, so it does not ask for this.",
            "affects": "Nothing. Vibe Play finds your terminal from the active application.",
            "state": "notRequired",
            "canOpen": false,
            "pane": "accessibility"
        },
        {
            "id": "screenRecording",
            "title": "Screen Recording",
            "why": "Vibe Play never captures your screen, so it does not ask for this.",
            "affects": "Nothing.",
            "state": "notRequired",
            "canOpen": false,
            "pane": "screenRecording"
        },
        {
            "id": "network",
            "title": "Network",
            "why": "Coding agents report to Vibe Play over 127.0.0.1, inside your Mac. The entertainment sites use your normal internet connection.",
            "affects": "Nothing leaves your Mac except the sites you open.",
            "state": "notRequired",
            "canOpen": false,
            "pane": ""
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_panes_open() {
        assert!(pane_url("loginItems").unwrap().starts_with("x-apple.systempreferences:"));
        assert!(pane_url("../../etc/passwd").is_none());
        assert!(pane_url("").is_none());
    }

    #[test]
    fn every_entry_has_the_fields_the_page_reads() {
        for p in list(true).as_array().unwrap() {
            for k in ["id", "title", "why", "affects", "state", "canOpen", "pane"] {
                assert!(p.get(k).is_some(), "{} missing in {}", k, p);
            }
        }
        assert_eq!(list(true)[0]["state"], "unknown");
        assert_eq!(list(false)[0]["state"], "notRequired");
    }
}
