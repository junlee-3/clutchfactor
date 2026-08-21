// Display-name prettifier for map slugs ("de_mirage" -> "Mirage"), used by
// MatchHeader (design-system.md §8) for the Fraunces map-name moment.
//
// MIRROR: ported 1:1 from cf-narrator's map_name — keep both in sync:
//   src-tauri/crates/cf-narrator/src/templates.rs (fn map_name, ~:1165)
//
// Only the first character of the whole result gets uppercased (not each
// word) — so a compound slug like "de_dust2" becomes "Dust2", not "Dust 2".

const PREFIX_RE = /^(de|cs|ar)_/;

export function mapName(map: string): string {
  const raw = map.trim();
  if (raw.length === 0) return "";
  const stripped = raw.replace(PREFIX_RE, "").replace(/_/g, " ");
  return stripped.charAt(0).toUpperCase() + stripped.slice(1);
}
