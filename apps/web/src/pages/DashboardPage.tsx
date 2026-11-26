import React, { useState } from "react";
import {
  Gamepad2,
  Settings as SettingsIcon,
  Moon,
  Sun,
  Sparkles,
  Film,
  Tv,
} from "lucide-react";
import { UserPreferences, GameScoreRecord } from "@waitplay/shared";
import { useBridgeConnection } from "../hooks/useBridgeConnection";
import { useAgentStateMachine } from "../hooks/useAgentStateMachine";
import { AgentStatusCard } from "../components/dashboard/AgentStatusCard";
import { ConnectionCard } from "../components/dashboard/ConnectionCard";
import { SessionCard } from "../components/dashboard/SessionCard";
import { RecentScoresCard } from "../components/dashboard/RecentScoresCard";
import { SimulatorCard } from "../components/dashboard/SimulatorCard";
import { CompactGamePanel } from "../components/game/CompactGamePanel";
import { FullScreenGameModal } from "../components/game/FullScreenGameModal";
import { GameResultModal } from "../components/game/GameResultModal";
import { SettingsModal } from "../components/settings/SettingsModal";

export interface DashboardPageProps {
  preferences: UserPreferences;
  onUpdatePreferences: (updated: UserPreferences) => void;
  onOpenShowcase?: () => void;
}

export const DashboardPage: React.FC<DashboardPageProps> = ({
  preferences,
  onUpdatePreferences,
  onOpenShowcase,
}) => {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const isPopup =
    typeof window !== "undefined" &&
    (new URLSearchParams(window.location.search).get("popup") === "1" ||
      new URLSearchParams(window.location.search).get("popup") === "true" ||
      new URLSearchParams(window.location.search).get("mode") === "fullscreen");

  const [overrideDisplayMode, setOverrideDisplayMode] = useState<"compact" | "fullscreen" | null>(
    isPopup ? "fullscreen" : null
  );
  const [companionTab, setCompanionTab] = useState<"games" | "instagram" | "youtube">("instagram");
  const bridge = useBridgeConnection({
    onStateChange: (enabled) => {
      if (preferences.companionEnabled !== enabled) {
        onUpdatePreferences({ ...preferences, companionEnabled: enabled });
      }
    },
  });

  const stateMachine = useAgentStateMachine(
    bridge.activeSession,
    preferences,
    (_payload) => {
      // Event processed
    }
  );

  const activeDisplayMode = overrideDisplayMode ?? preferences.displayMode;

  const handleToggleTheme = () => {
    const nextTheme = preferences.theme === "dark" ? "light" : "dark";
    onUpdatePreferences({ ...preferences, theme: nextTheme });
  };

  const handleGameComplete = (score: GameScoreRecord) => {
    stateMachine.recordFinishedGame(score);
  };

  return (
    <div className="min-h-screen bg-canvas text-ink font-sans flex flex-col antialiased">
      {/* Top Application Bar */}
      <header className="sm:sticky top-0 z-30 border-b border-ink/10 bg-canvas/95 px-5 sm:px-8 lg:px-12 py-3.5 flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
        <div className="flex items-center gap-3 min-w-0">
          <div className="w-10 h-10 shrink-0 rounded-xl bg-forest flex items-center justify-center text-canvas">
            <Gamepad2 className="w-5 h-5" />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
              <span className="font-serif text-[26px] leading-7 font-normal text-ink">Vibe Play</span>
              <span className="text-[10px] uppercase tracking-[0.08em] font-semibold px-2 py-0.5 rounded-full bg-canvas-muted text-ink-soft">
                Claude Code Companion
              </span>
            </div>
            <p className="text-xs text-ink-soft hidden sm:block mt-0.5">
              Interactive micro-games while your AI coding agent works
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {onOpenShowcase && !isPopup && (
            <button
              onClick={onOpenShowcase}
              className="flex items-center gap-2 h-10 px-4 rounded-xl text-[13px] font-semibold text-ink bg-lavender hover:bg-lavender-hover border border-ink/15 transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas"
              title="Return to Wispr Flow Showcase Website"
            >
              <Sparkles className="w-4 h-4 text-forest" />
              <span>Showcase Website</span>
            </button>
          )}
          <button
            className="flex items-center justify-center w-10 h-10 rounded-xl text-ink bg-white hover:bg-canvas-muted/50 border border-ink/20 transition-colors duration-200 motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas"
            title="Toggle Theme"
          >
            {preferences.theme === "dark" ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
          </button>
          <button
            onClick={() => setIsSettingsOpen(true)}
            className="flex items-center gap-2 h-10 px-4 rounded-xl text-[13px] font-semibold text-ink bg-white hover:bg-canvas-muted/50 border border-ink/20 transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas"
          >
            <SettingsIcon className="w-4 h-4" />
            <span>Settings</span>
          </button>
        </div>
      </header>

      {/* Popup View Waiting Screen */}
      {isPopup && !stateMachine.isGameOpen && !stateMachine.isResultsOpen && (
        <div className="flex-1 flex flex-col items-center justify-center px-6 py-12 text-center space-y-6">
          <div className="w-16 h-16 rounded-2xl bg-forest text-canvas flex items-center justify-center animate-pulse">
            <Gamepad2 className="w-8 h-8" />
          </div>
          <div>
            <h2 className="font-serif text-4xl leading-[1.1] font-normal text-ink">Vibe Play for Codex</h2>
            <p className="text-[15px] leading-[1.5] text-ink-soft max-w-sm mt-2 mx-auto">
              {stateMachine.status === "WORKING"
                ? "Codex is reasoning... Starting game now!"
                : "Waiting for Codex prompt to begin..."}
            </p>
          </div>
          <button
            onClick={stateMachine.openGameManually}
            className="h-11 px-6 rounded-xl bg-lavender hover:bg-lavender-hover border border-ink/15 text-ink text-sm font-semibold transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas"
          >
            Play Now
          </button>
        </div>
      )}

      {/* Main Dashboard Body */}
      {!isPopup && (
        <>
          <main className="flex-1 max-w-[1200px] w-full mx-auto px-5 sm:px-8 lg:px-12 py-8 sm:py-10 space-y-6">
            {/* Companion Experience Quick Launcher */}
            <div className="p-6 sm:p-8 rounded-[24px] bg-forest text-canvas flex flex-col lg:flex-row lg:items-center justify-between gap-6">
              <div className="flex items-start gap-4">
                <div className="w-11 h-11 shrink-0 rounded-xl bg-canvas/10 border border-canvas/20 flex items-center justify-center text-lavender">
                  <Sparkles className="w-5 h-5" />
                </div>
                <div>
                  <h3 className="font-serif text-[28px] sm:text-[32px] leading-[1.1] font-normal text-canvas">AI Waiting Experiences</h3>
                  <p className="text-sm leading-[1.5] text-canvas/75 mt-1.5 max-w-md">Authentic social entertainment while your coding agent reasons</p>
                </div>
              </div>

              <div className="flex flex-col sm:flex-row items-stretch gap-2.5 w-full lg:w-auto shrink-0">
                <button
                  onClick={() => {
                    setCompanionTab("instagram");
                    stateMachine.openGameManually();
                  }}
                  className="sm:flex-1 lg:flex-none flex items-center justify-center gap-2 h-11 px-4 rounded-xl bg-white hover:bg-canvas border border-ink/15 text-ink text-sm font-semibold whitespace-nowrap transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-lavender focus-visible:ring-offset-2 focus-visible:ring-offset-forest"
                >
                  <Film className="w-4 h-4 text-[#C13584]" />
                  <span>Instagram (Reels & Feed)</span>
                </button>

                <button
                  onClick={() => {
                    setCompanionTab("youtube");
                    stateMachine.openGameManually();
                  }}
                  className="sm:flex-1 lg:flex-none flex items-center justify-center gap-2 h-11 px-4 rounded-xl bg-white hover:bg-canvas border border-ink/15 text-ink text-sm font-semibold whitespace-nowrap transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-lavender focus-visible:ring-offset-2 focus-visible:ring-offset-forest"
                >
                  <Tv className="w-4 h-4 text-[#CC0000]" />
                  <span>YouTube</span>
                </button>

                <button
                  onClick={() => {
                    setCompanionTab("games");
                    stateMachine.openGameManually();
                  }}
                  className="sm:flex-1 lg:flex-none flex items-center justify-center gap-2 h-11 px-4 rounded-xl bg-lavender hover:bg-lavender-hover border border-ink/15 text-ink text-sm font-semibold whitespace-nowrap transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-canvas focus-visible:ring-offset-2 focus-visible:ring-offset-forest"
                >
                  <Gamepad2 className="w-4 h-4 text-forest" />
                  <span>Micro-Games</span>
                </button>
              </div>
            </div>

            {/* Row 1: Agent Telemetry & Connection */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2">
                <AgentStatusCard
                  status={stateMachine.status}
                  taskElapsedSeconds={stateMachine.currentTaskElapsedSeconds}
                  delayRemainingSeconds={stateMachine.delayRemainingSeconds}
                  waitingForInputPrompt={bridge.activeSession?.waitingForInputPrompt}
                  lastError={bridge.activeSession?.lastError}
                  onOpenGame={stateMachine.openGameManually}
                  isGameOpen={stateMachine.isGameOpen}
                />
              </div>
              <div>
                <ConnectionCard
                  status={bridge.status}
                  health={bridge.health}
                  latencyMs={bridge.latencyMs}
                  hasActiveSession={bridge.sessions.length > 0}
                  bridgeToken={bridge.bridgeToken}
                  onReconnect={bridge.reconnect}
                />
              </div>
            </div>

            {/* Row 2: Session & Recent Scores */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <SessionCard
                sessions={bridge.sessions}
                activeSession={bridge.activeSession}
                onSelectSession={bridge.selectSession}
              />
              <RecentScoresCard />
            </div>

            {/* Row 3: Simulator Control Panel */}
            <div>
              <SimulatorCard
                onSimulateEvent={stateMachine.processEvent}
                sendToBridge={bridge.sendEventToBridge}
                isBridgeConnected={bridge.status === "connected"}
              />
            </div>
          </main>

          {/* Footer */}
          <footer className="border-t border-ink/10 px-5 sm:px-8 py-6 text-center text-xs leading-[1.5] text-ink-soft">
            Vibe Play • Browser-based gaming companion for AI coding agents • Localhost companion bridge
          </footer>
        </>
      )}
      {/* Compact Floating Game Mode */}
      {stateMachine.isGameOpen && activeDisplayMode === "compact" && (
        <CompactGamePanel
          isOpen={stateMachine.isGameOpen}
          isMinimized={stateMachine.isGameMinimized}
          agentStatus={stateMachine.status}
          taskElapsedSeconds={stateMachine.currentTaskElapsedSeconds}
          preferences={preferences}
          isPaused={stateMachine.isPaused}
          initialTab={companionTab}
          onToggleMinimize={stateMachine.toggleMinimize}
          onSwitchToFullscreen={() => setOverrideDisplayMode("fullscreen")}
          onClose={stateMachine.closeGameManually}
          onGameComplete={handleGameComplete}
        />
      )}

      {/* Full-Screen Game Mode */}
      {stateMachine.isGameOpen && activeDisplayMode === "fullscreen" && (
        <FullScreenGameModal
          isOpen={stateMachine.isGameOpen}
          agentStatus={stateMachine.status}
          taskElapsedSeconds={stateMachine.currentTaskElapsedSeconds}
          preferences={preferences}
          isPaused={stateMachine.isPaused}
          initialTab={companionTab}
          onExitFullscreen={() => setOverrideDisplayMode("compact")}
          onClose={stateMachine.closeGameManually}
          onGameComplete={handleGameComplete}
        />
      )}

      {/* Results Screen Modal */}
      <GameResultModal
        isOpen={stateMachine.isResultsOpen}
        scoreRecord={stateMachine.lastScore}
        agentResponseTimeSeconds={stateMachine.currentTaskElapsedSeconds}
        onDismiss={stateMachine.dismissResults}
        onPlayAgain={stateMachine.openGameManually}
      />

      {/* Settings Modal */}
      <SettingsModal
        isOpen={isSettingsOpen}
        preferences={preferences}
        onClose={() => setIsSettingsOpen(false)}
        onSavePreferences={onUpdatePreferences}
      />
    </div>
  );
};
