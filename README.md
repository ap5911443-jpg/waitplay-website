# Vibe Play 🎮

> **The zero-interruption gaming companion for AI coding agents.**  
> Play quick micro-games while your coding agent thinks, reasons, and executes tools.

![Vibe Play Banner](docs/banner.png) *(optional preview)*

Vibe Play detects when your AI coding agent begins working and automatically opens a companion popup. By default it presents an authentic **Instagram experience** (feed, stories, and Reels) so you can scroll while the agent reasons; **YouTube** and **X** are one click away in the header, and the widget's app picker opens any of them. When the agent finishes, Vibe Play surfaces a completion summary and returns focus to your terminal.

---

## ✨ Features

- **⚡ Instant Agent Trigger:** Hooks directly into prompt submission and tool execution events across 8 major coding agent environments.
- **🔘 Wispr Flow-Style Desktop Toggle Widget:** A sleek, always-on-top, draggable floating pill widget docked to your screen edge (with `Alt+W` global hotkey, cyan active indicator dot, and direct gamepad launcher) to toggle Vibe Play ON / OFF instantly.
- **🪟 Parent Window Synchronization:** Automatically detects when you minimize your parent terminal or IDE (Windows Terminal, VS Code, Cursor, Command Prompt) and minimizes Vibe Play off-screen. Restoring the terminal restores Vibe Play without losing your game board or score.
- **🧠 Stays in RAM, resumes where you left off, costs almost none:** The task ending, scrolling or typing in the terminal that owns the window, **X**, **Esc** and **Alt+F4** hide the companion instead of closing it. The page, the reel position, the scroll and the game board stay in memory (your login is kept on disk), the video is paused and muted while the window is hidden, and the next prompt or the widget button brings the same window back from the same spot. Instagram, X and YouTube each keep their own window. A hidden window is frozen like an Edge "sleeping tab" and its physical RAM is handed back to Windows (nothing is discarded; it pages back in on demand, about 1-2 s): measured on a loaded youtube.com page the whole companion drops from about 345 MB in Task Manager's Memory column to about 10 MB, and its commit charge shrinks by roughly 30%. Right-click the widget and choose **Quit companion** to free everything, or turn Vibe Play off with `Alt+W`.
- **🔑 Sign in once:** Instagram, YouTube and X run in Vibe Play's own browser profile, `%LOCALAPPDATA%\WaitPlay\webview2` (not `%TEMP%`, which Storage Sense and cleaner tools empty). Logins survive restarts, rebuilds and updates. Vibe Play does not read your default browser's logins: Chromium browsers lock and encrypt them against other apps. Set `WAITPLAY_PROFILE_DIR` to keep the profile somewhere else.
- **🎯 Multi-Agent & Multi-Terminal Support:** Isolated state tracking across simultaneous terminal sessions, preventing agent conflicts.
- **🔒 Local Security First:** Bridge operates strictly on loopback (`127.0.0.1:3150`) with auto-generated secure token authentication and payload validation.
- **🦀 Written in Rust:** The bridge, every agent hook and watcher, the MCP server and the native Windows app are Rust crates in one Cargo workspace (`crates/`). The native app (`waitplay.exe`) is a small executable that hosts the browser in WebView2.
- **🌐 Web HUD (`apps/web`):** A React 18 + Vite + Tailwind dashboard with live event stream and simulator, served by the Rust bridge (`waitplay-bridge web`).

---

## 🤖 Supported Agents & Integrations

Every integration is a subcommand of one executable, `target/release/waitplay-hook.exe`.

| Agent | Trigger Mechanism | Command |
| :--- | :--- | :--- |
| **Claude Code** | Lifecycle hooks (JSON on stdin) | `waitplay-hook claude-code <start\|tool\|notify\|stop\|end>` |
| **OpenAI Codex** | `notify` turn-end hook + log watcher | `waitplay-hook codex-notify`, `waitplay-hook codex-watch` |
| **Cline** | Real-time state watcher | `waitplay-hook cline-watch` |
| **Cursor** | Editor hook | `waitplay-hook cursor <start\|tool\|stop>` |
| **Google Antigravity** | Task dispatcher hook | `waitplay-hook antigravity <start\|tool\|stop>` |
| **OpenCode** | Session watcher | `waitplay-hook opencode-watch` |
| **Oh-My-Pi (OMP)** | Session-log watcher (+ optional extension `integrations/omp/waitplay.ts`) | `waitplay-hook omp-watch` |
| **Model Context Protocol** | MCP server over stdio | `waitplay-hook mcp` |

---

## 📁 Repository Structure

```
waitplay/
├── crates/
│   ├── waitplay-core/    # Protocol types, paths, tokens, signal files, bridge client
│   ├── waitplay-bridge/  # WebSocket & REST event bridge (port 3150) + web HUD server
│   ├── waitplay-hooks/   # waitplay-hook: agent hooks, watchers, MCP server, simulator
│   └── waitplay-app/     # waitplay.exe: floating widget + companion window (WebView2)
├── apps/web/             # React 18 + Vite web HUD (served by the bridge on port 5200)
├── packages/shared/      # TypeScript types for the web HUD
├── integrations/         # Per-agent setup notes (+ the OMP extension)
├── docs/                 # Architecture, event protocols, and guides
├── Cargo.toml            # Rust workspace
└── package.json          # npm scripts that wrap cargo + the web HUD build
```

---

## 🚀 Quick Start

### 1. Build

Requires a stable Rust toolchain (and Node 18+ only for the web HUD).

```bash
cargo build --release --workspace     # bridge, hooks, app -> target/release/
npm install && npm run build:web      # optional: web HUD
```

Run `waitplay-bridge.exe` first; the hooks and the app talk to it on `127.0.0.1:3150`.

> **Companion UI:** The native Windows app embeds a **WebView2** (Chromium) control that acts as a browser: on agent start it opens the live **[instagram.com](https://www.instagram.com/)**. The header carries the real logos of Instagram, YouTube and X (the site on screen is underlined); click one to switch. Launch a site directly with `waitplay.exe --game --site=instagram|youtube|x`. To add a site, add an entry to `crates/waitplay-app/src/sites.rs`. The WebView2 runtime ships with modern Windows; no extra install is required.

> **macOS:** Vibe Play also ships as a native menu-bar app (`WaitPlay.app` / `.dmg`, Apple Silicon and Intel) with a settings window, a floating status widget and a WKWebView entertainment window. See [docs/MACOS.md](docs/MACOS.md) for the architecture, permissions, build and signing steps.

### 2. Test

```bash
cargo test --workspace
```

---

### Phone completion notifications (optional)

The bridge can send an SMS through Twilio whenever an agent transitions to `COMPLETED`. Set these environment variables before starting the bridge:

```bash
TWILIO_ACCOUNT_SID=AC...
TWILIO_AUTH_TOKEN=...
TWILIO_FROM_NUMBER=+15550001111
WAITPLAY_NOTIFICATION_PHONE=+15550002222
```

If any variable is missing, phone notifications stay disabled and Vibe Play continues normally. Notification failures are logged without blocking agent events.

## 🕹️ Running Vibe Play

### Mode A: Floating Widget (Desktop Overlay)

```bash
npm run widget
# Or run the executable directly
./target/release/waitplay.exe --widget
```
- Click the **ON/OFF** pill or press **Alt+W** anywhere to toggle Vibe Play.
- Click the **🎮 Gamepad** button to open the companion.
- Right-click the widget → **Quit companion (free RAM)** to really close the parked windows (X only hides them). They exit cleanly, so the browser saves your logins first, and nothing is left running.
- Toggle state instantly via CLI: `npm run toggle`

### Mode B: Hooks and watchers

Start the bridge, then wire your agent to `waitplay-hook` (see `integrations/`), or start a watcher:

```bash
./target/release/waitplay-bridge.exe
./target/release/waitplay-hook.exe omp-watch       # or codex-watch / opencode-watch / cline-watch
./target/release/waitplay-hook.exe simulate        # 25 s end-to-end test
```

### Mode C: Browser Web HUD

```bash
# Terminal 1: bridge (port 3150)
npm run start:bridge

# Terminal 2: web HUD (port 5200, serves apps/web/dist)
npm run start:web
```
Open `http://localhost:5200` to interact with the dashboard, live event stream, and simulator. Use `npm run dev:web` for the Vite dev server instead.

---

## 📖 Documentation

- [System Architecture](docs/ARCHITECTURE.md)
- [Event Protocol Specification](docs/EVENT_PROTOCOL.md)
- [Getting Started Guide](docs/GETTING_STARTED.md)

---

## 📄 License

MIT License. Free to use and modify.
