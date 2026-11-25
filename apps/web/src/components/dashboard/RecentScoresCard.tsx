import React from "react";
import { Trophy, Clock, Zap, Gamepad2 } from "lucide-react";
import { getGameStats } from "../../store/gameHistory";
import { Card } from "../ui/Card";

export const RecentScoresCard: React.FC = () => {
  const stats = getGameStats();

  const formatTime = (sec: number) => {
    const mins = Math.floor(sec / 60);
    const remainder = sec % 60;
    return `${mins}m ${remainder}s`;
  };

  return (
    <Card
      title="Games Played & Scores"
      subtitle="Gameplay performance while waiting for coding agents"
      icon={<Trophy className="w-[18px] h-[18px]" />}
    >
      <div className="grid grid-cols-3 gap-3 mb-4">
        <div className="p-3 sm:p-4 bg-canvas rounded-2xl border border-ink/10 text-center">
          <div className="text-xs font-semibold text-ink-soft">Total Games</div>
          <div className="font-serif text-[32px] leading-9 text-ink mt-1 lining-nums tabular-nums">
            {stats.totalGames}
          </div>
        </div>
        <div className="p-3 sm:p-4 bg-canvas rounded-2xl border border-ink/10 text-center">
          <div className="text-xs font-semibold text-ink-soft">High Score</div>
          <div className="font-serif text-[32px] leading-9 text-forest mt-1 lining-nums tabular-nums">
            {stats.highScore}
          </div>
        </div>
        <div className="p-3 sm:p-4 bg-canvas rounded-2xl border border-ink/10 text-center">
          <div className="text-xs font-semibold text-ink-soft">Play Time</div>
          <div className="font-serif text-[22px] min-[420px]:text-[26px] sm:text-[32px] leading-9 text-ink mt-1 lining-nums tabular-nums whitespace-nowrap">
            {formatTime(stats.totalTimeSeconds)}
          </div>
        </div>
      </div>

      {stats.recentScores.length === 0 ? (
        <div className="p-6 text-center border border-dashed border-ink/25 rounded-2xl text-[13px] leading-[1.5] text-ink-soft">
          <Gamepad2 className="w-8 h-8 mx-auto text-forest-soft mb-2" />
          No games played yet. Scores will appear here once Claude Code starts working or when you test a game.
        </div>
      ) : (
        <div className="space-y-2 max-h-56 overflow-y-auto pr-1">
          {stats.recentScores.map((score) => (
            <div
              key={score.id}
              className="flex items-center justify-between p-3 bg-canvas hover:bg-canvas-muted/40 border border-ink/10 rounded-xl text-xs transition-colors duration-200 motion-reduce:transition-none"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-lavender border border-ink/10 text-ink flex items-center justify-center font-semibold text-[11px]">
                  {score.difficulty[0].toUpperCase()}
                </div>
                <div>
                  <div className="font-semibold text-[13px] text-ink">{score.gameTitle}</div>
                  <div className="text-[11px] text-ink-soft flex items-center gap-2">
                    <span className="flex items-center gap-0.5">
                      <Clock className="w-3 h-3" />
                      <span>{score.elapsedSeconds}s</span>
                    </span>
                    {score.accuracyPercent !== undefined && (
                      <span className="flex items-center gap-0.5 text-ink-soft">
                        <Zap className="w-3 h-3 text-forest" />
                        <span>{score.accuracyPercent}%</span>
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <div className="text-right">
                <div className="font-serif text-xl leading-6 text-ink lining-nums tabular-nums">{score.score}</div>
                <div className="text-[11px] text-ink-soft">
                  {new Date(score.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
};
