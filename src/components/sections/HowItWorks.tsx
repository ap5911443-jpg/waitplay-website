import React from "react";
import { SectionContainer } from "../layout/SectionContainer";
import { Eyebrow } from "../ui/Eyebrow";
import { EditorialHeading } from "../ui/EditorialHeading";
import { Gamepad2, ArrowRight } from "lucide-react";

export const HowItWorks: React.FC = () => {

  return (
    <SectionContainer id="how-it-works" bg="canvas" className="overflow-hidden">
      <div className="text-center max-w-[840px] mx-auto mb-14 sm:mb-20">
        <Eyebrow variant="lavender" dot className="mb-4">
          HOW IT WORKS
        </Eyebrow>

        <EditorialHeading level="section" className="mb-5">
          Stay in flow at the speed you think, <em className="italic font-normal">in every terminal and IDE.</em>
        </EditorialHeading>

        <p className="font-sans text-[17px] sm:text-[19px] text-[#666666] leading-[1.6]">
          Three thoughtful interactions engineered to keep your mind sharp while autonomous agents do the heavy lifting.
        </p>
      </div>

      {/* Chapters Container */}
      <div className="space-y-16 sm:space-y-24">
        {/* Chapter 1: Hook Naturally */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 sm:gap-12 items-center">
          <div className="lg:col-span-5 order-2 lg:order-1">
            <span className="font-mono text-[12px] font-bold text-[#034F46] tracking-widest uppercase mb-2 block">
              Chapter 01 — Seamless Hook
            </span>
            <EditorialHeading level="sub" className="mb-4">
              Hooks naturally into your agent's lifecycle.
            </EditorialHeading>
            <p className="font-sans text-[16px] text-[#666666] leading-[1.6] mb-6">
              Zero manual toggles. Vibe Play listens for prompt submission and tool call dispatch across Claude Code, Cursor, OMP, Codex, and Cline via localhost loopback sockets.
            </p>
            <div className="flex flex-wrap gap-2">
              <span className="px-3 py-1 rounded-full text-[12px] font-semibold bg-[#E4E4D0]/60 text-[#1A1A1A] border border-[rgba(26,26,26,0.08)]">
                ✓ Claude Code pre_tool_use
              </span>
              <span className="px-3 py-1 rounded-full text-[12px] font-semibold bg-[#E4E4D0]/60 text-[#1A1A1A] border border-[rgba(26,26,26,0.08)]">
                ✓ Cursor turn listener
              </span>
              <span className="px-3 py-1 rounded-full text-[12px] font-semibold bg-[#E4E4D0]/60 text-[#1A1A1A] border border-[rgba(26,26,26,0.08)]">
                ✓ OMP session watcher
              </span>
            </div>
          </div>

          <div className="lg:col-span-7 order-1 lg:order-2">
            <div className="bg-[#1A1A1A] rounded-[24px] p-5 sm:p-7 border border-[rgba(255,255,255,0.1)] shadow-card text-[#FFFFEB]">
              <div className="flex items-center justify-between pb-3 mb-4 border-b border-[rgba(255,255,255,0.1)] font-mono text-[12px] text-[#888888]">
                <div className="flex items-center gap-2">
                  <span className="w-2.5 h-2.5 rounded-full bg-[#034F46]" />
                  <span>integrations/claude-code/waitplay-hook.cjs</span>
                </div>
                <span>NODE.JS 24</span>
              </div>
              <pre className="font-mono text-[13px] sm:text-[14px] leading-[1.6] text-[#A0A090] overflow-x-auto">
                <code>{`// Hooks directly into prompt submission & tool execution
module.exports = async function onAgentEvent(event) {
  if (event.type === "pre_tool_use" || event.type === "session_start") {
    await bridgeClient.emit("task_started", {
      sessionId: event.session_id,
      agent: "claude-code",
      tool: event.tool_name, // e.g. "BashCommand", "EditAST"
      startDelayMs: 3000     // 3s smart delay threshold
    });
  }
};`}</code>
              </pre>
            </div>
          </div>
        </div>

        {/* Chapter 2: Playful Micro-Bursts */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 sm:gap-12 items-center">
          <div className="lg:col-span-7 order-1">
            <div className="bg-[#FFFFFF] rounded-[24px] p-6 sm:p-8 border border-[rgba(26,26,26,0.12)] shadow-card">
              <div className="flex items-center justify-between pb-3 mb-5 border-b border-[rgba(26,26,26,0.08)]">
                <span className="font-sans font-semibold text-[14px] text-[#1A1A1A] flex items-center gap-2">
                  <Gamepad2 className="w-4 h-4 text-[#034F46]" />
                  Vibe Play Micro-Suite
                </span>
                <span className="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-[#F0D7FF] text-[#1A1A1A]">
                  Fast 15-45s Sessions
                </span>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="p-4 rounded-[16px] bg-[#FFFFEB] border border-[rgba(26,26,26,0.08)]">
                  <div className="text-2xl mb-1">🧠</div>
                  <h4 className="font-sans font-semibold text-[15px] text-[#1A1A1A]">Memory Match</h4>
                  <p className="font-sans text-[12.5px] text-[#666666] mt-1">
                    Match developer symbols & logic pairs to sharpen working memory.
                  </p>
                </div>
                <div className="p-4 rounded-[16px] bg-[#FFFFEB] border border-[rgba(26,26,26,0.08)]">
                  <div className="text-2xl mb-1">⚡</div>
                  <h4 className="font-sans font-semibold text-[15px] text-[#1A1A1A]">Reaction Sprint</h4>
                  <p className="font-sans text-[12.5px] text-[#666666] mt-1">
                    Click millisecond targets to keep visual and cognitive reflexes primed.
                  </p>
                </div>
                <div className="p-4 rounded-[16px] bg-[#FFFFEB] border border-[rgba(26,26,26,0.08)]">
                  <div className="text-2xl mb-1">🔢</div>
                  <h4 className="font-sans font-semibold text-[15px] text-[#1A1A1A]">Math Blitz</h4>
                  <p className="font-sans text-[12.5px] text-[#666666] mt-1">
                    Quick binary & arithmetic mental math puzzles.
                  </p>
                </div>
                <div className="p-4 rounded-[16px] bg-[#FFFFEB] border border-[rgba(26,26,26,0.08)]">
                  <div className="text-2xl mb-1">🧩</div>
                  <h4 className="font-sans font-semibold text-[15px] text-[#1A1A1A]">Riddle Challenge</h4>
                  <p className="font-sans text-[12.5px] text-[#666666] mt-1">
                    Snappy 1-line algorithmic riddles to solve before tests pass.
                  </p>
                </div>
              </div>
            </div>
          </div>

          <div className="lg:col-span-5 order-2">
            <span className="font-mono text-[12px] font-bold text-[#034F46] tracking-widest uppercase mb-2 block">
              Chapter 02 — Micro-Play
            </span>
            <EditorialHeading level="sub" className="mb-4">
              Playful micro-bursts, calibrated for agent wait times.
            </EditorialHeading>
            <p className="font-sans text-[16px] text-[#666666] leading-[1.6] mb-6">
              Rather than letting your attention fracture across social media feeds, Vibe Play channels your cognitive energy into delightful 20-second puzzles that leave you alert and focused.
            </p>
            <div className="p-4 rounded-[16px] bg-[#E4E4D0]/60 border border-[rgba(26,26,26,0.1)] flex items-center gap-3">
              <span className="w-8 h-8 rounded-full bg-[#FFA946] flex items-center justify-center font-bold text-[#1A1A1A] text-sm">
                Alt
              </span>
              <span className="text-[13.5px] text-[#1A1A1A] font-medium">
                Press <strong>Alt + W</strong> anywhere to toggle companion on screen.
              </span>
            </div>
          </div>
        </div>

        {/* Chapter 3: Zero-Interruption Return */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 sm:gap-12 items-center">
          <div className="lg:col-span-5 order-2 lg:order-1">
            <span className="font-mono text-[12px] font-bold text-[#034F46] tracking-widest uppercase mb-2 block">
              Chapter 03 — Zero Interruption
            </span>
            <EditorialHeading level="sub" className="mb-4">
              Never steals focus. Return when you're ready.
            </EditorialHeading>
            <p className="font-sans text-[16px] text-[#666666] leading-[1.6] mb-6">
              When your agent finishes its turn, Vibe Play avoids intrusive popups that steal your cursor. A quiet completion badge appears with session duration metrics, letting you wrap up your game or hit Return to inspect diffs.
            </p>
            <div className="grid grid-cols-2 gap-3">
              <div className="p-3.5 rounded-[14px] bg-[#FFFFFF] border border-[rgba(26,26,26,0.1)] shadow-subtle">
                <div className="text-[11px] font-mono text-[#888888] uppercase">Agent Execution</div>
                <div className="text-[20px] font-serif font-bold text-[#034F46]">34s avg</div>
              </div>
              <div className="p-3.5 rounded-[14px] bg-[#FFFFFF] border border-[rgba(26,26,26,0.1)] shadow-subtle">
                <div className="text-[11px] font-mono text-[#888888] uppercase">Code Diff Return</div>
                <div className="text-[20px] font-serif font-bold text-[#1A1A1A]">1 click</div>
              </div>
            </div>
          </div>

          <div className="lg:col-span-7 order-1 lg:order-2">
            <div className="bg-[#FFFFFF] rounded-[24px] p-6 sm:p-8 border-2 border-[#034F46] shadow-card">
              <div className="flex items-center justify-between pb-3 mb-4 border-b border-[rgba(26,26,26,0.08)]">
                <div className="flex items-center gap-2">
                  <span className="w-2.5 h-2.5 rounded-full bg-[#034F46] animate-pulse" />
                  <span className="font-sans font-bold text-[14px] text-[#1A1A1A]">
                    Task Completed Notification
                  </span>
                </div>
                <span className="font-mono text-[12px] text-[#034F46] font-semibold">
                  Claude Code (pid: 4892)
                </span>
              </div>

              <div className="bg-[#FFFFEB] p-4 rounded-[14px] border border-[rgba(26,26,26,0.08)] mb-5">
                <p className="font-sans text-[14px] text-[#1A1A1A] leading-[1.5]">
                  "Refactor auth middleware complete. 3 files changed, 28 tests passing. Ready for your review."
                </p>
              </div>

              <div className="flex items-center gap-3">
                <button className="flex-1 py-3 px-4 rounded-[12px] bg-[#F0D7FF] text-[#1A1A1A] font-semibold text-[14.5px] hover:bg-[#E8C4FF] transition-colors flex items-center justify-center gap-2 cursor-pointer shadow-sm">
                  <span>Return to Terminal (Esc)</span>
                  <ArrowRight className="w-4 h-4" />
                </button>
                <button className="py-3 px-5 rounded-[12px] bg-[#F5F5EA] text-[#666666] font-semibold text-[14.5px] hover:text-[#1A1A1A] hover:bg-[#EAEAD8] transition-colors cursor-pointer">
                  Finish Game
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </SectionContainer>
  );
};
