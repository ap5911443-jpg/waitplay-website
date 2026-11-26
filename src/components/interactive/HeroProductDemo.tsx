import React, { useState, useEffect } from "react";
import { RotateCcw, Terminal, Gamepad2, ArrowRight, Video, ShieldCheck } from "lucide-react";
import { WisprFloatingPillWidget } from "./WisprFloatingPillWidget";
import { PlayableMemoryGame } from "./PlayableMemoryGame";

export const HeroProductDemo: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"interactive" | "video">("interactive");
  const [agentState, setAgentState] = useState<"idle" | "running" | "completed">("running");
  const [seconds, setSeconds] = useState(18);

  // Timer simulation
  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (agentState === "running") {
      timer = setInterval(() => {
        setSeconds((s) => {
          if (s >= 32) {
            setAgentState("completed");
            return 32;
          }
          return s + 1;
        });
      }, 1000);
    }
    return () => clearInterval(timer);
  }, [agentState]);

  const handleStartSim = () => {
    setSeconds(0);
    setAgentState("running");
  };

  const handleReturnTerminal = () => {
    setAgentState("idle");
  };

  return (
    <div className="w-full max-w-[1080px] mx-auto mt-10 sm:mt-14 select-none">
      {/* Tab Switcher */}
      <div className="flex items-center justify-between mb-3 px-2">
        <div className="inline-flex p-1 rounded-xl bg-[#E4E4D0]/70 border border-[rgba(26,26,26,0.1)] text-[13px] font-semibold">
          <button
            onClick={() => setActiveTab("interactive")}
            className={`px-3.5 py-1.5 rounded-lg transition-all cursor-pointer flex items-center gap-1.5 ${
              activeTab === "interactive"
                ? "bg-[#FFFFFF] text-[#1A1A1A] shadow-sm"
                : "text-[#666666] hover:text-[#1A1A1A]"
            }`}
          >
            <Gamepad2 className="w-3.5 h-3.5 text-[#034F46]" />
            <span>Interactive Simulator</span>
          </button>
          <button
            onClick={() => setActiveTab("video")}
            className={`px-3.5 py-1.5 rounded-lg transition-all cursor-pointer flex items-center gap-1.5 ${
              activeTab === "video"
                ? "bg-[#FFFFFF] text-[#1A1A1A] shadow-sm"
                : "text-[#666666] hover:text-[#1A1A1A]"
            }`}
          >
            <Video className="w-3.5 h-3.5 text-[#FFA946]" />
            <span>Launch Film (25s)</span>
          </button>
        </div>

        <div className="hidden sm:flex items-center gap-2 text-[12px] font-medium text-[#666666]">
          <span className="w-2 h-2 rounded-full bg-[#034F46]" />
          <span>Simulated Agent Bridge: 127.0.0.1:3150</span>
        </div>
      </div>

      {/* Main Card Frame */}
      <div className="bg-[#FFFFFF] rounded-[24px] sm:rounded-[28px] border border-[rgba(26,26,26,0.14)] shadow-[0_12px_40px_rgba(26,26,15,0.07)] overflow-hidden">
        {/* Mock Window Titlebar */}
        <div className="px-5 py-3.5 bg-[#F9F9F0] border-b border-[rgba(26,26,26,0.08)] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[#FF5F56] border border-[#E0443E]" />
            <span className="w-3 h-3 rounded-full bg-[#FFBD2E] border border-[#DEA123]" />
            <span className="w-3 h-3 rounded-full bg-[#27C93F] border border-[#1AAB29]" />
            <span className="ml-3 font-mono text-[12px] text-[#666666] flex items-center gap-1.5">
              <Terminal className="w-3 h-3 text-[#1A1A1A]" />
              ~/workspace/billing-api — claude-code + waitplay
            </span>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-[11px] font-mono uppercase tracking-wider px-2 py-0.5 rounded bg-[#E4E4D0] text-[#1A1A1A] font-semibold">
              {agentState === "running" ? "AGENT REASONING" : agentState === "completed" ? "FINISHED" : "IDLE"}
            </span>
            {agentState === "running" && (
              <span className="text-[12px] font-mono font-bold text-[#034F46]">
                {seconds}s
              </span>
            )}
          </div>
        </div>

        {/* Tab Content */}
        {activeTab === "video" ? (
          /* Video Player View */
          <div className="relative aspect-video w-full bg-[#000000] flex items-center justify-center">
            <video
              src="/launch-video.mp4"
              controls
              autoPlay
              loop
              muted
              playsInline
              className="w-full h-full object-contain"
            />
          </div>
        ) : (
          /* Interactive Simulator View */
          <div className="p-5 sm:p-7 md:p-8 bg-[#FAFAF3] relative min-h-[440px] flex flex-col justify-between">
            {/* Terminal Background Snippet */}
            <div className="bg-[#1A1A1A] text-[#FFFFEB] rounded-[16px] p-4 sm:p-5 font-mono text-[12.5px] sm:text-[13.5px] leading-[1.6] shadow-sm mb-6 border border-[rgba(255,255,255,0.08)]">
              <div className="flex items-center justify-between pb-2 mb-2 border-b border-[rgba(255,255,255,0.1)] text-[11px] text-[#888888]">
                <span>AGENT STDIN/STDOUT</span>
                <span>HOOK: claude-code-hook.cjs</span>
              </div>
              <div className="text-[#A0A090]">
                $ claude-code "Refactor stripe webhook signature verification and add unit tests"
              </div>
              <div className="text-[#00f3ff] mt-1">
                ❯ Inspecting src/services/webhook.ts and src/test/stripe.test.ts...
              </div>
              <div className="text-[#FFA946] mt-0.5">
                ❯ Running bash: npm test -- --grep "StripeWebhook" (tool #3 of 4)
              </div>
              <div className="mt-2 text-[#888888] flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-[#00f3ff] animate-ping" />
                <span>[Vibe Play Event Dispatcher]: State working, duration {seconds}s</span>
              </div>
            </div>

            {/* Split Interactive View: Floating Pill + Micro-Game */}
            <div className="grid grid-cols-1 md:grid-cols-12 gap-5 items-start">
              {/* Left Column: Wispr Floating Pill Desktop Widget Simulation */}
              <div className="md:col-span-5 flex flex-col gap-3">
                <div className="p-4 rounded-[16px] bg-[#FFFFFF] border border-[rgba(26,26,26,0.12)] shadow-subtle">
                  <div className="flex items-center justify-between mb-3">
                    <span className="text-[12px] font-sans font-semibold uppercase tracking-wider text-[#666666]">
                      Desktop Floating Pill
                    </span>
                    <span className="text-[11px] text-[#888888] font-mono">Alt + W</span>
                  </div>
                  <div className="py-2 flex justify-center">
                    <WisprFloatingPillWidget
                      isAgentActive={agentState === "running"}
                      onOpenGame={() => setAgentState("running")}
                    />
                  </div>
                  <p className="text-[12px] text-[#666666] text-center mt-2">
                    Docked to screen edge. Click ON/OFF or Gamepad to toggle.
                  </p>
                </div>

                {/* Quick Simulation Controller */}
                <div className="p-4 rounded-[16px] bg-[#FFFFFF] border border-[rgba(26,26,26,0.12)] shadow-subtle flex flex-col gap-2.5">
                  <span className="text-[12px] font-sans font-semibold uppercase tracking-wider text-[#666666]">
                    Simulation Controls
                  </span>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleStartSim}
                      className="flex-1 py-2 px-3 rounded-[10px] bg-[#034F46] text-[#FFFFEB] text-[13px] font-semibold hover:bg-[#023932] transition-colors cursor-pointer flex items-center justify-center gap-1.5"
                    >
                      <RotateCcw className="w-3.5 h-3.5" />
                      Trigger Agent Run (32s)
                    </button>
                    {agentState === "running" && (
                      <button
                        onClick={() => setAgentState("completed")}
                        className="py-2 px-3 rounded-[10px] bg-[#E4E4D0] text-[#1A1A1A] text-[13px] font-semibold hover:bg-[#D4D4C0] transition-colors cursor-pointer"
                      >
                        Finish Now
                      </button>
                    )}
                  </div>
                </div>
              </div>

              {/* Right Column: Playable Micro-Game Panel or Completion Modal */}
              <div className="md:col-span-7">
                {agentState === "completed" ? (
                  /* Interactive Completion Dialog */
                  <div className="bg-[#FFFFFF] rounded-[20px] p-6 border-2 border-[#034F46] shadow-card flex flex-col justify-between animate-fadeIn">
                    <div>
                      <div className="flex items-center gap-2 mb-2 text-[#034F46]">
                        <ShieldCheck className="w-5 h-5 text-[#034F46]" />
                        <span className="font-sans font-bold text-[16px] text-[#1A1A1A]">
                          Agent Finished Execution
                        </span>
                      </div>
                      <p className="font-sans text-[14.5px] text-[#666666] leading-[1.5] mb-4">
                        Claude Code completed in <strong>32 seconds</strong>. 4 tool calls executed. 2 test suites passed. Your terminal is ready for diff review.
                      </p>

                      <div className="grid grid-cols-2 gap-3 mb-5 p-3 rounded-[12px] bg-[#FFFFEB] border border-[rgba(26,26,26,0.08)]">
                        <div>
                          <div className="text-[11px] text-[#888888] uppercase">Agent Duration</div>
                          <div className="text-[18px] font-serif font-bold text-[#034F46]">32.4s</div>
                        </div>
                        <div>
                          <div className="text-[11px] text-[#888888] uppercase">Focus Saved</div>
                          <div className="text-[18px] font-serif font-bold text-[#FFA946]">~15 mins</div>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-3">
                      <button
                        onClick={handleReturnTerminal}
                        className="flex-1 py-2.5 px-4 rounded-[12px] bg-[#F0D7FF] text-[#1A1A1A] font-semibold text-[14px] hover:bg-[#E8C4FF] transition-all cursor-pointer flex items-center justify-center gap-2 shadow-sm"
                      >
                        <span>Return to Terminal</span>
                        <ArrowRight className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => setAgentState("idle")}
                        className="py-2.5 px-4 rounded-[12px] bg-[#F5F5EA] text-[#666666] font-semibold text-[14px] hover:text-[#1A1A1A] hover:bg-[#EAEAD8] transition-colors cursor-pointer"
                      >
                        Keep Playing
                      </button>
                    </div>
                  </div>
                ) : (
                  /* Playable Memory Match Game */
                  <PlayableMemoryGame onGameComplete={() => {}} />
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
