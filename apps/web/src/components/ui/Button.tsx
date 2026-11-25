import React from "react";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger" | "ghost" | "outline";
  size?: "sm" | "md" | "lg";
  icon?: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  variant = "primary",
  size = "md",
  icon,
  children,
  className = "",
  disabled,
  ...props
}) => {
  const variantStyles = {
    primary: "bg-lavender hover:bg-lavender-hover text-ink border border-ink/15",
    secondary: "bg-white hover:bg-canvas text-ink border border-ink/20 hover:border-ink/35",
    outline: "bg-transparent hover:bg-canvas-muted/50 text-ink border border-ink/25",
    danger: "bg-[#9B2C1F] hover:bg-[#82241A] text-white border border-transparent",
    ghost: "bg-transparent hover:bg-canvas-muted/60 text-ink-soft hover:text-ink border border-transparent",
  };

  const sizeStyles = {
    sm: "h-9 px-3 text-[13px] rounded-[10px]",
    md: "h-10 px-4 text-sm rounded-xl",
    lg: "h-11 px-5 text-sm rounded-xl",
  };

  return (
    <button
      className={`inline-flex items-center justify-center gap-2 font-semibold whitespace-nowrap transition-colors duration-200 motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-forest focus-visible:ring-offset-2 focus-visible:ring-offset-canvas disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${variantStyles[variant]} ${sizeStyles[size]} ${className}`}
      disabled={disabled}
      {...props}
    >
      {icon && <span className="shrink-0 inline-flex">{icon}</span>}
      {children && <span>{children}</span>}
    </button>
  );
};
