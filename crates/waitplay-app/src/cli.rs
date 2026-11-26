//! Command line, argument for argument the same as the C# `WaitPlay.exe` (hooks and npm scripts depend on it).
//!
//! * `--widget` / `--flow`        run the floating widget
//! * `--toggle`                   flip the ON/OFF switch and exit
//! * `--dock=left|right|bottom|free`   save the widget's dock side and exit
//! * `--game` / `--open`          open the companion even when nothing is running (and force Vibe Play on)
//! * `--site=instagram|youtube|x` which site this companion hosts
//! * `--session=<id>` `--terminal=<label>` `--promptTime=<ms>` `--parentPid=<pid>` `--parentHwnd=<hwnd>`  set by agent hooks
//! * any other argument containing `omp`, `codex`/`chatgpt`, `opencode` or `cline` names the session (legacy launchers)

use crate::sites;

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub widget: bool,
    pub toggle: bool,
    pub force_open: bool,
    pub dock: Option<String>,
    pub site: Option<&'static sites::Site>,
    pub session: String,
    pub terminal: String,
    pub prompt_time: i64,
    pub parent_pid: u32,
    pub parent_hwnd: u64,
}

pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Args {
    let mut a = Args::default();
    for arg in args {
        if arg == "--widget" || arg == "--flow" {
            a.widget = true;
        } else if arg == "--toggle" {
            a.toggle = true;
        } else if arg == "--game" || arg == "--open" {
            a.force_open = true;
            if a.terminal.is_empty() {
                a.terminal = "Desktop".into();
            }
        } else if let Some(v) = arg.strip_prefix("--site=") {
            a.site = Some(sites::by_name(v).unwrap_or_else(sites::default_site));
            a.force_open = true;
            if a.terminal.is_empty() {
                a.terminal = "Desktop".into();
            }
        } else if let Some(v) = arg.strip_prefix("--session=") {
            a.session = v.to_string();
        } else if let Some(v) = arg.strip_prefix("--terminal=") {
            a.terminal = v.trim_matches(|c| c == '"' || c == '\'').to_string();
        } else if let Some(v) = arg.strip_prefix("--promptTime=") {
            a.prompt_time = v.parse().unwrap_or(a.prompt_time);
        } else if let Some(v) = arg.strip_prefix("--parentPid=") {
            a.parent_pid = v.parse().unwrap_or(a.parent_pid);
        } else if let Some(v) = arg.strip_prefix("--parentHwnd=") {
            a.parent_hwnd = v.parse::<i64>().map(|h| h as u64).unwrap_or(a.parent_hwnd);
        } else if arg.contains("omp") {
            a.session = "omp-session".into();
        } else if arg.contains("codex") || arg.contains("chatgpt") {
            a.session = "codex-session".into();
        } else if arg.contains("opencode") {
            a.session = "opencode-session".into();
        } else if arg.contains("cline") {
            a.session = "cline-session".into();
        } else if let Some(v) = arg.strip_prefix("--dock=") {
            a.dock = Some(v.to_lowercase());
            return a; // the C# entry point saved the dock and returned on the spot
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &[&str]) -> Args {
        parse(s.iter().map(|x| x.to_string()))
    }

    #[test]
    fn hook_launch() {
        let a = p(&["--session=claude-code-session", "--parentPid=4242", "--promptTime=1700000000000"]);
        assert_eq!(a.session, "claude-code-session");
        assert_eq!(a.parent_pid, 4242);
        assert_eq!(a.prompt_time, 1700000000000);
        assert!(!a.force_open);
    }

    #[test]
    fn site_launch_forces_open() {
        let a = p(&["--game", "--site=yt"]);
        assert!(a.force_open);
        assert_eq!(a.site.unwrap().key, "youtube");
        assert_eq!(a.terminal, "Desktop");
        // unknown site falls back to the default site, as before
        assert_eq!(p(&["--site=reddit"]).site.unwrap().key, "instagram");
    }

    #[test]
    fn legacy_names_and_dock() {
        assert_eq!(p(&["omp"]).session, "omp-session");
        assert_eq!(p(&["--chatgpt"]).session, "codex-session");
        assert_eq!(p(&["--dock=LEFT"]).dock.as_deref(), Some("left"));
        assert_eq!(p(&["--terminal=\"Windows Terminal\""]).terminal, "Windows Terminal");
    }
}
