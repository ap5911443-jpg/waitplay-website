import React, { useState } from "react";
import { ChevronDown } from "lucide-react";

export const FAQSection: React.FC = () => {
  const [openIndex, setOpenIndex] = useState<number | null>(0);

  const faqs = [
    {
      q: "Where does Vibe Play work?",
      a: "Vibe Play works across Windows Terminal, PowerShell, CMD, VS Code integrated terminal, Cursor, and iTerm2. It supports Claude Code, OpenAI Codex, OMP (One More Prompt), Cline, OpenCode, and MCP tools.",
    },
    {
      q: "How does it detect when an agent starts and finishes?",
      a: "Vibe Play provides lightweight event hooks that listen to standard agent lifecycle signals (e.g. prompt submission, tool invocation, and turn completion). The moment your agent starts thinking, the game board pops open; the instant it outputs code or asks for user approval, the game auto-stops and yields focus back to your terminal.",
    },
    {
      q: "What if I am managing multiple terminal tabs at the same time?",
      a: "Each terminal tab has an isolated session tracker (e.g. Term - 1, Term - 2). When you move between tabs, Vibe Play pairs with the active agent. You can also click the floating Wispr Flow desktop button to turn Vibe Play OFF in one click — muting all popups across all terminal tabs so you can guide another agent in peace.",
    },
    {
      q: "Does Vibe Play upload any code, telemetry, or prompts?",
      a: "Never. Vibe Play's bridge binds exclusively to loopback 127.0.0.1. There are zero cloud servers, zero analytics, zero cookies, and zero external network calls. All game logic and agent status remain 100% on your machine.",
    },
    {
      q: "Is Vibe Play free and open source?",
      a: "Yes. Vibe Play is 100% free, MIT licensed, and open-source on GitHub.",
    },
  ];

  return (
    <section id="faq" className="w-full py-20 px-4 sm:px-8 bg-canvas-muted/30 border-t border-ink/10">
      <div className="max-w-4xl mx-auto space-y-12">
        <div className="text-center space-y-2">
          <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest">
            Questions & Details
          </span>
          <h2 className="font-serif font-normal text-4xl sm:text-5xl text-ink">
            Frequently Asked Questions
          </h2>
        </div>

        <div className="divide-y divide-ink/10 bg-white rounded-3xl border border-ink/15 shadow-subtle p-6 sm:p-8 text-left">
          {faqs.map((faq, idx) => {
            const isOpen = openIndex === idx;
            return (
              <div key={idx} className="py-5 first:pt-0 last:pb-0">
                <button
                  onClick={() => setOpenIndex(isOpen ? null : idx)}
                  className="w-full flex items-center justify-between text-left group cursor-pointer"
                  aria-expanded={isOpen}
                >
                  <span className="font-serif font-normal text-xl sm:text-2xl text-ink group-hover:text-forest transition-colors">
                    {faq.q}
                  </span>
                  <div
                    className={`w-7 h-7 rounded-full bg-canvas flex items-center justify-center text-ink-soft transition-transform duration-200 ${
                      isOpen ? "rotate-180 bg-forest text-canvas" : ""
                    }`}
                  >
                    <ChevronDown className="w-4 h-4" />
                  </div>
                </button>

                {isOpen && (
                  <p className="mt-3.5 font-sans text-sm sm:text-base text-ink-soft leading-relaxed pr-8">
                    {faq.a}
                  </p>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
};
