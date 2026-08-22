import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import type { TrendsDto } from "../lib/ipc";
import { mapName } from "../lib/mapName";
import { useMatches, useTrends } from "../lib/queries";
import { getToken } from "../lib/theme";
import { extrema, sparkPoints, streakCallout } from "../lib/trends";
import { Card } from "../components/ui/Card";
import { EmptyState } from "../components/ui/EmptyState";
import { Segmented } from "../components/ui/Segmented";
import { Skeleton } from "../components/ui/Skeleton";

const ALL_MAPS = "all";

const SPARK_W = 220;
const SPARK_H = 26;
const LINE_W = 560;
const LINE_H = 72;

function polyline(values: number[], w: number, h: number): string {
  return sparkPoints(values, w, h)
    .map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join(" ");
}

/** Where a point-side annotation sits without running off the box: below
 *  the point when it's in the chart's top half (room underneath), above it
 *  otherwise; anchored so text near either edge doesn't clip. */
function annotationPos(
  x: number,
  y: number,
  w: number,
): { x: number; y: number; anchor: "start" | "middle" | "end" } {
  const dy = y < LINE_H / 2 ? 11 : -7;
  if (x < 28) return { x: x + 4, y: y + dy, anchor: "start" };
  if (x > w - 28) return { x: x - 4, y: y + dy, anchor: "end" };
  return { x, y: y + dy, anchor: "middle" };
}

/** Indexes of matches surviving the map filter (null = all). */
function filterIndexes(data: TrendsDto, map: string | null): number[] {
  return data.matches
    .map((m, i) => ({ m, i }))
    .filter(({ m }) => map === null || m.map === map)
    .map(({ i }) => i);
}

export function Trends() {
  const navigate = useNavigate();
  const trends = useTrends();
  const matches = useMatches();
  const [mapFilter, setMapFilter] = useState<string | null>(null);

  const resultById = useMemo(() => {
    const map = new Map<number, "win" | "loss" | "tie" | null>();
    for (const m of matches.data ?? []) map.set(m.id, m.tracked_result);
    return map;
  }, [matches.data]);

  const data = trends.data;
  const view = useMemo(() => {
    if (!data) return null;
    const idx = filterIndexes(data, mapFilter);
    return {
      matches: idx.map((i) => data.matches[i]),
      class13: idx.map((i) => data.matches[i].class13_pct),
      rules: data.rules
        .map((r) => ({
          ...r,
          counts: idx.map((i) => r.counts[i]),
        }))
        .filter((r) => r.counts.some((c) => c > 0)),
    };
  }, [data, mapFilter]);

  const maps = useMemo(
    () => [...new Set((data?.matches ?? []).map((m) => m.map))],
    [data],
  );

  const isLoading = trends.isLoading;
  const enough = (view?.matches.length ?? 0) >= 2;

  return (
    <div className="trends">
      <div className="trends-head">
        <h1 className="type-display">Trends</h1>
        {maps.length > 1 && (
          <Segmented
            ariaLabel="Map filter"
            value={mapFilter ?? ALL_MAPS}
            onChange={(v) => setMapFilter(v === ALL_MAPS ? null : v)}
            options={[
              { value: ALL_MAPS, label: "All maps" },
              ...maps.map((m) => ({ value: m, label: mapName(m) })),
            ]}
          />
        )}
      </div>

      {isLoading ? (
        <div className="trends-loading" role="status" aria-label="Loading trends">
          <Skeleton kind="rows" count={1} className="trends-ribbon-skeleton" />
          <Skeleton kind="block" className="trends-line-skeleton" />
          <Skeleton kind="rows" count={3} className="trends-rule-skeleton" />
        </div>
      ) : !enough ? (
        <EmptyState
          title="Not enough matches yet"
          body={`Trends need at least 2 matches${mapFilter ? ` on ${mapName(mapFilter)}` : ""} to plot a line — import more demos and patterns will show here.`}
          action={{ label: "Go to library", onClick: () => navigate("/") }}
        />
      ) : (
        view && (
          <>
            <div className="trends-ribbon" aria-label="Matches, oldest first">
              {view.matches.map((m) => {
                const result = resultById.get(m.match_id) ?? null;
                return (
                  <span
                    key={m.match_id}
                    className={`trends-ribbon-cell${result ? ` trends-ribbon-cell-${result}` : ""}`}
                    title={`${mapName(m.map)} · ${m.imported_at} · ${m.deaths} deaths${result ? ` · ${result}` : ""}`}
                  >
                    {m.map.replace(/^de_/, "").slice(0, 3)}
                  </span>
                );
              })}
              <span className="trends-ribbon-note type-micro">
                {view.matches.length} matches, oldest → newest
              </span>
            </div>

            <Card eyebrow="Pure aim duels" className="trends-line-card">
              <div className="trends-line-head">
                <p className="type-body trends-line-note">
                  Share of deaths that were fair fights lost, per match. Higher
                  is cleaner: these deaths were mechanics, not positioning —
                  the rows below are the fixable part.
                </p>
                <span className="type-data trends-line-value">
                  {view.class13[view.class13.length - 1].toFixed(0)}%
                </span>
              </div>
              <ClassLine values={view.class13} />
            </Card>

            <section className="trends-panel">
              <h2 className="type-heading trends-panel-title">
                Recurring mistakes per match
              </h2>
              {view.rules.length === 0 ? (
                <EmptyState
                  title="Clean run"
                  body={`Nothing recurring${mapFilter ? ` on ${mapName(mapFilter)}` : ""} across these matches.`}
                />
              ) : (
                <ul className="trends-rules">
                  {view.rules.map((r) => {
                    const last = r.counts[r.counts.length - 1];
                    const callout = streakCallout(r.title, r.counts);
                    const good = callout?.startsWith("Good news:") ?? false;
                    return (
                      <li key={r.rule_id} className="trends-rule-row">
                        <span className="type-ui trends-rule-title">{r.title}</span>
                        <svg
                          className="trends-spark"
                          viewBox={`0 0 ${SPARK_W} ${SPARK_H}`}
                          role="img"
                          aria-label={`${r.title}: ${r.counts.join(", ")}`}
                        >
                          <title>{r.counts.join(" · ")}</title>
                          <polyline
                            points={polyline(r.counts, SPARK_W, SPARK_H)}
                            fill="none"
                            stroke={getToken("--chalk-dim")}
                            strokeWidth="2"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                          <EndDot values={r.counts} w={SPARK_W} h={SPARK_H} />
                        </svg>
                        <span className="type-data trends-rule-count">{last}</span>
                        <span
                          className={`type-body trends-rule-callout${good ? " trends-rule-callout-good" : ""}`}
                        >
                          {callout ?? ""}
                        </span>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          </>
        )
      )}
    </div>
  );
}

/** The big class-13 line: full-weight chalk, min/max/last marked directly on
 *  the line (dataviz: "label the endpoint, the extreme" — never every
 *  point). A flat series skips min/max annotation — there's no extreme to
 *  call out beyond the line's own shape. */
function ClassLine({ values }: { values: number[] }) {
  const points = sparkPoints(values, LINE_W, LINE_H);
  const { minIndex, maxIndex } = extrema(values);
  const lastIndex = values.length - 1;
  const flat = minIndex === maxIndex;

  const dotIndexes = [...new Set([minIndex, maxIndex, lastIndex])];
  const labelIndexes = flat ? [] : [minIndex, maxIndex].filter((i) => i !== lastIndex);

  return (
    <svg
      className="trends-line"
      viewBox={`0 0 ${LINE_W} ${LINE_H}`}
      role="img"
      aria-label={`Class 13 share per match across ${values.length} matches, last ${values[lastIndex].toFixed(0)}%`}
    >
      <polyline
        points={polyline(values, LINE_W, LINE_H)}
        fill="none"
        stroke={getToken("--chalk")}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      {points.map((p, i) => (
        <circle key={`hit-${i}`} cx={p.x} cy={p.y} r={10} className="trends-line-hit">
          <title>{`${values[i].toFixed(0)}%`}</title>
        </circle>
      ))}
      {dotIndexes.map((i) => (
        <g key={`dot-${i}`}>
          <circle cx={points[i].x} cy={points[i].y} r={6} className="trends-line-dot-ring" />
          <circle cx={points[i].x} cy={points[i].y} r={4} className="trends-line-dot" />
        </g>
      ))}
      {labelIndexes.map((i) => {
        const pos = annotationPos(points[i].x, points[i].y, LINE_W);
        return (
          <text
            key={`label-${i}`}
            x={pos.x}
            y={pos.y}
            textAnchor={pos.anchor}
            className="trends-line-annotation"
          >
            {values[i].toFixed(0)}%
          </text>
        );
      })}
    </svg>
  );
}

/** The "current period" dot on a de-emphasized rule sparkline (dataviz stat-
 *  tile figure spec: line in the de-emphasis hue, current point in the
 *  accent) — ties the row's mono count to its place on the line. */
function EndDot({ values, w, h }: { values: number[]; w: number; h: number }) {
  const points = sparkPoints(values, w, h);
  if (points.length === 0) return null;
  const last = points[points.length - 1];
  return (
    <g>
      <circle cx={last.x} cy={last.y} r={4} className="trends-spark-dot-ring" />
      <circle cx={last.x} cy={last.y} r={3} className="trends-spark-dot" />
    </g>
  );
}
