# Getting Started with Vibe Play

Vibe Play is a local gaming companion for users of AI coding agents like Claude Code.

## Prerequisites

- Rust (stable) with Cargo
- Node.js 18+ and npm 9+ (only for the React web HUD)

## Installation

```bash
cd waitplay
# Build the bridge, hooks and native app
cargo build --release --workspace
# Optional: install and build the web HUD
npm install
```

## Running the Application

### 1. Start the Local Companion Bridge

The bridge runs on `http://127.0.0.1:3150` (override with `BRIDGE_PORT`) and generates a local auth token:

```bash
npm run start:bridge
# or
target/release/waitplay-bridge.exe
```

### 2. Start the Browser Web Application

Open a second terminal:

```bash
npm run dev:web
```

The Vite dev server will start at `http://127.0.0.1:5200`. Open this URL in your browser.

## Building for Production

```bash
npm run build
```

This builds all Rust crates into `target/release/` and creates production assets for `@waitplay/web` in `apps/web/dist` (served by `npm run start:web`).

## Testing All Simulated Agent Events
Vibe Play includes a complete interactive simulator directly in the web dashboard so you can test all agent lifecycle behaviors without Claude Code installed:

1. Open `http://127.0.0.1:5200`.
2. Scroll to the **Agent Event Simulator** card at the bottom.
3. Test the automated scenarios:
   - **Standard (8s):** Task starts, game launches after 4 seconds, results screen appears at 8s when the task finishes.
   - **Quick (2s):** Task completes before the 4s launch delay. Verifies that the game never opens for short queries.
   - **Permission:** Task works, pauses when input/confirmation is required, and resumes when confirmed.
   - **Failure:** Simulates an agent error, stopping the game with an error notification.
4. Or use the **Manual Event Controls** to step through `Session Start`, `Task Start`, `Waiting for Input`, and `Task Completed` individually.
5. Toggle **Route via Bridge** to verify end-to-end WebSocket loopback round-trip delivery through the Rust bridge.

## Running Automated Unit & Integration Tests

```bash
npm test
```

Runs the Rust workspace tests (event validation, security authentication, multi-session management, state machine transitions, WebSocket reconnection, the hooks and the native app logic) plus the shared TypeScript package tests.
