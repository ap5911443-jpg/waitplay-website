import React from "react";
import { Terminal, Gamepad2, CheckCircle2, Zap, ArrowRight, ShieldCheck } from "lucide-react";

export const HowItWorks: React.FC = () => {
  const chapters = [
    {
      number: "01",
      eyebrow: "Zero Configuration Hook",
      title: "Hook naturally as your agent thinks",
      description:
        "You don't need to learn a new workflow or change terminals. Vibe Play hooks directly into prompt submission and tool execution events across Claude Code, Codex, OMP, and Cursor.",
      demoTitle: "Instant Lifecycle Hook",
      demoSubtitle: "Event Bridge • Port 3150 Loopback",
      demoSnippet: [
        { label: "Prompt", text: "$ omp 'Refactor Postgres queries with indexing'" },
        { label: "Hook", text: "⚡ task_started → Vibe Play state machine activated" },
        { label: "Latency", text: "⏱ Agent estimating 45s model thought latency" },
      ],
      tag: "Deterministic Hook",
    },
    {
      number: "02",
      eyebrow: "Playful Cognitive Sprint",
      title: "Micro-games appear without breaking stride",
      description:
        "The instant your agent starts reasoning, a lightweight companion micro-game launches on screen. Memory matrix, math sprints, or reflex tests keep your brain warm without context drift.",
      demoTitle: "Cognitive Companion",
      demoSubtitle: "Always-on-top • Screen edge docked",
      demoSnippet: [
        { label: "Game", text: "🎮 Memory Grid (Level 2 Active)" },
        { label: "Sync", text: "🪟 Minimized parent terminal auto-minimizes game" },
        { label: "Control", text: "🔘 Alt+W toggle widget controls state across all tabs" },
      ],
      tag: "Focus Shield",
    },
    {
      number: "03",
      eyebrow: "Frictionless Hand-Off",
      title: "Auto-stops the exact second work finishes",
      description:
        "No abrupt cutoffs or forgotten tasks. When your agent completes its turn or asks for confirmation, Vibe Play smoothly presents your execution stats and returns your cursor directly to the terminal.",
      demoTitle: "Turn Completion Summary",
      demoSubtitle: "Seamless Cursor Return",
      demoSnippet: [
        { label: "Status", text: "✓ task_completed: 4 files refactored in 38.4s" },
        { label: "Score", text: "🏆 180 points earned • 100% focus retained" },
        { label: "Return", text: "↵ Enter to return to terminal prompt immediately" },
      ],
      tag: "Zero Lag Return",
    },
  ];

  return (
    <section id="how-it-works" className="w-full py-24 px-4 sm:px-8 bg-canvas">
      <div className="max-w-6xl mx-auto space-y-16">
        {/* Section Header */}
        <div className="max-w-2xl text-left space-y-3">
          <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest">
            Product Architecture
          </span>
          <h2 className="font-serif font-normal text-4xl sm:text-5xl lg:text-6xl text-ink leading-tight">
            How it works in three chapters.
          </h2>
          <p className="font-sans text-lg text-ink-soft leading-relaxed">
            Built as an invisible, evidence-based companion that protects your focus while LLM coding agents do the heavy lifting.
          </p>
        </div>

        {/* 3 Chapters */}
        <div className="space-y-16">
          {chapters.map((chap, i) => (
            <div
              key={i}
              className={`grid grid-cols-1 lg:grid-cols-12 gap-8 lg:gap-12 items-center ${
                i % 2 === 1 ? "lg:flex-row-reverse" : ""
              }`}
            >
              {/* Text Description Column */}
              <div
                className={`lg:col-span-6 space-y-4 text-left ${
                  i % 2 === 1 ? "lg:order-2" : "lg:order-1"
                }`}
              >
                <div className="flex items-center space-x-3">
                  <span className="font-serif text-3xl sm:text-4xl text-forest/40 font-normal">
                    {chap.number}
                  </span>
                  <span className="text-xs font-sans font-semibold uppercase tracking-wider px-2.5 py-0.5 rounded-full bg-forest/8 text-forest">
                    {chap.eyebrow}
                  </span>
                </div>

                <h3 className="font-serif font-normal text-3xl sm:text-4xl text-ink leading-snug">
                  {chap.title}
                </h3>

                <p className="font-sans text-base sm:text-lg text-ink-soft leading-relaxed">
                  {chap.description}
                </p>

                <div className="pt-2 flex items-center space-x-2 text-xs font-sans font-semibold text-forest">
                  <CheckCircle2 className="w-4 h-4 text-forest" />
                  <span>{chap.tag}</span>
                </div>
              </div>

              {/* Real UI Demonstration Card Column */}
              <div
                className={`lg:col-span-6 ${
                  i % 2 === 1 ? "lg:order-1" : "lg:order-2"
                }`}
              >
                <div className="bg-white rounded-3xl border border-ink/15 p-6 sm:p-7 shadow-popover space-y-5 text-left">
                  <div className="flex items-center justify-between border-b border-ink/10 pb-3.5">
                    <div>
                      <h4 className="font-serif font-normal text-lg text-ink">{chap.demoTitle}</h4>
                      <p className="text-[11px] font-sans text-ink-muted">{chap.demoSubtitle}</p>
                    </div>
                    <span className="w-2.5 h-2.5 rounded-full bg-forest animate-pulse" />
                  </div>

                  <div className="bg-[#14141a] rounded-2xl p-4 font-mono text-xs text-zinc-300 space-y-2.5 border border-zinc-800 shadow-inner">
                    {chap.demoSnippet.map((line, j) => (
                      <div key={j} className="flex items-start space-x-2">
                        <span className="text-zinc-500 font-semibold shrink-0">[{line.label}]:</span>
                        <span className={j === 1 ? "text-cyan-400 font-semibold" : "text-zinc-200"}>
                          {line.text}
                        </span>
                      </div>
                    ))}
                  </div>

                  <div className="flex items-center justify-between text-xs font-sans text-ink-soft pt-1">
                    <span>Multi-terminal tab isolation</span>
                    <span className="font-semibold text-ink">Zero cloud telemetry</span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
