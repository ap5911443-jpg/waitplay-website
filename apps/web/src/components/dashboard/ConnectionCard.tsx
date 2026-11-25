import React from "react";
import { Server, Wifi, WifiOff, Key, RefreshCw, Cpu } from "lucide-react";
import { BridgeHealth } from "@waitplay/shared";
import { BridgeStatus } from "../../hooks/useBridgeConnection";
import { Card } from "../ui/Card";
import { Badge } from "../ui/Badge";

export interface ConnectionCardProps {
  status: BridgeStatus;
  health: BridgeHealth | null;
  latencyMs: number | null;
  hasActiveSession: boolean;
  bridgeToken: string;
  onReconnect: () => void;
}

export const ConnectionCard: React.FC<ConnectionCardProps> = ({
  status,
  health,
  latencyMs,
  hasActiveSession,
  bridgeToken,
  onReconnect,
}) => {
  const isConnected = status === "connected";

  return (
    <Card
      title="Companion Service"
      subtitle="Local Node.js event bridge running on 127.0.0.1"
      icon={<Server className="w-[18px] h-[18px]" />}
      action={
        <button
          onClick={onReconnect}
          className="p-2 rounded-[10px] text-ink-soft hover:text-ink hover:bg-canvas-muted/60 border border-ink/15 transition-colors duration-200 motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-forest"
          title="Refresh Connection"
        >
          <RefreshCw className={`w-4 h-4 ${status === "connecting" ? "animate-spin" : ""}`} />
        </button>
      }
    >
      <div className="grid grid-cols-1 gap-3">
        {/* Bridge Status */}
        <div className="p-4 bg-canvas rounded-2xl border border-ink/10">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[13px] font-semibold text-ink">Bridge Status</span>
            {isConnected ? (
              <Badge variant="success" size="sm">
                <Wifi className="w-3 h-3 mr-1" />
                Connected
              </Badge>
            ) : status === "connecting" ? (
              <Badge variant="warning" size="sm" pulse>
                Connecting...
              </Badge>
            ) : (
              <Badge variant="danger" size="sm">
                <WifiOff className="w-3 h-3 mr-1" />
                Disconnected
              </Badge>
            )}
          </div>
          <div className="mt-2 text-xs font-mono text-ink-soft flex items-center justify-between">
            <span>Port: {health?.port ?? 3001}</span>
            {latencyMs !== null && (
              <span className="text-[11px] text-ink-soft">{latencyMs} ms latency</span>
            )}
          </div>
        </div>

        {/* Claude Code Session Status */}
        <div className="p-4 bg-canvas rounded-2xl border border-ink/10">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[13px] font-semibold text-ink">Claude Code</span>
            {hasActiveSession ? (
              <Badge variant="info" size="sm" pulse>
                <Cpu className="w-3 h-3 mr-1" />
                Active Session
              </Badge>
            ) : (
              <Badge variant="neutral" size="sm">
                Standby
              </Badge>
            )}
          </div>
          <div className="mt-2 text-xs font-mono text-ink-soft flex items-center justify-between truncate">
            <span className="flex items-center gap-1.5">
              <Key className="w-3 h-3 text-ink-muted shrink-0" />
              <span className="truncate max-w-[180px]">
                {bridgeToken ? `Token: ${bridgeToken.slice(0, 8)}...` : "Local Auth Active"}
              </span>
            </span>
          </div>
        </div>
      </div>
    </Card>
  );
};
