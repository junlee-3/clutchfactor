// Pure class-string builders for the ui/ primitives — the unit-testable
// surface (repo convention: logic lives in lib-like modules, components stay
// thin JSX over these). Every class name returned here is defined in
// src/styles/components.css. An unrecognized input falls back to the
// component's canonical default rather than emitting a class
// components.css never defines — callers can pass untyped/external strings
// safely.

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "md" | "sm";
export type CardEdge = "win" | "loss" | "severity";
export type ChipVariant = "default" | "evidence" | "side-ct" | "side-t" | "count";
export type SkeletonKind = "rows" | "card" | "block";

const BUTTON_VARIANTS: readonly ButtonVariant[] = [
  "primary",
  "secondary",
  "ghost",
  "danger",
];
const BUTTON_SIZES: readonly ButtonSize[] = ["md", "sm"];
const CARD_EDGES: readonly CardEdge[] = ["win", "loss", "severity"];
const CHIP_VARIANTS: readonly ChipVariant[] = [
  "default",
  "evidence",
  "side-ct",
  "side-t",
  "count",
];
const SKELETON_KINDS: readonly SkeletonKind[] = ["rows", "card", "block"];
/** kind -> the single-instance class it stacks (design-system.md §6: rows
 * 40px, card 96px, block 240px). */
const SKELETON_SHAPE: Record<SkeletonKind, string> = {
  rows: "ui-skel-row",
  card: "ui-skel-card",
  block: "ui-skel-block",
};

function isButtonVariant(value: string): value is ButtonVariant {
  return (BUTTON_VARIANTS as readonly string[]).includes(value);
}

function isButtonSize(value: string): value is ButtonSize {
  return (BUTTON_SIZES as readonly string[]).includes(value);
}

function isCardEdge(value: string): value is CardEdge {
  return (CARD_EDGES as readonly string[]).includes(value);
}

function isChipVariant(value: string): value is ChipVariant {
  return (CHIP_VARIANTS as readonly string[]).includes(value);
}

function isSkeletonKind(value: string): value is SkeletonKind {
  return (SKELETON_KINDS as readonly string[]).includes(value);
}

/** `ui-btn ui-btn-{variant} ui-btn-{size}` — unknown variant/size fall back
 * to the canonical default ("primary"/"md"). */
export function buttonClass(variant: ButtonVariant, size: ButtonSize = "md"): string {
  const v = isButtonVariant(variant) ? variant : "primary";
  const s = isButtonSize(size) ? size : "md";
  return `ui-btn ui-btn-${v} ui-btn-${s}`;
}

/** `ui-card` (+ `ui-card-edge-{win|loss|sev}` when an edge is given).
 * `severity` maps to the `sev` suffix to match the color-mix modifier in
 * components.css. Unknown edge falls back to no edge. */
export function cardClass(edge?: CardEdge): string {
  if (edge === undefined || !isCardEdge(edge)) return "ui-card";
  const suffix = edge === "severity" ? "sev" : edge;
  return `ui-card ui-card-edge-${suffix}`;
}

/** `ui-chip` (+ `ui-chip-{variant}` for anything but the default variant).
 * Unknown variant falls back to "default". */
export function chipClass(variant: ChipVariant = "default"): string {
  const v = isChipVariant(variant) ? variant : "default";
  return v === "default" ? "ui-chip" : `ui-chip ui-chip-${v}`;
}

/** `ui-skel ui-skel-{row|card|block}` — shimmer-free per the spec's motion
 * restraint (static bg2 block at final layout size). Unknown kind falls back
 * to "rows". */
export function skeletonClass(kind: SkeletonKind): string {
  const k = isSkeletonKind(kind) ? kind : "rows";
  return `ui-skel ${SKELETON_SHAPE[k]}`;
}

/** Roving-tabindex decision for one Segmented option. `activeIndex` is the
 * index of the option matching the controlled `value`, or -1 when nothing
 * matches (stale filter value, async default, call-site typo). Per the
 * WAI-ARIA APG radiogroup pattern: when a radio is checked, only it is
 * tabbable; when none is checked, the first radio is tabbable — so the
 * group is never unreachable by Tab. */
export function segTabIndex(index: number, activeIndex: number): 0 | -1 {
  const tabbable = activeIndex === -1 ? 0 : activeIndex;
  return index === tabbable ? 0 : -1;
}
