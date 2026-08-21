import { useState } from "react";
import { useAppSettings, useSetTrackedOverride } from "../lib/queries";

export function Settings() {
  const settings = useAppSettings();
  const setOverride = useSetTrackedOverride();
  const [draft, setDraft] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);

  const s = settings.data;
  const input = draft ?? s?.tracked_override ?? "";

  async function save(steamid: string | null) {
    setError(null);
    setSaved(null);
    try {
      await setOverride.mutateAsync(steamid);
      setDraft(null);
      setSaved(
        steamid
          ? `Now tracking ${steamid}.`
          : "Override cleared — back to the auto-detected player.",
      );
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="content">
      <div className="section-head">
        <h1>Settings</h1>
      </div>

      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}
      {saved && (
        <p className="settings-saved" role="status">
          {saved}
        </p>
      )}

      {s && (
        <div className="settings-grid">
          <section className="settings-card">
            <h2 className="corpus-subhead">Tracked player</h2>
            <p className="settings-line">
              <span className="settings-label">Coaching</span>
              <span className="mono">
                {s.tracked_name ? `${s.tracked_name} · ` : ""}
                {s.tracked_effective ?? "nobody yet — import a demo"}
              </span>
              {!s.tracked_override && s.tracked_effective && (
                <em className="settings-hint"> (auto-detected)</em>
              )}
            </p>
            <div className="settings-override">
              <input
                className="settings-input mono"
                placeholder="SteamID64, e.g. 76561199228328773"
                value={input}
                onChange={(e) => setDraft(e.target.value)}
                aria-label="Tracked SteamID64 override"
              />
              <button
                className="btn-primary"
                disabled={setOverride.isPending || input.trim() === ""}
                onClick={() => void save(input.trim())}
              >
                Track this player
              </button>
              {s.tracked_override && (
                <button
                  className="btn-secondary"
                  disabled={setOverride.isPending}
                  onClick={() => void save(null)}
                >
                  Clear override
                </button>
              )}
            </div>
            <p className="settings-hint">
              Applies to new imports. To re-analyze an existing match,
              delete it in the Library and import the demo again.
            </p>
          </section>

          <section className="settings-card">
            <h2 className="corpus-subhead">Detector thresholds</h2>
            <table className="grid-table">
              <tbody>
                {s.thresholds.map((t) => (
                  <tr key={t.name}>
                    <td>{t.name}</td>
                    <td>{t.value}</td>
                    <td>{t.unit}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p className="settings-hint">
              v0 ships these fixed defaults — tuned against real demos, not
              editable yet.
            </p>
          </section>

          <section className="settings-card">
            <h2 className="corpus-subhead">Data</h2>
            <p className="settings-line">
              <span className="settings-label">Database</span>
              <span className="mono settings-path">{s.db_path}</span>
            </p>
            <p className="settings-line">
              <span className="settings-label">Your matches</span>
              <span className="mono">{s.own_matches}</span>
            </p>
            <p className="settings-line">
              <span className="settings-label">Reference demos</span>
              <span className="mono">{s.corpus_demos}</span>
            </p>
          </section>
        </div>
      )}
    </div>
  );
}
