import React, { useState } from "react";
import { Sparkles, AlertTriangle } from "lucide-react";

export const InteractiveSpeedSlider: React.FC = () => {
  const [activeMode, setActiveMode] = useState<"with" | "without">("with");

  return (
    <div className="w-full max-w-[1120px] mx-auto select-none">
      {/* Mode Switcher Buttons */}
      <div className="flex justify-center mb-8">
        <div className="inline-flex p-1.5 rounded-full bg-[#023932] border border-[rgba(255,255,235,0.2)]">
          <button
            onClick={() => setActiveMode("without")}
            className={`px-5 py-2 rounded-full text-[14px] font-semibold transition-all duration-200 cursor-pointer ${
              activeMode === "without"
                ? "bg-[#FFA946] text-[#1A1A1A] shadow-md"
                : "text-[#FFFFEB]/70 hover:text-[#FFFFEB]"
            }`}
          >
            Without Vibe Play (Broken Habit)
          </button>
          <button
            onClick={() => setActiveMode("with")}
            className={`px-5 py-2 rounded-full text-[14px] font-semibold transition-all duration-200 cursor-pointer ${
              activeMode === "with"
                ? "bg-[#F0D7FF] text-[#1A1A1A] shadow-md"
                : "text-[#FFFFEB]/70 hover:text-[#FFFFEB]"
            }`}
          >
            With Vibe Play (Micro-Flow)
          </button>
        </div>
      </div>

      {/* Comparison Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 items-stretch">
        {/* Card 1: Without Vibe Play */}
        <div
          className={`rounded-[28px] p-7 sm:p-9 transition-all duration-300 border ${
            activeMode === "without"
              ? "bg-[#023932] border-[#FFA946] shadow-[0_8px_32px_rgba(255,169,70,0.15)] scale-[1.01]"
              : "bg-[#023932]/60 border-[rgba(255,255,235,0.12)] opacity-75"
          }`}
        >
          <div className="flex items-center justify-between pb-4 mb-6 border-b border-[rgba(255,255,235,0.15)]">
            <div className="flex items-center gap-2 text-[#FFA946]">
              <AlertTriangle className="w-4 h-4" />
              <span className="font-sans font-semibold text-[13px] uppercase tracking-wider">
                The Distraction Trap
              </span>
            </div>
            <span className="text-[12px] font-mono text-[#FFFFEB]/60">Traditional Waiting</span>
          </div>

          <div className="mb-6">
            <div className="text-[56px] sm:text-[68px] font-serif font-normal text-[#FFFFEB] leading-none mb-1">
              15+ <span className="text-[28px] font-sans font-normal text-[#FFA946]">mins lost</span>
            </div>
            <p className="text-[15px] text-[#FFFFEB]/80 leading-[1.5]">
              Average time spent adrift after a 45-second agent turn triggers a subconscious tab-switch.
            </p>
          </div>

          {/* Timeline steps */}
          <div className="space-y-3 font-mono text-[13px] pt-4 border-t border-[rgba(255,255,235,0.1)]">
            <div className="p-3 rounded-[12px] bg-[#012520] text-[#FFFFEB]/70 flex items-start gap-2.5">
              <span className="text-[#FFA946]">00:15</span>
              <span>Agent is reasoning... Developer opens Twitter or Slack.</span>
            </div>
            <div className="p-3 rounded-[12px] bg-[#012520] text-[#FFFFEB]/70 flex items-start gap-2.5">
              <span className="text-[#FFA946]">00:42</span>
              <span>Agent finishes silently in terminal background. Unnoticed.</span>
            </div>
            <div className="p-3 rounded-[12px] bg-[#012520] text-[#FFA946] flex items-start gap-2.5">
              <span className="text-[#FFA946]">16:20</span>
              <span>Developer remembers prompt: "Wait, what was I coding again?"</span>
            </div>
          </div>
        </div>

        {/* Card 2: With Vibe Play */}
        <div
          className={`rounded-[28px] p-7 sm:p-9 transition-all duration-300 border ${
            activeMode === "with"
              ? "bg-[#023932] border-[#F0D7FF] shadow-[0_8px_32px_rgba(240,215,255,0.2)] scale-[1.01]"
              : "bg-[#023932]/60 border-[rgba(255,255,235,0.12)] opacity-75"
          }`}
        >
          <div className="flex items-center justify-between pb-4 mb-6 border-b border-[rgba(255,255,235,0.15)]">
            <div className="flex items-center gap-2 text-[#F0D7FF]">
              <Sparkles className="w-4 h-4 text-[#F0D7FF]" />
              <span className="font-sans font-semibold text-[13px] uppercase tracking-wider text-[#F0D7FF]">
                The Micro-Flow State
              </span>
            </div>
            <span className="text-[12px] font-mono text-[#F0D7FF] bg-[#034F46] px-2 py-0.5 rounded-full border border-[#639085]">
              Vibe Play Companion
            </span>
          </div>

          <div className="mb-6">
            <div className="text-[56px] sm:text-[68px] font-serif font-normal text-[#F0D7FF] leading-none mb-1">
              &lt; 1 <span className="text-[28px] font-sans font-normal text-[#FFFFEB]">sec return</span>
            </div>
            <p className="text-[15px] text-[#FFFFEB]/90 leading-[1.5]">
              Seamless transition back to terminal diff review the instant your agent finishes. Zero lost train of thought.
            </p>
          </div>

          {/* Timeline steps */}
          <div className="space-y-3 font-mono text-[13px] pt-4 border-t border-[rgba(255,255,235,0.1)]">
            <div className="p-3 rounded-[12px] bg-[#034F46] text-[#FFFFEB] flex items-start gap-2.5 border border-[#639085]/40">
              <span className="text-[#00f3ff]">00:03</span>
              <span>Agent hook fires. Memory match micro-game opens instantly.</span>
            </div>
            <div className="p-3 rounded-[12px] bg-[#034F46] text-[#FFFFEB] flex items-start gap-2.5 border border-[#639085]/40">
              <span className="text-[#00f3ff]">00:34</span>
              <span>Agent completes. Non-intrusive summary dialog alerts you.</span>
            </div>
            <div className="p-3 rounded-[12px] bg-[#034F46] text-[#F0D7FF] flex items-start gap-2.5 border border-[#F0D7FF]/40 font-semibold">
              <span className="text-[#F0D7FF]">00:35</span>
              <span>1-click "Return to Terminal" — brain remains fully locked into the PR.</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
