# OpenAI Codex Integration for Vibe Play

Real-time integration between **OpenAI Codex** and **Vibe Play**, implemented by the Rust `waitplay-hook` binary (`crates/waitplay-hooks`).

## How It Works

1. **Prompt submission & reasoning detection:** `waitplay-hook codex-watch` monitors Codex's local operational database (`~/.codex/logs_2.sqlite`) with read-only polling. When the user submits a prompt and Codex starts reasoning or loading tools it emits `task_started` to the bridge (`http://127.0.0.1:3150/events`) and opens the Vibe Play companion window.
2. **Work execution:** `task_running` events keep the telemetry current while Codex runs commands and tools.
3. **Turn completion:** `notify` in `~/.codex/config.toml` runs `waitplay-hook codex-notify`, which emits `task_completed` and closes the companion.

## Configuration

In `~/.codex/config.toml`:

```toml
notify = [ "C:\\Users\\ASUS\\waitplay\\target\\release\\waitplay-hook.exe", "codex-notify" ]
```

## Running the Watcher

```bash
npm run codex:watch
# or
target/release/waitplay-hook.exe codex-watch
```
