import { skeletonClass, type SkeletonKind } from "./classes";

interface SkeletonProps {
  kind: SkeletonKind;
  count?: number;
  className?: string;
}

// Shimmer-free loading placeholder (design-system.md §4 motion restraint,
// §6): static bg2 blocks at final layout size, so data arrival never shifts
// the page (§10). Every screen's loading state uses this, never a bare
// sentence.
//
// Silent by default (no role/aria) — a screen can render several Skeleton
// groups per loading state, and a role="status" on each would spam the a11y
// tree with duplicate live regions. Each screen wraps its whole loading
// block in ONE `<div role="status" aria-label="Loading …">` instead.
export function Skeleton({ kind, count = 1, className }: SkeletonProps) {
  const cls = [skeletonClass(kind), className].filter(Boolean).join(" ");
  return (
    <div className="ui-skel-group">
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className={cls} aria-hidden="true" />
      ))}
    </div>
  );
}
