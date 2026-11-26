//! Vibe Play desktop app: the tray application (no arguments or `--widget`: tray icon, floating widget, settings) and the companion
//! window that shows Instagram, YouTube or X while a coding agent works. One executable, same command line as the C# build (see `cli`).

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(any(target_os = "macos", windows))]
mod account_ui;
mod cli;
mod companion;
mod logging;
#[cfg(any(target_os = "macos", test))]
mod mac;
mod platform;
#[cfg(any(target_os = "macos", windows, test))]
mod placement;
mod profile;
mod single_instance;
mod sites;
#[cfg(windows)]
mod win;
// The older floating widget UI: still the `--dock` side store; the tray app replaced it on Windows and macOS
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
mod widget;

use waitplay_core::signals::{self, LaunchSignal};
use waitplay_core::{now_ms, state, BridgeClient};

fn main() {
    let args = cli::parse(std::env::args().skip(1));

    // The window that was in front when we started: for a hook launch, that is the terminal the prompt was typed in
    let mut initial_fg = platform::foreground_window();
    if platform::window_pid(initial_fg) == platform::current_pid() {
        initial_fg = platform::NO_WINDOW;
    }

    if let Some(side) = &args.dock {
        widget::save_dock(side);
        return;
    }

    if args.toggle {
        platform::attach_parent_console();
        let enabled = state::toggle();
        BridgeClient::new().post_state(enabled);
        println!("Vibe Play enabled: {}", if enabled { "True" } else { "False" });
        return;
    }

    // macOS and Windows: no arguments (or --widget) starts the tray application, whatever the automation switch says
    #[cfg(any(target_os = "macos", windows))]
    if args.widget || (args.session.is_empty() && !args.force_open) {
        #[cfg(target_os = "macos")]
        let result = mac::app::run();
        #[cfg(windows)]
        let result = win::app::run();
        if let Err(e) = result {
            eprintln!("waitplay: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let enabled = state::is_enabled();
    if args.force_open && !enabled {
        state::set_enabled(true);
        BridgeClient::new().post_state(true);
    }
    // Vibe Play is OFF and nobody asked for a window explicitly: stay silent
    if !args.force_open && !args.widget && !enabled {
        return;
    }

    let mut parent_hwnd = args.parent_hwnd;
    if parent_hwnd == platform::NO_WINDOW && args.parent_pid > 0 {
        parent_hwnd = platform::find_top_window_for_pid(args.parent_pid);
    }
    if parent_hwnd == platform::NO_WINDOW && initial_fg != platform::NO_WINDOW {
        parent_hwnd = initial_fg;
    }

    // No arguments at all: the widget
    let widget = args.widget || (args.session.is_empty() && !args.force_open);
    if widget {
        if let Err(e) = widget::run() {
            eprintln!("waitplay: widget failed: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if signals::is_prompt_dismissed(&args.session) {
        return;
    }

    // The identity of the companion (single-instance lock, signal files) is fixed; the page it opens on is the user's choice
    let site = args.site.unwrap_or_else(sites::default_site);
    let start = args.site.unwrap_or_else(sites::preferred_site);
    let Some(guard) = single_instance::acquire(&single_instance::name_for_site(site.key)) else {
        // A companion for this site is already running: hand it the launch and leave
        if !state::is_enabled() {
            return;
        }
        LaunchSignal { session_id: args.session, terminal: args.terminal, ts: now_ms(), hwnd: parent_hwnd as i64 }.write(site.key);
        return;
    };

    let opts = companion::Options {
        site,
        start,
        session: args.session,
        terminal: args.terminal,
        parent_hwnd,
    };
    if let Err(e) = companion::run(opts, guard) {
        eprintln!("waitplay: companion failed: {}", e);
        std::process::exit(1);
    }
}
