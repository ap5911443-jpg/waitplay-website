import React from "react";
import { AgentStatus } from "@waitplay/shared";

export interface BadgeProps {
  variant?: "default" | "success" | "warning" | "danger" | "info" | "neutral";
  size?: "sm" | "md";
  pulse?: boolean;
  children: React.ReactNode;
  className?: string;
}

export const Badge: React.FC<BadgeProps> = ({
  variant = "default",
  size = "md",
  pulse = false,
  children,
  className = "",
}) => {
  const variantStyles = {
    default: "bg-canvas-muted text-ink border-ink/10",
    success: "bg-[#E3EFEA] text-forest border-forest/25",
    warning: "bg-[#FFEBD0] text-[#7A4300] border-orange/60",
    danger: "bg-[#F8E3DF] text-[#9B2C1F] border-[#9B2C1F]/25",
    info: "bg-lavender text-ink border-ink/10",
    neutral: "bg-white text-ink-soft border-ink/15",
  };

  const sizeStyles = {
    sm: "px-2 py-0.5 text-[11px]",
    md: "px-2.5 py-1 text-xs",
  };

  return (
    <span
      className={`inline-flex items-center font-semibold whitespace-nowrap rounded-full border ${variantStyles[variant]} ${sizeStyles[size]} ${className}`}
    >
      {pulse && (
        <span className="relative flex h-2 w-2 mr-1.5">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-current opacity-60"></span>
          <span className="relative inline-flex rounded-full h-2 w-2 bg-current"></span>
        </span>
      )}
      {children}
    </span>
  );
};

export const AgentStatusBadge: React.FC<{ status: AgentStatus }> = ({ status }) => {
  switch (status) {
    case "IDLE":
      return <Badge variant="neutral">Idle</Badge>;
    case "WORKING":
      return (
        <Badge variant="info" pulse>
          Working
        </Badge>
      );
    case "GAME_ACTIVE":
      return (
        <Badge variant="success" pulse>
          Playing • Working
        </Badge>
      );
    case "WAITING_FOR_INPUT":
      return (
        <Badge variant="warning" pulse>
          Waiting for Input
        </Badge>
      );
    case "COMPLETED":
      return <Badge variant="success">Completed</Badge>;
    case "FAILED":
      return <Badge variant="danger">Failed</Badge>;
  }
};
