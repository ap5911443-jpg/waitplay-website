import React, { useState } from "react";
import {
  Settings,
  Monitor,
  Layout,
  Volume2,
  VolumeX,
  Clock,
  Flame,
  PauseCircle,
  Sun,
  Moon,
  RotateCcw,
} from "lucide-react";
import { UserPreferences, DEFAULT_PREFERENCES, GameCategory, GameDifficulty, DisplayMode } from "@waitplay/shared";
import { Modal } from "../ui/Modal";

export interface SettingsModalProps {
  isOpen: boolean;
  preferences: UserPreferences;
  onClose: () => void;
  onSavePreferences: (updated: UserPreferences) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  preferences,
  onClose,
  onSavePreferences,
}) => {
  const [prefs, setPrefs] = useState<UserPreferences>(preferences);

  const handleDisplayModeChange = (mode: DisplayMode) => {
    const updated = { ...prefs, displayMode: mode };
    setPrefs(updated);
    onSavePreferences(updated);
  };

  const handleDifficultyChange = (diff: GameDifficulty) => {
    const updated = { ...prefs, difficulty: diff };
    setPrefs(updated);
    onSavePreferences(updated);
  };

  const handleCategoryToggle = (category: GameCategory) => {
    const exists = prefs.preferredCategories.includes(category);
    let next: GameCategory[];
    if (exists) {
      // Keep at least memory
      next = prefs.preferredCategories.filter((c) => c !== category);
      if (next.length === 0) next = ["memory"];
    } else {
      next = [...prefs.preferredCategories, category];
    }
    const updated = { ...prefs, preferredCategories: next };
    setPrefs(updated);
    onSavePreferences(updated);
  };

  const handleToggle = (key: keyof UserPreferences) => {
    const current = prefs[key];
    if (typeof current === "boolean") {
      const updated = { ...prefs, [key]: !current };
      setPrefs(updated);
      onSavePreferences(updated);
    }
  };

  const handleStartDelayChange = (seconds: number) => {
    const updated = { ...prefs, startDelaySeconds: Math.max(0, Math.min(30, seconds)) };
    setPrefs(updated);
    onSavePreferences(updated);
  };

  const handleResultsDurationChange = (seconds: number) => {
    const updated = { ...prefs, resultsDurationSeconds: Math.max(0, Math.min(30, seconds)) };
    setPrefs(updated);
    onSavePreferences(updated);
  };

  const handleResetDefaults = () => {
    setPrefs(DEFAULT_PREFERENCES);
    onSavePreferences(DEFAULT_PREFERENCES);
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Preferences & Behavior"
      subtitle="Configure auto-launch rules, display mode, and gameplay parameters."
      maxWidth="lg"
    >
      <div className="space-y-6 text-[13px] text-ink">
        {/* Display Mode */}
        <div>
          <label className="block text-sm font-semibold text-ink mb-2.5">
            Display Mode
          </label>
          <div className="grid grid-cols-1 min-[480px]:grid-cols-2 gap-3">
            <button
              onClick={() => handleDisplayModeChange("compact")}
              className={`flex items-start p-4 rounded-2xl border text-left transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas ${
                prefs.displayMode === "compact"
                  ? "bg-lavender/50 border-forest ring-1 ring-forest text-forest"
                  : "bg-white border-ink/20 hover:border-ink/40 text-ink-soft"
              }`}
            >
              <Layout className="w-5 h-5 mr-3 mt-0.5 shrink-0" />
              <div>
                <div className="font-semibold text-ink">Compact Floating Panel</div>
                <div className="text-xs leading-[1.45] text-ink-soft mt-1">
                  Docked in corner. Minimizable with active telemetry.
                </div>
              </div>
            </button>

            <button
              onClick={() => handleDisplayModeChange("fullscreen")}
              className={`flex items-start p-4 rounded-2xl border text-left transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas ${
                prefs.displayMode === "fullscreen"
                  ? "bg-lavender/50 border-forest ring-1 ring-forest text-forest"
                  : "bg-white border-ink/20 hover:border-ink/40 text-ink-soft"
              }`}
            >
              <Monitor className="w-5 h-5 mr-3 mt-0.5 shrink-0" />
              <div>
                <div className="font-semibold text-ink">Full-Screen Overlay</div>
                <div className="text-xs leading-[1.45] text-ink-soft mt-1">
                  Full browser viewport with agent progress header.
                </div>
              </div>
            </button>
          </div>
        </div>

        {/* Auto Launch & Delays */}
        <div className="p-5 bg-white rounded-2xl border border-ink/10 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <div className="font-semibold text-sm text-ink">Auto-Launch During Inference</div>
              <div className="text-xs leading-[1.45] text-ink-soft mt-0.5">
                Automatically opens a mini-game when Claude Code starts working.
              </div>
            </div>
            <input
              type="checkbox"
              checked={prefs.autoLaunch}
              onChange={() => handleToggle("autoLaunch")}
              className="w-5 h-5 shrink-0 rounded accent-forest cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white"
            />
          </div>

          {prefs.autoLaunch && (
            <div className="pt-4 border-t border-ink/10 space-y-4">
              <div>
                <div className="flex items-center justify-between gap-2 mb-1.5">
                  <div className="flex items-center gap-1.5 font-semibold text-ink">
                    <Clock className="w-4 h-4 text-forest" />
                    <span>Start Delay: <strong>{prefs.startDelaySeconds}s</strong></span>
                  </div>
                  <span className="text-[11px] text-ink-soft">Default: 4 seconds</span>
                </div>
                <input
                  type="range"
                  min={0}
                  max={15}
                  value={prefs.startDelaySeconds}
                  onChange={(e) => handleStartDelayChange(parseInt(e.target.value, 10))}
                  className="w-full accent-forest cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest rounded"
                />
                <div className="text-[11px] text-ink-soft mt-0.5">
                  Tasks finishing before this delay will not open the game.
                </div>
              </div>

              <div>
                <div className="flex items-center justify-between gap-2 mb-1.5">
                  <span className="font-semibold text-ink">
                    Results Screen Duration: <strong>{prefs.resultsDurationSeconds}s</strong>
                  </span>
                  <span className="text-[11px] text-ink-soft">0 = Stay open</span>
                </div>
                <input
                  type="range"
                  min={0}
                  max={20}
                  value={prefs.resultsDurationSeconds}
                  onChange={(e) => handleResultsDurationChange(parseInt(e.target.value, 10))}
                  className="w-full accent-forest cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest rounded"
                />
              </div>
            </div>
          )}
        </div>

        {/* Gameplay Parameters */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
          <div>
            <label className="block text-sm font-semibold text-ink mb-2.5">
              Memory Game Difficulty
            </label>
            <div className="grid grid-cols-3 gap-2">
              {(["easy", "medium", "hard"] as GameDifficulty[]).map((d) => (
                <button
                  key={d}
                  onClick={() => handleDifficultyChange(d)}
                  className={`py-2.5 px-2 text-center rounded-xl border capitalize font-semibold transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas ${
                    prefs.difficulty === d
                      ? "bg-lavender/50 border-forest ring-1 ring-forest text-ink"
                      : "bg-white border-ink/20 text-ink hover:border-ink/40"
                  }`}
                >
                  {d}
                  <span className="block text-[11px] text-ink-soft font-normal">
                    {d === "easy" ? "6 pairs" : d === "medium" ? "8 pairs" : "10 pairs"}
                  </span>
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-sm font-semibold text-ink mb-2.5">
              Sound Effects
            </label>
            <button
              onClick={() => handleToggle("soundEnabled")}
              className={`w-full min-h-[52px] py-2.5 px-3.5 rounded-xl border flex items-center justify-between gap-2 transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas ${
                prefs.soundEnabled
                  ? "bg-lavender/50 border-forest ring-1 ring-forest text-forest"
                  : "bg-white border-ink/20 text-ink-soft hover:border-ink/40"
              }`}
            >
              <div className="flex items-center gap-2">
                {prefs.soundEnabled ? <Volume2 className="w-4 h-4" /> : <VolumeX className="w-4 h-4" />}
                <span className="font-semibold text-ink text-left">
                  {prefs.soundEnabled ? "Synthesized Sound Enabled" : "Muted"}
                </span>
              </div>
              <span className="text-[11px] font-semibold tracking-wide">{prefs.soundEnabled ? "ON" : "OFF"}</span>
            </button>
          </div>
        </div>

        {/* Agent Interaction Rules */}
        <div className="p-5 bg-white rounded-2xl border border-ink/10 space-y-3">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <PauseCircle className="w-5 h-5 text-forest mt-0.5 shrink-0" />
              <div>
                <div className="font-semibold text-sm text-ink">Pause on Input Request</div>
                <div className="text-xs leading-[1.45] text-ink-soft mt-0.5">
                  Temporarily pause active gameplay when Claude Code requests user confirmation or tool permissions.
                </div>
              </div>
            </div>
            <input
              type="checkbox"
              checked={prefs.pauseOnInputRequired}
              onChange={() => handleToggle("pauseOnInputRequired")}
              className="w-5 h-5 shrink-0 rounded accent-forest cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white"
            />
          </div>
        </div>

        {/* Categories Preference */}
        <div>
          <label className="block text-sm font-semibold text-ink mb-1">
            Game Categories
          </label>
          <p className="text-xs leading-[1.45] text-ink-soft mb-3">
            Playable games matching selected categories will be launched. Unfinished placeholder games are excluded.
          </p>
          <div className="flex flex-wrap gap-2">
            {[
              { id: "memory" as GameCategory, label: "Memory Matching", ready: true },
              { id: "reaction" as GameCategory, label: "Reaction Test", ready: false },
              { id: "riddle" as GameCategory, label: "Riddle Challenge", ready: false },
              { id: "math" as GameCategory, label: "Math Sprint", ready: false },
            ].map((cat) => {
              const isSelected = prefs.preferredCategories.includes(cat.id);
              return (
                <button
                  key={cat.id}
                  onClick={() => handleCategoryToggle(cat.id)}
                  className={`px-3.5 py-2 rounded-[10px] border text-[13px] font-semibold transition-colors duration-200 motion-reduce:transition-none cursor-pointer flex items-center gap-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas ${
                    isSelected
                      ? "bg-lavender/50 border-forest ring-1 ring-forest text-ink"
                      : "bg-white border-ink/20 text-ink-soft hover:border-ink/40"
                  }`}
                >
                  <span>{cat.label}</span>
                  {!cat.ready && (
                    <span className="text-[10px] px-1.5 py-0.5 bg-canvas-muted text-ink-soft rounded-full font-semibold">
                      Soon
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Footer actions */}
        <div className="pt-5 border-t border-ink/10 flex items-center justify-between gap-3">
          <button
            onClick={handleResetDefaults}
            className="flex items-center gap-2 text-[13px] font-semibold text-ink-soft hover:text-ink transition-colors duration-200 motion-reduce:transition-none rounded-lg px-2 py-2 -ml-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-forest"
          >
            <RotateCcw className="w-4 h-4" />
            <span>Reset Defaults</span>
          </button>

          <button
            onClick={onClose}
            className="h-10 px-5 bg-lavender hover:bg-lavender-hover text-ink border border-ink/15 rounded-xl text-sm font-semibold transition-colors duration-200 motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas"
          >
            Done
          </button>
        </div>
      </div>
    </Modal>
  );
};
