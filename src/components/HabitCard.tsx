import { useNavigate } from "react-router-dom";
import { evidenceUrl } from "../lib/evidence";
import type { HabitReport } from "../lib/ipc";
import { mapName } from "../lib/mapName";
import { Card } from "./ui/Card";
import { Chip } from "./ui/Chip";

interface Props {
  habit: HabitReport;
}

// A cross-demo recurring pattern (Report.tsx side rail, capped at 4). Same
// Card + evidence-chip grammar as InsightCard, but no edge: HabitReport
// carries a recurrence `score`, not a per-instance 0-1 `severity`, so
// color-mixing an edge from it would misrepresent what the number means.
// The recurrence count is a plain "count" Chip (mono, chalk) — this is
// where the legacy build colored it with `--t` (T-side amber), a side hue
// leaking onto app chrome (design-system.md §2's `--ct`/`--t` rule).
export function HabitCard({ habit }: Props) {
  const navigate = useNavigate();
  return (
    <Card>
      <header className="rpt-head">
        <h4 className="type-heading">{habit.title}</h4>
        <Chip
          variant="count"
          title={`Recurred in ${habit.matches_hit} of your last ${habit.window} matches (${habit.total} times total)`}
        >
          {habit.matches_hit} matches
        </Chip>
      </header>
      <p className="rpt-body type-body">{habit.body}</p>
      {habit.evidence.length > 0 && (
        <div className="rpt-chips">
          {habit.evidence.map((he, i) => (
            <Chip
              key={i}
              variant="evidence"
              title={`Watch on ${mapName(he.map)}`}
              onClick={() =>
                navigate(
                  evidenceUrl(he.match_id, {
                    round: he.evidence.round,
                    tick_start: he.evidence.tick_start,
                    tick_end: he.evidence.tick_end,
                    focus_players: he.evidence.focus_players,
                    camera_hint: he.evidence.camera_hint ?? undefined,
                  }),
                )
              }
            >
              {mapName(he.map)} R{he.evidence.round}
            </Chip>
          ))}
        </div>
      )}
    </Card>
  );
}
