import React from "react";
import { Gamepad2, ArrowRight, Laptop, Terminal } from "lucide-react";

export interface SiteHeaderProps {
  onOpenDashboard?: () => void;
  onOpenGame?: () => void;
  isDashboardActive?: boolean;
}

export const SiteHeader: React.FC<SiteHeaderProps> = ({
  onOpenDashboard,
  onOpenGame,
  isDashboardActive,
}) => {
  return (
    <header className="sticky top-0 z-40 w-full px-4 sm:px-8 py-3.5 bg-canvas/90 backdrop-blur-md border-b border-ink/10 transition-colors">
      <div className="max-w-7xl mx-auto flex items-center justify-between">
        {/* Left Branding */}
        <a href="#" className="flex items-center space-x-2.5 text-ink group">
          <div className="w-8 h-8 rounded-xl bg-forest text-canvas flex items-center justify-center shadow-subtle group-hover:bg-forest-dark transition-colors">
            <Gamepad2 className="w-4.5 h-4.5" />
          </div>
          <div className="flex items-baseline space-x-1.5">
            <span className="font-serif font-normal text-2xl tracking-tight text-ink">Vibe Play</span>
            <span className="text-[10px] font-sans font-semibold tracking-wider uppercase px-1.5 py-0.5 rounded-full bg-forest/10 text-forest">
              Flow
            </span>
          </div>
        </a>

        {/* Center Editorial Navigation */}
        <nav className="hidden md:flex items-center space-x-8 text-[15px] font-sans font-medium text-ink-soft">
          <a href="#how-it-works" className="hover:text-ink transition-colors">
            How It Works
          </a>
          <a href="#comparison" className="hover:text-ink transition-colors">
            Comparison
          </a>
          <a href="#games" className="hover:text-ink transition-colors">
            Micro-Games
          </a>
          <a href="#security" className="hover:text-ink transition-colors">
            Privacy & Security
          </a>
          <a href="#faq" className="hover:text-ink transition-colors">
            FAQ
          </a>
        </nav>

        {/* Right CTA Actions */}
        <div className="flex items-center space-x-3">
          <button
            onClick={onOpenDashboard}
            className={`hidden sm:flex items-center space-x-1.5 text-xs font-sans font-medium px-3.5 py-2 rounded-xl border transition-all cursor-pointer ${
              isDashboardActive
                ? "bg-forest text-canvas border-forest shadow-subtle"
                : "bg-white text-ink border-ink/15 hover:border-ink/30 hover:bg-canvas-muted/40 shadow-subtle"
            }`}
            title="Toggle Live Agent Telemetry & Developer HUD"
          >
            <Terminal className="w-3.5 h-3.5" />
            <span>{isDashboardActive ? "Showcase Site" : "Live Agent HUD"}</span>
          </button>

          <a
            href="https://github.com/ap5911443-jpg/Waitplay.git"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center space-x-1.5 text-xs sm:text-sm font-sans font-semibold px-4 py-2 rounded-xl bg-lavender text-ink border border-ink/15 hover:bg-lavender-hover hover:shadow-subtle hover:-translate-y-0.5 active:translate-y-0 transition-all cursor-pointer"
          >
            <span>Get Started</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </a>
        </div>
      </div>
    </header>
  );
};
