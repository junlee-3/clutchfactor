import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProgressEvent } from "../lib/ipc";
import {
  finishFile,
  initQueue,
  queueDone,
  startFile,
  type QueueFile,
} from "../lib/importQueue";
import {
  useBuildCorpus,
  useCorpusStatus,
  useGrid,
  useImportCorpusDemo,
} from "../lib/queries";
import { mapName } from "../lib/mapName";
import { ImportProgress } from "../components/ImportProgress";
import { HeatmapCanvas } from "../components/HeatmapCanvas";
import { Button } from "../components/ui/Button";
import { DataTable } from "../components/ui/DataTable";
import { EmptyState } from "../components/ui/EmptyState";
import { ImportQueuePanel } from "../components/ui/ImportQueuePanel";
import { Segmented } from "../components/ui/Segmented";
import { Skeleton } from "../components/ui/Skeleton";
import { cardClass } from "../components/ui/classes";

const PHASES = [
  { id: "freeze_end", label: "freeze end" },
  { id: "early", label: "early" },
  { id: "mid", label: "mid" },
  { id: "post_plant", label: "post-plant" },
] as const;

const SIDE_OPTIONS = [
  { value: "CT", label: "CT" },
  { value: "T", label: "T" },
] as const;

export function Corpus() {
  const status = useCorpusStatus();
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [queue, setQueue] = useState<QueueFile[] | null>(null);
  const [building, setBuilding] = useState(false);
  // Build-only error (V1.0 deferred minor, closed here): the import queue's
  // own per-file errors live in ImportQueuePanel now, so this state is
  // build_corpus's alone — cleared at the start of EITHER action so a stale
  // failure from a previous build never survives to sit alongside a fresh
  // import.
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
    const paths = await open({
      multiple: true,
      filters: [{ name: "CS2 demo", extensions: ["dem"] }],
    });
    if (!Array.isArray(paths) || paths.length === 0) return;
    setError(null);
    let q = initQueue(paths);
    setQueue(q);
    for (let i = 0; i < q.length; i++) {
      q = startFile(q, i);
      setQueue(q);
      setProgress(null);
      try {
        await importCorpus.mutateAsync(q[i].path);
        q = finishFile(q, i);
      } catch (e) {
        q = finishFile(q, i, String(e));
      }
      setQueue(q);
    }
  }

  const importing = queue !== null && !queueDone(queue);
  const current = queue?.find((f) => f.status === "importing") ?? null;

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

  const gridRows = grids.map((g) => [
    mapName(g.map),
    g.side,
    g.phase.replace("_", " "),
    g.demos,
    g.samples,
    g.built_at,
  ]);

  return (
    <div className="cps-shell">
      <div className="cps-head">
        <h1 className="type-display">Reference corpus</h1>
        <div className="cps-actions">
          <Button
            variant="secondary"
            onClick={() => void build()}
            disabled={building || importing || maps.length === 0}
          >
            {building ? "Building…" : "Build grids"}
          </Button>
          <Button
            variant="primary"
            onClick={() => void pickAndImport()}
            disabled={importing || building}
          >
            {importing ? "Importing…" : "Add pro demos"}
          </Button>
        </div>
      </div>

      {error && (
        <div className={`${cardClass("loss")} cps-error`} role="alert">
          <span className="type-body">{error}</span>
        </div>
      )}

      {queue && (
        <div className="cps-queue">
          <ImportQueuePanel
            queue={queue}
            progress={progress}
            current={current}
            onDismiss={() => setQueue(null)}
          />
        </div>
      )}
      {building && (
        <div className="cps-queue">
          <ImportProgress fileName="Building occupancy grids" progress={progress} />
        </div>
      )}

      {status.isLoading ? (
        <Skeleton kind="block" className="cps-loading-block" />
      ) : maps.length === 0 && !importing ? (
        <EmptyState
          title="No reference demos yet"
          body={`Add pro demos (.dem) and ClutchFactor learns where strong players stand. ${gate} demos per map unlock positioning insights.`}
          action={{ label: "Add pro demos", onClick: () => void pickAndImport() }}
        />
      ) : (
        <>
          <div className="cps-grid">
            <section className="cps-inventory">
              <h2 className="type-micro cps-eyebrow">Demos per map</h2>
              <ul className="cps-gate-list">
                {maps.map((m) => (
                  <li key={m.map}>
                    <button
                      type="button"
                      className={`cps-gate-row${m.map === map ? " cps-gate-row-active" : ""}`}
                      onClick={() => setPickedMap(m.map)}
                      title={`View ${mapName(m.map)} heatmaps`}
                    >
                      <span className="type-body cps-gate-map">{mapName(m.map)}</span>
                      <span
                        className="cps-gate-meter"
                        role="img"
                        aria-label={`${m.demos} of ${gate} demos`}
                      >
                        {Array.from({ length: gate }, (_, i) => (
                          <i
                            key={i}
                            className={`cps-gate-cell${i < m.demos ? " cps-gate-cell-filled" : ""}`}
                          />
                        ))}
                      </span>
                      <span className="type-data cps-gate-count">
                        {m.demos}/{gate}
                      </span>
                      {m.demos < gate && (
                        <span className="type-micro cps-gate-note">
                          silent until {gate}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
            </section>

            <section className="cps-viewer">
              <h2 className="type-micro cps-eyebrow">Viewer</h2>
              <div className="cps-controls">
                <select
                  value={map ?? ""}
                  onChange={(e) => setPickedMap(e.target.value)}
                  aria-label="Map"
                >
                  {maps.map((m) => (
                    <option key={m.map} value={m.map}>
                      {mapName(m.map)}
                    </option>
                  ))}
                </select>
                <Segmented
                  ariaLabel="Side"
                  value={side}
                  onChange={(v) => setSide(v as "CT" | "T")}
                  options={[...SIDE_OPTIONS]}
                />
                <Segmented
                  ariaLabel="Round phase"
                  value={phase}
                  onChange={setPhase}
                  options={PHASES.map((p) => ({ value: p.id, label: p.label }))}
                />
              </div>
              {map ? (
                <HeatmapCanvas grid={grid.data ?? null} map={map} />
              ) : (
                <p className="type-body cps-note">Add demos to view heatmaps.</p>
              )}
              {gridMeta && (
                <p className="type-micro cps-viewer-built">built {gridMeta.built_at}</p>
              )}
            </section>
          </div>

          <section className="cps-built">
            <h2 className="type-micro cps-eyebrow">Built grids</h2>
            {grids.length === 0 ? (
              <p className="type-body cps-note">
                Grids not built yet — press Build grids after adding demos.
              </p>
            ) : (
              <DataTable
                head={["map", "side", "phase", "demos", "samples", "built"]}
                rows={gridRows}
                rowKey={(i) => {
                  const g = grids[i];
                  return `${g.map}-${g.side}-${g.phase}`;
                }}
              />
            )}
          </section>
        </>
      )}
    </div>
  );
}
