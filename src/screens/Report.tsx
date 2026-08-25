import { useNavigate, useParams } from "react-router-dom";
import { ClassBreakdown } from "../components/ClassBreakdown";
import { HabitCard } from "../components/HabitCard";
import { InsightCard } from "../components/InsightCard";
import { RoundStripReport } from "../components/RoundStripReport";
import { EmptyState } from "../components/ui/EmptyState";
import { MatchHeader } from "../components/ui/MatchHeader";
import { Skeleton } from "../components/ui/Skeleton";
import { useHabits, useMatches, useMatchReport } from "../lib/queries";
import { CATEGORY_TITLES, groupInsights } from "../lib/report";

const TICKRATE = 64;

export function Report() {
  const navigate = useNavigate();
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
    // Skeletons at (approximately) final layout size, per §10 — no bare
    // loading sentence, no shift when the real header/lead/cards land.
    return (
      <div className="rpt-shell" role="status" aria-label="Loading report">
        <Skeleton kind="block" className="report-header-skeleton" />
        <div className="rpt-main">
          <div className="rpt-feed">
            <Skeleton kind="block" className="report-lead-skeleton" />
            <Skeleton kind="card" count={3} />
          </div>
          <div className="rpt-side">
            <Skeleton kind="card" count={2} />
          </div>
        </div>
      </div>
    );
  }

  const r = report.data;
  if (!r) {
    return (
      <EmptyState
        title="Match not found"
        body="This match may have been deleted from the library."
        action={{ label: "Back to library", onClick: () => navigate("/") }}
      />
    );
  }

  const groups = groupInsights(r.insights);

  return (
    <div className="rpt-shell">
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

      <div className="rpt-main">
        <div className="rpt-feed">
          {r.summary && (
            // The editorial lead (design-system.md §9): the coach's write-up
            // in its own voice, the one place the display-sans font speaks
            // at length. A solid 2px ink edge, not dashed — this isn't
            // evidence, it's furniture around a quote.
            <blockquote className="report-lead">
              <p className="report-lead-title type-title">{r.summary.title}</p>
              <p className="report-lead-body">{r.summary.body}</p>
            </blockquote>
          )}

          <RoundStripReport matchId={matchId} rounds={r.per_round} />

          {groups.length === 0 && (
            <EmptyState
              title="Nothing recurring to coach"
              body="No insight cleared the recurrence bar this match. The death breakdown on the right still shows how each round ended for you."
            />
          )}

          {groups.map((g) => (
            <section key={g.category} className="rpt-group">
              <h3 className="type-micro rpt-section-title">{CATEGORY_TITLES[g.category]}</h3>
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

        <aside className="rpt-side">
          <ClassBreakdown
            rows={r.death_classes}
            class13SharePct={r.class_13_share_pct}
            classesNotBuilt={r.classes_not_built}
          />
          <section className="report-habits" aria-label="Recurring habits">
            <h3 className="type-micro rpt-section-title">Across your matches</h3>
            {habits.data && habits.data.length > 0 ? (
              habits.data
                .slice(0, 4)
                .map((h, i) => <HabitCard key={`${h.rule_id}-${i}`} habit={h} />)
            ) : (
              <EmptyState
                title="No habits yet"
                body="Habits appear once a pattern recurs in 3+ of your recent matches."
              />
            )}
          </section>
        </aside>
      </div>
    </div>
  );
}
