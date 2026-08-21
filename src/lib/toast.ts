// Pure toast queue (design-system.md §6). No Tauri/DOM/timer imports — the
// provider (Toast.tsx) drives this with real Date.now() and a poll interval,
// tests drive it with fixed clocks. `now` is always injected so the module
// stays deterministic and testable (repo convention, PROMPT.md §11.1).

export type ToastKind = "status" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  text: string;
  createdAt: number;
}

/** At most this many toasts are visible at once — the oldest is dropped
 * first (design-system.md §6). */
const MAX_VISIBLE = 3;

/** Auto-dismiss TTL, milliseconds. */
const TTL_MS = 5000;

/** Ids increase monotonically for as long as the queue holds members: each
 * new id is one past the highest id currently in the list. If the queue
 * empties out (every toast expired/dismissed) the count resets — safe,
 * since uniqueness only needs to hold among toasts rendered at once, and a
 * fully-drained queue has nothing left to collide with. */
function nextId(list: ToastItem[]): number {
  return list.reduce((max, t) => Math.max(max, t.id), 0) + 1;
}

/** Append a toast, capped at MAX_VISIBLE (oldest dropped first). */
export function addToast(
  list: ToastItem[],
  kind: ToastKind,
  text: string,
  now: number,
): ToastItem[] {
  const item: ToastItem = { id: nextId(list), kind, text, createdAt: now };
  const next = [...list, item];
  return next.length > MAX_VISIBLE ? next.slice(next.length - MAX_VISIBLE) : next;
}

/** Drop toasts at or past the 5s TTL. */
export function expire(list: ToastItem[], now: number): ToastItem[] {
  return list.filter((t) => now - t.createdAt < TTL_MS);
}
