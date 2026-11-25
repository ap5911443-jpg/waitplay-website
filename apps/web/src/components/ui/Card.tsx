import React from "react";

export interface CardProps {
  title?: string;
  subtitle?: string;
  icon?: React.ReactNode;
  action?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  footer?: React.ReactNode;
}

export const Card: React.FC<CardProps> = ({
  title,
  subtitle,
  icon,
  action,
  children,
  className = "",
  footer,
}) => {
  return (
    <div
      className={`bg-white border border-ink/10 rounded-[20px] overflow-hidden shadow-subtle h-full flex flex-col ${className}`}
    >
      {(title || icon || action) && (
        <div className="px-5 sm:px-6 py-5 border-b border-ink/10 flex flex-wrap items-start justify-between gap-x-3 gap-y-3">
          <div className="flex items-start gap-3 min-w-0 flex-1 basis-60">
            {icon && (
              <div className="shrink-0 w-10 h-10 rounded-xl bg-canvas border border-ink/10 text-forest flex items-center justify-center">
                {icon}
              </div>
            )}
            <div className="min-w-0">
              {title && <h3 className="font-serif text-[22px] leading-7 font-normal text-ink">{title}</h3>}
              {subtitle && <p className="text-[13px] leading-[1.45] text-ink-soft mt-0.5">{subtitle}</p>}
            </div>
          </div>
          {action && <div className="flex items-center gap-2 shrink-0">{action}</div>}
        </div>
      )}
      <div className="p-5 sm:p-6 flex-1">{children}</div>
      {footer && (
        <div className="px-5 sm:px-6 py-3 border-t border-ink/10 bg-canvas/60 text-xs text-ink-soft">
          {footer}
        </div>
      )}
    </div>
  );
};
