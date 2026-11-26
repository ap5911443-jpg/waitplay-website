import React from "react";
import { SectionContainer } from "../layout/SectionContainer";
import { Eyebrow } from "../ui/Eyebrow";
import { EditorialHeading } from "../ui/EditorialHeading";
import { ShieldCheck, Lock, EyeOff, FileCode2, ArrowUpRight } from "lucide-react";

export const PrivacySection: React.FC = () => {
  const securityPillars = [
    {
      icon: <Lock className="w-5 h-5 text-[#F0D7FF]" />,
      title: "100% Local Loopback Only",
      description:
        "The Vibe Play bridge binds exclusively to 127.0.0.1:3150. External interfaces and remote IP addresses are rejected at the TCP socket layer.",
    },
    {
      icon: <EyeOff className="w-5 h-5 text-[#FFA946]" />,
      title: "No Telemetry, No Cloud",
      description:
        "Agent hooks send status (idle, working, finished) and, only to show it in your own widget, the first 100 characters of your prompt or the agent's question, to the bridge on this computer. That text stays in the bridge's memory and is never written to disk by Vibe Play, never sent to a server and never forwarded to other programs. Your source files, diffs and API keys are not read.",
    },
    {
      icon: <ShieldCheck className="w-5 h-5 text-[#F0D7FF]" />,
      title: "Token-Authenticated Bridge",
      description:
        "Every session requires a cryptographically secure token persisted with restricted OS file permissions (0o600) to prevent unauthorized local cross-process requests.",
    },
    {
      icon: <FileCode2 className="w-5 h-5 text-[#FFA946]" />,
      title: "100% Auditable Open Source",
      description:
        "Under the permissive MIT license, every line of bridge and native companion code is open for your team's security audit on GitHub.",
    },
  ];

  return (
    <SectionContainer id="security" bg="forest" className="relative overflow-hidden">
      <div className="max-w-[1120px] mx-auto">
        <div className="text-center max-w-[800px] mx-auto mb-14 sm:mb-16">
          <Eyebrow variant="forest" dot className="mb-4">
            ZERO-TELEMETRY SECURITY MODEL
          </Eyebrow>

          <EditorialHeading level="section" theme="forest" className="mb-5">
            Your code <em className="italic font-normal text-[#F0D7FF]">stays yours.</em>
          </EditorialHeading>

          <p className="font-sans text-[17px] sm:text-[19px] text-[#FFFFEB]/85 leading-[1.6]">
            Developer tools should never compromise on privacy. Vibe Play has no analytics and works fully without an account: everything it knows about your agents stays on your machine. The only outgoing connections are the entertainment site you choose to open, SMS through Twilio if you set it up, and, only if you choose to create an optional account, sign-in with Supabase.
          </p>
        </div>

        {/* Security Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-5 sm:gap-6 mb-12">
          {securityPillars.map((pillar) => (
            <div
              key={pillar.title}
              className="p-6 sm:p-8 rounded-[24px] bg-[#023932] border border-[rgba(255,255,235,0.14)] hover:border-[rgba(255,255,235,0.3)] transition-colors shadow-card flex flex-col justify-between"
            >
              <div>
                <div className="w-10 h-10 rounded-[12px] bg-[#034F46] border border-[rgba(255,255,235,0.15)] flex items-center justify-center mb-4">
                  {pillar.icon}
                </div>
                <h3 className="font-sans font-semibold text-[18px] text-[#FFFFEB] mb-2">
                  {pillar.title}
                </h3>
                <p className="font-sans text-[14.5px] text-[#FFFFEB]/75 leading-[1.6]">
                  {pillar.description}
                </p>
              </div>
            </div>
          ))}
        </div>

        {/* Security Callout Box */}
        <div className="p-6 sm:p-7 rounded-[20px] bg-[#023932]/70 border border-[rgba(255,255,235,0.12)] flex flex-col sm:flex-row items-center justify-between gap-4 text-center sm:text-left">
          <div className="flex items-center gap-3.5">
            <span className="w-3 h-3 rounded-full bg-[#00f3ff] animate-pulse" />
            <span className="text-[14.5px] text-[#FFFFEB]/90 font-medium">
              Read the privacy policy and our Architecture & Security Specification on GitHub
            </span>
          </div>
          <div className="flex flex-wrap items-center justify-center gap-2.5">
            <a
              href="https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/PRIVACY.md"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 px-4 py-2 rounded-full text-[13px] font-semibold bg-[#FFFFEB] text-[#034F46] hover:bg-[#F0D7FF] transition-colors cursor-pointer"
            >
              <span>Privacy policy</span>
              <ArrowUpRight className="w-3.5 h-3.5" />
            </a>
            <a
              href="https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/ARCHITECTURE.md"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 px-4 py-2 rounded-full text-[13px] font-semibold bg-[#FFFFEB] text-[#034F46] hover:bg-[#F0D7FF] transition-colors cursor-pointer"
            >
              <span>View docs/ARCHITECTURE.md</span>
              <ArrowUpRight className="w-3.5 h-3.5" />
            </a>
          </div>
        </div>
      </div>
    </SectionContainer>
  );
};
