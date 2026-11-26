import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";

interface PiEmitter {
  on(event: string, handler: (event?: unknown) => Promise<void> | void): void;
}

export default function (pi: PiEmitter) {
  const BRIDGE_PORT = 3150;
  const NATIVE_APP = ["C:\\Users\\ASUS\\waitplay\\target\\release\\waitplay.exe", "C:\\Users\\ASUS\\AppData\\Local\\Temp\\waitplay\\target\\release\\waitplay.exe"].find((p) => fs.existsSync(p)) ?? "";
  const DISMISS_FILE = path.join(process.env.TEMP || "C:\\Users\\ASUS\\AppData\\Local\\Temp", "waitplay", ".waitplay-dismissed.json");
  const REG_FILE = path.join(process.env.TEMP || "C:\\Users\\ASUS\\AppData\\Local\\Temp", "waitplay", ".waitplay-terminals.json");

  function isPidAlive(pid: number): boolean {
    try {
      process.kill(pid, 0);
      return true;
    } catch (err: unknown) {
      if (err && typeof err === "object" && "code" in err && (err as { code: string }).code === "EPERM") {
        return true;
      }
      return false;
    }
  }

  // Determine stable terminal tab label (reusing lowest available slot 1, 2, ... among active processes)
  function getTerminalInfo(): { label: string; sessionKey: string } {
    try {
      const dir = path.dirname(REG_FILE);
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      let data: Record<string, unknown> = {};
      if (fs.existsSync(REG_FILE)) {
        try { data = JSON.parse(fs.readFileSync(REG_FILE, "utf-8")); } catch {}
      }
      const pids: Record<string, number> = (data.pids as Record<string, number>) || {};

      // Prune dead process IDs so slots are reclaimed immediately
      for (const p of Object.keys(pids)) {
        const pidNum = parseInt(p, 10);
        if (!isPidAlive(pidNum)) {
          delete pids[p];
        }
      }

      const pidStr = String(process.pid);
      let num: number;
      if (pids[pidStr]) {
        num = pids[pidStr];
      } else {
        const used = new Set(Object.values(pids));
        num = 1;
        while (used.has(num)) {
          num++;
        }
        pids[pidStr] = num;
        data.pids = pids;
        fs.writeFileSync(REG_FILE, JSON.stringify(data, null, 2), "utf-8");
      }
      const label = `Term - ${num}`;
      const sessionKey = `omp-term-${num}`;
      return { label, sessionKey };
    } catch {
      return { label: "Term - 1", sessionKey: "omp-session" };
    }
  }

  const { label: termLabel, sessionKey } = getTerminalInfo();
  // One close file per session: a prompt starting in another terminal must never delete ours.
  // Name must match the signal file id of the native waitplay app (crates/waitplay-core signals).
  const CLOSE_FILE = path.join(process.env.TEMP || "C:\\Users\\ASUS\\AppData\\Local\\Temp", "waitplay",
    `.waitplay-close-${sessionKey.replace(/[^A-Za-z0-9_-]/g, "_")}.json`);

  function isVibe PlayEnabled(): boolean {
    try {
      const candidates = [
        path.join(process.env.TEMP || "C:\\Users\\ASUS\\AppData\\Local\\Temp", "waitplay", ".waitplay-enabled.json"),
        "C:\\Users\\ASUS\\waitplay\\.waitplay-enabled.json",
      ];
      let latestTime = -1;
      let latestEnabled = true;
      let found = false;
      for (const p of candidates) {
        if (fs.existsSync(p)) {
          try {
            const data = JSON.parse(fs.readFileSync(p, "utf-8"));
            if (data && typeof data.enabled === "boolean") {
              const time = Number(data.updatedAt) || 0;
              if (time > latestTime) {
                latestTime = time;
                latestEnabled = data.enabled;
                found = true;
              }
            }
          } catch {}
        }
      }
      if (found) return latestEnabled;
    } catch {}
    return true;
  }

  function isPromptDismissed(sessionId: string): boolean {
    try {
      if (!sessionId) return false;
      if (fs.existsSync(DISMISS_FILE)) {
        const data = JSON.parse(fs.readFileSync(DISMISS_FILE, "utf-8"));
        const entry = data[sessionId];
        if (entry && entry.dismissed === true) {
          const now = Date.now();
          if (entry.dismissedAt && (now - entry.dismissedAt) > 300000) {
            clearDismissal(sessionId);
            return false;
          }
          return true;
        }
      }
    } catch {}
    return false;
  }

  function clearDismissal(sessionId: string) {
    try {
      if (!sessionId) return;
      if (fs.existsSync(DISMISS_FILE)) {
        const data = JSON.parse(fs.readFileSync(DISMISS_FILE, "utf-8"));
        if (data[sessionId]) {
          delete data[sessionId];
          fs.writeFileSync(DISMISS_FILE, JSON.stringify(data, null, 2), "utf-8");
        }
      }
    } catch {}
  }

  function writeCloseSignal() {
    try {
      fs.mkdirSync(path.dirname(CLOSE_FILE), { recursive: true });
      // Write-then-rename so the companion never reads a half-written file
      const tmp = `${CLOSE_FILE}.${process.pid}.tmp`;
      fs.writeFileSync(tmp, JSON.stringify({ sessionId: sessionKey, ts: Date.now() }));
      fs.renameSync(tmp, CLOSE_FILE);
    } catch {}
  }

  function clearCloseSignal() {
    try { if (fs.existsSync(CLOSE_FILE)) fs.unlinkSync(CLOSE_FILE); } catch {}
  }

  // Guard: Open interface EXACTLY ONE TIME per prompt per terminal!
  let hasPoppedForPrompt = false;

  function popApp() {
    if (!isVibe PlayEnabled()) {
      return;
    }
    if (hasPoppedForPrompt) {
      return;
    }
    if (isPromptDismissed(sessionKey)) {
      return;
    }
    hasPoppedForPrompt = true;
    clearCloseSignal();
    try {
      if (fs.existsSync(NATIVE_APP)) {
        const child = spawn(NATIVE_APP, [
          `--session=${sessionKey}`,
          `--terminal=${termLabel}`,
          `--parentPid=${process.pid}`
        ], {
          detached: true,
          stdio: "ignore",
        });
        child.unref();
      }
    } catch {}
  }

  function getToken() {
    try {
      const candidates = [
        path.resolve(process.cwd(), "waitplay", ".bridge-token"),
        path.resolve(process.cwd(), ".bridge-token"),
        path.resolve(__dirname, "..", "..", ".bridge-token"),
    "C:/Users/ASUS/waitplay/.bridge-token",
    "C:/Users/ASUS/AppData/Local/Temp/waitplay/.bridge-token",
      ];
      for (const p of candidates) {
        if (fs.existsSync(p)) {
          return fs.readFileSync(p, "utf-8").trim();
        }
      }
    } catch {}
    return "";
  }

  function emit(event: string, data?: Record<string, unknown>) {
    const token = getToken();
    const payload = JSON.stringify({
      agent: "omp",
      sessionId: sessionKey,
      event,
      timestamp: Date.now(),
      model: data?.model || "Gemini 3.8 Flash",
      terminal: termLabel,
      data: {
        ...data,
        terminal: termLabel,
      },
    });

    const req = http.request(
      {
        hostname: "127.0.0.1",
        port: BRIDGE_PORT,
        path: "/events",
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(payload),
          "x-bridge-token": token,
        },
        timeout: 1000,
      },
      (res) => res.resume()
    );

    req.on("error", () => {});
    req.write(payload);
    req.end();
  }

  // Hook into user input (runs once when prompt is submitted)
  pi.on("input", async () => {
    hasPoppedForPrompt = false; // Reset for new prompt!
    clearDismissal(sessionKey);
    popApp();
    emit("task_started", { model: "Gemini 3.8 Flash", terminal: termLabel });
  });

  pi.on("before_agent_start", async () => {
    // Backup: only pop if input didn't already pop
    popApp();
    emit("task_started", { model: "Gemini 3.8 Flash", terminal: termLabel });
  });

  pi.on("agent_start", async () => {
    popApp();
    emit("task_started", { model: "Gemini 3.8 Flash", terminal: termLabel });
  });

  pi.on("tool_call", async (event: unknown) => {
    let toolName = "tool";
    if (event && typeof event === "object" && "toolName" in event) {
      const candidate = event.toolName;
      if (typeof candidate === "string") {
        toolName = candidate;
      }
    }
    if (toolName === "ask") {
      emit("waiting_for_input", { prompt: "OMP asking for clarification/choice", terminal: termLabel });
    } else {
      emit("task_running", { tool: toolName, model: "Gemini 3.8 Flash", terminal: termLabel });
      if (!hasPoppedForPrompt) popApp();
    }
  });

  // Terminal completion: clear dismissal so NEXT prompt pops fresh!
  pi.on("agent_end", async () => {
    hasPoppedForPrompt = false;
    clearDismissal(sessionKey);
    emit("task_completed", { model: "Gemini 3.8 Flash", terminal: termLabel });
    writeCloseSignal();
  });

  pi.on("session_stop", async () => {
    hasPoppedForPrompt = false;
    clearDismissal(sessionKey);
    emit("task_completed", { model: "Gemini 3.8 Flash", terminal: termLabel });
    writeCloseSignal();
  });
}
