//! The permission screen's content for Windows. Windows has no per-app privacy prompts for what Vibe Play does: agent events travel
//! over loopback, nothing captures the screen, and the browser engine asks the user itself before any site may use a camera or
//! microphone. This list says so explicitly (and links to the "Startup apps" page where the one user-controllable item, launch at
//! login, can be switched off) instead of asking for anything it does not use.
//!
//! Only the panes in [`pane_url`] can be opened; the page sends a pane id, never a URL.

use serde_json::{json, Value};

pub fn pane_url(pane: &str) -> Option<&'static str> {
    match pane {
        "startupApps" => Some("ms-settings:startupapps"),
        _ => None,
    }
}

pub fn list(launch_at_login: bool) -> Value {
    json!([
        {
            "id": "startupApps",
            "title": "Startup apps",
            "why": "Lets Vibe Play start by itself when you sign in to Windows.",
            "affects": "Launch at login. If you switch Vibe Play off under Startup apps in Windows Settings, it will not start automatically.",
            "state": if launch_at_login { "unknown" } else { "notRequired" },
            "canOpen": true,
            "pane": "startupApps"
        },
        {
            "id": "input",
            "title": "Keyboard and mouse",
            "why": "Vibe Play does not record what you type. While the entertainment window is open it only notices that you are typing or scrolling in the terminal that started the agent, so it can step aside.",
            "affects": "Nothing is stored or sent anywhere.",
            "state": "notRequired",
            "canOpen": false,
            "pane": ""
        },
        {
            "id": "screenCapture",
            "title": "Screen capture",
            "why": "Vibe Play never captures your screen, so it does not ask for this.",
            "affects": "Nothing.",
            "state": "notRequired",
            "canOpen": false,
            "pane": ""
        },
        {
            "id": "network",
            "title": "Network",
            "why": "Coding agents report to Vibe Play over 127.0.0.1, inside your PC. The entertainment sites use your normal internet connection.",
            "affects": "Nothing leaves your PC except the sites you open.",
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
        assert_eq!(pane_url("startupApps"), Some("ms-settings:startupapps"));
        assert!(pane_url("https://example.com").is_none());
        assert!(pane_url("ms-settings:privacy-webcam").is_none());
        assert!(pane_url("..\\..\\windows\\system32\\calc.exe").is_none());
        assert!(pane_url("").is_none());
    }

    #[test]
    fn every_entry_has_the_fields_the_page_reads() {
        for p in list(true).as_array().unwrap() {
            for k in ["id", "title", "why", "affects", "state", "canOpen", "pane"] {
                assert!(p.get(k).is_some(), "{} missing in {}", k, p);
            }
            if p["canOpen"] == true {
                assert!(pane_url(p["pane"].as_str().unwrap()).is_some(), "an openable entry must name a whitelisted pane: {}", p);
            }
        }
        assert_eq!(list(true)[0]["state"], "unknown");
        assert_eq!(list(false)[0]["state"], "notRequired");
    }
}
