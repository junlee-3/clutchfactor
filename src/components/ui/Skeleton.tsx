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
export function Skeleton({ kind, count = 1, className }: SkeletonProps) {
  const cls = [skeletonClass(kind), className].filter(Boolean).join(" ");
  return (
    <div className="ui-skel-group" role="status" aria-label="Loading">
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className={cls} aria-hidden="true" />
      ))}
    </div>
  );
}
