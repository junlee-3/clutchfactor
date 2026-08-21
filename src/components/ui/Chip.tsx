import type { ReactNode } from "react";
import { chipClass, type ChipVariant } from "./classes";

interface ChipProps {
  variant?: ChipVariant;
  onClick?: () => void;
  className?: string;
  title?: string;
  children?: ReactNode;
}

// Mono, r-sm, one shape for chip/badge use (design-system.md §6). variant
// "evidence" is the chalk-annotation grammar (§5): dashed underline, always
// interactive, jumps to the tape — never used for decoration.
export function Chip({ variant = "default", onClick, className, title, children }: ChipProps) {
  const classes = [chipClass(variant), className].filter(Boolean).join(" ");
  if (onClick) {
    return (
      <button type="button" className={classes} onClick={onClick} title={title}>
        {children}
      </button>
    );
  }
  return (
    <span className={classes} title={title}>
      {children}
    </span>
  );
}
