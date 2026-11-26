import { FAQItem } from "../types";

export const FAQS: FAQItem[] = [
  {
    id: "how-it-works",
    question: "How does Vibe Play detect when my coding agent starts and finishes?",
    answer:
      "Vibe Play hooks natively into your agent's lifecycle events. For Claude Code, it uses pre_tool_use and notification hooks; for Cursor, OMP, and Codex, it monitors local turn events and CLI process execution. The moment your agent begins thinking or running bash tools, a lightweight event is emitted to Vibe Play's local bridge (127.0.0.1:3150).",
  },
  {
    id: "security",
    question: "Does Vibe Play capture or upload my code, prompts, or API keys?",
    answer:
      "Vibe Play has no analytics and works fully without an account. The local bridge listens exclusively on 127.0.0.1 (loopback) and every request needs an auto-generated cryptographic token. Your source code, diffs, file contents and API keys are never read. To show status in your own widget, agent hooks send the bridge the state (idle, working, finished), the model and tool names and the first 100 characters of your prompt or of the agent's question; this stays in the bridge's memory on your computer and is never sent anywhere. Outgoing connections are limited to the entertainment site you choose to open (Instagram, X or YouTube), SMS through Twilio if you configure it, and, only if you choose to create an optional account, sign-in with Supabase, which receives your email and device name but never your prompts or activity.",
  },
  {
    id: "supported-agents",
    question: "Which coding agents, IDEs, and environments are supported?",
    answer:
      "Vibe Play includes native watchers and hooks for 8 major agent environments: Claude Code, Cursor, OpenAI Codex, Oh-My-Pi (OMP), OpenCode, Cline, Windsurf, and Google Antigravity. It also includes an official Model Context Protocol (MCP) server integration so any MCP-compliant agent can trigger Vibe Play automatically.",
  },
  {
    id: "interruption",
    question: "What happens if my agent finishes while I'm in the middle of a micro-game?",
    answer:
      "Vibe Play follows a zero-interruption philosophy. It never abruptly closes or freezes your screen. Instead, a clean, subtle completion summary card slides in showing the agent's execution duration and tool calls, with two distinct buttons: 'Return to Terminal' or 'Keep Playing'. You can finish your turn, celebrate your high score, and switch back on your own terms.",
  },
  {
    id: "delay-threshold",
    question: "Can I prevent Vibe Play from popping up on quick 2-second queries?",
    answer:
      "Yes. Vibe Play features a configurable start-delay threshold (default: 3 seconds). If your agent answers a simple query or reads a small file within 3 seconds, Vibe Play stays completely silent and hidden. Only when reasoning or tool execution exceeds your threshold does the companion appear.",
  },
  {
    id: "game-library",
    question: "What micro-games are included, and can I add custom games?",
    answer:
      "Out of the box, Vibe Play includes Memory Match, Reaction Speed Sprint, Math Blitz, and Riddle Challenge — all calibrated for 15 to 45-second play sessions. Vibe Play's open game registry architecture lets you easily drop in custom React or HTML5 canvas games via simple plug-and-play interfaces.",
  },
  {
    id: "window-sync",
    question: "How does the parent window synchronization work?",
    answer:
      "The native desktop app monitors your active IDE and terminal windows (VS Code, Cursor, Windows Terminal). If you minimize your terminal to check documentation, Vibe Play automatically minimizes off-screen so it never blocks your workflow. Restoring your terminal instantly restores your active game board without losing your progress.",
  },
  {
    id: "pricing-license",
    question: "Is Vibe Play free and open source?",
    answer:
      "Yes. Vibe Play is 100% free and open source under the permissive MIT license. You can clone the repository, audit every line of code, run the app entirely on your own machine, and build custom plugins for your team without any subscription or account required.",
  },
];
