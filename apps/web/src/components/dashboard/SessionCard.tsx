import React from "react";
import { FolderGit2, CheckSquare, Clock, Layers } from "lucide-react";
import { AgentSessionInfo } from "@waitplay/shared";
import { Card } from "../ui/Card";

export interface SessionCardProps {
  sessions: AgentSessionInfo[];
  activeSession: AgentSessionInfo | null;
  onSelectSession: (sessionId: string) => void;
}

export const SessionCard: React.FC<SessionCardProps> = ({
  sessions,
  activeSession,
  onSelectSession,
}) => {
  const formatTime = (ms?: number) => {
    if (!ms) return "0s";
    const sec = Math.round(ms / 1000);
    return `${sec}s`;
  };

  return (
    <Card
      title="Session Information"
      subtitle="Claude Code session metadata and multi-session routing"
      icon={<FolderGit2 className="w-[18px] h-[18px]" />}
      action={
        sessions.length > 1 ? (
          <select
            value={activeSession?.sessionId ?? ""}
            onChange={(e) => onSelectSession(e.target.value)}
            className="bg-white border border-ink/25 text-ink text-[13px] rounded-lg px-2.5 py-1.5 focus:outline-none focus-visible:ring-2 focus-visible:ring-forest"
          >
            {sessions.map((s) => (
              <option key={s.sessionId} value={s.sessionId}>
                {s.sessionId.slice(0, 16)} ({s.status})
              </option>
            ))}
          </select>
        ) : undefined
      }
    >
      <div className="grid grid-cols-1 gap-3 h-full">
        <div className="px-4 py-3 bg-canvas rounded-2xl border border-ink/10 min-w-0 flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 text-ink-soft text-xs font-semibold">
            <Layers className="w-4 h-4 text-forest" />
            <span>Session ID</span>
          </div>
          <div className="font-mono text-[13px] text-ink font-medium truncate min-w-0">
            {activeSession?.sessionId || "standalone-local"}
          </div>
        </div>

        <div className="px-4 py-3 bg-canvas rounded-2xl border border-ink/10 flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 text-ink-soft text-xs font-semibold">
            <CheckSquare className="w-4 h-4 text-forest" />
            <span>Tasks Finished</span>
          </div>
          <div className="font-serif text-[30px] leading-9 text-ink lining-nums tabular-nums">
            {activeSession?.totalTasksCompleted ?? 0}
          </div>
        </div>

        <div className="px-4 py-3 bg-canvas rounded-2xl border border-ink/10 flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 text-ink-soft text-xs font-semibold">
            <Clock className="w-4 h-4 text-forest" />
            <span>Last Task Time</span>
          </div>
          <div className="font-serif text-[30px] leading-9 text-ink lining-nums tabular-nums">
            {formatTime(activeSession?.lastTaskDurationMs)}
          </div>
        </div>
      </div>
    </Card>
  );
};
