import type { DeathClassRow } from "../lib/ipc";
import { classLabel, GOOD_NEWS_CLASS, HYGIENE_CLASSES } from "../lib/report";
import { Card } from "./ui/Card";

interface Props {
  rows: DeathClassRow[];
  class13SharePct: number;
  classesNotBuilt: number[];
}

/** Horizontal bar list ranked by count (dataviz pass): single baseline,
 *  4px rounded data-end / square at the baseline, count labeled directly at
 *  the tip — no ring, no donut, no rainbow. Color is a status signal on a
 *  neutral scale, not a categorical palette: chalk for the ordinary
 *  preventable classes, win-green reserved ONLY for class 13 (the one
 *  "good news" status: a fair duel lost on mechanics, not a mistake), and
 *  chalk-faint for the two hygiene/unclassified classes so they read as
 *  quieter without drawing a border around them (a border-as-separator is
 *  a dataviz anti-pattern — the legacy .cb-hygiene did exactly this).
 *  Every bar keeps its label and count in text, so color is never the only
 *  encoding (design-system.md §10; dataviz color-formula "status colors are
 *  reserved, never color alone"). */
export function ClassBreakdown({ rows, class13SharePct, classesNotBuilt }: Props) {
  const counts = new Map<number, number>();
  for (const r of rows) counts.set(r.class_id, (counts.get(r.class_id) ?? 0) + 1);
  const entries = [...counts.entries()].sort((a, b) => b[1] - a[1]);
  const max = entries[0]?.[1] ?? 1;

  return (
    <Card eyebrow="How you died">
      {entries.map(([id, count]) => {
        const kind =
          id === GOOD_NEWS_CLASS
            ? "good"
            : HYGIENE_CLASSES.includes(id)
              ? "hygiene"
              : "preventable";
        return (
          <div
            key={id}
            className="rpt-cb-row"
            title={`${classLabel(id)}: ${count} of ${rows.length} deaths`}
          >
            <span className="rpt-cb-label type-data">{classLabel(id)}</span>
            <span className="rpt-cb-count type-data">{count}</span>
            <div className="rpt-cb-track" aria-hidden="true">
              <div
                className={`rpt-cb-fill rpt-cb-fill-${kind}`}
                style={{ width: `${(count / max) * 100}%` }}
              />
            </div>
          </div>
        );
      })}
      <p className="rpt-cb-note type-body">
        {class13SharePct}% were fair duels you lost on mechanics — good news:
        the rest had a fixable cause.
      </p>
      {classesNotBuilt.length > 0 && (
        <p className="rpt-cb-honesty type-ui">
          Not yet detected: {classesNotBuilt.map(classLabel).join(", ")}. Some
          deaths above may belong there.
        </p>
      )}
    </Card>
  );
}
