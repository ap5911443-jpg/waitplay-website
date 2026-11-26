import React, { useState } from "react";
import { Gamepad2, Power, Move } from "lucide-react";

interface WisprFloatingPillWidgetProps {
  onOpenGame?: () => void;
  className?: string;
  isAgentActive?: boolean;
}

export const WisprFloatingPillWidget: React.FC<WisprFloatingPillWidgetProps> = ({
  onOpenGame,
  className = "",
  isAgentActive = true,
}) => {
  const [enabled, setEnabled] = useState(true);

  return (
    <div
      className={`inline-flex items-center gap-2 p-1.5 sm:p-2 rounded-full bg-[#1A1A1A] text-[#FFFFEB] shadow-[0_8px_30px_rgba(0,0,0,0.25)] border border-[rgba(255,255,255,0.15)] transition-all duration-300 select-none ${className}`}
    >
      {/* Drag handle / grip */}
      <div className="pl-1.5 pr-0.5 text-[#888888] cursor-grab active:cursor-grabbing hover:text-[#FFFFEB] transition-colors">
        <Move className="w-3.5 h-3.5" />
      </div>

      {/* Status indicator dot */}
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-full bg-[#2A2A2A]">
        <span
          className={`w-2 h-2 rounded-full transition-all duration-300 ${
            enabled && isAgentActive
              ? "bg-[#00f3ff] shadow-[0_0_8px_#00f3ff] animate-pulse"
              : enabled
              ? "bg-[#FFA946]"
              : "bg-[#666666]"
          }`}
        />
        <span className="text-[11px] font-mono tracking-wider font-semibold uppercase text-[#CCCCCC]">
          {enabled && isAgentActive ? "THINKING" : enabled ? "ARMED" : "OFF"}
        </span>
      </div>

      {/* Pill Toggle Button */}
      <button
        onClick={() => setEnabled(!enabled)}
        className={`px-3 py-1 rounded-full text-[12px] font-semibold transition-all duration-200 flex items-center gap-1 cursor-pointer ${
          enabled
            ? "bg-[#034F46] text-[#FFFFEB] hover:bg-[#023932]"
            : "bg-[#333333] text-[#888888] hover:bg-[#444444]"
        }`}
        title="Toggle Vibe Play (Alt+W)"
      >
        <Power className="w-3 h-3" />
        <span>{enabled ? "ON" : "OFF"}</span>
      </button>

      {/* Gamepad Launch Button */}
      <button
        onClick={onOpenGame}
        className="w-7 h-7 rounded-full bg-[#F0D7FF] text-[#1A1A1A] hover:bg-[#E8C4FF] flex items-center justify-center transition-all duration-200 hover:scale-105 active:scale-95 cursor-pointer shadow-sm"
        title="Launch Micro-Game"
      >
        <Gamepad2 className="w-3.5 h-3.5 text-[#1A1A1A]" />
      </button>
    </div>
  );
};
