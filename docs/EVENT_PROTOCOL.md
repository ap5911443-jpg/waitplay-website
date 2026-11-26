# Vibe Play Normalized Event Protocol
   - Repeat it while a turn is open and silent. The bridge marks a `WORKING` or `GAME_ACTIVE` session `COMPLETED` after 20 seconds without any event.

This document defines the contract for agent lifecycle communication between coding agents and the Vibe Play bridge.

## TypeScript Interface

```typescript
export type AgentEvent =
  | "session_started"
  | "task_started"
  | "task_running"
  | "waiting_for_input"
  | "task_completed"
  | "task_failed"
  | "session_ended";

export interface AgentEventPayload {
  agent: "claude-code";
  sessionId: string;
  event: AgentEvent;
  timestamp: number;
  data?: Record<string, unknown>;
}
```

## Event Semantics

1. **`session_started`**
   - Triggered when Claude Code starts an interactive chat session.
   - Cleans up state and transitions session to `IDLE`.

2. **`task_started`**
   - Triggered when the user submits a prompt or task execution begins.
   - Transitions session to `WORKING`. Starts the configurable launch delay timer (default 4 seconds).
   - Cancels any previous completion modal.

3. **`task_running`**
   - Dispatched during intermediate agent operations (tool executions, file reads, linting).
   - Keeps game active or resumes game if paused from an input request.

4. **`waiting_for_input`**
   - Dispatched when Claude Code prompts for user confirmation (e.g., bash tool approval, question).
   - Pauses the active game and alerts the developer with a notification.

5. **`task_completed`**
   - Dispatched when Claude Code's response or turn finishes (`Stop` event).
   - If the task completed before the launch delay expired: game never opens.
   - If game was active: halts game and displays the results modal.

6. **`task_failed`**
   - Dispatched when execution fails unrecoverably.
   - Stops the game and displays failure diagnostics.

7. **`session_ended`**
   - Dispatched when the user exits Claude Code.
   - Terminates session tracking in the bridge.
