import type { TrackedPlayer } from "./ipc";

/** Sidebar label for the tracked player, with the issue #39 fallback chain:
 *  Steam persona / in-game name → shortened steamid → "Unknown player". The
 *  steamid is shortened rather than shown whole because 17 digits is exactly
 *  the unreadable string this chip exists to replace. */
export function trackedLabel(player: TrackedPlayer): string {
  const name = player.name?.trim();
  if (name) return name;
  const id = player.steamid?.trim();
  if (!id) return "Unknown player";
  return id.length > 10 ? `${id.slice(0, 4)}…${id.slice(-4)}` : id;
}

/** Two-letter placeholder for a missing avatar — the same glyph grammar the
 *  rail nav uses (design-system.md §7). One initial per word for names that
 *  have several, else the first two letters. Punctuation is skipped: clan
 *  tags and decorated names would otherwise render as "-=" or "[]". */
export function trackedInitials(label: string): string {
  const alnum = /\p{L}|\p{N}/u;
  const words = label
    .split(/[^\p{L}\p{N}]+/u)
    .filter((w) => w.length > 0);
  const picked =
    words.length >= 2
      ? words.slice(0, 2).map((w) => [...w][0])
      : [...label].filter((c) => alnum.test(c)).slice(0, 2);
  const out = picked.join("").toUpperCase();
  // A name made entirely of symbols still deserves something to show.
  return out || [...label.trim()][0] || "?";
}
