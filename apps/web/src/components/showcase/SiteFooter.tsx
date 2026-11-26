import React from "react";
import { Gamepad2, Github } from "lucide-react";

export const SiteFooter: React.FC = () => {
  return (
    <footer className="w-full py-16 px-4 sm:px-8 bg-canvas-muted/40 border-t border-ink/10 text-left">
      <div className="max-w-6xl mx-auto space-y-12">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-8">
          {/* Brand Column */}
          <div className="col-span-2 space-y-3">
            <div className="flex items-center space-x-2.5 text-ink">
              <div className="w-7 h-7 rounded-lg bg-forest text-canvas flex items-center justify-center">
                <Gamepad2 className="w-4 h-4" />
              </div>
              <span className="font-serif font-normal text-2xl tracking-tight text-ink">Vibe Play</span>
            </div>
            <p className="text-xs font-sans text-ink-soft max-w-sm leading-relaxed">
              Editorial, human-led gaming companion for developers waiting on AI coding agents. Turn 45-second latency into cognitive flow.
            </p>
            <div className="pt-2 flex items-center space-x-2 text-xs font-sans text-forest font-semibold">
              <span className="w-2 h-2 rounded-full bg-forest animate-pulse" />
              <span>Bridge Localhost 127.0.0.1:3150 Active</span>
            </div>
          </div>

          {/* Links Column 1 */}
          <div className="space-y-3 text-xs font-sans">
            <div className="font-semibold uppercase tracking-wider text-ink">Product</div>
            <ul className="space-y-2 text-ink-soft">
              <li>
                <a href="#how-it-works" className="hover:text-ink transition-colors">
                  How It Works
                </a>
              </li>
              <li>
                <a href="#comparison" className="hover:text-ink transition-colors">
                  Speed Comparison
                </a>
              </li>
              <li>
                <a href="#games" className="hover:text-ink transition-colors">
                  Micro-Game Suite
                </a>
              </li>
              <li>
                <a href="#security" className="hover:text-ink transition-colors">
                  Localhost Privacy
                </a>
              </li>
            </ul>
          </div>

          {/* Links Column 2 */}
          <div className="space-y-3 text-xs font-sans">
            <div className="font-semibold uppercase tracking-wider text-ink">Agents</div>
            <ul className="space-y-2 text-ink-soft">
              <li>
                <a href="#how-it-works" className="hover:text-ink transition-colors">
                  Claude Code Hook
                </a>
              </li>
              <li>
                <a href="#how-it-works" className="hover:text-ink transition-colors">
                  OpenAI Codex Watcher
                </a>
              </li>
              <li>
                <a href="#how-it-works" className="hover:text-ink transition-colors">
                  OMP Session Integration
                </a>
              </li>
              <li>
                <a href="#how-it-works" className="hover:text-ink transition-colors">
                  Cursor & Cline Support
                </a>
              </li>
            </ul>
          </div>

          {/* Links Column 3 */}
          <div className="space-y-3 text-xs font-sans">
            <div className="font-semibold uppercase tracking-wider text-ink">Resources</div>
            <ul className="space-y-2 text-ink-soft">
              <li>
                <a
                  href="https://github.com/ap5911443-jpg/Waitplay.git"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-ink transition-colors inline-flex items-center space-x-1"
                >
                  <Github className="w-3.5 h-3.5 mr-1" />
                  <span>GitHub Repository</span>
                </a>
              </li>
              <li>
                <a href="#faq" className="hover:text-ink transition-colors">
                  FAQ & Documentation
                </a>
              </li>
              <li>
                <span className="text-ink-muted">MIT License</span>
              </li>
            </ul>
          </div>
        </div>

        <div className="pt-8 border-t border-ink/10 flex flex-col sm:flex-row items-center justify-between text-xs font-sans text-ink-muted gap-4">
          <p>© 2026 Vibe Play. Designed with editorial warmth inspired by Wispr Flow.</p>
          <p>Localhost companion bridge • Zero cloud telemetry</p>
        </div>
      </div>
    </footer>
  );
};
