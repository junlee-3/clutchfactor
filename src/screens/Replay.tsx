import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { CoachRail } from "../components/CoachRail";
import { KillFeed } from "../components/KillFeed";
import { RosterPanel } from "../components/RosterPanel";
import { Scrubber } from "../components/Scrubber";
import { StatsStrip } from "../components/StatsStrip";
import { Button } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { MatchHeader } from "../components/ui/MatchHeader";
import { Segmented } from "../components/ui/Segmented";
import { Skeleton } from "../components/ui/Skeleton";
import { useToast } from "../components/ui/Toast";
import { parseEvidenceParams } from "../lib/evidence";
import type { BombInfo, KillInfo, MatchDetail, RoundReviewDto } from "../lib/ipc";
import { mapName } from "../lib/mapName";
import {
  useCoachRounds,
  useCoachStatus,
  useMatchDetail,
  useMatches,
  useRegenerateCoachRound,
  useRoundReview,
  useRoundTicks,
} from "../lib/queries";
import { radarImageUrl } from "../replay/coords";
import type { MapCalibration } from "../replay/coords";
import { buildTracks, stateAt } from "../replay/interp";
import type { PlayerTrack } from "../replay/interp";
import { annotationMomentIndex } from "../replay/rail";
import { ReplayCanvas } from "../replay/ReplayCanvas";
import type { BombState, Scene } from "../replay/Renderer";
import { utilityWindows } from "../replay/utility";
import type { UtilityWindow } from "../replay/utility";
import type { TimelineSpec } from "../replay/timeline";

const SPEEDS = [1, 2, 4] as const;
type Speed = (typeof SPEEDS)[number];
const SPEED_OPTIONS = SPEEDS.map((s) => ({ value: String(s), label: `${s}×` }));

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

/** Skeleton for the tape's [radar well][side rail][coach rail] area, at
 * (approximately) final layout size (design-system.md §10: no layout shift
 * on data arrival) — shared by both loading branches below. The coach rail
 * column renders unconditionally here (not gated on the reviews fetch,
 * which this function knows nothing about) so the outer loading state and
 * the real 3-column layout never differ in shape.
 *
 * `standalone` marks this as its own screen-level loading state (the
 * round-tick reload, once the header/round-strip are already real content)
 * so it gets the one `role="status"` landmark itself. When nested inside
 * the full-screen loading branch, the outer `.rpl-shell` already owns that
 * landmark, so this stays a plain group — an extra wrapping div would also
 * break `.rpl-main`'s `flex: 1` (it needs to be a direct child of the
 * `.rpl-shell` flex column to fill the remaining height). */
function PlayerAreaSkeleton({ standalone }: { standalone?: boolean } = {}) {
  return (
    <div
      className="rpl-main"
      role={standalone ? "status" : undefined}
      aria-label={standalone ? "Loading round" : undefined}
    >
      <Skeleton kind="block" className="rpl-well-skeleton" />
      <div className="rpl-side">
        <Skeleton kind="card" count={2} />
      </div>
      <aside className="rpl-coach-rail">
        <Skeleton kind="card" count={1} />
      </aside>
    </div>
  );
}

export function Replay() {
  const navigate = useNavigate();
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
  // One fetch serves the coach rail (Task 7) and the round-strip attention
  // dots (this task) — both read the same RoundReviewDto[], threaded down.
  const reviews = useRoundReview(matchId);
  // Round number -> its review, for the strip's attention dots below (issue
  // #9 §3: presence/size carry attention, never color).
  const reviewByRound = useMemo(() => {
    const m = new Map<number, RoundReviewDto>();
    for (const rv of reviews.data ?? []) m.set(rv.round, rv);
    return m;
  }, [reviews.data]);
  const cal = useCalibration(d !== null);
  const mapCal = d && cal.data ? (cal.data[d.map] ?? null) : null;

  if (detail.isLoading || cal.isLoading) {
    return (
      <div className="rpl-shell" role="status" aria-label="Loading replay">
        <Skeleton kind="block" className="rpl-header-skeleton" />
        <Skeleton kind="rows" className="rpl-round-strip-skeleton" />
        <PlayerAreaSkeleton />
      </div>
    );
  }
  if (!d) {
    return (
      <EmptyState
        title="Match not found"
        body="This match may have been deleted from the library."
        action={{ label: "Back to library", onClick: () => navigate("/") }}
      />
    );
  }
  if (!mapCal) {
    return (
      <EmptyState
        title="No radar calibration yet"
        body={`Replay isn't available for ${mapName(d.map)} yet — its radar calibration hasn't been added.`}
        action={{ label: "Back to library", onClick: () => navigate("/") }}
      />
    );
  }

  const setRound = (n: number) => {
    setSearchParams({ round: String(n) });
  };

  return (
    <div className="rpl-shell">
      <MatchHeader
        map={d.map}
        score={{ a: d.score_a, b: d.score_b }}
        result={summary?.tracked_result ?? null}
        date={summary?.imported_at ?? null}
        strip={<StatsStrip matchId={matchId} />}
        back={{ to: "/", label: "← Library" }}
        crossLink={{ to: `/report/${matchId}`, label: "Read report →" }}
      />
      <div className="rpl-round-strip" role="tablist" aria-label="Rounds">
        {d.rounds.map((r) => {
          const rv = reviewByRound.get(r.number);
          const attention = rv?.attention ?? "none";
          const title =
            attention === "none"
              ? `Round ${r.number} — ${r.winner} (${r.reason})`
              : `Round ${r.number} — ${r.winner} (${r.reason}) · ${rv!.verdict_label}`;
          return (
            <button
              key={r.number}
              role="tab"
              aria-selected={r.number === round}
              className={`rpl-round-chip type-data rpl-round-chip-winner-${r.winner.toLowerCase()}${
                r.number === round ? " rpl-round-chip-active" : ""
              }`}
              title={title}
              onClick={() => setRound(r.number)}
            >
              {attention !== "none" && (
                <span
                  className={`rpl-att rpl-att-${attention}`}
                  aria-hidden="true"
                />
              )}
              {r.number}
            </button>
          );
        })}
      </div>
      {ticks.isLoading || !ticks.data ? (
        <PlayerAreaSkeleton standalone />
      ) : (
        <RoundPlayer
          // Remount on round / evidence change: playback state resets cleanly.
          key={`${matchId}:${round}:${evidence.tick ?? "start"}`}
          matchId={matchId}
          detail={d}
          round={round}
          mapCal={mapCal}
          roundTicksData={ticks.data}
          evidenceTick={evidence.tick}
          focus={evidence.focus}
          reviews={reviews.data}
          reviewsLoading={reviews.isLoading}
          reviewsError={reviews.isError}
          onRound={setRound}
        />
      )}
    </div>
  );
}

interface RoundPlayerProps {
  matchId: number;
  detail: MatchDetail;
  round: number;
  mapCal: MapCalibration;
  roundTicksData: import("../lib/ipc").RoundTicks;
  evidenceTick: number | null;
  focus: string[];
  reviews: RoundReviewDto[] | undefined;
  reviewsLoading: boolean;
  reviewsError: boolean;
  onRound: (round: number) => void;
}

function RoundPlayer({
  matchId,
  detail: d,
  round,
  mapCal,
  roundTicksData,
  evidenceTick,
  focus,
  reviews,
  reviewsLoading,
  reviewsError,
  onRound,
}: RoundPlayerProps) {
  const coachStatus = useCoachStatus();
  const coachOn = coachStatus.data?.enabled ?? false;
  const coach = useCoachRounds(matchId, coachOn);
  const regenerate = useRegenerateCoachRound();
  const toast = useToast();
  const coachRound = coach.data?.rounds.find((r) => r.round === round) ?? null;
  async function regenerateRound() {
    try {
      await regenerate.mutateAsync({ matchId, round });
    } catch (e) {
      toast.push("error", String(e));
    }
  }
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

  // This round's own review moments, read directly off the `reviews` fetch
  // this component already holds — decoupled from CoachRail's own
  // `activeMomentIndex` (last moment with tick <= displayTick), which can
  // only become true AT OR AFTER a moment's tick and therefore can never be
  // active during the -5s pre-roll. That pre-roll is the entire point of
  // the overlay window: showing the play develop BEFORE the death, while
  // the victim is still alive (issue #9 §5's mockup frame). CoachRail's own
  // highlight (the bolded moment in its list) is untouched by this — it
  // still uses `activeMomentIndex` for that unrelated purpose.
  const review = useMemo(
    () => reviews?.find((r) => r.round === round) ?? null,
    [reviews, round],
  );
  const moments = useMemo(() => review?.moments ?? [], [review]);

  // Whichever tracked_death moment's overlay window (-5s/+2s around the
  // death, per rail.ts's `overlayWindow`) CONTAINS displayTick right now —
  // by containment, not "most recently passed." Round change remounts this
  // whole component (the `key` on RoundPlayer in Replay()), which resets
  // `displayTick` back to the round start — overrides never leak across
  // rounds.
  const annotationIdx = useMemo(
    () => annotationMomentIndex(moments, displayTick, d.tickrate),
    [moments, displayTick, d.tickrate],
  );
  const annotationMoment = annotationIdx >= 0 ? moments[annotationIdx] : null;

  // While inside that window, the canvas annotation takes over both dimming
  // and the ink diagram; outside it (or with no active death moment), the
  // URL's evidence focus is what's dimmed and there's no annotation.
  const focusSet = useMemo(
    () => new Set(annotationMoment ? annotationMoment.focus : focus),
    [annotationMoment, focus],
  );

  const annotation = useMemo(() => {
    if (!annotationMoment) return null;
    // `focus` is presence-ordered, not fixed-slot (victim, then killer,
    // then nearest teammate, each only when known) — focus[1] is NOT
    // reliably the killer (e.g. it's the nearest teammate when the killer
    // is unknown). Read the killer off the explicit field instead.
    const victimId = annotationMoment.focus[0];
    return victimId
      ? { victimId, killerId: annotationMoment.killer ?? null }
      : null;
  }, [annotationMoment]);

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
      annotation,
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
      annotation,
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
    <div className="rpl-player">
      <div className="rpl-main">
        <div className="rpl-radar-well">
          <ReplayCanvas getScene={getScene} onFrame={onFrame} onFps={setFps} />
        </div>
        <aside className="rpl-side">
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
        {reviewsLoading ? (
          <aside className="rpl-coach-rail">
            <Skeleton kind="card" count={1} />
          </aside>
        ) : reviewsError || !reviews ? null : (
          <CoachRail
            reviews={reviews}
            round={round}
            spec={spec}
            tickrate={d.tickrate}
            displayTick={displayTick}
            onJump={seek}
            onRound={onRound}
            coach={coachRound}
            coachLoading={coachOn && (coach.isLoading || regenerate.isPending)}
            coachError={coach.data?.error ?? null}
            onRegenerate={coachOn ? () => void regenerateRound() : null}
          />
        )}
      </div>
      <div className="rpl-transport">
        <Button
          variant="primary"
          onClick={() => setPlaying(!playingRef.current)}
          aria-label={playing ? "Pause" : "Play"}
        >
          {playing ? "Pause" : "Play"}
        </Button>
        <Segmented
          options={SPEED_OPTIONS}
          value={String(speed)}
          onChange={(v) => setSpeed(Number(v) as Speed)}
          ariaLabel="Playback speed"
        />
        <Scrubber
          spec={spec}
          tick={displayTick}
          tickrate={d.tickrate}
          kills={roundKills}
          bombEvents={roundBombEvents}
          names={names}
          onSeek={seek}
        />
        <span className="type-micro" data-testid="fps">
          {fps} fps
        </span>
      </div>
    </div>
  );
}
