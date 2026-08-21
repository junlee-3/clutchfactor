import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { TopNav } from "../components/TopNav";
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
import {
  useDeleteMatch,
  useImportDemo,
  useMatches,
  useTrackedPlayer,
} from "../lib/queries";
import { formatMatchRow } from "../lib/score";
import { ImportProgress } from "../components/ImportProgress";

export function Library() {
  const navigate = useNavigate();
  const matches = useMatches();
  const tracked = useTrackedPlayer();
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [queue, setQueue] = useState<QueueFile[] | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<number | null>(null);
  const importDemo = useImportDemo(setProgress);
  const deleteMatch = useDeleteMatch();

  async function reallyDelete(id: number) {
    setDeleteError(null);
    try {
      await deleteMatch.mutateAsync(id);
    } catch (e) {
      setDeleteError(String(e));
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
  }

  const importing = queue !== null && !queueDone(queue);
  const current = queue?.find((f) => f.status === "importing") ?? null;
  const rows = matches.data ?? [];

  return (
    <div className="app-shell">
      <header className="topbar">
        <span className="wordmark">ClutchFactor</span>
        {tracked.data && (
          <span className="tracked-chip" title="Tracked player (auto-detected)">
            tracking {tracked.data}
          </span>
        )}
        <TopNav />
      </header>

      <main className="content">
        <div className="section-head">
          <h1>Library</h1>
          <button
            className="btn-primary"
            onClick={() => void pickAndImport()}
            disabled={importing}
          >
            {importing ? "Importing…" : "Import demos"}
          </button>
        </div>

        {deleteError && (
          <div className="error-banner" role="alert">
            {deleteError}
          </div>
        )}

        {queue && (
          <div className="import-queue">
            {importing && current && (
              <ImportProgress
                fileName={`${queue.indexOf(current) + 1} of ${queue.length}: ${current.name}`}
                progress={progress}
              />
            )}
            <ul className="queue-list">
              {queue.map((f) => (
                <li key={f.path} className={`queue-row queue-${f.status}`}>
                  <span className="import-file">{f.name}</span>
                  <span className="import-detail">
                    {f.status === "done" && "imported"}
                    {f.status === "skipped" && "already in library"}
                    {f.status === "failed" && f.error}
                    {f.status === "pending" && "waiting"}
                    {f.status === "importing" && "importing…"}
                  </span>
                </li>
              ))}
            </ul>
            {queueDone(queue) && (
              <div
                className={
                  queue.some((f) => f.status === "failed") ? "error-banner" : "queue-summary"
                }
                role={queue.some((f) => f.status === "failed") ? "alert" : "status"}
              >
                {queueSummary(queue)}
                <button className="btn-secondary" onClick={() => setQueue(null)}>
                  Dismiss
                </button>
              </div>
            )}
          </div>
        )}

        {matches.isLoading ? (
          <p className="empty-note">Loading library…</p>
        ) : rows.length === 0 && !importing ? (
          <div className="empty-state">
            <p className="empty-title">No matches yet</p>
            <p className="empty-note">
              Import a CS2 demo (.dem) — matchmaking or FACEIT — and
              ClutchFactor will break it down round by round.
            </p>
          </div>
        ) : (
          <ul className="match-list">
            {rows.map((m) => {
              const row = formatMatchRow(m);
              const outcome = row.resultLetter?.toLowerCase() ?? "none";
              return (
                <li key={m.id}>
                  <button
                    className={`match-row outcome-${outcome}`}
                    onClick={() => navigate(`/report/${m.id}`)}
                    title="Open match report"
                  >
                  <span className="map">{row.mapLabel}</span>
                  <span className="score">
                    {row.resultLetter && (
                      <b className={`letter-${outcome}`}>{row.resultLetter}</b>
                    )}
                    {row.scoreline}
                  </span>
                  <span className="stat">{row.kd ?? "—"}</span>
                  <span className="stat">{row.hs ?? ""}</span>
                  <span className="meta">{m.rounds} rounds</span>
                  <span className="meta date">{m.imported_at}</span>
                  </button>
                  {confirmDelete === m.id ? (
                    <span className="row-confirm">
                      <button
                        className="row-delete row-delete-armed"
                        onClick={() => void reallyDelete(m.id)}
                        disabled={deleteMatch.isPending}
                      >
                        Delete match
                      </button>
                      <button
                        className="row-delete"
                        onClick={() => setConfirmDelete(null)}
                      >
                        Cancel
                      </button>
                    </span>
                  ) : (
                    <button
                      className="row-delete"
                      title="Delete this match (the demo file is untouched — re-import any time)"
                      onClick={() => setConfirmDelete(m.id)}
                    >
                      Delete
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </main>
    </div>
  );
}
