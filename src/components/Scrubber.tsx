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
    <div className="scrubber">
      <span className="clock">{fmtClock(spec, tick, tickrate)}</span>
      <div className="scrubber-body">
        <input
          type="range"
          className="scrubber-range"
          min={0}
          max={1000}
          value={Math.round(frac * 1000)}
          aria-label="Round timeline"
          aria-valuetext={fmtClock(spec, tick, tickrate)}
          onChange={(e) =>
            onSeek(fracToTick(spec, Number(e.target.value) / 1000))
          }
        />
        <div className="pips" aria-hidden="true">
          {kills.map((k, i) => (
            <button
              key={`k${i}`}
              className="pip pip-kill"
              style={{ left: `${tickToFrac(spec, k.tick) * 100}%` }}
              title={`${name(k.attacker)} → ${name(k.victim)} (${k.weapon})`}
              tabIndex={-1}
              onClick={() => onSeek(Math.max(spec.startTick, k.tick - 2 * tickrate))}
            />
          ))}
          {bombEvents.map((b, i) => (
            <button
              key={`b${i}`}
              className={`pip pip-bomb pip-${b.kind}`}
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
