# Claude Code Integration for Vibe Play

This directory contains the hook definitions and adapters for integrating Claude Code with the Vibe Play companion service.

> **Status:** Architecture definition & reference hooks. The foundation and local event simulator are active in MVP v0.1.0.

## Overview

Claude Code supports shell-based and IPC hooks for agent lifecycle events. The Vibe Play bridge listens on loopback (`http://127.0.0.1:3150/events`) and requires the local token found in `.bridge-token`.

## Event Mapping

| Claude Code Lifecycle Point | Vibe Play Event | Notes |
|---|---|---|
| Session initialized | `session_started` | Resets state for a new interactive session |
| User submits prompt / tool execution starts | `task_started` | Triggers launch delay countdown |
| Multi-step tool use / thinking progress | `task_running` | Maintains active game state |
| Permission / bash prompt confirmation required | `waiting_for_input` | Pauses game and alerts user |
| Response / turn completes | `task_completed` | Stops game, displays results screen |
| Unhandled tool failure / execution error | `task_failed` | Flags failure status, halts game |
| Session terminated | `session_ended` | Cleans up session state |

## Reference Hook Script

Run `waitplay-hook claude-code <start|tool|notify|stop|end>` (the Rust `waitplay-hook` binary) as the hook command; it reads the Claude Code JSON on stdin and forwards events to the Vibe Play bridge.
