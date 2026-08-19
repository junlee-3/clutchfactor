import type { KillInfo } from "../lib/ipc";

interface Props {
  kills: KillInfo[]; // this round's kills, sorted by tick
  tick: number;
  tickrate: number;
  names: Map<string, string>;
  sides: Map<string, "CT" | "T">;
  onJump: (tick: number) => void;
}

export function KillFeed({ kills, tick, tickrate, names, sides, onJump }: Props) {
  const shown = kills.filter((k) => k.tick <= tick).slice(-6);
  const cls = (sid: string | null) =>
    sid ? (sides.get(sid) === "T" ? "side-t" : "side-ct") : "side-none";
  const name = (sid: string | null) =>
    (sid && names.get(sid)) || (sid ? sid : "world");

  return (
    <div className="killfeed" aria-label="Kill feed">
      <h3>Kill feed</h3>
      {shown.length === 0 && <p className="killfeed-empty">No kills yet</p>}
      {shown.map((k, i) => (
        <button
          key={`${k.tick}-${i}`}
          className={`kill-row ${tick - k.tick < 3 * tickrate ? "recent" : ""}`}
          title="Jump to this kill"
          onClick={() => onJump(k.tick - 2 * tickrate)}
        >
          <span className={cls(k.attacker)}>{name(k.attacker)}</span>
          <span className="kill-weapon">
            {k.weapon}
            {k.headshot ? " ⌖" : ""}
          </span>
          <span className={cls(k.victim)}>{name(k.victim)}</span>
        </button>
      ))}
    </div>
  );
}
