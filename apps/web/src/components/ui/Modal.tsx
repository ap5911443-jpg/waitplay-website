import React, { useEffect } from "react";
import { X } from "lucide-react";

export interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  subtitle?: string;
  children: React.ReactNode;
  maxWidth?: "sm" | "md" | "lg" | "xl";
}

export const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  title,
  subtitle,
  children,
  maxWidth = "md",
}) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const maxWidthStyles = {
    sm: "max-w-sm",
    md: "max-w-md",
    lg: "max-w-lg",
    xl: "max-w-2xl",
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-ink/40">
      <div
        className={`bg-canvas border border-ink/15 rounded-[20px] w-full ${maxWidthStyles[maxWidth]} max-h-[calc(100vh-2rem)] flex flex-col shadow-popover overflow-hidden`}
      >
        <div className="flex items-start justify-between gap-4 px-5 sm:px-6 py-5 border-b border-ink/10 shrink-0">
          <div>
            <h3 className="font-serif text-2xl leading-7 font-normal text-ink">{title}</h3>
            {subtitle && <p className="text-[13px] leading-[1.45] text-ink-soft mt-1">{subtitle}</p>}
          </div>
          <button
            onClick={onClose}
            className="shrink-0 p-2 rounded-[10px] text-ink-soft hover:text-ink hover:bg-canvas-muted/60 transition-colors duration-200 motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-forest"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="p-5 sm:p-6 overflow-y-auto">{children}</div>
      </div>
    </div>
  );
};
