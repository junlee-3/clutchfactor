import type { BombInfo, KillInfo } from "../lib/ipc";
import { fmtClock, fracToTick, tickToFrac } from "../replay/timeline";
import type { TimelineSpec } from "../replay/timeline";

interface Props {
  spec: TimelineSpec;
  tick: number;
  tickrate: number;
  kills: KillInfo[];
  bombEvents: BombInfo[];
  names: Map<string, string>;
  onSeek: (tick: number) => void;
}

// The transport's scrub bar (design-system.md §9): chalk accent-color on the
// native range input — the app's one accent is chalk, never --ct. Kill pips
// are chalk-faint ticks; bomb pips are the round's one loss-colored marker
// (no legacy plant/defuse/explode split — §2 reserves win/loss-mixed marks
// for outcome and severity, not a decorative second palette).
export function Scrubber({
  spec,
  tick,
  tickrate,
  kills,
  bombEvents,
  names,
  onSeek,
}: Props) {
  const frac = tickToFrac(spec, tick);
  const name = (sid: string | null) =>
    (sid && names.get(sid)) || sid || "world";

  return (
    <div className="rpl-scrubber">
      <span className="rpl-scrubber-clock type-data">{fmtClock(spec, tick, tickrate)}</span>
      <div className="rpl-scrubber-body">
        <input
          type="range"
          className="rpl-scrubber-range"
          min={0}
          max={1000}
          value={Math.round(frac * 1000)}
          aria-label="Round timeline"
          aria-valuetext={fmtClock(spec, tick, tickrate)}
          onChange={(e) =>
            onSeek(fracToTick(spec, Number(e.target.value) / 1000))
          }
        />
        <div className="rpl-pips" aria-hidden="true">
          {kills.map((k, i) => (
            <button
              key={`k${i}`}
              className="rpl-pip rpl-pip-kill"
              style={{ left: `${tickToFrac(spec, k.tick) * 100}%` }}
              title={`${name(k.attacker)} → ${name(k.victim)} (${k.weapon})`}
              tabIndex={-1}
              onClick={() => onSeek(Math.max(spec.startTick, k.tick - 2 * tickrate))}
            />
          ))}
          {bombEvents.map((b, i) => (
            <button
              key={`b${i}`}
              className="rpl-pip rpl-pip-bomb"
              style={{ left: `${tickToFrac(spec, b.tick) * 100}%` }}
              title={`bomb ${b.kind}`}
              tabIndex={-1}
              onClick={() => onSeek(Math.max(spec.startTick, b.tick - 2 * tickrate))}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
