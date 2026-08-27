const FALLBACK = "something went wrong on the Rust side — the log has the details";
const MAX = 200;

/** Tauri rejects with the command's `Err(String)`; anything else is a bug
 *  we still describe calmly (§7 voice: what failed, never blame). */
export function errorMessage(e: unknown): string {
  const text = e instanceof Error ? e.message : typeof e === "string" ? e : "";
  const t = text.trim();
  return t.length === 0 ? FALLBACK : t.length > MAX ? t.slice(0, MAX) : t;
}
