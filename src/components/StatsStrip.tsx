import { Link } from "react-router-dom";
import { useMatchStats, useReAnalyzeMatch } from "../lib/queries";
import { formatStat, STAT_KEYS, STAT_TITLES } from "../lib/statFormat";
import { Button } from "./ui/Button";
import { Skeleton } from "./ui/Skeleton";
import { useToast } from "./ui/Toast";

// No progress UI here — this is a header-sized action, not an import queue.
// re_analyze_match still streams Progress events over its channel either way.
function noopProgress() {
  /* the header strip has no room for a progress readout */
}

/** Coaching-first stats strip (spec §3). Every chip links to the rules
 *  behind the number on the Watches screen; a match analyzed before V1.4
 *  has no `match_stats` row and gets a placeholder with the one action
 *  that fixes it. */
export function StatsStrip({ matchId }: { matchId: number }) {
  const stats = useMatchStats(matchId);
  const reanalyze = useReAnalyzeMatch(noopProgress);
  const toast = useToast();

  if (stats.isLoading) {
    return (
      <div className="stats-strip" role="status" aria-label="Loading stats">
        <Skeleton kind="rows" count={1} className="stats-strip-skeleton" />
      </div>
    );
  }

  const s = stats.data ?? null;

  if (!s) {
    const reanalyzeForStats = async () => {
      try {
        const result = await reanalyze.mutateAsync({ matchId, path: null });
        if (result.needs_file) {
          // The demo moved since import — the file picker that resolves
          // this lives on the Library row's own re-analyze action; point
          // there rather than duplicating it in a header-sized widget.
          toast.push(
            "error",
            `Locate ${result.file_name} from Library to re-analyze this match.`,
          );
          return;
        }
        toast.push("status", "Re-analyzed — stats are in.");
      } catch (e) {
        toast.push("error", String(e));
      }
    };
    return (
      <div className="stats-strip stats-strip-empty">
        {STAT_KEYS.map((k) => (
          <span key={k} className="stats-chip stats-chip-empty">
            <span className="type-micro stats-chip-label">{STAT_TITLES[k]}</span>
            <span className="type-data stats-chip-value">—</span>
          </span>
        ))}
        <Button
          variant="secondary"
          size="sm"
          disabled={reanalyze.isPending}
          onClick={() => void reanalyzeForStats()}
        >
          {reanalyze.isPending ? "Re-analyzing…" : "Re-analyze for stats"}
        </Button>
      </div>
    );
  }

  return (
    <div className="stats-strip" aria-label="Match stats">
      {STAT_KEYS.map((k) => {
        const f = formatStat(k, s);
        return (
          <Link
            key={k}
            to={`/watches?stat=${k}`}
            className="stats-chip"
            title={`${f.detail} — what the coach watches`}
          >
            <span className="type-micro stats-chip-label">{STAT_TITLES[k]}</span>
            <span className="type-data stats-chip-value">{f.value}</span>
          </Link>
        );
      })}
    </div>
  );
}
