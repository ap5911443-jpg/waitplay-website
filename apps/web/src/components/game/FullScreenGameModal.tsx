import React, { useEffect, useState } from "react";
import { Shrink, X, Gamepad2, Clock, Film, Tv } from "lucide-react";
import { AgentStatus, UserPreferences, GameScoreRecord } from "@waitplay/shared";
import { GameContainer } from "./GameContainer";
import { AgentStatusBadge } from "../ui/Badge";
import { InstagramExperience } from "../platforms/Instagram/InstagramExperience";
import { YouTubeExperience } from "../platforms/YouTube/YouTubeExperience";

export type CompanionExperienceTab = "games" | "instagram" | "youtube";

export interface FullScreenGameModalProps {
  isOpen: boolean;
  agentStatus: AgentStatus;
  taskElapsedSeconds: number;
  preferences: UserPreferences;
  isPaused: boolean;
  initialTab?: CompanionExperienceTab;
  onExitFullscreen: () => void;
  onClose: () => void;
  onGameComplete: (score: GameScoreRecord) => void;
}

export const FullScreenGameModal: React.FC<FullScreenGameModalProps> = ({
  isOpen,
  agentStatus,
  taskElapsedSeconds,
  preferences,
  isPaused,
  initialTab = "instagram",
  onExitFullscreen,
  onClose,
  onGameComplete,
}) => {
  const [activeTab, setActiveTab] = useState<CompanionExperienceTab>(initialTab);

  useEffect(() => {
    if (initialTab) {
      setActiveTab(initialTab);
    }
  }, [initialTab]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onExitFullscreen();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onExitFullscreen]);

  if (!isOpen) return null;

  const formatTaskTime = (sec: number) => {
    const mins = Math.floor(sec / 60);
    const remainder = sec % 60;
    return `${mins}:${remainder.toString().padStart(2, "0")}`;
  };

  return (
    <div className={`fixed inset-0 z-50 flex flex-col animate-in fade-in duration-200 overflow-hidden ${activeTab === "games" ? "bg-canvas text-ink" : "bg-[#0C0F12] text-zinc-100"}`}>
      {/* Top Telemetry & Experience Switcher Bar */}
      <div className={`flex items-center justify-between px-4 sm:px-6 py-2.5 border-b backdrop-blur-md shrink-0 z-30 ${activeTab === "games" ? "bg-canvas/90 border-ink/10" : "bg-zinc-950/80 border-zinc-800"}`}>
        <div className="flex items-center space-x-3 sm:space-x-4">
          <div className="flex items-center space-x-2">
            <div className={`p-1.5 rounded-xl ${activeTab === "games" ? "bg-forest text-canvas" : "bg-blue-600/20 text-blue-400"}`}>
              <Gamepad2 className="w-4 h-4" />
            </div>
            <span className={`text-base font-serif font-normal hidden sm:inline ${activeTab === "games" ? "text-ink" : "text-zinc-100"}`}>Vibe Play</span>
          </div>

          <div className={`h-4 w-px ${activeTab === "games" ? "bg-ink/10" : "bg-zinc-800"}`} />

          {/* Telemetry Status */}
          <div className="flex items-center space-x-2">
            <span className={`text-xs hidden sm:inline ${activeTab === "games" ? "text-ink-soft" : "text-zinc-400"}`}>Agent:</span>
            <AgentStatusBadge status={agentStatus} />
          </div>

          {taskElapsedSeconds > 0 && (
            <div className={`hidden md:flex items-center space-x-1.5 text-xs font-mono ${activeTab === "games" ? "text-ink-soft" : "text-zinc-400"}`}>
              <Clock className="w-3.5 h-3.5 text-forest" />
              <span>Working: {formatTaskTime(taskElapsedSeconds)}</span>
            </div>
          )}
        </div>

        {/* Center Experience Tabs (Micro-Games | Instagram | YouTube) */}
        <div className={`flex items-center rounded-xl p-0.5 shadow-inner ${activeTab === "games" ? "bg-white border border-ink/10" : "bg-zinc-900/90 border border-zinc-800"}`}>
          <button
            onClick={() => setActiveTab("games")}
            className={`flex items-center space-x-1.5 px-3 py-1 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              activeTab === "games"
                ? "bg-forest text-canvas shadow-subtle"
                : "text-ink-soft hover:text-ink"
            }`}
          >
            <Gamepad2 className="w-3.5 h-3.5" />
            <span>Games</span>
          </button>

          <button
            onClick={() => setActiveTab("instagram")}
            className={`flex items-center space-x-1.5 px-3 py-1 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              activeTab === "instagram"
                ? "bg-gradient-to-r from-[#833ab4] via-[#fd1d1d] to-[#fcb045] text-white shadow-xs"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <Film className="w-3.5 h-3.5" />
            <span>Instagram</span>
          </button>

          <button
            onClick={() => setActiveTab("youtube")}
            className={`flex items-center space-x-1.5 px-3 py-1 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              activeTab === "youtube"
                ? "bg-[#CC0000] text-white shadow-xs"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <Tv className="w-3.5 h-3.5" />
            <span>YouTube</span>
          </button>
        </div>

        {/* Right Controls */}
        <div className="flex items-center space-x-2">
          <button
            onClick={onExitFullscreen}
            className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-xl text-xs font-medium transition-colors cursor-pointer border ${activeTab === "games" ? "bg-white hover:bg-canvas-muted text-ink border-ink/15 shadow-subtle" : "bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border-zinc-700"}`}
            title="Compact Mode (Esc)"
          >
            <Shrink className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Compact</span>
          </button>
          <button
            onClick={onClose}
            className={`p-1.5 rounded-xl transition-colors cursor-pointer ${activeTab === "games" ? "text-ink-soft hover:text-rose-600 hover:bg-rose-100" : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800"}`}
            title="Exit Companion"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Main Experience Body */}
      <div className="flex-1 w-full h-[calc(100vh-49px)] overflow-hidden relative">
        {activeTab === "games" && (
          <div className="w-full h-full overflow-auto flex items-center justify-center p-6 bg-canvas">
            <div className="w-full max-w-2xl bg-white border border-ink/15 rounded-3xl p-6 sm:p-8 shadow-popover">
              <GameContainer
                preferences={preferences}
                isPaused={isPaused}
                onGameComplete={onGameComplete}
                onGameQuit={onClose}
              />
            </div>
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
    </div>
  );
};
