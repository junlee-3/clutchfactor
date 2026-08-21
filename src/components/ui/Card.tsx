import type { ReactNode } from "react";
import { cardClass, type CardEdge } from "./classes";

interface CardProps {
  eyebrow?: string;
  edge?: CardEdge;
  /** 0-1 severity, only meaningful with edge="severity" — drives the
   * color-mix left edge (see .ui-card-edge-sev in components.css, the same
   * pattern as the legacy .ic-spine). */
  sevValue?: number;
  className?: string;
  children?: ReactNode;
}

// The one card surface (design-system.md §6): bg1, line border, r-md,
// s4 padding, optional eyebrow slot, optional win/loss/severity left edge.
export function Card({ eyebrow, edge, sevValue, className, children }: CardProps) {
  const classes = [cardClass(edge), className].filter(Boolean).join(" ");
  const style =
    edge === "severity" && sevValue !== undefined
      ? ({ "--sev": sevValue } as React.CSSProperties)
      : undefined;
  return (
    <div className={classes} style={style}>
      {eyebrow && <p className="ui-card-eyebrow type-micro">{eyebrow}</p>}
      {children}
    </div>
  );
}
