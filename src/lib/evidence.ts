// The evidence contract's frontend half (PROMPT.md §4): every Insight carries
// an EvidenceRef the replay viewer can jump to. M3 detectors will emit these;
// M2 also builds them from kill-feed/timeline clicks.

export interface EvidenceRef {
  round: number;
  tick_start: number;
  tick_end: number;
  focus_players: string[];
  camera_hint?: string;
}

export function evidenceUrl(matchId: number, ev: EvidenceRef): string {
  const params = new URLSearchParams();
  params.set("round", String(ev.round));
  params.set("tick", String(ev.tick_start));
  if (ev.focus_players.length > 0) {
    params.set("focus", ev.focus_players.join(","));
  }
  if (ev.camera_hint) params.set("camera", ev.camera_hint);
  return `/replay/${matchId}?${params.toString()}`;
}

export interface EvidenceParams {
  round: number;
  tick: number | null;
  focus: string[];
}

export function parseEvidenceParams(sp: URLSearchParams): EvidenceParams {
  const round = Number(sp.get("round") ?? "1");
  const tickRaw = sp.get("tick");
  const focusRaw = sp.get("focus");
  return {
    round: Number.isFinite(round) && round >= 1 ? Math.floor(round) : 1,
    tick: tickRaw !== null && Number.isFinite(Number(tickRaw)) ? Number(tickRaw) : null,
    focus: focusRaw ? focusRaw.split(",").filter(Boolean) : [],
  };
}
