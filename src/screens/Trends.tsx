import { useMemo, useState } from "react";
import type { TrendsDto } from "../lib/ipc";
import { useTrends } from "../lib/queries";
import { getToken } from "../lib/theme";
import { sparkPoints, streakCallout } from "../lib/trends";

const SPARK_W = 220;
const SPARK_H = 26;
const LINE_W = 560;
const LINE_H = 72;

function polyline(values: number[], w: number, h: number): string {
  return sparkPoints(values, w, h)
    .map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join(" ");
}

/** Indexes of matches surviving the map filter (null = all). */
function filterIndexes(data: TrendsDto, map: string | null): number[] {
  return data.matches
    .map((m, i) => ({ m, i }))
    .filter(({ m }) => map === null || m.map === map)
    .map(({ i }) => i);
}

export function Trends() {
  const trends = useTrends();
  const [mapFilter, setMapFilter] = useState<string | null>(null);

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

  const enough = (view?.matches.length ?? 0) >= 2;

  return (
    <div className="content">
      <div className="section-head">
        <h1>Trends</h1>
        {maps.length > 1 && (
          <div className="map-chips" role="radiogroup" aria-label="Map filter">
            <button
              role="radio"
              aria-checked={mapFilter === null}
              className={`phase-chip${mapFilter === null ? " phase-chip-active" : ""}`}
              onClick={() => setMapFilter(null)}
            >
              All maps
            </button>
            {maps.map((m) => (
              <button
                key={m}
                role="radio"
                aria-checked={mapFilter === m}
                className={`phase-chip${mapFilter === m ? " phase-chip-active" : ""}`}
                onClick={() => setMapFilter(m)}
              >
                {m.replace(/^de_/, "")}
              </button>
            ))}
          </div>
        )}
      </div>

      {trends.isLoading ? (
        <p className="empty-note">Loading trends…</p>
      ) : !enough ? (
        <div className="empty-state">
          <p className="empty-title">Not enough matches yet</p>
          <p className="empty-note">
            Trends need at least 2 matches
            {mapFilter ? ` on ${mapFilter}` : ""} — import more demos and
            patterns will show here.
          </p>
        </div>
      ) : (
        view && (
          <>
            <div className="trend-ribbon" aria-label="Matches, oldest first">
              {view.matches.map((m) => (
                <span
                  key={m.match_id}
                  className="ribbon-cell"
                  title={`${m.map} · ${m.imported_at} · ${m.deaths} deaths`}
                >
                  {m.map.replace(/^de_/, "").slice(0, 3)}
                </span>
              ))}
              <span className="ribbon-note">
                {view.matches.length} matches, oldest → newest
              </span>
            </div>

            <section className="trend-panel">
              <h2 className="corpus-subhead">
                Pure aim duels — share of deaths that were fair fights lost
              </h2>
              <div className="trend-line-row">
                <svg
                  className="trend-line"
                  viewBox={`0 0 ${LINE_W} ${LINE_H}`}
                  role="img"
                  aria-label="Class 13 share per match"
                >
                  <polyline
                    points={polyline(view.class13, LINE_W, LINE_H)}
                    fill="none"
                    stroke={getToken("--chalk")}
                    strokeWidth="2"
                  />
                </svg>
                <span className="trend-last mono">
                  {view.class13[view.class13.length - 1].toFixed(0)}%
                </span>
              </div>
              <p className="trend-note">
                Higher is cleaner: these deaths were mechanics, not
                positioning. The rows below are the fixable part.
              </p>
            </section>

            <section className="trend-panel">
              <h2 className="corpus-subhead">Recurring mistakes per match</h2>
              {view.rules.length === 0 ? (
                <p className="empty-note">
                  Nothing recurring{mapFilter ? ` on ${mapFilter}` : ""} —
                  clean run.
                </p>
              ) : (
                <ul className="trend-rules">
                  {view.rules.map((r) => {
                    const last = r.counts[r.counts.length - 1];
                    const callout = streakCallout(r.title, r.counts);
                    return (
                      <li key={r.rule_id} className="trend-rule-row">
                        <span className="trend-rule-title">{r.title}</span>
                        <svg
                          className="trend-spark"
                          viewBox={`0 0 ${SPARK_W} ${SPARK_H}`}
                          role="img"
                          aria-label={`${r.title}: ${r.counts.join(", ")}`}
                        >
                          <title>{r.counts.join(" · ")}</title>
                          <polyline
                            points={polyline(r.counts, SPARK_W, SPARK_H)}
                            fill="none"
                            stroke={getToken("--chalk")}
                            strokeWidth="2"
                          />
                        </svg>
                        <span className="trend-last mono">{last}</span>
                        <span className="trend-callout">{callout ?? ""}</span>
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
