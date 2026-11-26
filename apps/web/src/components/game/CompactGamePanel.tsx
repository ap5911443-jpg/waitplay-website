import React, { useState } from "react";
import {
  Minimize2,
  Maximize2,
  X,
  Gamepad2,
  Expand,
  Clock,
  Film,
  Tv,
} from "lucide-react";
import { AgentStatus, UserPreferences, GameScoreRecord } from "@waitplay/shared";
import { GameContainer } from "./GameContainer";
import { AgentStatusBadge } from "../ui/Badge";
import { InstagramExperience } from "../platforms/Instagram/InstagramExperience";
import { YouTubeExperience } from "../platforms/YouTube/YouTubeExperience";

export type CompanionExperienceTab = "games" | "instagram" | "youtube";

export interface CompactGamePanelProps {
  isOpen: boolean;
  isMinimized: boolean;
  agentStatus: AgentStatus;
  taskElapsedSeconds: number;
  preferences: UserPreferences;
  isPaused: boolean;
  initialTab?: CompanionExperienceTab;
  onToggleMinimize: () => void;
  onSwitchToFullscreen: () => void;
  onClose: () => void;
  onGameComplete: (score: GameScoreRecord) => void;
}

export const CompactGamePanel: React.FC<CompactGamePanelProps> = ({
  isOpen,
  isMinimized,
  agentStatus,
  taskElapsedSeconds,
  preferences,
  isPaused,
  initialTab = "instagram",
  onToggleMinimize,
  onSwitchToFullscreen,
  onClose,
  onGameComplete,
}) => {
  const [activeTab, setActiveTab] = useState<CompanionExperienceTab>(initialTab);

  if (!isOpen) return null;

  const formatTaskTime = (sec: number) => {
    const mins = Math.floor(sec / 60);
    const remainder = sec % 60;
    return `${mins}:${remainder.toString().padStart(2, "0")}`;
  };

  return (
    <div
      className={`fixed z-40 right-4 bottom-4 transition-all duration-200 ease-out shadow-popover rounded-3xl border border-ink/15 bg-canvas/98 text-ink backdrop-blur-md overflow-hidden ${
        isMinimized
          ? "w-72 h-14"
          : "w-[440px] sm:w-[480px] h-[640px] max-h-[calc(100vh-32px)] flex flex-col"
      }`}
    >
      {/* Floating Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-canvas-muted/40 border-b border-ink/10 shrink-0">
        <div className="flex items-center space-x-2.5">
          <div className="w-7 h-7 rounded-xl bg-forest text-canvas flex items-center justify-center shadow-subtle">
            <Gamepad2 className="w-4 h-4" />
          </div>
          <div>
            <div className="flex items-center space-x-1.5">
              <span className="text-sm font-serif font-normal text-ink">Vibe Play</span>
              <AgentStatusBadge status={agentStatus} />
            </div>
            {taskElapsedSeconds > 0 && (
              <div className="flex items-center space-x-1 text-[10px] text-ink-soft font-mono mt-0.5">
                <Clock className="w-3 h-3 text-forest" />
                <span>Agent time: {formatTaskTime(taskElapsedSeconds)}</span>
              </div>
            )}
          </div>
        </div>

        {/* Experience switcher icons when not minimized */}
        {!isMinimized && (
          <div className="flex items-center bg-white border border-ink/10 rounded-xl p-0.5 shadow-subtle">
            <button
              onClick={() => setActiveTab("games")}
              className={`p-1.5 rounded-lg transition-colors cursor-pointer ${
                activeTab === "games"
                  ? "bg-forest text-canvas"
                  : "text-ink-soft hover:text-ink"
              }`}
              title="Games"
            >
              <Gamepad2 className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setActiveTab("instagram")}
              className={`p-1.5 rounded-lg transition-colors cursor-pointer ${
                activeTab === "instagram"
                  ? "bg-gradient-to-r from-[#833ab4] via-[#fd1d1d] to-[#fcb045] text-white"
                  : "text-ink-soft hover:text-ink"
              }`}
              title="Instagram"
            >
              <Film className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setActiveTab("youtube")}
              className={`p-1.5 rounded-lg transition-colors cursor-pointer ${
                activeTab === "youtube"
                  ? "bg-[#CC0000] text-white"
                  : "text-ink-soft hover:text-ink"
              }`}
              title="YouTube"
            >
              <Tv className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        <div className="flex items-center space-x-1">
          <button
            onClick={onSwitchToFullscreen}
            className="p-1.5 rounded-lg hover:bg-canvas-muted text-ink-soft hover:text-ink transition-colors cursor-pointer"
            title="Full Screen Mode"
          >
            <Expand className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onToggleMinimize}
            className="p-1.5 rounded-lg hover:bg-canvas-muted text-ink-soft hover:text-ink transition-colors cursor-pointer"
            title={isMinimized ? "Expand Companion" : "Minimize"}
          >
            {isMinimized ? <Maximize2 className="w-3.5 h-3.5" /> : <Minimize2 className="w-3.5 h-3.5" />}
          </button>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-rose-100 hover:text-rose-600 text-ink-soft transition-colors cursor-pointer"
            title="Close Companion"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Main Experience Container if not minimized */}
      {!isMinimized && (
        <div className={`flex-1 overflow-hidden flex flex-col relative ${activeTab === "games" ? "bg-canvas" : "bg-[#0C0F12]"}`}>
          {activeTab === "games" && (
            <div className="p-4 flex-1 overflow-auto flex items-center justify-center">
              <GameContainer
                preferences={preferences}
                isPaused={isPaused}
                onGameComplete={onGameComplete}
                onGameQuit={onClose}
              />
            </div>
          )}

          {activeTab === "instagram" && (
            <div className="w-full h-full">
              <InstagramExperience />
            </div>
          )}

          {activeTab === "youtube" && (
            <div className="w-full h-full">
              <YouTubeExperience />
            </div>
          )}
        </div>
      )}
    </div>
  );
};
