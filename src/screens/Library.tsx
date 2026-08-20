import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { TopNav } from "../components/TopNav";
import { open } from "@tauri-apps/plugin-dialog";
import { basename } from "../lib/basename";
import {
  classifyFailure,
  summarizeBatch,
  type BatchSummary,
  type ImportOutcome,
} from "../lib/importBatch";
import type { ProgressEvent } from "../lib/ipc";
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
  const [importingFile, setImportingFile] = useState<string | null>(null);
  const [notice, setNotice] = useState<BatchSummary | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<number | null>(null);
  const importDemo = useImportDemo(setProgress);
  const deleteMatch = useDeleteMatch();

  async function reallyDelete(id: number) {
    setNotice(null);
    try {
      await deleteMatch.mutateAsync(id);
    } catch (e) {
      setNotice({ message: String(e), hadFailures: true });
    } finally {
      setConfirmDelete(null);
    }
  }

  async function pickAndImport() {
    setNotice(null);
    const picked = await open({
      multiple: true,
      filters: [{ name: "CS2 demo", extensions: ["dem"] }],
    });
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    if (paths.length === 0) return;

    // Sequential on purpose. The store sits behind a mutex and a parse peaks
    // over a gigabyte of RSS, so importing concurrently would contend on the
    // lock and multiply peak memory for no wall-clock gain. One failure must
    // not abort the rest of the batch, so every result is collected instead.
    const outcomes: ImportOutcome[] = [];
    for (const [i, path] of paths.entries()) {
      const file = basename(path);
      setImportingFile(
        paths.length > 1 ? `${i + 1} of ${paths.length}: ${file}` : file,
      );
      setProgress(null);
      try {
        await importDemo.mutateAsync(path);
        outcomes.push({ kind: "imported", file });
      } catch (e) {
        outcomes.push(classifyFailure(file, e));
      }
    }
    setImportingFile(null);
    setNotice(summarizeBatch(outcomes));
  }

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
            disabled={importingFile !== null}
          >
            {importingFile ? "Importing…" : "Import demos"}
          </button>
        </div>

        {notice && (
          <div
            className={notice.hadFailures ? "error-banner" : "info-banner"}
            role={notice.hadFailures ? "alert" : "status"}
          >
            {notice.message}
          </div>
        )}

        {importingFile && (
          <ImportProgress fileName={importingFile} progress={progress} />
        )}

        {matches.isLoading ? (
          <p className="empty-note">Loading library…</p>
        ) : rows.length === 0 && !importingFile ? (
          <div className="empty-state">
            <p className="empty-title">No matches yet</p>
            <p className="empty-note">
              Import CS2 demos (.dem) — matchmaking or FACEIT, one or a whole
              folder at a time — and ClutchFactor will break them down round by
              round.
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
