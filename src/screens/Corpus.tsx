import { useState } from "react";
import { Link } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { basename } from "../lib/basename";
import type { ProgressEvent } from "../lib/ipc";
import {
  useBuildCorpus,
  useCorpusStatus,
  useGrid,
  useImportCorpusDemo,
} from "../lib/queries";
import { ImportProgress } from "../components/ImportProgress";
import { HeatmapCanvas } from "../components/HeatmapCanvas";

const PHASES = [
  { id: "freeze_end", label: "freeze end" },
  { id: "early", label: "early" },
  { id: "mid", label: "mid" },
  { id: "post_plant", label: "post-plant" },
] as const;

export function Corpus() {
  const status = useCorpusStatus();
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [importing, setImporting] = useState<string | null>(null);
  const [building, setBuilding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [side, setSide] = useState<"CT" | "T">("CT");
  const [phase, setPhase] = useState<string>("freeze_end");
  const [pickedMap, setPickedMap] = useState<string | null>(null);
  const importCorpus = useImportCorpusDemo(setProgress);
  const buildCorpus = useBuildCorpus(setProgress);

  const maps = status.data?.maps ?? [];
  const gate = status.data?.min_demos_per_map ?? 8;
  const grids = status.data?.grids ?? [];
  const map = pickedMap ?? maps[0]?.map ?? null;
  const grid = useGrid(map, side, phase);
  const gridMeta = grids.find(
    (g) => g.map === map && g.side === side && g.phase === phase,
  );

  async function pickAndImport() {
    setError(null);
    const paths = await open({
      multiple: true,
      filters: [{ name: "CS2 demo", extensions: ["dem"] }],
    });
    if (!Array.isArray(paths) || paths.length === 0) return;
    for (const [i, path] of paths.entries()) {
      const name = basename(path);
      setImporting(`${i + 1} of ${paths.length}: ${name}`);
      setProgress(null);
      try {
        await importCorpus.mutateAsync(path);
      } catch (e) {
        setError(`${name}: ${String(e)}`);
      }
    }
    setImporting(null);
  }

  async function build() {
    setError(null);
    setBuilding(true);
    setProgress(null);
    try {
      await buildCorpus.mutateAsync(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBuilding(false);
    }
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <span className="wordmark">ClutchFactor</span>
        <nav className="topnav">
          <Link className="topnav-link" to="/">
            Library
          </Link>
        </nav>
      </header>

      <main className="content">
        <div className="section-head">
          <h1>Reference corpus</h1>
          <div className="corpus-actions">
            <button
              className="btn-secondary"
              onClick={() => void build()}
              disabled={building || importing !== null || maps.length === 0}
            >
              {building ? "Building…" : "Build grids"}
            </button>
            <button
              className="btn-primary"
              onClick={() => void pickAndImport()}
              disabled={importing !== null || building}
            >
              {importing ? "Importing…" : "Add pro demos"}
            </button>
          </div>
        </div>

        {error && (
          <div className="error-banner" role="alert">
            {error}
          </div>
        )}

        {importing && <ImportProgress fileName={importing} progress={progress} />}
        {building && (
          <ImportProgress fileName="Building occupancy grids" progress={progress} />
        )}

        {maps.length === 0 && !importing ? (
          <div className="empty-state">
            <p className="empty-title">No reference demos yet</p>
            <p className="empty-note">
              Add pro demos (.dem) and ClutchFactor learns where strong players
              stand. {gate} demos per map unlock positioning insights.
            </p>
          </div>
        ) : (
          <div className="corpus-grid">
            <section className="corpus-inventory">
              <h2 className="corpus-subhead">Demos per map</h2>
              <ul className="gate-list">
                {maps.map((m) => (
                  <li key={m.map}>
                    <button
                      className={`gate-row${m.map === map ? " gate-row-active" : ""}`}
                      onClick={() => setPickedMap(m.map)}
                      title={`View ${m.map} heatmaps`}
                    >
                      <span className="gate-map">{m.map}</span>
                      <span
                        className="gate-meter"
                        role="img"
                        aria-label={`${m.demos} of ${gate} demos`}
                      >
                        {Array.from({ length: gate }, (_, i) => (
                          <i
                            key={i}
                            className={
                              i < m.demos ? "gate-cell gate-cell-filled" : "gate-cell"
                            }
                          />
                        ))}
                      </span>
                      <span className="gate-count">
                        {m.demos}/{gate}
                        {m.demos < gate && (
                          <em className="gate-note">
                            {" "}
                            — detector silent until {gate}
                          </em>
                        )}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>

              <h2 className="corpus-subhead">Built grids</h2>
              {grids.length === 0 ? (
                <p className="empty-note">
                  Grids not built yet — press Build grids after adding demos.
                </p>
              ) : (
                <table className="grid-table">
                  <thead>
                    <tr>
                      <th>map</th>
                      <th>side</th>
                      <th>phase</th>
                      <th>demos</th>
                      <th>samples</th>
                      <th>built</th>
                    </tr>
                  </thead>
                  <tbody>
                    {grids.map((g) => (
                      <tr key={`${g.map}-${g.side}-${g.phase}`}>
                        <td>{g.map}</td>
                        <td>{g.side}</td>
                        <td>{g.phase.replace("_", " ")}</td>
                        <td>{g.demos}</td>
                        <td>{g.samples}</td>
                        <td>{g.built_at}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>

            <section className="corpus-viewer">
              <div className="viewer-controls">
                <select
                  className="map-select"
                  value={map ?? ""}
                  onChange={(e) => setPickedMap(e.target.value)}
                  aria-label="Map"
                >
                  {maps.map((m) => (
                    <option key={m.map} value={m.map}>
                      {m.map}
                    </option>
                  ))}
                </select>
                <div className="side-chips" role="radiogroup" aria-label="Side">
                  {(["CT", "T"] as const).map((s) => (
                    <button
                      key={s}
                      role="radio"
                      aria-checked={side === s}
                      className={`side-chip side-chip-${s.toLowerCase()}${
                        side === s ? " side-chip-active" : ""
                      }`}
                      onClick={() => setSide(s)}
                    >
                      {s}
                    </button>
                  ))}
                </div>
                <div
                  className="phase-strip"
                  role="radiogroup"
                  aria-label="Round phase"
                >
                  {PHASES.map((p) => (
                    <button
                      key={p.id}
                      role="radio"
                      aria-checked={phase === p.id}
                      className={`phase-chip${phase === p.id ? " phase-chip-active" : ""}`}
                      onClick={() => setPhase(p.id)}
                    >
                      {p.label}
                    </button>
                  ))}
                </div>
              </div>
              {map ? (
                <HeatmapCanvas grid={grid.data ?? null} map={map} />
              ) : (
                <p className="empty-note">Add demos to view heatmaps.</p>
              )}
              {gridMeta && (
                <p className="viewer-built">built {gridMeta.built_at}</p>
              )}
            </section>
          </div>
        )}
      </main>
    </div>
  );
}
