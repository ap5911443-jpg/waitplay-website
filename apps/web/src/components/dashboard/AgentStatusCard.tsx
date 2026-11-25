import React from "react";
import { Activity, Clock, Play, AlertCircle, CheckCircle2, XCircle } from "lucide-react";
import { AgentStatus } from "@waitplay/shared";
import { Card } from "../ui/Card";
import { AgentStatusBadge } from "../ui/Badge";
import { Button } from "../ui/Button";

export interface AgentStatusCardProps {
  status: AgentStatus;
  taskElapsedSeconds: number;
  delayRemainingSeconds: number | null;
  waitingForInputPrompt?: string;
  lastError?: string;
  onOpenGame: () => void;
  isGameOpen: boolean;
}

export const AgentStatusCard: React.FC<AgentStatusCardProps> = ({
  status,
  taskElapsedSeconds,
  delayRemainingSeconds,
  waitingForInputPrompt,
  lastError,
  onOpenGame,
  isGameOpen,
}) => {
  const formatTime = (sec: number) => {
    const mins = Math.floor(sec / 60);
    const remainder = sec % 60;
    return `${mins}:${remainder.toString().padStart(2, "0")}`;
  };

  return (
    <Card
      title="Agent Telemetry"
      subtitle="Real-time Claude Code task state & game trigger coordinator"
      icon={<Activity className="w-[18px] h-[18px]" />}
      action={<AgentStatusBadge status={status} />}
    >
      <div className="space-y-4 h-full flex flex-col">
        {/* Main status indicator */}
        <div className="flex-1 flex flex-col sm:flex-row sm:items-center justify-between p-5 bg-canvas rounded-2xl border border-ink/10 gap-4">
          <div>
            <div className="text-[11px] font-semibold uppercase tracking-[0.08em] text-ink-soft">Current Agent State</div>
            <div className="font-serif text-[34px] leading-[1.1] font-normal text-ink mt-1 capitalize flex flex-wrap items-baseline gap-x-3">
              <span>{status.toLowerCase().replace(/_/g, " ")}</span>
              {status === "WORKING" && (
                <span className="text-xs font-semibold text-forest font-sans animate-pulse">
                  (Analyzing / Generating...)
                </span>
              )}
            </div>

            {/* Launch delay countdown */}
            {delayRemainingSeconds !== null && delayRemainingSeconds > 0 && (
              <div className="mt-3 inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-lavender border border-ink/10 text-ink text-[13px]">
                <Clock className="w-4 h-4 text-forest animate-spin" />
                <span>
                  Auto-launching game in <strong>{delayRemainingSeconds}s</strong> if task continues...
                </span>
              </div>
            )}
          </div>

          <div className="flex items-center gap-4">
            {taskElapsedSeconds > 0 && (
              <div className="text-right">
                <div className="text-[11px] font-semibold text-ink-soft uppercase tracking-[0.08em]">Duration</div>
                <div className="font-serif text-[26px] leading-8 text-ink lining-nums tabular-nums">
                  {formatTime(taskElapsedSeconds)}
                </div>
              </div>
            )}

            <Button
              variant="primary"
              size="md"
              icon={<Play className="w-4 h-4" />}
              onClick={onOpenGame}
            >
              {isGameOpen ? "Focus Game" : "Launch Game"}
            </Button>
          </div>
        </div>

        {/* Input prompt notice if waiting for input */}
        {status === "WAITING_FOR_INPUT" && (
          <div className="p-4 bg-[#FFF1DC] border border-orange/60 rounded-2xl text-[13px] text-ink flex items-start gap-3">
            <AlertCircle className="w-4 h-4 text-[#9A5600] shrink-0 mt-0.5" />
            <div>
              <strong className="font-semibold">Claude Code needs your confirmation:</strong>
              <p className="mt-0.5 text-ink-soft">
                {waitingForInputPrompt || "Tool execution or permission confirmation requested in your terminal."}
              </p>
            </div>
          </div>
        )}

        {/* Error message if failed */}
        {status === "FAILED" && (
          <div className="p-4 bg-[#FAEBE8] border border-[#9B2C1F]/30 rounded-2xl text-[13px] text-ink flex items-start gap-3">
            <XCircle className="w-4 h-4 text-[#9B2C1F] shrink-0 mt-0.5" />
            <div>
              <strong className="font-semibold">Task Failed:</strong>
              <p className="mt-0.5 text-ink-soft">{lastError || "The agent encountered an error during execution."}</p>
            </div>
          </div>
        )}

        {/* Completed notice */}
        {status === "COMPLETED" && (
          <div className="p-4 bg-[#E8F1EE] border border-forest/30 rounded-2xl text-[13px] text-ink flex items-start gap-3">
            <CheckCircle2 className="w-4 h-4 text-forest shrink-0 mt-0.5" />
            <div>
              <strong className="font-semibold">Turn Finished:</strong>
              <p className="mt-0.5 text-ink-soft">
                Claude Code completed its turn. Ready for the next instruction.
              </p>
            </div>
          </div>
        )}
      </div>
    </Card>
  );
};
