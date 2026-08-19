import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProgressEvent } from "../lib/ipc";
import { useImportDemo, useMatches, useTrackedPlayer } from "../lib/queries";
import { formatMatchRow } from "../lib/score";
import { ImportProgress } from "../components/ImportProgress";

export function Library() {
  const navigate = useNavigate();
  const matches = useMatches();
  const tracked = useTrackedPlayer();
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [importingFile, setImportingFile] = useState<string | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const importDemo = useImportDemo(setProgress);

  async function pickAndImport() {
    setImportError(null);
    const path = await open({
      multiple: false,
      filters: [{ name: "CS2 demo", extensions: ["dem"] }],
    });
    if (typeof path !== "string") return;
    setImportingFile(path.split("/").pop() ?? path);
    setProgress(null);
    try {
      await importDemo.mutateAsync(path);
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImportingFile(null);
    }
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
        <nav className="topnav">
          <Link className="topnav-link" to="/corpus">
            Reference corpus
          </Link>
        </nav>
      </header>

      <main className="content">
        <div className="section-head">
          <h1>Library</h1>
          <button
            className="btn-primary"
            onClick={() => void pickAndImport()}
            disabled={importingFile !== null}
          >
            {importingFile ? "Importing…" : "Import demo"}
          </button>
        </div>

        {importError && (
          <div className="error-banner" role="alert">
            {importError}
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
                </li>
              );
            })}
          </ul>
        )}
      </main>
    </div>
  );
}
