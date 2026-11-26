import React, { useState } from "react";
import { Download, Terminal, Check, Copy } from "lucide-react";
import { EditorialHeading } from "../ui/EditorialHeading";
import { PrimaryButton } from "../ui/PrimaryButton";
import { Eyebrow } from "../ui/Eyebrow";

export const FinalCTA: React.FC = () => {
  const [copied, setCopied] = useState(false);
  const installCmd = "npm install -g @waitplay/cli && waitplay init";

  const handleCopy = () => {
    navigator.clipboard.writeText(installCmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section id="download" className="w-full py-20 sm:py-28 px-5 sm:px-8 bg-[#FFFFEB] border-t border-[rgba(26,26,26,0.08)] select-none">
      <div className="max-w-[1080px] mx-auto text-center flex flex-col items-center">
        <Eyebrow variant="orange" dot className="mb-6">
          GET STARTED IN SECONDS
        </Eyebrow>

        <EditorialHeading level="hero" className="max-w-[900px] mb-6">
          Coding agent running? <em className="italic font-normal">Make waiting fun.</em>
        </EditorialHeading>

        <p className="font-sans text-[17px] sm:text-[20px] text-[#666666] max-w-[620px] mx-auto leading-[1.6] mb-8 font-normal">
          Join developers around the world reclaiming deep focus. Download Vibe Play free for Windows, macOS, and Linux — 100% local, no account required.
        </p>

        {/* CTA buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 w-full max-w-[560px] mb-6">
          <PrimaryButton
            size="lg"
            href="https://github.com/ap5911443-jpg/waitplay-website/releases"
            icon={<Download className="w-4 h-4" />}
            className="w-full sm:w-auto"
          >
            Download Vibe Play Free
          </PrimaryButton>

          {/* Quick CLI command */}
          <button
            onClick={handleCopy}
            className="w-full sm:w-auto px-4 py-3.5 rounded-[12px] bg-[#FFFFFF] border border-[rgba(26,26,26,0.18)] hover:border-[rgba(26,26,26,0.35)] shadow-subtle flex items-center justify-between sm:justify-center gap-3 font-mono text-[13px] text-[#1A1A1A] transition-all cursor-pointer group"
            title="Click to copy install command"
          >
            <div className="flex items-center gap-2">
              <Terminal className="w-3.5 h-3.5 text-[#034F46]" />
              <span className="truncate max-w-[240px]">{installCmd}</span>
            </div>
            <span className="p-1 rounded bg-[#E4E4D0]/60 group-hover:bg-[#E4E4D0] transition-colors">
              {copied ? <Check className="w-3.5 h-3.5 text-[#034F46]" /> : <Copy className="w-3.5 h-3.5 text-[#666666]" />}
            </span>
          </button>
        </div>

        {/* Trust Badges */}
        <div className="flex flex-wrap items-center justify-center gap-4 sm:gap-6 text-[13px] text-[#888888] font-medium pt-4">
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-[#034F46]" />
            100% Free & MIT Licensed
          </span>
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-[#034F46]" />
            Localhost Loopback Only
          </span>
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-[#034F46]" />
            Zero Data Telemetry
          </span>
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-[#034F46]" />
            Account Optional, No Analytics
          </span>
        </div>
      </div>
    </section>
  );
};
