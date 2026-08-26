import { useState, type ReactNode } from "react";
import { Link, useNavigate } from "react-router-dom";
import type { PlayerRoundStatsDto } from "../lib/ipc";
import { useRoundScoreboard } from "../lib/queries";
import { aggregate, sortRoundRows, type ScoreRow } from "../lib/scoreboard";
import { Chip } from "./ui/Chip";
import { DataTable } from "./ui/DataTable";
import { EmptyState } from "./ui/EmptyState";
import { Segmented } from "./ui/Segmented";
import { Skeleton } from "./ui/Skeleton";

type Tab = "round" | "match";

interface Props {
  matchId: number;
  round: number | null;
}

const ROUND_HEAD = ["Player", "K", "A", "D", "DMG", "HS", "Entry", "Traded"];
const MATCH_HEAD = ["Player", "K", "A", "D", "ADR", "HS%", "KAST", "Entry", "Traded"];

function playerCell(name: string, side: string): ReactNode {
  return (
    <span className="sb-player">
      <Chip variant={side === "CT" ? "side-ct" : "side-t"} className="sb-side-chip">
        {side}
      </Chip>
      <span className="sb-player-name">{name}</span>
    </span>
  );
}

function entryLabel(entry: string | null): string {
  if (entry === "win") return "W";
  if (entry === "loss") return "L";
  return "—";
}

function roundRow(r: PlayerRoundStatsDto): ReactNode[] {
  return [
    playerCell(r.name, r.side),
    r.kills,
    r.assists,
    r.deaths,
    r.damage,
    r.headshots,
    entryLabel(r.entry),
    r.traded ? "✓" : "—",
  ];
}

function matchRow(r: ScoreRow): ReactNode[] {
  return [
    playerCell(r.name, r.side),
    r.kills,
    r.assists,
    r.deaths,
    r.adr.toFixed(1),
    r.hsPct === null ? "—" : `${r.hsPct}%`,
    r.kastPct === null ? "—" : `${r.kastPct}%`,
    r.entryAttempts > 0 ? `${r.entryWins}/${r.entryAttempts}` : "—",
    r.traded > 0 ? r.traded : "—",
  ];
}

/** The Report's per-round / whole-match scoreboard (spec Task 7). Reads the
 *  round the strip above has selected; `round: null` (the all-rounds query)
 *  backs the Match tab's client-side `aggregate`. A match analyzed before
 *  V1.4 has no `round_player_stats` rows at all — same "evidence pending"
 *  situation as StatsStrip, but the fix lives on the Library row, not here,
 *  so the empty state points there rather than duplicating a re-analyze
 *  control this component has no file path to drive. */
export function Scoreboard({ matchId, round }: Props) {
  const [tab, setTab] = useState<Tab>("round");
  const navigate = useNavigate();
  const scoreboard = useRoundScoreboard(matchId, tab === "round" ? round : null);

  const rawRows = scoreboard.data ?? [];
  const isEmpty = !scoreboard.isLoading && rawRows.length === 0;
  const roundRows = tab === "round" ? sortRoundRows(rawRows) : [];
  const matchRows = tab === "match" ? aggregate(rawRows) : [];

  const options = [
    { value: "round", label: round === null ? "Round" : `Round ${round}` },
    { value: "match", label: "Match" },
  ];

  return (
    <section className="sb" aria-label="Scoreboard">
      <div className="sb-head">
        <h3 className="type-heading sb-title">
          {round === null ? "Scoreboard" : `Round ${round} scoreboard`}
        </h3>
        {round !== null && (
          <Link to={`/replay/${matchId}?round=${round}`} className="sb-watch-link">
            Watch round {round} →
          </Link>
        )}
      </div>

      <Segmented
        options={options}
        value={tab}
        onChange={(v) => setTab(v as Tab)}
        ariaLabel="Scoreboard view"
      />

      {scoreboard.isLoading && (
        <div className="sb-loading" role="status" aria-label="Loading scoreboard">
          <Skeleton kind="rows" count={10} />
        </div>
      )}

      {isEmpty && (
        <EmptyState
          className="sb-empty"
          title="No scoreboard for this match yet"
          body="Re-analyze from the Library."
          action={{ label: "Go to Library", onClick: () => navigate("/") }}
        />
      )}

      {!scoreboard.isLoading &&
        !isEmpty &&
        (tab === "round" ? (
          <DataTable
            head={ROUND_HEAD}
            rows={roundRows.map(roundRow)}
            rowKey={(i) => roundRows[i].steamid}
            rowClassName={(i) => (roundRows[i].tracked ? "sb-row-tracked" : undefined)}
          />
        ) : (
          <DataTable
            head={MATCH_HEAD}
            rows={matchRows.map(matchRow)}
            rowKey={(i) => matchRows[i].steamid}
            rowClassName={(i) => (matchRows[i].tracked ? "sb-row-tracked" : undefined)}
          />
        ))}
    </section>
  );
}
