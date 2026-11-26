import React from "react";
import { ShieldCheck, Lock, Terminal, Cpu, CheckCircle } from "lucide-react";

export const PrivacySection: React.FC = () => {
  return (
    <section id="security" className="w-full py-20 px-4 sm:px-8 bg-canvas">
      <div className="max-w-6xl mx-auto">
        {/* High Contrast Deep Green Panel as specified in Section 9.6 */}
        <div className="bg-forest text-canvas rounded-4xl p-8 sm:p-14 border border-forest-soft/30 shadow-popover space-y-10">
          <div className="max-w-2xl space-y-3 text-left">
            <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest-soft">
              Localhost Architecture
            </span>
            <h2 className="font-serif font-normal text-4xl sm:text-5xl text-canvas leading-tight">
              100% Localhost. <br />
              <span className="text-lavender italic">Zero cloud telemetry or code leakage.</span>
            </h2>
            <p className="font-sans text-base sm:text-lg text-canvas/80 leading-relaxed">
              We believe a coding companion must have zero attack surface. Vibe Play does not maintain cloud servers, accounts, or analytics. Everything runs isolated on your local machine.
            </p>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 text-left">
            <div className="bg-forest-dark/70 rounded-2xl p-6 border border-white/10 space-y-3">
              <div className="w-9 h-9 rounded-xl bg-white/10 text-lavender flex items-center justify-center">
                <Lock className="w-5 h-5" />
              </div>
              <h3 className="font-serif text-xl font-normal text-canvas">Loopback Only</h3>
              <p className="text-xs font-sans text-canvas/70 leading-relaxed">
                The event bridge strictly binds to <code className="text-lavender">127.0.0.1</code>. Non-loopback network packets are discarded with 403 Forbidden.
              </p>
            </div>

            <div className="bg-forest-dark/70 rounded-2xl p-6 border border-white/10 space-y-3">
              <div className="w-9 h-9 rounded-xl bg-white/10 text-lavender flex items-center justify-center">
                <ShieldCheck className="w-5 h-5" />
              </div>
              <h3 className="font-serif text-xl font-normal text-canvas">Token-Gated IPC</h3>
              <p className="text-xs font-sans text-canvas/70 leading-relaxed">
                Every agent event must present the local cryptographically random security token stored in your session temporary files.
              </p>
            </div>

            <div className="bg-forest-dark/70 rounded-2xl p-6 border border-white/10 space-y-3">
              <div className="w-9 h-9 rounded-xl bg-white/10 text-lavender flex items-center justify-center">
                <Cpu className="w-5 h-5" />
              </div>
              <h3 className="font-serif text-xl font-normal text-canvas">Zero Code Access</h3>
              <p className="text-xs font-sans text-canvas/70 leading-relaxed">
                Vibe Play listens only for lifecycle signals (start, tool run, finish). Your source code, proprietary algorithms, and tokens are never read or stored.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
