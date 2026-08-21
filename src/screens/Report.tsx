import { Link, useParams } from "react-router-dom";
import { ClassBreakdown } from "../components/ClassBreakdown";
import { HabitCard } from "../components/HabitCard";
import { InsightCard } from "../components/InsightCard";
import { RoundStripReport } from "../components/RoundStripReport";
import { MatchHeader } from "../components/ui/MatchHeader";
import { useHabits, useMatches, useMatchReport } from "../lib/queries";
import { CATEGORY_TITLES, groupInsights } from "../lib/report";

const TICKRATE = 64;

export function Report() {
  const { matchId: raw } = useParams();
  const matchId = Number(raw);
  const report = useMatchReport(matchId);
  const habits = useHabits();
  // MatchReport (per-command DTO) doesn't carry date/K-D/HS% — those live on
  // MatchSummary from the library list, which is already cached by the time
  // a match is opened from Library. Reuse it rather than growing the report
  // command's payload for a header-only need.
  const matches = useMatches();
  const summary = matches.data?.find((m) => m.id === matchId);

  if (report.isLoading) {
    return <div className="replay-shell centered">Building report…</div>;
  }
  const r = report.data;
  if (!r) {
    return (
      <div className="replay-shell centered">
        Match not found. <Link to="/">Back to library</Link>
      </div>
    );
  }

  const groups = groupInsights(r.insights);

  return (
    <div className="report-shell">
      <MatchHeader
        map={r.map}
        score={{ a: r.score_a, b: r.score_b }}
        result={r.tracked_result}
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
        crossLink={{ to: `/replay/${matchId}`, label: "Watch replay →" }}
      />

      <div className="report-main">
        <div className="report-feed">
          {r.summary && (
            <blockquote className="coach-note">
              <p className="cn-title">{r.summary.title}</p>
              <p>{r.summary.body}</p>
            </blockquote>
          )}

          <RoundStripReport matchId={matchId} rounds={r.per_round} />

          {groups.length === 0 && (
            <div className="empty-state">
              <p className="empty-title">Nothing recurring to coach</p>
              <p className="empty-note">
                No insight cleared the recurrence bar this match. The death
                breakdown on the right still shows how each round ended for
                you.
              </p>
            </div>
          )}

          {groups.map((g) => (
            <section key={g.category} className="insight-group">
              <h3>{CATEGORY_TITLES[g.category]}</h3>
              {g.insights.map((i, idx) => (
                <InsightCard
                  key={`${i.detector}-${idx}`}
                  matchId={matchId}
                  insight={i}
                  rounds={r.per_round}
                  tickrate={TICKRATE}
                />
              ))}
            </section>
          ))}
        </div>

        <aside className="report-side">
          <ClassBreakdown
            rows={r.death_classes}
            class13SharePct={r.class_13_share_pct}
            classesNotBuilt={r.classes_not_built}
          />
          <section className="habits" aria-label="Recurring habits">
            <h3>Across your matches</h3>
            {habits.data && habits.data.length > 0 ? (
              habits.data
                .slice(0, 4)
                .map((h, i) => <HabitCard key={`${h.rule_id}-${i}`} habit={h} />)
            ) : (
              <p className="empty-note">
                Habits appear once a pattern recurs in {3}+ of your recent
                matches.
              </p>
            )}
          </section>
        </aside>
      </div>
    </div>
  );
}
