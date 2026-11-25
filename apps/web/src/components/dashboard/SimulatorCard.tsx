import React, { useState, useRef } from "react";
import {
  Sparkles,
  Play,
  CheckCircle,
  XCircle,
  HelpCircle,
  FastForward,
  RotateCcw,
  Zap,
  Radio,
  Power,
} from "lucide-react";
import { AgentEvent, AgentEventPayload } from "@waitplay/shared";
import { Card } from "../ui/Card";
import { Button } from "../ui/Button";

export interface SimulatorCardProps {
  onSimulateEvent: (event: AgentEvent, data?: Record<string, unknown>) => void;
  sendToBridge?: (event: AgentEventPayload) => Promise<boolean>;
  isBridgeConnected: boolean;
}

export const SimulatorCard: React.FC<SimulatorCardProps> = ({
  onSimulateEvent,
  sendToBridge,
  isBridgeConnected,
}) => {
  const [activeScenario, setActiveScenario] = useState<string | null>(null);
  const [routeThroughBridge, setRouteThroughBridge] = useState<boolean>(isBridgeConnected);
  const scenarioTimersRef = useRef<NodeJS.Timeout[]>([]);

  const stopActiveScenario = () => {
    scenarioTimersRef.current.forEach((t) => clearTimeout(t));
    scenarioTimersRef.current = [];
    setActiveScenario(null);
  };

  const dispatchEvent = (event: AgentEvent, data?: Record<string, unknown>) => {
    onSimulateEvent(event, data);

    if (routeThroughBridge && sendToBridge && isBridgeConnected) {
      const payload: AgentEventPayload = {
        agent: "claude-code",
        sessionId: "demo-simulator-session",
        event,
        timestamp: Date.now(),
        data,
      };
      void sendToBridge(payload);
    }
  };

  // Scenario 1: Standard Task (Game launches after delay, completes after 8s)
  const runStandardScenario = () => {
    stopActiveScenario();
    setActiveScenario("standard");

    dispatchEvent("task_started");

    const t1 = setTimeout(() => {
      dispatchEvent("task_running");
    }, 2000);

    const t2 = setTimeout(() => {
      dispatchEvent("task_completed");
      setActiveScenario(null);
    }, 8000);

    scenarioTimersRef.current = [t1, t2];
  };

  // Scenario 2: Quick Task (Completes in 2s, before launch delay)
  const runQuickScenario = () => {
    stopActiveScenario();
    setActiveScenario("quick");

    dispatchEvent("task_started");

    const t1 = setTimeout(() => {
      dispatchEvent("task_completed");
      setActiveScenario(null);
    }, 2000);

    scenarioTimersRef.current = [t1];
  };

  // Scenario 3: Permission Request (Working -> Needs Input -> Resumes -> Completes)
  const runPermissionScenario = () => {
    stopActiveScenario();
    setActiveScenario("permission");

    dispatchEvent("task_started");

    const t1 = setTimeout(() => {
      dispatchEvent("waiting_for_input", {
        prompt: "Execute bash command `npm test`?",
      });
    }, 5000);

    const t2 = setTimeout(() => {
      dispatchEvent("task_running");
    }, 9000);

    const t3 = setTimeout(() => {
      dispatchEvent("task_completed");
      setActiveScenario(null);
    }, 13000);

    scenarioTimersRef.current = [t1, t2, t3];
  };

  // Scenario 4: Task Failure
  const runFailureScenario = () => {
    stopActiveScenario();
    setActiveScenario("failure");

    dispatchEvent("task_started");

    const t1 = setTimeout(() => {
      dispatchEvent("task_failed", {
        error: "Process terminated with exit code 1 (Command failed)",
      });
      setActiveScenario(null);
    }, 5500);

    scenarioTimersRef.current = [t1];
  };

  return (
    <Card
      title="Agent Event Simulator"
      subtitle="Test the state machine, launch delays, and game transitions without Claude Code"
      icon={<Sparkles className="w-[18px] h-[18px]" />}
      action={
        <div className="flex items-center gap-2 text-[13px]">
          <label className="flex items-center gap-2 cursor-pointer text-ink font-semibold">
            <Radio className="w-4 h-4 text-forest" />
            <span>Route via Bridge</span>
            <input
              type="checkbox"
              checked={routeThroughBridge}
              onChange={(e) => setRouteThroughBridge(e.target.checked)}
              className="w-4 h-4 rounded accent-forest cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white"
            />
          </label>
        </div>
      }
    >
      <div className="space-y-6">
        {/* Automated Scenario Presets */}
        <div>
          <div className="text-[11px] font-semibold uppercase tracking-[0.08em] text-ink-soft mb-3 flex items-center justify-between">
            <span>Automated Testing Scenarios</span>
            {activeScenario && (
              <button
                onClick={stopActiveScenario}
                className="text-xs normal-case tracking-normal text-[#9B2C1F] hover:underline flex items-center gap-1 font-semibold rounded focus:outline-none focus-visible:ring-2 focus-visible:ring-forest"
              >
                <Power className="w-3.5 h-3.5" />
                <span>Stop Scenario</span>
              </button>
            )}
          </div>
          <div className="grid grid-cols-1 min-[480px]:grid-cols-2 lg:grid-cols-4 gap-3">
            <button
              onClick={runStandardScenario}
              className={`p-4 rounded-2xl border text-left transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white ${
                activeScenario === "standard"
                  ? "bg-lavender/50 border-forest text-ink ring-1 ring-forest"
                  : "bg-white border-ink/20 hover:border-ink/40 hover:bg-canvas text-ink"
              }`}
            >
              <div className="flex items-center gap-2 font-semibold text-sm">
                <FastForward className="w-4 h-4 text-forest" />
                <span>Standard (8s)</span>
              </div>
              <div className="text-xs leading-[1.45] text-ink-soft mt-1.5">
                Game opens at 4s, results at 8s
              </div>
            </button>

            <button
              onClick={runQuickScenario}
              className={`p-4 rounded-2xl border text-left transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white ${
                activeScenario === "quick"
                  ? "bg-lavender/50 border-forest text-ink ring-1 ring-forest"
                  : "bg-white border-ink/20 hover:border-ink/40 hover:bg-canvas text-ink"
              }`}
            >
              <div className="flex items-center gap-2 font-semibold text-sm">
                <Zap className="w-4 h-4 text-forest" />
                <span>Quick (2s)</span>
              </div>
              <div className="text-xs leading-[1.45] text-ink-soft mt-1.5">
                Completes before 4s, no game
              </div>
            </button>

            <button
              onClick={runPermissionScenario}
              className={`p-4 rounded-2xl border text-left transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white ${
                activeScenario === "permission"
                  ? "bg-lavender/50 border-forest text-ink ring-1 ring-forest"
                  : "bg-white border-ink/20 hover:border-ink/40 hover:bg-canvas text-ink"
              }`}
            >
              <div className="flex items-center gap-2 font-semibold text-sm">
                <HelpCircle className="w-4 h-4 text-forest" />
                <span>Permission</span>
              </div>
              <div className="text-xs leading-[1.45] text-ink-soft mt-1.5">
                Pauses on input, then resumes
              </div>
            </button>

            <button
              onClick={runFailureScenario}
              className={`p-4 rounded-2xl border text-left transition-colors duration-200 motion-reduce:transition-none cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-white ${
                activeScenario === "failure"
                  ? "bg-[#FAEBE8] border-[#9B2C1F] text-ink ring-1 ring-[#9B2C1F]"
                  : "bg-white border-ink/20 hover:border-ink/40 hover:bg-canvas text-ink"
              }`}
            >
              <div className="flex items-center gap-2 font-semibold text-sm">
                <XCircle className="w-4 h-4 text-[#9B2C1F]" />
                <span>Failure</span>
              </div>
              <div className="text-xs leading-[1.45] text-ink-soft mt-1.5">
                Encounter task error
              </div>
            </button>
          </div>
        </div>

        {/* Manual Step Event Triggers */}
        <div className="pt-6 border-t border-ink/10">
          <div className="text-[11px] font-semibold uppercase tracking-[0.08em] text-ink-soft mb-3">
            Manual Event Controls
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="secondary"
              size="sm"
              icon={<RotateCcw className="w-4 h-4 text-ink-muted" />}
              onClick={() => dispatchEvent("session_started")}
            >
              Session Start
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon={<Play className="w-4 h-4 text-forest" />}
              onClick={() => dispatchEvent("task_started")}
            >
              Task Start
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon={<Zap className="w-4 h-4 text-forest" />}
              onClick={() => dispatchEvent("task_running")}
            >
              Task Running
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon={<HelpCircle className="w-4 h-4 text-[#9A5600]" />}
              onClick={() =>
                dispatchEvent("waiting_for_input", {
                  prompt: "Allow Claude Code to edit src/index.ts?",
                })
              }
            >
              Waiting for Input
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon={<CheckCircle className="w-4 h-4 text-forest" />}
              onClick={() => dispatchEvent("task_completed")}
            >
              Task Completed
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon={<XCircle className="w-4 h-4 text-[#9B2C1F]" />}
              onClick={() =>
                dispatchEvent("task_failed", {
                  error: "Compilation error: missing module reference",
                })
              }
            >
              Task Failed
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon={<Power className="w-4 h-4 text-ink-muted" />}
              onClick={() => dispatchEvent("session_ended")}
            >
              Session End
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
};
