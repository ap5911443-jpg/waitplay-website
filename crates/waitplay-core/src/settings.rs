//! User preferences (`settings.json` in the app-support folder), shared by the menu-bar app and the companion windows.
//!
//! One code path reads every change: [`Settings::apply_patch`] takes a (possibly partial) JSON object, validates each field on its
//! own, keeps the old value for anything invalid and reports what it rejected. Loading the file is the same call on top of the
//! defaults, so a hand-edited or older file can never stop the app from starting.

use crate::paths;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

pub const SITE_KEYS: [&str; 3] = ["instagram", "x", "youtube"];
pub const DEFAULT_TOGGLE_AUTOMATION: &str = "Control+Alt+KeyW";
pub const DEFAULT_TOGGLE_WIDGET: &str = "Control+Alt+KeyH";

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? } default $default:ident) => {
        #[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
        pub enum $name { $( #[serde(rename = $text)] $variant ),+ }
        impl $name {
            pub fn as_str(self) -> &'static str { match self { $( $name::$variant => $text ),+ } }
            pub fn parse(s: &str) -> Option<$name> { match s { $( $text => Some($name::$variant), )+ _ => None } }
        }
        impl Default for $name { fn default() -> Self { $name::$default } }
    };
}

string_enum!(Appearance { System => "system", Light => "light", Dark => "dark" } default System);
string_enum!(WidgetPosition { TopRight => "top-right", TopCenter => "top-center", BottomRight => "bottom-right", BottomLeft => "bottom-left" } default BottomRight);
/// Where a fresh install puts the widget: at the top centre of the screen, which on a MacBook is the camera notch and on Windows the
/// notch-style island. Existing installs keep the position they chose.
pub const DEFAULT_WIDGET_POSITION: WidgetPosition = if cfg!(any(windows, target_os = "macos")) { WidgetPosition::TopCenter } else { WidgetPosition::BottomRight };
string_enum!(MascotIntensity { Subtle => "subtle", Playful => "playful" } default Subtle);
string_enum!(OnFinish { Hide => "hide", Close => "close" } default Hide);

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Platforms {
    pub instagram: bool,
    pub x: bool,
    pub youtube: bool,
}

impl Platforms {
    pub fn get(&self, key: &str) -> Option<bool> {
        match key {
            "instagram" => Some(self.instagram),
            "x" => Some(self.x),
            "youtube" => Some(self.youtube),
            _ => None,
        }
    }
    fn set(&mut self, key: &str, on: bool) {
        match key {
            "instagram" => self.instagram = on,
            "x" => self.x = on,
            "youtube" => self.youtube = on,
            _ => {}
        }
    }
    pub fn first_enabled(&self) -> Option<&'static str> {
        SITE_KEYS.iter().copied().find(|k| self.get(k) == Some(true))
    }
}

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Shortcuts {
    pub toggle_automation: String,
    pub toggle_widget: String,
}

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub launch_at_login: bool,
    pub auto_start_automation: bool,
    pub show_widget: bool,
    pub show_dock_icon: bool,
    pub appearance: Appearance,
    pub widget_position: WidgetPosition,
    pub widget_compact: bool,
    pub widget_show_status: bool,
    pub platforms: Platforms,
    pub preferred_platform: String,
    pub on_finish: OnFinish,
    pub shortcuts: Shortcuts,
    /// Agents whose watcher process Vibe Play runs (`opencode`, `codex`, ...). Internal: the settings page changes it through the
    /// install/remove actions, not through `set`.
    pub watchers: Vec<String>,
    /// Island notifications (toasts) on/off and how long one stays, in seconds.
    pub notifications: bool,
    pub notification_secs: u32,
    /// Sounds: a global mute, volume 0 to 100, which sound plays, and which events play it.
    pub sound_muted: bool,
    pub sound_volume: u32,
    pub sound_name: String,
    /// File name of the user's own sound inside the app-support `sounds` folder (`custom.wav`, ...); empty when there is none.
    /// Internal: the app sets it when the user picks a file, the page cannot point it anywhere else.
    pub sound_custom_file: String,
    pub sound_events: SoundEvents,
    /// The animated companion in the island: on/off, how lively, idle animations, pointer eyes, and a manual reduce-motion switch
    /// (the system's reduce-motion setting is honoured by the page on top of this).
    pub mascot: bool,
    pub mascot_intensity: MascotIntensity,
    pub mascot_idle: bool,
    pub mascot_pointer: bool,
    pub reduce_motion: bool,
}

/// Built-in sounds (synthesized by the app, so nothing to license): `classic` is the system alert sound on a Mac.
pub const SOUND_NAMES: [&str; 6] = ["soft", "chime", "retro", "gentle", "futuristic", "classic"];
pub const CUSTOM_SOUND_EXTENSIONS: [&str; 6] = ["wav", "aiff", "aif", "caf", "mp3", "m4a"];

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SoundEvents {
    pub started: bool,
    pub completed: bool,
    pub all_done: bool,
    pub error: bool,
    pub opened: bool,
}

/// `custom.<ext>` with a supported extension: the only file name `sound_custom_file` may hold.
pub fn valid_custom_sound_file(s: &str) -> bool {
    s.is_empty() || s.strip_prefix("custom.").is_some_and(|e| CUSTOM_SOUND_EXTENSIONS.contains(&e))
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_at_login: false,
            auto_start_automation: true,
            show_widget: true,
            show_dock_icon: false,
            appearance: Appearance::System,
            widget_position: DEFAULT_WIDGET_POSITION,
            widget_compact: false,
            widget_show_status: true,
            platforms: Platforms { instagram: true, x: true, youtube: true },
            preferred_platform: "instagram".into(),
            on_finish: OnFinish::Hide,
            shortcuts: Shortcuts { toggle_automation: DEFAULT_TOGGLE_AUTOMATION.into(), toggle_widget: DEFAULT_TOGGLE_WIDGET.into() },
            watchers: Vec::new(),
            notifications: true,
            notification_secs: 5,
            sound_muted: false,
            sound_volume: 70,
            sound_name: "gentle".into(),
            sound_custom_file: String::new(),
            sound_events: SoundEvents { started: false, completed: true, all_done: false, error: false, opened: false },
            mascot: true,
            mascot_intensity: MascotIntensity::Subtle,
            mascot_idle: true,
            mascot_pointer: true,
            reduce_motion: false,
        }
    }
}

/// A shortcut is `Modifier+Modifier+Code` made of letters and digits only (what `KeyboardEvent.code` produces).
pub fn valid_shortcut_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 48 && s.split('+').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

impl Settings {
    pub fn path() -> PathBuf {
        paths::app_support_dir().join("settings.json")
    }

    /// The saved settings; defaults for a missing file and for every field that is missing or invalid.
    pub fn load() -> Settings {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &std::path::Path) -> Settings {
        let mut s = Settings::default();
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                let _ = s.apply_patch(&v);
            }
        }
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&Self::path())
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        let body = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        paths::write_atomic(path, &body)
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Applies the fields present in `patch`. Returns the names of the fields that were rejected (they keep their old value).
    pub fn apply_patch(&mut self, patch: &Value) -> Vec<String> {
        let mut rejected = Vec::new();
        let Some(obj) = patch.as_object() else { return vec!["(not an object)".into()] };

        macro_rules! boolean {
            ($key:literal, $field:expr) => {
                if let Some(v) = obj.get($key) {
                    match v.as_bool() {
                        Some(b) => $field = b,
                        None => rejected.push($key.to_string()),
                    }
                }
            };
        }
        boolean!("launchAtLogin", self.launch_at_login);
        boolean!("autoStartAutomation", self.auto_start_automation);
        boolean!("showWidget", self.show_widget);
        boolean!("showDockIcon", self.show_dock_icon);
        boolean!("widgetCompact", self.widget_compact);
        boolean!("widgetShowStatus", self.widget_show_status);
        boolean!("notifications", self.notifications);
        boolean!("soundMuted", self.sound_muted);
        boolean!("mascot", self.mascot);
        boolean!("mascotIdle", self.mascot_idle);
        boolean!("mascotPointer", self.mascot_pointer);
        boolean!("reduceMotion", self.reduce_motion);
        for (key, ok) in [("notificationSecs", 2..=15u64), ("soundVolume", 0..=100u64)] {
            if let Some(v) = obj.get(key) {
                match v.as_u64().filter(|n| ok.contains(n)) {
                    Some(n) if key == "soundVolume" => self.sound_volume = n as u32,
                    Some(n) => self.notification_secs = n as u32,
                    None => rejected.push(key.to_string()),
                }
            }
        }
        if let Some(v) = obj.get("soundName") {
            match v.as_str().filter(|s| SOUND_NAMES.contains(s) || *s == "custom") {
                Some(n) => self.sound_name = n.to_string(),
                None => rejected.push("soundName".into()),
            }
        }
        if let Some(v) = obj.get("soundCustomFile") {
            match v.as_str().filter(|s| valid_custom_sound_file(s)) {
                Some(f) => self.sound_custom_file = f.to_string(),
                None => rejected.push("soundCustomFile".into()),
            }
        }
        if let Some(v) = obj.get("soundEvents") {
            match v.as_object() {
                Some(m) => {
                    for (k, val) in m {
                        let slot = match k.as_str() {
                            "started" => Some(&mut self.sound_events.started),
                            "completed" => Some(&mut self.sound_events.completed),
                            "allDone" => Some(&mut self.sound_events.all_done),
                            "error" => Some(&mut self.sound_events.error),
                            "opened" => Some(&mut self.sound_events.opened),
                            _ => None,
                        };
                        match (slot, val.as_bool()) {
                            (Some(s), Some(b)) => *s = b,
                            _ => rejected.push(format!("soundEvents.{}", k)),
                        }
                    }
                }
                None => rejected.push("soundEvents".into()),
            }
        }

        macro_rules! choice {
            ($key:literal, $ty:ty, $field:expr) => {
                if let Some(v) = obj.get($key) {
                    match v.as_str().and_then(<$ty>::parse) {
                        Some(x) => $field = x,
                        None => rejected.push($key.to_string()),
                    }
                }
            };
        }
        choice!("appearance", Appearance, self.appearance);
        choice!("widgetPosition", WidgetPosition, self.widget_position);
        choice!("onFinish", OnFinish, self.on_finish);
        choice!("mascotIntensity", MascotIntensity, self.mascot_intensity);

        if let Some(v) = obj.get("preferredPlatform") {
            match v.as_str().filter(|s| SITE_KEYS.contains(s)) {
                Some(k) => self.preferred_platform = k.to_string(),
                None => rejected.push("preferredPlatform".into()),
            }
        }
        if let Some(v) = obj.get("platforms") {
            match v.as_object() {
                Some(m) => {
                    for (k, val) in m {
                        match (SITE_KEYS.contains(&k.as_str()), val.as_bool()) {
                            (true, Some(b)) => self.platforms.set(k, b),
                            _ => rejected.push(format!("platforms.{}", k)),
                        }
                    }
                }
                None => rejected.push("platforms".into()),
            }
        }
        if let Some(v) = obj.get("shortcuts") {
            match v.as_object() {
                Some(m) => {
                    for (k, val) in m {
                        let text = val.as_str().filter(|s| valid_shortcut_text(s));
                        match (k.as_str(), text) {
                            ("toggleAutomation", Some(t)) => self.shortcuts.toggle_automation = t.to_string(),
                            ("toggleWidget", Some(t)) => self.shortcuts.toggle_widget = t.to_string(),
                            _ => rejected.push(format!("shortcuts.{}", k)),
                        }
                    }
                }
                None => rejected.push("shortcuts".into()),
            }
        }
        if let Some(v) = obj.get("watchers") {
            let list: Option<Vec<String>> = v.as_array().and_then(|a| {
                a.iter()
                    .map(|x| x.as_str().filter(|s| !s.is_empty() && s.len() <= 24 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')).map(str::to_string))
                    .collect()
            });
            match list {
                Some(mut l) if l.len() <= 16 => {
                    l.sort();
                    l.dedup();
                    self.watchers = l;
                }
                _ => rejected.push("watchers".into()),
            }
        }
        self.normalize();
        rejected
    }

    /// Invariants: at least one platform is on, and the preferred platform is one of the enabled ones.
    pub fn normalize(&mut self) {
        if self.platforms.first_enabled().is_none() {
            self.platforms.set(&self.preferred_platform.clone(), true);
        }
        if self.platforms.get(&self.preferred_platform) != Some(true) {
            self.preferred_platform = self.platforms.first_enabled().unwrap_or("instagram").to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_round_trip_through_json() {
        let d = Settings::default();
        let mut back = Settings { launch_at_login: true, ..Settings::default() };
        assert!(back.apply_patch(&d.to_json()).is_empty());
        assert_eq!(back, d);
    }

    #[test]
    fn partial_patch_only_touches_named_fields() {
        let mut s = Settings::default();
        assert!(s.apply_patch(&json!({"platforms": {"x": false}, "widgetPosition": "top-right"})).is_empty());
        assert_eq!(s.platforms, Platforms { instagram: true, x: false, youtube: true });
        assert_eq!(s.widget_position, WidgetPosition::TopRight);
        assert_eq!(s.on_finish, OnFinish::Hide);
    }

    #[test]
    fn invalid_fields_are_rejected_individually() {
        let mut s = Settings::default();
        let rejected = s.apply_patch(&json!({"widgetPosition": "middle", "showWidget": "yes", "appearance": "dark", "shortcuts": {"toggleWidget": "Ctrl+Alt+K!"}}));
        let mut rejected = rejected;
        rejected.sort();
        assert_eq!(rejected, ["shortcuts.toggleWidget", "showWidget", "widgetPosition"]);
        assert_eq!(s.appearance, Appearance::Dark, "valid siblings still apply");
        assert_eq!(s.widget_position, DEFAULT_WIDGET_POSITION);
        assert!(s.show_widget);
        assert_eq!(s.shortcuts.toggle_widget, DEFAULT_TOGGLE_WIDGET);
    }

    #[test]
    fn at_least_one_platform_stays_on_and_preferred_follows() {
        let mut s = Settings::default();
        s.apply_patch(&json!({"platforms": {"instagram": false}}));
        assert_eq!(s.preferred_platform, "x");
        s.apply_patch(&json!({"platforms": {"x": false, "youtube": false}}));
        assert!(SITE_KEYS.iter().any(|k| s.platforms.get(k) == Some(true)), "never all off");
        assert_eq!(s.platforms.get(&s.preferred_platform), Some(true));
    }

    #[test]
    fn disabled_platform_cannot_become_preferred_by_patch() {
        let mut s = Settings::default();
        s.apply_patch(&json!({"platforms": {"youtube": false}, "preferredPlatform": "youtube"}));
        assert_eq!(s.preferred_platform, "instagram");
    }

    #[test]
    fn save_and_load_survive_garbage() {
        let dir = std::env::temp_dir().join(format!("wp-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("settings.json");
        assert_eq!(Settings::load_from(&p), Settings::default(), "missing file");
        let s = Settings { widget_compact: true, on_finish: OnFinish::Close, ..Settings::default() };
        s.save_to(&p).unwrap();
        assert_eq!(Settings::load_from(&p), s);
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(Settings::load_from(&p), Settings::default(), "corrupt file");
        std::fs::write(&p, r#"{"onFinish":"close","widgetPosition":42,"future":1}"#).unwrap();
        let l = Settings::load_from(&p);
        assert_eq!((l.on_finish, l.widget_position), (OnFinish::Close, DEFAULT_WIDGET_POSITION));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_uses_the_ui_field_names() {
        let v = Settings::default().to_json();
        assert_eq!(v["widgetPosition"], DEFAULT_WIDGET_POSITION.as_str());
        assert_eq!(v["shortcuts"]["toggleAutomation"], DEFAULT_TOGGLE_AUTOMATION);
        assert_eq!(v["platforms"]["x"], true);
        assert_eq!(v["onFinish"], "hide");
    }

    #[test]
    fn watcher_list_is_validated_sorted_and_deduplicated() {
        let mut s = Settings::default();
        assert!(s.apply_patch(&json!({"watchers": ["opencode", "codex", "opencode"]})).is_empty());
        assert_eq!(s.watchers, ["codex", "opencode"]);
        assert_eq!(s.apply_patch(&json!({"watchers": ["ok", "Bad Name"]})), ["watchers"]);
        assert_eq!(s.apply_patch(&json!({"watchers": "codex"})), ["watchers"]);
        assert_eq!(s.watchers, ["codex", "opencode"], "rejected values change nothing");
        assert!(s.apply_patch(&json!({"watchers": []})).is_empty());
        assert!(s.watchers.is_empty());
    }
}

#[cfg(test)]
mod sound_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sound_and_notification_fields_are_validated_individually() {
        let mut s = Settings::default();
        assert!(s.sound_events.completed && !s.sound_events.started, "completion sound is the default");
        assert!(s.apply_patch(&json!({"soundVolume": 40, "soundName": "retro", "soundMuted": true, "notificationSecs": 8, "soundEvents": {"started": true, "allDone": true}})).is_empty());
        assert_eq!((s.sound_volume, s.sound_name.as_str(), s.sound_muted, s.notification_secs), (40, "retro", true, 8));
        assert!(s.sound_events.started && s.sound_events.all_done && s.sound_events.completed);
        let mut bad = s.apply_patch(&json!({"soundVolume": 101, "soundName": "../../x", "notificationSecs": 1, "soundEvents": {"nope": true}, "soundCustomFile": "../evil.wav"}));
        bad.sort();
        assert_eq!(bad, ["notificationSecs", "soundCustomFile", "soundEvents.nope", "soundName", "soundVolume"]);
        assert_eq!(s.sound_volume, 40);
        assert!(s.apply_patch(&json!({"soundCustomFile": "custom.mp3", "soundName": "custom"})).is_empty());
        assert!(!valid_custom_sound_file("custom.exe") && !valid_custom_sound_file("other.wav") && valid_custom_sound_file(""));
    }
}

#[cfg(test)]
mod mascot_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mascot_preferences_default_to_subtle_and_validate() {
        let mut s = Settings::default();
        assert!(s.mascot && s.mascot_idle && s.mascot_pointer && !s.reduce_motion);
        assert_eq!(s.mascot_intensity, MascotIntensity::Subtle);
        assert!(s.apply_patch(&json!({"mascot": false, "mascotIntensity": "playful", "reduceMotion": true})).is_empty());
        assert!(!s.mascot && s.reduce_motion && s.mascot_intensity == MascotIntensity::Playful);
        assert_eq!(s.apply_patch(&json!({"mascotIntensity": "wild"})), ["mascotIntensity"]);
        assert_eq!(s.to_json()["mascotIntensity"], "playful");
    }
}
