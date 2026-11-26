import React from "react";
import { ArrowRight, Terminal } from "lucide-react";

export const FinalCTA: React.FC<{ onOpenDashboard?: () => void }> = ({ onOpenDashboard }) => {
  return (
    <section className="w-full py-20 px-4 sm:px-8 bg-canvas">
      <div className="max-w-4xl mx-auto text-center space-y-7 py-12 px-6 sm:px-12 bg-white rounded-4xl border border-ink/15 shadow-popover">
        <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest">
          Ready for Flow State?
        </span>

        <h2 className="font-serif font-normal text-4xl sm:text-6xl text-ink leading-tight max-w-2xl mx-auto">
          Never sit through another <br />
          <span className="italic text-forest">boring 45-second latency.</span>
        </h2>

        <p className="font-sans text-base sm:text-lg text-ink-soft max-w-xl mx-auto leading-relaxed">
          Clone the repository, launch the desktop widget, and let Vibe Play automatically turn agent downtime into cognitive focus.
        </p>

        <div className="flex flex-col sm:flex-row items-center justify-center gap-3 pt-2">
          <a
            href="https://github.com/ap5911443-jpg/Waitplay.git"
            target="_blank"
            rel="noopener noreferrer"
            className="w-full sm:w-auto inline-flex items-center justify-center space-x-2 text-base font-sans font-semibold px-8 py-4 rounded-xl bg-lavender text-ink border border-ink/15 hover:bg-lavender-hover hover:shadow-subtle hover:-translate-y-0.5 active:translate-y-0 transition-all cursor-pointer shadow-subtle"
          >
            <span>Download Vibe Play Free</span>
            <ArrowRight className="w-4 h-4" />
          </a>

          <button
            onClick={onOpenDashboard}
            className="w-full sm:w-auto inline-flex items-center justify-center space-x-2 text-base font-sans font-semibold px-6 py-4 rounded-xl bg-white text-ink border border-ink/15 hover:bg-canvas-muted/60 transition-all cursor-pointer"
          >
            <Terminal className="w-4 h-4 text-forest" />
            <span>Open Developer HUD</span>
          </button>
        </div>

        <p className="text-xs font-sans text-ink-muted">
          MIT License • Native C# WPF Widget + React 18 Web HUD • 100% Localhost
        </p>
      </div>
    </section>
  );
};
