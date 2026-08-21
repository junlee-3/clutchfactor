import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { KillFeed } from "../components/KillFeed";
import { RosterPanel } from "../components/RosterPanel";
import { Scrubber } from "../components/Scrubber";
import { MatchHeader } from "../components/ui/MatchHeader";
import { parseEvidenceParams } from "../lib/evidence";
import type { BombInfo, KillInfo, MatchDetail } from "../lib/ipc";
import { useMatchDetail, useMatches, useRoundTicks } from "../lib/queries";
import { radarImageUrl } from "../replay/coords";
import type { MapCalibration } from "../replay/coords";
import { buildTracks, stateAt } from "../replay/interp";
import type { PlayerTrack } from "../replay/interp";
import { ReplayCanvas } from "../replay/ReplayCanvas";
import type { BombState, Scene } from "../replay/Renderer";
import { utilityWindows } from "../replay/utility";
import type { UtilityWindow } from "../replay/utility";
import type { TimelineSpec } from "../replay/timeline";

const SPEEDS = [1, 2, 4] as const;
type Speed = (typeof SPEEDS)[number];

function useCalibration(enabled: boolean) {
  return useQuery({
    queryKey: ["map-data"],
    queryFn: async () => {
      const res = await fetch("/maps/map-data.json");
      if (!res.ok) throw new Error("map calibration data missing");
      return (await res.json()) as Record<string, MapCalibration>;
    },
    staleTime: Infinity,
    enabled,
  });
}

/** Image element without state: draw() polls `complete` each frame. */
function useImage(url: string | null): HTMLImageElement | null {
  return useMemo(() => {
    if (!url) return null;
    const el = new Image();
    el.src = url;
    return el;
  }, [url]);
}

export function Replay() {
  const { matchId: matchIdRaw } = useParams();
  const matchId = Number(matchIdRaw);
  const [searchParams, setSearchParams] = useSearchParams();
  const evidence = parseEvidenceParams(searchParams);

  const detail = useMatchDetail(matchId);
  const d = detail.data ?? null;
  // MatchDetail (per-command DTO) has no tracked_result/date/K-D/HS% — those
  // live on MatchSummary from the library list, already cached by the time
  // a match is opened from Library. Reuse it for the header (see Report.tsx).
  const matches = useMatches();
  const summary = matches.data?.find((m) => m.id === matchId);
  const roundCount = d?.rounds.length ?? 0;
  const round = Math.min(Math.max(evidence.round, 1), Math.max(roundCount, 1));
  const ticks = useRoundTicks(matchId, round);
  const cal = useCalibration(d !== null);
  const mapCal = d && cal.data ? (cal.data[d.map] ?? null) : null;

  if (detail.isLoading || cal.isLoading) {
    return <div className="replay-shell centered">Loading match…</div>;
  }
  if (!d) {
    return (
      <div className="replay-shell centered">
        Match not found. <Link to="/">Back to library</Link>
      </div>
    );
  }
  if (!mapCal) {
    return (
      <div className="replay-shell centered">
        No radar calibration for {d.map} — replay unavailable for this map yet.{" "}
        <Link to="/">Back to library</Link>
      </div>
    );
  }

  const setRound = (n: number) => {
    setSearchParams({ round: String(n) });
  };

  return (
    <div className="replay-shell">
      <MatchHeader
        map={d.map}
        score={{ a: d.score_a, b: d.score_b }}
        result={summary?.tracked_result ?? null}
        date={summary?.imported_at ?? null}
        stats={{
          kd:
            summary?.tracked_kills != null && summary?.tracked_deaths != null
              ? `${summary.tracked_kills}-${summary.tracked_deaths}`
              : null,
          hsPct:
            summary?.tracked_hs_pct != null
              ? `${Math.round(summary.tracked_hs_pct)}%`
              : null,
        }}
        back={{ to: "/", label: "← Library" }}
        crossLink={{ to: `/report/${matchId}`, label: "Read report →" }}
      />
      <div className="round-strip" role="tablist" aria-label="Rounds">
        {d.rounds.map((r) => (
          <button
            key={r.number}
            role="tab"
            aria-selected={r.number === round}
            className={`round-chip winner-${r.winner.toLowerCase()} ${
              r.number === round ? "active" : ""
            }`}
            title={`Round ${r.number} — ${r.winner} (${r.reason})`}
            onClick={() => setRound(r.number)}
          >
            {r.number}
          </button>
        ))}
      </div>
      {ticks.isLoading || !ticks.data ? (
        <div className="centered">Loading round data…</div>
      ) : (
        <RoundPlayer
          // Remount on round / evidence change: playback state resets cleanly.
          key={`${matchId}:${round}:${evidence.tick ?? "start"}`}
          detail={d}
          round={round}
          mapCal={mapCal}
          roundTicksData={ticks.data}
          evidenceTick={evidence.tick}
          focus={evidence.focus}
        />
      )}
    </div>
  );
}

interface RoundPlayerProps {
  detail: MatchDetail;
  round: number;
  mapCal: MapCalibration;
  roundTicksData: import("../lib/ipc").RoundTicks;
  evidenceTick: number | null;
  focus: string[];
}

function RoundPlayer({
  detail: d,
  round,
  mapCal,
  roundTicksData,
  evidenceTick,
  focus,
}: RoundPlayerProps) {
  const roundInfo = d.rounds.find((r) => r.number === round) ?? null;
  const spec: TimelineSpec = useMemo(
    () =>
      roundInfo
        ? {
            startTick: roundInfo.freeze_end_tick ?? roundInfo.start_tick,
            endTick: roundInfo.officially_ended_tick ?? roundInfo.end_tick,
          }
        : { startTick: 0, endTick: 1 },
    [roundInfo],
  );
  const initialTick =
    evidenceTick !== null
      ? Math.min(Math.max(evidenceTick, spec.startTick), spec.endTick)
      : spec.startTick;

  const tracks: PlayerTrack[] = useMemo(
    () => buildTracks(roundTicksData),
    [roundTicksData],
  );
  const names = useMemo(
    () => new Map(d.players.map((p) => [p.steamid, p.name])),
    [d],
  );
  const sides = useMemo(
    () =>
      new Map(
        d.round_sides
          .filter((s) => s.number === round)
          .map((s) => [s.steamid, s.side]),
      ),
    [d, round],
  );
  const roundKills: KillInfo[] = useMemo(
    () => d.kills.filter((k) => k.round === round),
    [d, round],
  );
  const roundBombEvents: BombInfo[] = useMemo(
    () =>
      d.bomb_events.filter(
        (b) => b.tick >= spec.startTick && b.tick <= spec.endTick,
      ),
    [d, spec],
  );
  const killPositions = useMemo(() => {
    const m = new Map<KillInfo, { x: number; y: number; z: number }>();
    for (const k of roundKills) {
      const track = tracks.find((t) => t.steamid === k.victim);
      const s = track ? stateAt(track, k.tick) : null;
      if (s) m.set(k, { x: s.x, y: s.y, z: s.z });
    }
    return m;
  }, [roundKills, tracks]);
  const utility: UtilityWindow[] = useMemo(
    () =>
      utilityWindows(d.grenades, d.tickrate).filter(
        (u) => u.endTick >= spec.startTick && u.startTick <= spec.endTick,
      ),
    [d, spec],
  );
  const bomb: BombState | null = useMemo(() => {
    const plant = roundBombEvents.find((b) => b.kind === "planted");
    if (!plant) return null;
    const end = roundBombEvents.find(
      (b) => (b.kind === "defused" || b.kind === "exploded") && b.tick > plant.tick,
    );
    const planterTrack = plant.player
      ? tracks.find((t) => t.steamid === plant.player)
      : undefined;
    const s = planterTrack ? stateAt(planterTrack, plant.tick) : null;
    if (!s) return null;
    return {
      plantTick: plant.tick,
      endTick: Math.min(end?.tick ?? spec.endTick, spec.endTick),
      x: s.x,
      y: s.y,
    };
  }, [roundBombEvents, spec, tracks]);

  const upperImage = useImage(radarImageUrl(d.map, "upper"));
  const hasLower = mapCal.lower_level_max_units > -1000000.0;
  const lowerImage = useImage(hasLower ? radarImageUrl(d.map, "lower") : null);

  // Playback: tick advances in the rAF callback (60 fps) via tickRef;
  // displayTick mirrors it at ~10 Hz for the React panels.
  const tickRef = useRef(initialTick);
  const playingRef = useRef(false);
  const speedRef = useRef<Speed>(1);
  const [displayTick, setDisplayTick] = useState(initialTick);
  const [playing, setPlayingState] = useState(false);
  const [speed, setSpeedState] = useState<Speed>(1);
  const [fps, setFps] = useState(0);

  const setPlaying = useCallback((v: boolean) => {
    playingRef.current = v;
    setPlayingState(v);
  }, []);
  const setSpeed = useCallback((v: Speed) => {
    speedRef.current = v;
    setSpeedState(v);
  }, []);
  const seek = useCallback(
    (tick: number) => {
      const t = Math.min(Math.max(tick, spec.startTick), spec.endTick);
      tickRef.current = t;
      setDisplayTick(t);
    },
    [spec],
  );

  const focusSet = useMemo(() => new Set(focus), [focus]);
  const getScene = useCallback(
    (): Scene => ({
      cal: mapCal,
      upperImage,
      lowerImage,
      tracks,
      names,
      sides,
      kills: roundKills,
      killPositions,
      utility,
      bomb,
      tick: tickRef.current,
      tickrate: d.tickrate,
      focus: focusSet,
    }),
    [
      mapCal,
      upperImage,
      lowerImage,
      tracks,
      names,
      sides,
      roundKills,
      killPositions,
      utility,
      bomb,
      d.tickrate,
      focusSet,
    ],
  );

  const lastSync = useRef(0);
  const onFrame = useCallback(
    (dt: number) => {
      if (playingRef.current) {
        const next = tickRef.current + dt * d.tickrate * speedRef.current;
        tickRef.current = Math.min(next, spec.endTick);
        if (tickRef.current >= spec.endTick) {
          playingRef.current = false;
          setPlayingState(false);
        }
      }
      const now = performance.now();
      if (now - lastSync.current > 100) {
        lastSync.current = now;
        setDisplayTick(tickRef.current);
      }
    },
    [d.tickrate, spec.endTick],
  );

  // Window-scoped so transport works regardless of focus (round chips live
  // outside this subtree; scrubber/buttons stay individually operable).
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code === "Space") {
        e.preventDefault();
        setPlaying(!playingRef.current);
      } else if (e.code === "ArrowLeft" || e.code === "ArrowRight") {
        e.preventDefault();
        const dir = e.code === "ArrowRight" ? 1 : -1;
        const step = (e.shiftKey ? 10 : 2) * d.tickrate;
        seek(tickRef.current + dir * step);
      } else if (e.key === "1" || e.key === "2" || e.key === "3") {
        setSpeed(SPEEDS[Number(e.key) - 1]);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [d.tickrate, seek, setPlaying, setSpeed]);

  return (
    <div className="replay-player">
      <div className="replay-main">
        <div className="radar-wrap">
          <ReplayCanvas getScene={getScene} onFrame={onFrame} onFps={setFps} />
        </div>
        <aside className="replay-side">
          <RosterPanel
            tracks={tracks}
            names={names}
            sides={sides}
            tick={displayTick}
          />
          <KillFeed
            kills={roundKills}
            tick={displayTick}
            tickrate={d.tickrate}
            names={names}
            sides={sides}
            onJump={seek}
          />
        </aside>
      </div>
      <div className="transport">
        <button
          className="btn-primary"
          onClick={() => setPlaying(!playingRef.current)}
          aria-label={playing ? "Pause" : "Play"}
        >
          {playing ? "Pause" : "Play"}
        </button>
        <div className="speeds" role="group" aria-label="Playback speed">
          {SPEEDS.map((s) => (
            <button
              key={s}
              className={`speed-btn ${speed === s ? "active" : ""}`}
              onClick={() => setSpeed(s)}
            >
              {s}×
            </button>
          ))}
        </div>
        <Scrubber
          spec={spec}
          tick={displayTick}
          tickrate={d.tickrate}
          kills={roundKills}
          bombEvents={roundBombEvents}
          names={names}
          onSeek={seek}
        />
        <span className="fps-meter" data-testid="fps">
          {fps} fps
        </span>
      </div>
    </div>
  );
}
