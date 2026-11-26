import React, { useState } from "react";
import { Gamepad2, Brain, Zap, Sparkles, Trophy, Check, MousePointer } from "lucide-react";

export const PersonalizationSection: React.FC = () => {
  const [selectedGame, setSelectedGame] = useState<"memory" | "math" | "reflex">("memory");
  const [mathAnswer, setMathAnswer] = useState<number | null>(null);
  const [mathScore, setMathScore] = useState(0);
  const [isHoveringWidget, setIsHoveringWidget] = useState(false);
  const [widgetEnabled, setWidgetEnabled] = useState(true);

  return (
    <section id="games" className="w-full py-20 px-4 sm:px-8 bg-canvas-muted/40 border-y border-ink/10">
      <div className="max-w-6xl mx-auto space-y-12">
        {/* Section Header */}
        <div className="max-w-2xl text-left space-y-3">
          <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest">
            Micro-Game Suite & Desktop Companion
          </span>
          <h2 className="font-serif font-normal text-4xl sm:text-5xl lg:text-6xl text-ink leading-tight">
            Lightweight challenges designed for 45-second latency.
          </h2>
          <p className="font-sans text-base sm:text-lg text-ink-soft leading-relaxed">
            Every game is calibrated for short agent runtimes: quick cognitive engagement, satisfying completions, and immediate exit handoffs without frustrating resets.
          </p>
        </div>

        {/* Two-Column Showcase */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
          {/* Left Column: Interactive Micro-Game Sandbox */}
          <div className="lg:col-span-7 bg-white rounded-3xl border border-ink/15 p-6 sm:p-8 shadow-popover space-y-6 text-left">
            {/* Game Selector Tabs */}
            <div className="flex items-center space-x-2 border-b border-ink/10 pb-4 overflow-x-auto">
              <button
                onClick={() => setSelectedGame("memory")}
                className={`px-3.5 py-1.5 rounded-xl text-xs font-sans font-semibold transition-all cursor-pointer ${
                  selectedGame === "memory"
                    ? "bg-forest text-canvas shadow-subtle"
                    : "bg-canvas text-ink-soft hover:text-ink"
                }`}
              >
                🎮 Memory Grid
              </button>
              <button
                onClick={() => setSelectedGame("math")}
                className={`px-3.5 py-1.5 rounded-xl text-xs font-sans font-semibold transition-all cursor-pointer ${
                  selectedGame === "math"
                    ? "bg-forest text-canvas shadow-subtle"
                    : "bg-canvas text-ink-soft hover:text-ink"
                }`}
              >
                ⚡ Math Sprint
              </button>
              <button
                onClick={() => setSelectedGame("reflex")}
                className={`px-3.5 py-1.5 rounded-xl text-xs font-sans font-semibold transition-all cursor-pointer ${
                  selectedGame === "reflex"
                    ? "bg-forest text-canvas shadow-subtle"
                    : "bg-canvas text-ink-soft hover:text-ink"
                }`}
              >
                🎯 Reflex Challenge
              </button>
            </div>

            {/* Sandbox Stage */}
            {selectedGame === "memory" && (
              <div className="space-y-4">
                <div className="flex items-center justify-between text-xs font-sans text-ink-soft">
                  <span>Match all symbol pairs before agent finishes</span>
                  <span className="font-mono text-forest font-semibold">120 pts</span>
                </div>
                <div className="grid grid-cols-4 gap-2.5">
                  {["🚀", "👾", "💎", "⚡", "🚀", "👾", "💎", "⚡"].map((sym, idx) => (
                    <button
                      key={idx}
                      className="h-14 rounded-2xl bg-canvas border border-ink/10 flex items-center justify-center text-xl font-bold hover:border-forest/40 hover:bg-white hover:shadow-subtle transition-all cursor-pointer"
                    >
                      {sym}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {selectedGame === "math" && (
              <div className="space-y-4 py-2">
                <div className="flex items-center justify-between text-xs font-sans text-ink-soft">
                  <span>Quick mental math calculation</span>
                  <span className="font-mono text-forest font-semibold">Streak: {mathScore}</span>
                </div>
                <div className="text-center py-4 bg-canvas/60 rounded-2xl border border-ink/10 space-y-4">
                  <div className="font-serif text-3xl text-ink font-normal">
                    17 × 4 + 18 = <span className="text-forest font-semibold">?</span>
                  </div>
                  <div className="flex items-center justify-center gap-3">
                    {[76, 86, 94].map((choice) => (
                      <button
                        key={choice}
                        onClick={() => {
                          setMathAnswer(choice);
                          if (choice === 86) setMathScore((s) => s + 1);
                        }}
                        className={`px-5 py-2.5 rounded-xl font-sans font-semibold text-sm border transition-all cursor-pointer ${
                          mathAnswer === choice
                            ? choice === 86
                              ? "bg-emerald-600 text-white border-emerald-600 shadow-subtle"
                              : "bg-rose-600 text-white border-rose-600"
                            : "bg-white text-ink border-ink/15 hover:border-ink/30 hover:shadow-subtle"
                        }`}
                      >
                        {choice}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}

            {selectedGame === "reflex" && (
              <div className="space-y-4 py-2 text-center">
                <div className="p-8 rounded-2xl bg-emerald-500/10 border border-emerald-600/20 text-emerald-800 space-y-2">
                  <Zap className="w-8 h-8 mx-auto text-emerald-600" />
                  <div className="font-serif text-2xl">Click when Green!</div>
                  <p className="text-xs font-sans text-emerald-700">Average developer latency: 194ms</p>
                </div>
              </div>
            )}
          </div>

          {/* Right Column: Wispr Flow Floating Widget Live Showcase */}
          <div className="lg:col-span-5 bg-white rounded-3xl border border-ink/15 p-6 sm:p-8 shadow-popover space-y-6 text-left">
            <div className="space-y-2">
              <span className="text-[11px] font-sans font-semibold uppercase tracking-wider text-forest">
                Desktop Architecture
              </span>
              <h3 className="font-serif text-2xl font-normal text-ink">
                Wispr Flow Floating Widget
              </h3>
              <p className="text-xs font-sans text-ink-soft leading-relaxed">
                Always-on-top, draggable floating companion widget. Stays resting as a slender 10px capsule outline until hovered.
              </p>
            </div>

            {/* Interactive Widget Demonstration */}
            <div
              onMouseEnter={() => setIsHoveringWidget(true)}
              onMouseLeave={() => setIsHoveringWidget(false)}
              className="bg-[#141419] rounded-2xl p-8 border border-zinc-800 relative min-h-[170px] flex items-center justify-center transition-all cursor-default select-none shadow-inner"
            >
              <div className="flex items-center space-x-3">
                {/* Fading Text Pill */}
                <div
                  onClick={() => setWidgetEnabled(!widgetEnabled)}
                  className={`px-3 py-1.5 rounded-full bg-[#181820] border border-zinc-700 text-xs font-sans cursor-pointer transition-all duration-200 flex items-center space-x-1.5 shadow-subtle ${
                    isHoveringWidget
                      ? "opacity-100 translate-x-0"
                      : "opacity-0 translate-x-2 pointer-events-none"
                  }`}
                >
                  <span className="font-semibold text-white">Vibe Play</span>
                  <span className={`font-bold ${widgetEnabled ? "text-cyan-400" : "text-zinc-400"}`}>
                    {widgetEnabled ? "ON" : "OFF"}
                  </span>
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      widgetEnabled ? "bg-cyan-400 animate-pulse" : "bg-zinc-500"
                    }`}
                  />
                </div>

                {/* Vertical Button / Resting Capsule */}
                {!isHoveringWidget ? (
                  <div
                    className="w-2.5 h-12 rounded-full border-[1.5px] border-zinc-400 bg-zinc-800/80 shadow-subtle transition-all"
                    title="Hover to expand Wispr Flow widget"
                  />
                ) : (
                  <button
                    onClick={() => setWidgetEnabled(!widgetEnabled)}
                    className="w-7 h-7 rounded-full bg-zinc-800 border border-zinc-700 hover:border-zinc-500 flex items-center justify-center text-cyan-400 transition-all cursor-pointer shadow-subtle"
                    title="Click to toggle Vibe Play"
                  >
                    <span className="text-xs">⏻</span>
                  </button>
                )}
              </div>

              <div className="absolute bottom-2.5 text-[10px] font-sans text-zinc-500 flex items-center space-x-1">
                <MousePointer className="w-3 h-3" />
                <span>Hover over the widget to test live expansion</span>
              </div>
            </div>

            <div className="text-xs font-sans text-ink-soft space-y-1.5 pt-1">
              <div className="flex items-center space-x-2">
                <Check className="w-3.5 h-3.5 text-forest" />
                <span>Alt+W global hotkey toggles state anywhere</span>
              </div>
              <div className="flex items-center space-x-2">
                <Check className="w-3.5 h-3.5 text-forest" />
                <span>Dockable to Left, Right, Bottom, or Free Floating</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
