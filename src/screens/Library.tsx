import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProgressEvent } from "../lib/ipc";
import {
  finishFile,
  initQueue,
  queueDone,
  queueSummary,
  startFile,
  type QueueFile,
} from "../lib/importQueue";
import { useDeleteMatch, useImportDemo, useMatches } from "../lib/queries";
import { formatMatchRow } from "../lib/score";
import { mapName } from "../lib/mapName";
import { Button } from "../components/ui/Button";
import { Card } from "../components/ui/Card";
import { EmptyState } from "../components/ui/EmptyState";
import { ImportQueuePanel } from "../components/ui/ImportQueuePanel";
import { Skeleton } from "../components/ui/Skeleton";
import { useToast } from "../components/ui/Toast";

const RESULT_WORD: Record<"W" | "L" | "T", string> = {
  W: "WON",
  L: "LOST",
  T: "TIE",
};

// Only win/loss get the Card's 2px edge (design-system.md §9) — a tie is
// neutral furniture, not a claim.
const EDGE_BY_RESULT: Record<"W" | "L" | "T", "win" | "loss" | undefined> = {
  W: "win",
  L: "loss",
  T: undefined,
};

export function Library() {
  const navigate = useNavigate();
  const matches = useMatches();
  const toast = useToast();
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [queue, setQueue] = useState<QueueFile[] | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<number | null>(null);
  const importDemo = useImportDemo(setProgress);
  const deleteMatch = useDeleteMatch();

  async function reallyDelete(id: number) {
    try {
      await deleteMatch.mutateAsync(id);
    } catch (e) {
      toast.push("error", String(e));
    } finally {
      setConfirmDelete(null);
    }
  }

  async function pickAndImport() {
    const picked = await open({
      multiple: true,
      filters: [{ name: "CS2 demo", extensions: ["dem"] }],
    });
    const paths = Array.isArray(picked) ? picked : typeof picked === "string" ? [picked] : [];
    if (paths.length === 0) return;
    let q = initQueue(paths);
    setQueue(q);
    for (let i = 0; i < q.length; i++) {
      q = startFile(q, i);
      setQueue(q);
      setProgress(null);
      try {
        await importDemo.mutateAsync(q[i].path);
        q = finishFile(q, i);
      } catch (e) {
        q = finishFile(q, i, String(e));
      }
      setQueue(q);
    }
    // The panel keeps the per-file record; the toast is just the ping that
    // the batch is done (§7 voice: what happened, then where to look).
    toast.push("status", queueSummary(q));
  }

  const importing = queue !== null && !queueDone(queue);
  const current = queue?.find((f) => f.status === "importing") ?? null;
  const rows = matches.data ?? [];
  const showSkeleton = matches.isLoading;
  const showEmpty = !showSkeleton && rows.length === 0 && !importing;

  return (
    <div className="library">
      <div className="library-head">
        <h1 className="type-display">Library</h1>
        <Button variant="primary" onClick={() => void pickAndImport()} disabled={importing}>
          {importing ? "Importing…" : "Import demos"}
        </Button>
      </div>

      {queue && (
        <div className="library-queue">
          <ImportQueuePanel
            queue={queue}
            progress={progress}
            current={current}
            onDismiss={() => setQueue(null)}
          />
        </div>
      )}

      {showSkeleton ? (
        <div className="library-loading" role="status" aria-label="Loading matches">
          <Skeleton kind="rows" count={6} className="library-row-skeleton" />
        </div>
      ) : showEmpty ? (
        <EmptyState
          title="No matches yet"
          body="Import a CS2 demo (.dem) — matchmaking or FACEIT — and ClutchFactor will break it down round by round."
          action={{ label: "Import demos", onClick: () => void pickAndImport() }}
        />
      ) : (
        <ul className="library-list">
          {rows.map((m) => {
            const row = formatMatchRow(m);
            const edge = row.resultLetter ? EDGE_BY_RESULT[row.resultLetter] : undefined;
            const resultClass = edge ?? "tie";
            const accessibleLabel = [
              mapName(m.map),
              row.resultLetter ? RESULT_WORD[row.resultLetter] : null,
              `score ${row.scoreline}`,
              row.kd ? `K-D ${row.kd}` : null,
              row.hs,
              `${m.rounds} rounds`,
              `imported ${m.imported_at}`,
            ]
              .filter(Boolean)
              .join(", ");
            return (
              <li key={m.id}>
                <Card edge={edge} className="library-row">
                  <button
                    type="button"
                    className="library-row-open"
                    onClick={() => navigate(`/report/${m.id}`)}
                    title="Open match report"
                    aria-label={accessibleLabel}
                  >
                    <span className="library-row-map type-title">{mapName(m.map)}</span>
                    <span className="library-row-score">
                      {row.resultLetter && (
                        <span className={`library-result library-result-${resultClass}`}>
                          {RESULT_WORD[row.resultLetter]}
                        </span>
                      )}
                      <span className="type-data">{row.scoreline}</span>
                    </span>
                    <span className="type-data library-row-stat">{row.kd ?? "—"}</span>
                    <span className="type-data library-row-stat">{row.hs ?? "—"}</span>
                    <span className="type-data library-row-meta">{m.rounds} rounds</span>
                    <span className="type-data library-row-meta">{m.imported_at}</span>
                  </button>
                  <div className="library-row-actions">
                    {confirmDelete === m.id ? (
                      <>
                        <Button
                          variant="danger"
                          size="sm"
                          className="ui-btn-armed"
                          onClick={() => void reallyDelete(m.id)}
                          disabled={deleteMatch.isPending}
                        >
                          Delete match
                        </Button>
                        <Button variant="secondary" size="sm" onClick={() => setConfirmDelete(null)}>
                          Cancel
                        </Button>
                      </>
                    ) : (
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={() => setConfirmDelete(m.id)}
                        title="Delete this match (the demo file is untouched — re-import any time)"
                      >
                        Delete
                      </Button>
                    )}
                  </div>
                </Card>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
