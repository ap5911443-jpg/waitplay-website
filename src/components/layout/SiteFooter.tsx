import React from "react";
import { Github, Twitter, Shield } from "lucide-react";

export const SiteFooter: React.FC = () => {
  return (
    <footer className="w-full bg-[#E4E4D0]/60 border-t border-[rgba(26,26,26,0.12)] pt-16 pb-12 text-[#1A1A1A]">
      <div className="max-w-[1240px] mx-auto px-5 sm:px-8 md:px-12">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-8 sm:gap-10 pb-12 border-b border-[rgba(26,26,26,0.1)]">
          {/* Brand Col */}
          <div className="col-span-2">
            <a href="#" className="flex items-center gap-2.5 mb-4 group">
              <div className="w-8 h-8 rounded-lg bg-[#034F46] flex items-center justify-center text-[#FFFFEB]">
                <span className="font-serif font-bold text-lg leading-none">W</span>
              </div>
              <span className="font-serif text-[22px] font-normal tracking-tight text-[#1A1A1A]">
                Wait<span className="font-sans font-semibold text-[20px] text-[#034F46]">Play</span>
              </span>
            </a>
            <p className="font-sans text-[14.5px] text-[#666666] leading-[1.6] max-w-[320px] mb-5">
              The zero-interruption gaming companion for AI coding agents. Play micro-games while your agent reasons, then seamlessly return to code diffs.
            </p>
            <div className="flex items-center gap-3 text-[#666666]">
              <a
                href="https://github.com/ap5911443-jpg/waitplay-website"
                target="_blank"
                rel="noreferrer"
                className="w-8 h-8 rounded-full border border-[rgba(26,26,26,0.15)] flex items-center justify-center hover:text-[#1A1A1A] hover:border-[rgba(26,26,26,0.3)] transition-colors"
                aria-label="GitHub Repository"
              >
                <Github className="w-4 h-4" />
              </a>
              <a
                href="https://twitter.com"
                target="_blank"
                rel="noreferrer"
                className="w-8 h-8 rounded-full border border-[rgba(26,26,26,0.15)] flex items-center justify-center hover:text-[#1A1A1A] hover:border-[rgba(26,26,26,0.3)] transition-colors"
                aria-label="Twitter / X"
              >
                <Twitter className="w-4 h-4" />
              </a>
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-semibold bg-[#034F46]/10 text-[#034F46] border border-[#034F46]/20">
                <Shield className="w-3 h-3" />
                100% Loopback Only
              </span>
            </div>
          </div>

          {/* Product Links */}
          <div className="flex flex-col gap-2.5">
            <h4 className="font-sans font-semibold text-[13px] uppercase tracking-wider text-[#888888] mb-1">
              Product
            </h4>
            <a href="#overview" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Desktop Widget
            </a>
            <a href="#speed" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Speed Comparison
            </a>
            <a href="#how-it-works" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              How It Works
            </a>
            <a href="#personalize" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Micro-Game Library
            </a>
            <a href="#download" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Release Notes
            </a>
          </div>

          {/* Integrations Links */}
          <div className="flex flex-col gap-2.5">
            <h4 className="font-sans font-semibold text-[13px] uppercase tracking-wider text-[#888888] mb-1">
              Agents
            </h4>
            <span className="text-[14px] text-[#666666]">Claude Code</span>
            <span className="text-[14px] text-[#666666]">Cursor & Windsurf</span>
            <span className="text-[14px] text-[#666666]">OpenAI Codex</span>
            <span className="text-[14px] text-[#666666]">Oh-My-Pi (OMP)</span>
            <span className="text-[14px] text-[#666666]">Cline & OpenCode</span>
            <span className="text-[14px] text-[#666666]">MCP Protocol</span>
          </div>

          {/* Resources & Open Source */}
          <div className="flex flex-col gap-2.5">
            <h4 className="font-sans font-semibold text-[13px] uppercase tracking-wider text-[#888888] mb-1">
              Resources
            </h4>
            <a href="https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/ARCHITECTURE.md" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Architecture Spec
            </a>
            <a href="https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/EVENT_PROTOCOL.md" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Event Protocol
            </a>
            <a href="https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/GETTING_STARTED.md" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Getting Started
            </a>
            <a href="#faq" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              FAQ
            </a>
            <a href="#security" className="text-[14px] text-[#666666] hover:text-[#1A1A1A] transition-colors">
              Security Model
            </a>
          </div>
        </div>

        {/* Bottom row */}
        <div className="pt-8 flex flex-col sm:flex-row items-center justify-between gap-4 text-[13px] text-[#888888]">
          <p>© {new Date().getFullYear()} Vibe Play. Open source under the MIT License.</p>
          <div className="flex items-center gap-6">
            <a href="#security" className="hover:text-[#1A1A1A] transition-colors">
              Security Guarantee
            </a>
            <a href="https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/PRIVACY.md" className="hover:text-[#1A1A1A] transition-colors">
              Privacy Policy
            </a>
            <span className="text-[12px] text-[#A0A090]">
              Inspired by Wispr Flow editorial clarity
            </span>
          </div>
        </div>
      </div>
    </footer>
  );
};
