export const LEDGER_FIRST_MS = 400;
export const LEDGER_LAST_MS = 3600;
export const LEDGER_STAGGER_MS = 250;

/** "m:ss" → seconds. */
export function parseClock(t: string): number {
  const [m, s] = t.split(":").map(Number);
  return m * 60 + s;
}

/** Reveal delay per row: first at 400 ms, last at 3600 ms, linear in the
 *  timestamp; equal timestamps stagger by 250 ms so no two rows land at
 *  once. Rows are assumed to be in chronological order. */
export function ledgerSchedule(rows: readonly { t: string }[]): number[] {
  if (rows.length === 0) return [];
  const secs = rows.map((r) => parseClock(r.t));
  const first = secs[0];
  const span = secs[secs.length - 1] - first;
  const out: number[] = [];
  let prev = -Infinity;
  for (const s of secs) {
    let ms = span === 0 ? LEDGER_FIRST_MS : LEDGER_FIRST_MS + ((s - first) / span) * (LEDGER_LAST_MS - LEDGER_FIRST_MS);
    ms = Math.round(ms);
    if (ms <= prev) ms = prev + LEDGER_STAGGER_MS;
    out.push(ms);
    prev = ms;
  }
  return out;
}
