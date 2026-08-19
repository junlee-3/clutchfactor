import { Link, useParams } from "react-router-dom";
import { ClassBreakdown } from "../components/ClassBreakdown";
import { HabitCard } from "../components/HabitCard";
import { InsightCard } from "../components/InsightCard";
import { RoundStripReport } from "../components/RoundStripReport";
import { useHabits, useMatchReport } from "../lib/queries";
import { CATEGORY_TITLES, groupInsights } from "../lib/report";

const TICKRATE = 64;

export function Report() {
  const { matchId: raw } = useParams();
  const matchId = Number(raw);
  const report = useMatchReport(matchId);
  const habits = useHabits();

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
  const mapLabel = r.map.replace(/^(de|cs)_/, "");
  const resultClass = r.tracked_result ?? "none";

  return (
    <div className="report-shell">
      <header className="topbar">
        <Link to="/" className="back-link">
          ← Library
        </Link>
        <span className="replay-title">
          {mapLabel} ·{" "}
          <b className={`letter-${resultClass[0] ?? "n"}`}>
            {r.score_a}–{r.score_b}
          </b>
        </span>
        <Link to={`/replay/${matchId}`} className="back-link">
          Watch replay →
        </Link>
      </header>

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
