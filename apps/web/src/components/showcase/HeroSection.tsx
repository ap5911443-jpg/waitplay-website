import React, { useState } from "react";
import { ArrowRight, Sparkles, Terminal, Play, CheckCircle2, RotateCw } from "lucide-react";

export interface HeroSectionProps {
  onPlayGame?: () => void;
  onOpenDashboard?: () => void;
}

export const HeroSection: React.FC<HeroSectionProps> = ({ onPlayGame, onOpenDashboard }) => {
  const [activeStep, setActiveStep] = useState<0 | 1 | 2>(1);
  const [flippedCards, setFlippedCards] = useState<number[]>([0, 1]);

  const toggleCard = (index: number) => {
    setFlippedCards((prev) =>
      prev.includes(index) ? prev.filter((i) => i !== index) : [...prev, index]
    );
  };

  return (
    <section className="relative w-full pt-16 sm:pt-24 pb-20 px-4 sm:px-8 bg-canvas overflow-hidden">
      <div className="max-w-5xl mx-auto text-center space-y-7">
        {/* Eyebrow Label */}
        <div className="inline-flex items-center space-x-2 px-3.5 py-1.5 rounded-full bg-forest/8 border border-forest/15 shadow-subtle">
          <Sparkles className="w-3.5 h-3.5 text-forest" />
          <span className="text-[11px] sm:text-xs font-sans font-semibold tracking-wider uppercase text-forest">
            Wispr Flow for AI Coding Agents
          </span>
        </div>

        {/* Hero Serif Headline */}
        <h1 className="font-serif font-normal text-5xl sm:text-7xl lg:text-[88px] leading-[0.98] tracking-tight text-ink max-w-4xl mx-auto">
          Don’t type and wait. <br />
          <span className="italic font-normal text-forest">Play while your agent builds.</span>
        </h1>

        {/* Sans-serif Explanation */}
        <p className="font-sans font-normal text-lg sm:text-xl text-ink-soft max-w-2xl mx-auto leading-relaxed">
          AI coding agents take 30 to 90 seconds to reason, query tools, and generate code. Vibe Play turns that idle downtime into instant cognitive flow — then automatically returns your cursor the second work finishes.
        </p>

        {/* CTA Group */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-3 pt-2">
          <a
            href="https://github.com/ap5911443-jpg/Waitplay.git"
            target="_blank"
            rel="noopener noreferrer"
            className="w-full sm:w-auto inline-flex items-center justify-center space-x-2 text-base font-sans font-semibold px-8 py-4 rounded-xl bg-lavender text-ink border border-ink/15 hover:bg-lavender-hover hover:shadow-hover hover:-translate-y-0.5 active:translate-y-0 transition-all cursor-pointer shadow-subtle"
          >
            <span>Download Companion</span>
            <ArrowRight className="w-4 h-4" />
          </a>

          <button
            onClick={onPlayGame}
            className="w-full sm:w-auto inline-flex items-center justify-center space-x-2 text-base font-sans font-semibold px-7 py-4 rounded-xl bg-white text-ink border border-ink/15 hover:bg-canvas-muted/60 hover:shadow-subtle hover:-translate-y-0.5 active:translate-y-0 transition-all cursor-pointer"
          >
            <Play className="w-4 h-4 text-forest fill-forest" />
            <span>Try Interactive Micro-Game</span>
          </button>
        </div>

        {/* Platform Reassurance */}
        <p className="text-xs font-sans text-ink-muted">
          Free & open source • Native on Windows, macOS, Linux • Press <kbd className="px-1.5 py-0.5 rounded bg-canvas-muted text-ink font-mono text-[11px] border border-ink/15">Alt+W</kbd> to toggle anytime
        </p>
      </div>

      {/* Product Transformation Demo Card */}
      <div className="max-w-4xl mx-auto mt-14 sm:mt-18">
        <div className="bg-white rounded-3xl border border-ink/15 shadow-popover p-5 sm:p-8 space-y-6">
          {/* Demo Card Top Controls */}
          <div className="flex items-center justify-between border-b border-ink/10 pb-4">
            <div className="flex items-center space-x-2">
              <div className="w-3 h-3 rounded-full bg-rose-400" />
              <div className="w-3 h-3 rounded-full bg-amber-400" />
              <div className="w-3 h-3 rounded-full bg-emerald-400" />
              <span className="text-xs font-mono font-medium text-ink-muted ml-2">
                terminal ~ claude-code / codex / omp
              </span>
            </div>

            {/* Interactive Step Switcher */}
            <div className="inline-flex rounded-xl p-1 bg-canvas-muted/70 text-xs font-sans font-medium text-ink-soft">
              <button
                onClick={() => setActiveStep(0)}
                className={`px-3 py-1 rounded-lg transition-all cursor-pointer ${
                  activeStep === 0 ? "bg-white text-ink shadow-subtle font-semibold" : "hover:text-ink"
                }`}
              >
                1. Prompt Submitted
              </button>
              <button
                onClick={() => setActiveStep(1)}
                className={`px-3 py-1 rounded-lg transition-all cursor-pointer ${
                  activeStep === 1 ? "bg-white text-ink shadow-subtle font-semibold" : "hover:text-ink"
                }`}
              >
                2. Micro-Game Pops Up
              </button>
              <button
                onClick={() => setActiveStep(2)}
                className={`px-3 py-1 rounded-lg transition-all cursor-pointer ${
                  activeStep === 2 ? "bg-white text-ink shadow-subtle font-semibold" : "hover:text-ink"
                }`}
              >
                3. Work Finished
              </button>
            </div>
          </div>

          {/* Interactive Stage Content */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 items-center">
            {/* Left: Terminal Output */}
            <div className="bg-[#121217] rounded-2xl p-5 text-left font-mono text-xs text-zinc-300 space-y-3 border border-zinc-800 shadow-inner min-h-[220px] flex flex-col justify-between">
              <div className="space-y-2">
                <div className="flex items-center justify-between text-zinc-400 text-[11px]">
                  <span>bash · claude 3.7 sonnet</span>
                  <span className="inline-flex items-center text-emerald-400">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse mr-1.5" />
                    Agent Active
                  </span>
                </div>
                <div className="text-zinc-100 font-semibold">
                  $ claude-code "refactor auth to use OAuth 2.0 PKCE flow"
                </div>
                {activeStep === 0 && (
                  <p className="text-amber-300/90 text-[11px] animate-pulse">
                    → Submitting prompt to LLM... (Estimated 38s latency)
                  </p>
                )}
                {activeStep >= 1 && (
                  <div className="text-zinc-400 text-[11px] space-y-1">
                    <p className="text-cyan-400">⚡ Vibe Play hooked: task_started</p>
                    <p>• Generating cryptographic state & PKCE code_verifier...</p>
                    <p>• Writing loopback redirect listener on port 3150...</p>
                    {activeStep === 2 && (
                      <p className="text-emerald-400 font-bold">✓ Refactor complete in 34.2s (4 files modified)</p>
                    )}
                  </div>
                )}
              </div>

              <div className="pt-3 border-t border-zinc-800/80 flex items-center justify-between text-[11px] text-zinc-400">
                <span>Hook status: active</span>
                <span className="text-zinc-300">Hotkey: Alt+W</span>
              </div>
            </div>

            {/* Right: Vibe Play Flow Widget & Interactive Mini-Game */}
            <div className="bg-canvas-muted/40 rounded-2xl p-5 border border-ink/10 flex flex-col items-center justify-center min-h-[220px] space-y-4">
              {activeStep === 0 && (
                <div className="text-center space-y-3 py-6">
                  <div className="w-12 h-12 rounded-full bg-forest/10 text-forest flex items-center justify-center mx-auto animate-pulse">
                    <RotateCw className="w-6 h-6 animate-spin" />
                  </div>
                  <div>
                    <h3 className="font-serif text-xl font-normal text-ink">Detecting Agent Activity</h3>
                    <p className="text-xs font-sans text-ink-soft max-w-xs mt-1">
                      The floating Wispr Flow desktop button detects your prompt and prepares the micro-game.
                    </p>
                  </div>
                </div>
              )}

              {activeStep === 1 && (
                <div className="w-full space-y-3">
                  <div className="flex items-center justify-between px-1">
                    <div className="flex items-center space-x-2">
                      <span className="font-serif font-normal text-lg text-ink">Focus Memory Grid</span>
                      <span className="text-[10px] font-sans font-semibold px-2 py-0.5 rounded-full bg-forest text-canvas">
                        Playing
                      </span>
                    </div>
                    <span className="text-xs font-mono font-medium text-forest">⏱ 0:14</span>
                  </div>

                  {/* 4x2 Mini-Grid Demo */}
                  <div className="grid grid-cols-4 gap-2">
                    {[
                      { icon: "🎮", label: "Controller" },
                      { icon: "⚡", label: "Lightning" },
                      { icon: "🧩", label: "Puzzle" },
                      { icon: "🎯", label: "Target" },
                      { icon: "🎮", label: "Controller" },
                      { icon: "⚡", label: "Lightning" },
                      { icon: "🧩", label: "Puzzle" },
                      { icon: "🎯", label: "Target" },
                    ].map((card, i) => (
                      <button
                        key={i}
                        onClick={() => toggleCard(i)}
                        className={`h-12 rounded-xl flex items-center justify-center text-lg font-bold border transition-all cursor-pointer ${
                          flippedCards.includes(i)
                            ? "bg-white border-forest/30 shadow-subtle scale-102"
                            : "bg-white/60 border-ink/10 text-transparent hover:bg-white"
                        }`}
                      >
                        {flippedCards.includes(i) ? card.icon : "?"}
                      </button>
                    ))}
                  </div>

                  <p className="text-[11px] font-sans text-ink-muted text-center pt-1">
                    Click cards to flip • Auto-stops the instant agent finishes
                  </p>
                </div>
              )}

              {activeStep === 2 && (
                <div className="text-center space-y-3 py-4">
                  <div className="w-12 h-12 rounded-full bg-emerald-500/10 text-emerald-700 flex items-center justify-center mx-auto">
                    <CheckCircle2 className="w-6 h-6" />
                  </div>
                  <div>
                    <h3 className="font-serif text-2xl font-normal text-ink">Agent Finished!</h3>
                    <p className="text-xs font-sans text-ink-soft max-w-xs mt-1">
                      Score: <span className="font-semibold text-ink">140 pts</span> • 8 moves in 34s.
                      Focus maintained without Twitter or Slack distraction.
                    </p>
                  </div>
                  <button
                    onClick={() => setActiveStep(1)}
                    className="text-xs font-sans font-semibold px-4 py-2 rounded-xl bg-lavender text-ink border border-ink/15 hover:bg-lavender-hover transition-colors cursor-pointer"
                  >
                    Play Again
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
