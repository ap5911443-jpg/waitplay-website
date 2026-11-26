import React from "react";
import { Clock, Zap, ArrowRight, Brain, AlertTriangle, CheckCircle } from "lucide-react";

export const SpeedComparison: React.FC = () => {
  return (
    <section id="comparison" className="w-full py-20 px-4 sm:px-8 bg-canvas">
      <div className="max-w-6xl mx-auto">
        {/* Deep Forest-Green Contrast Panel as specified in Section 9.3 */}
        <div className="bg-forest text-canvas rounded-4xl p-8 sm:p-14 lg:p-18 shadow-popover space-y-12 border border-forest-soft/30">
          {/* Header */}
          <div className="max-w-2xl space-y-4 text-left">
            <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest-soft">
              Focus Retention Benchmark
            </span>
            <h2 className="font-serif font-normal text-4xl sm:text-5xl lg:text-6xl leading-[1.04] text-canvas">
              Developer latency is focus lost. <br />
              <span className="text-lavender italic">Vibe Play keeps you in the pocket.</span>
            </h2>
            <p className="font-sans text-base sm:text-lg text-canvas/80 leading-relaxed">
              When an AI agent runs a multi-step task, developers involuntarily switch tabs to Twitter, Slack, or YouTube. Gloria Mark’s research proves regaining deep coding focus takes 23 minutes.
            </p>
          </div>

          {/* Side-by-Side Metric Comparison */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            {/* Left: Traditional Idle Waiting */}
            <div className="bg-forest-dark/80 rounded-3xl p-6 sm:p-8 border border-white/10 space-y-6 flex flex-col justify-between">
              <div className="space-y-4">
                <div className="flex items-center justify-between text-xs font-sans font-semibold uppercase tracking-wider text-rose-300">
                  <div className="flex items-center space-x-1.5">
                    <AlertTriangle className="w-4 h-4" />
                    <span>Without Vibe Play</span>
                  </div>
                  <span className="text-white/60">Passive Latency</span>
                </div>

                <div className="space-y-1">
                  <div className="font-serif text-5xl sm:text-6xl font-normal text-rose-200">
                    23 <span className="text-2xl font-sans">mins</span>
                  </div>
                  <p className="text-sm font-sans text-white/70">Average time lost to regain focus after tab switching</p>
                </div>

                {/* Reality Sequence */}
                <div className="bg-black/25 rounded-2xl p-4 space-y-2.5 text-xs font-mono text-white/80 border border-white/5">
                  <div className="flex items-center text-rose-300">
                    <Clock className="w-3.5 h-3.5 mr-1.5 shrink-0" />
                    <span>0:00 — Submit complex refactor prompt</span>
                  </div>
                  <div className="text-white/50 pl-5">0:12 — Waiting... opens Twitter in 2nd monitor</div>
                  <div className="text-white/50 pl-5">0:45 — Agent finishes, but developer is in thread hole</div>
                  <div className="text-rose-400 font-semibold pl-5">23:00 — Finally remembers to review agent output</div>
                </div>
              </div>

              <div className="pt-4 border-t border-white/10 text-xs text-white/60 font-sans">
                Result: Fragmented flow, high cognitive fatigue, forgotten agent approvals.
              </div>
            </div>

            {/* Right: Vibe Play Flow */}
            <div className="bg-white/10 backdrop-blur-md rounded-3xl p-6 sm:p-8 border border-white/20 space-y-6 flex flex-col justify-between relative overflow-hidden">
              <div className="space-y-4">
                <div className="flex items-center justify-between text-xs font-sans font-semibold uppercase tracking-wider text-lavender">
                  <div className="flex items-center space-x-1.5">
                    <CheckCircle className="w-4 h-4 text-lavender" />
                    <span>With Vibe Play Flow</span>
                  </div>
                  <span className="text-white/80 font-bold">Zero Context Switch</span>
                </div>

                <div className="space-y-1">
                  <div className="font-serif text-5xl sm:text-6xl font-normal text-lavender">
                    0 <span className="text-2xl font-sans">sec</span>
                  </div>
                  <p className="text-sm font-sans text-white/90">Zero external tab switches. Cursor stays locked to terminal.</p>
                </div>

                {/* Flow Sequence */}
                <div className="bg-forest-dark/90 rounded-2xl p-4 space-y-2.5 text-xs font-mono text-white/90 border border-lavender/20 shadow-subtle">
                  <div className="flex items-center text-emerald-300">
                    <Zap className="w-3.5 h-3.5 mr-1.5 shrink-0" />
                    <span>0:00 — Submit prompt to Claude / Codex / OMP</span>
                  </div>
                  <div className="text-lavender font-semibold pl-5">0:01 — Micro-game pops up instantly (Memory / Math Sprint)</div>
                  <div className="text-emerald-300 pl-5">0:34 — Agent finishes! Game auto-stops immediately</div>
                  <div className="text-white font-bold pl-5">0:35 — Seamlessly review code with warmed-up cognitive focus</div>
                </div>
              </div>

              <div className="pt-4 border-t border-white/10 text-xs text-lavender font-sans font-medium flex items-center justify-between">
                <span>Result: Continuous flow state, rapid approvals, playful mastery.</span>
                <Brain className="w-4 h-4 text-lavender" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
