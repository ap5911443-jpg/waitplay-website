# WaitPlay on macOS

WaitPlay for macOS is a menu-bar utility. It watches your AI coding agents; when one starts working it opens your chosen
entertainment site (Instagram, X or YouTube) in a compact floating window, and when the agent is done it hides or closes that
window.

## What runs

| Process | Binary | Job |
|---|---|---|
| Menu-bar app | `waitplay` (no arguments) | tray icon and menu, floating widget, settings window, global shortcuts, login item, starts the bridge and the agent watchers |
| Entertainment window | `waitplay --session=… / --open` | one WKWebView window (header + site), started by agent hooks or *Open Now*; one per Mac |
| Bridge | `waitplay-bridge` | loopback event hub (`127.0.0.1:3150`), session state machine |
| Hooks / watchers | `waitplay-hook` | Claude Code hooks, OpenCode / Codex / OMP / Cline watchers, MCP server |

All four are inside `WaitPlay.app/Contents/MacOS/`. The app is an agent app (`LSUIElement`): no Dock icon unless
*Settings > General > Show Dock icon* is on.

Event flow: agent -> `waitplay-hook` -> bridge `POST /events` and, on a prompt, `waitplay --session=<id> --parentPid=<pid>` ->
the entertainment window registers the session with the application that hosts the terminal (the hook's nearest ancestor that is
a regular macOS app) and shows itself while that application is in front. Sessions are keyed by id; the window stays while **any**
session is still working. Claude Code now reports one session per conversation (`claude-code-<id>`), so two terminals are
independent.

## Native pieces

* `crates/waitplay-app/src/platform/macos.rs`: the platform contract on macOS. A `WindowId` is an application PID.
  Frontmost application: `NSWorkspace`. Displays: `NSScreen` (visible frame, scale; Cocoa -> top-left coordinates).
  Terminal lookup: process-tree walk to the nearest regular `NSRunningApplication`. Standard Edit/Window menus so Cmd+C/V/A work in
  the web pages (needed to paste into sign-in forms).
* `mac/app.rs`: tao event loop (Accessory activation policy), `tray-icon` status item, wry/WKWebView windows, `global-hotkey`.
* `placement.rs`: pure geometry for the widget corners (and the Windows top-centre notch) and the centred 70% x 82% entertainment window; unit tested.
* `mac/login_item.rs`: launch at login = one LaunchAgent plist (`com.waitplay.app`), rewritten when the app moves.
* `mac/integrations.rs`: idempotent install/remove of Claude Code hooks in `~/.claude/settings.json` (keeps every other setting,
  never overwrites a file it cannot parse, writes a one-time `.waitplay-backup`).
* `waitplay-core::settings`: `~/Library/Application Support/WaitPlay/settings.json`, validated field by field.
* `logging.rs`: `~/Library/Logs/WaitPlay/waitplay.log`, rotating at 1 MiB.

Pages: `assets/settings.html` and `assets/mac_widget.html` are self-contained HTML/CSS/JS (system font, `prefers-color-scheme`,
reduced-motion aware) that talk to Rust over wry IPC. They are plain HTML like the existing `header.html`/`widget.html`, not a
React build: the repo's React app (`apps/web`) is the bridge's web dashboard, not part of the desktop shell.

**Liquid Glass bar** (`mac_widget.html` + `platform::Glass`): the glass is a native `NSVisualEffectView` (real behind-window blur,
follows light/dark, Reduce Transparency, Increase Contrast) under the transparent page and shaped to the page's pill; the page
reports the pill rectangle on every animation frame, so the glass grows with the capsule into the expanded panel. The page adds the
edge light, inner highlight and contents. It uses the vibrancy material available on every supported macOS, not the macOS 26
`NSGlassEffectView`. To preview the look without a Mac, open `crates/waitplay-app/assets/mac_widget.html` in a browser (demo mode
fakes the blur). `WAITPLAY_TEST_EXPAND_WIDGET=1` makes the app expand the widget after start (used by the CI screenshots).

## Permissions

None are requested. The frontmost application comes from `NSWorkspace` (no Accessibility), no screen is captured (no Screen
Recording), agent events use loopback. *Settings > Permissions* says so and links to the Login Items pane. Consequence: the
Windows-only "typing or scrolling in the terminal steps the window aside" and per-tab tracking are not available on macOS.

## Settings that take effect where

| Setting | Applies |
|---|---|
| Launch at login, Dock icon, widget on/off and position, appearance (menu-bar app windows), shortcuts | immediately |
| Preferred platform, enabled platforms (header tabs), Hide/Close on finish, appearance of the entertainment window | next time the entertainment window starts |

## Build

```sh
rustup target add aarch64-apple-darwin
cargo build --release --workspace
WAITPLAY_BUILD=$(git rev-parse --short HEAD) bash packaging/macos/build_app.sh aarch64-apple-darwin
# -> dist/aarch64-apple-darwin/WaitPlay.app and WaitPlay-<version>-arm64.dmg
```

The script ad-hoc signs by default, so a local build never needs an Apple account. For distribution set
`APPLE_SIGNING_IDENTITY` (Hardened Runtime + timestamp) and `APPLE_ID` / `APPLE_TEAM_ID` / `APPLE_APP_PASSWORD` (notarize and
staple). The GitHub Actions workflow `.github/workflows/macos.yml` builds and tests on a macOS runner for `aarch64-apple-darwin`
and `x86_64-apple-darwin`, signs when the `APPLE_*` secrets exist, and attaches the DMGs to a `v*` tag release.

## Known limits

* Automatic updates are not implemented (the About pane says so).
* The entertainment window cannot be frozen like WebView2: when parked it is hidden and, after ten idle minutes, the process quits
  (logins are kept in the system WebKit store).
* Sites may limit embedded browsers. WaitPlay uses the standard WKWebView and does not alter the user agent or bypass any check.

## Verification status

**Unit tests** (Windows dev machine and the macOS CI job): settings validation, status derivation, tray menu model, placement geometry
(three corners, offset and secondary displays), Claude Code hook install/remove/outdated/corrupt-file handling, launch-at-login plist
sync, logging format, per-conversation Claude sessions, watcher idle polling, companion visibility rules (including "step aside
while the agent asks a question"), macOS process-tree and PID helpers.

**Runtime smoke test on a real macOS session** (`packaging/macos/smoke.sh`, run by CI on macOS 15 / x86_64 against the packaged
`WaitPlay.app`; screenshots and logs are uploaded as the `smoke-output` artifact of every run). It passed for:

- app starts, menu-bar item created, bridge started by the app, widget shown bottom-right, settings window opened by a second launch
- idle app: ~0.02 s CPU per 10 s, ~44 MB resident
- a Claude Code prompt through the real hook opens the Instagram window (native title bar, header tabs, page loads and shows the
  login form); a second conversation becomes its own session served by the same window (no second window process)
- finishing one of two sessions keeps the window; finishing the last hides it
- second configuration (dark appearance, widget top-right, YouTube preferred, X off, close on finish): dark windows, the window opens
  on YouTube below the widget, and the window process exits when the agent finishes; the WebKit cookie store is written to disk
- no `ERROR` lines in the app log

The arm64 build is cross-compiled and packaged by CI. An extra job runs the same smoke test on an Apple Silicon runner when GitHub
has capacity (it is allowed to be cancelled); check the latest run for its result.

**Not verified (needs a real Mac and a person):**

- [ ] Sign in to Instagram / X / YouTube, then quit and reopen WaitPlay and confirm you are still signed in
- [ ] Retina display scaling, a second display, display plug/unplug, sleep/wake
- [ ] Launch at login actually starting WaitPlay after a real login; no duplicate entry after moving the app
- [ ] Global shortcuts firing, and conflict messages in Settings > Shortcuts
- [ ] OpenCode / Codex / OMP / Cline watchers with real sessions; Install Hooks button against a real Claude Code
- [ ] Terminal.app / iTerm2 / VS Code: the window follows the terminal that ran the prompt
- [ ] Reduce Motion, VoiceOver, keyboard navigation of the settings window
- [ ] Gatekeeper behaviour of a Developer ID signed and notarized DMG (the CI build is ad-hoc signed)
