//! `waitplay-hook`: every agent integration of Vibe Play in one binary.
//!
//! Hooks (short-lived, read their input, talk to the bridge once, always exit 0, never block the agent):
//!   claude-code <start|tool|stop|notify|end>      Claude Code hook (JSON on stdin)
//!   cursor|antigravity <start|tool|stop> [a] [b]  IDE hooks
//!   codex-notify [codex args...]                  Codex `notify` turn-end hook
//! Watchers (long-running):
//!   codex-watch | cline-watch | opencode-watch | omp-watch
//! Servers and tools:
//!   mcp                                           MCP server over stdio
//!   simulate                                      25 s end-to-end test mode
//!   codex-test-turn                               simulated Codex turn
//!   popup <show|close>                            show or quit the companion window

mod claude;
mod cline;
mod codex;
mod common;
mod env;
mod ide;
mod mcp;
mod omp;
mod opencode;
mod simulate;

const USAGE: &str = "Usage: waitplay-hook <command> [args]

Hooks:
  claude-code <start|tool|stop|notify|end>
  cursor <start|tool|stop> [model|tool] [task]
  antigravity <start|tool|stop> [model|tool] [task]
  codex-notify [args...]
Watchers:
  codex-watch    cline-watch    opencode-watch    omp-watch
Other:
  mcp            MCP server over stdio
  simulate       25 second end-to-end test mode
  codex-test-turn
  popup <show|close>";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{}", USAGE);
        std::process::exit(1);
    };
    let rest = &args[1..];
    if matches!(cmd.as_str(), "claude-code" | "cursor" | "antigravity" | "codex-notify") {
        // The release profile aborts on panic, which would fail (and for Claude Code possibly block) the agent. A hook must
        // never do that: whatever goes wrong, leave quietly with success.
        std::panic::set_hook(Box::new(|_| std::process::exit(0)));
    }
    match cmd.as_str() {
        "claude-code" => claude::run(rest),
        "cursor" => ide::run(&ide::CURSOR, rest),
        "antigravity" => ide::run(&ide::ANTIGRAVITY, rest),
        "codex-notify" => codex::run_notify(rest),
        "codex-watch" => codex::run_watch(),
        "codex-test-turn" => codex::run_test_turn(),
        "popup" => std::process::exit(codex::run_popup(rest)),
        "cline-watch" => cline::run_watch(),
        "opencode-watch" => opencode::run_watch(),
        "omp-watch" => omp::run_watch(),
        "mcp" => mcp::run(),
        "simulate" => simulate::run(),
        "-h" | "--help" | "help" => println!("{}", USAGE),
        other => {
            eprintln!("waitplay-hook: unknown command '{}'\n\n{}", other, USAGE);
            std::process::exit(1);
        }
    }
    // Hooks never fail the agent: whatever happened above, exit 0
}
