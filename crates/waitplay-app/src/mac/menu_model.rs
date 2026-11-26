//! The menu bar menu as data. `app.rs` renders it with native NSMenu items; keeping it as a value means the content (which items
//! exist, their text, their checkmarks) is unit tested and the menu is only rebuilt when it actually changes.

use super::status::{Activity, SessionsView, Status};
use waitplay_core::settings::{Settings, SITE_KEYS};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Item { id: String, text: String, enabled: bool, checked: Option<bool> },
    Separator,
    Submenu { text: String, items: Vec<Entry> },
}

fn item(id: &str, text: &str) -> Entry {
    Entry::Item { id: id.into(), text: text.into(), enabled: true, checked: None }
}
fn label(text: &str) -> Entry {
    Entry::Item { id: String::new(), text: text.into(), enabled: false, checked: None }
}
fn check(id: &str, text: &str, on: bool) -> Entry {
    Entry::Item { id: id.into(), text: text.into(), enabled: true, checked: Some(on) }
}

pub fn site_label(key: &str) -> &'static str {
    match key {
        "instagram" => "Instagram",
        "x" => "X",
        "youtube" => "YouTube",
        _ => "",
    }
}

pub fn build(status: Status, title: &str, enabled: bool, view: &SessionsView, detected: &[(String, String)], settings: &Settings) -> Vec<Entry> {
    let mut m = Vec::new();
    let dot = match status {
        Status::Idle => "\u{25cb}",
        Status::Working => "\u{25cf}",
        Status::Entertainment => "\u{25b6}",
        Status::Paused => "\u{23f8}",
        Status::Error => "\u{26a0}",
    };
    m.push(label(&format!("{} {}", dot, title)));
    m.push(Entry::Separator);
    m.push(item("toggle", if enabled { "Disable Automation" } else { "Enable Automation" }));
    m.push(Entry::Separator);

    m.push(label("AI Agents"));
    if detected.is_empty() {
        m.push(label("No agents detected"));
    }
    for (id, name) in detected {
        let activity = view.activity_of(id);
        let text = match activity {
            Activity::Idle => name.clone(),
            Activity::Working => format!("{} \u{2014} working", name),
            Activity::Waiting => format!("{} \u{2014} needs you", name),
        };
        m.push(label(&text));
    }
    m.push(Entry::Separator);

    let enabled_sites: Vec<Entry> = SITE_KEYS
        .iter()
        .filter(|k| settings.platforms.get(k) == Some(true))
        .map(|k| check(&format!("platform:{}", k), site_label(k), settings.preferred_platform == *k))
        .collect();
    let mut sub = enabled_sites;
    sub.push(Entry::Separator);
    sub.push(item("open-entertainment", &format!("Open {} Now", site_label(&settings.preferred_platform))));
    m.push(Entry::Submenu { text: format!("Entertainment: {}", site_label(&settings.preferred_platform)), items: sub });
    m.push(check("widget", "Show Floating Widget", settings.show_widget));
    m.push(Entry::Separator);
    m.push(item("account", "Account\u{2026}"));
    m.push(item("settings", "Settings\u{2026}"));
    m.push(Entry::Separator);
    m.push(item("quit", "Quit Vibe Play"));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(m: &[Entry]) -> Vec<String> {
        m.iter()
            .filter_map(|e| match e {
                Entry::Item { id, .. } if !id.is_empty() => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn automation_item_flips_with_the_state() {
        let s = Settings::default();
        let on = build(Status::Idle, "Idle", true, &SessionsView::default(), &[], &s);
        let off = build(Status::Paused, "Paused", false, &SessionsView::default(), &[], &s);
        let text = |m: &[Entry]| {
            m.iter().find_map(|e| match e {
                Entry::Item { id, text, .. } if id == "toggle" => Some(text.clone()),
                _ => None,
            })
        };
        assert_eq!(text(&on).as_deref(), Some("Disable Automation"));
        assert_eq!(text(&off).as_deref(), Some("Enable Automation"));
        assert_eq!(ids(&on), ["toggle", "widget", "account", "settings", "quit"]);
    }

    #[test]
    fn lists_detected_agents_with_their_activity_and_the_platform_choice() {
        let mut s = Settings::default();
        s.apply_patch(&serde_json::json!({"platforms": {"x": false}, "preferredPlatform": "youtube"}));
        let view = super::super::status::parse_sessions(r#"{"sessions":[{"agent":"claude-code","status":"WORKING","lastEventAt":1000}]}"#, 1000).unwrap();
        let m = build(Status::Working, "Claude Code is working", true, &view, &[("claude-code".into(), "Claude Code".into()), ("codex".into(), "Codex".into())], &s);
        let texts: Vec<String> = m
            .iter()
            .filter_map(|e| match e {
                Entry::Item { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Claude Code \u{2014} working".to_string()));
        assert!(texts.contains(&"Codex".to_string()));
        let Some(Entry::Submenu { text, items }) = m.iter().find(|e| matches!(e, Entry::Submenu { .. })) else { panic!("no submenu") };
        assert_eq!(text, "Entertainment: YouTube");
        assert_eq!(ids(items), ["platform:instagram", "platform:youtube", "open-entertainment"], "disabled X is not offered");
        assert!(matches!(&items[1], Entry::Item { checked: Some(true), .. }), "preferred one is checked");
    }

    #[test]
    fn no_agents_says_so() {
        let m = build(Status::Idle, "Idle", true, &SessionsView::default(), &[], &Settings::default());
        assert!(m.contains(&label("No agents detected")));
    }
}
