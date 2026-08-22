import { useState } from "react";
import { useAppSettings, useSetTrackedOverride } from "../lib/queries";
import { Button } from "../components/ui/Button";
import { Card } from "../components/ui/Card";
import { DataTable } from "../components/ui/DataTable";
import { Input } from "../components/ui/Input";
import { Skeleton } from "../components/ui/Skeleton";
import { useToast } from "../components/ui/Toast";

export function Settings() {
  const settings = useAppSettings();
  const setOverride = useSetTrackedOverride();
  const toast = useToast();
  const [draft, setDraft] = useState<string | null>(null);

  const s = settings.data;
  const input = draft ?? s?.tracked_override ?? "";

  // Routing both outcomes through the toast queue (design-system.md §6, §9:
  // "§7-voice errors via Toast or inline") closes the V1.0 deferred minor
  // for this screen by construction — a toast auto-expires (lib/toast.ts),
  // so a save's result can never linger as a stale banner once a later
  // save starts.
  async function save(steamid: string | null) {
    try {
      await setOverride.mutateAsync(steamid);
      setDraft(null);
      toast.push(
        "status",
        steamid
          ? `Now tracking ${steamid}.`
          : "Override cleared — back to the auto-detected player.",
      );
    } catch (e) {
      toast.push("error", String(e));
    }
  }

  if (settings.isLoading) {
    return (
      <div className="stg-shell">
        <div className="stg-head">
          <h1 className="type-display">Settings</h1>
        </div>
        <div className="stg-cards" role="status" aria-label="Loading settings">
          <Skeleton kind="card" count={3} className="stg-card-skeleton" />
        </div>
      </div>
    );
  }

  return (
    <div className="stg-shell">
      <div className="stg-head">
        <h1 className="type-display">Settings</h1>
      </div>

      {s && (
        <div className="stg-cards">
          <Card eyebrow="Tracked player">
            <p className="type-body stg-line">
              <span className="type-micro stg-label">Coaching</span>
              <span className="type-data">
                {s.tracked_name ? `${s.tracked_name} · ` : ""}
                {s.tracked_effective ?? "nobody yet — import a demo"}
              </span>
              {!s.tracked_override && s.tracked_effective && (
                <span className="type-body stg-hint-inline"> (auto-detected)</span>
              )}
            </p>
            <div className="stg-override">
              <Input
                mono
                placeholder="SteamID64, e.g. 76561199228328773"
                value={input}
                onChange={(e) => setDraft(e.target.value)}
                aria-label="Tracked SteamID64 override"
              />
              <Button
                variant="primary"
                disabled={setOverride.isPending || input.trim() === ""}
                onClick={() => void save(input.trim())}
              >
                Track this player
              </Button>
              {s.tracked_override && (
                <Button
                  variant="secondary"
                  disabled={setOverride.isPending}
                  onClick={() => void save(null)}
                >
                  Clear override
                </Button>
              )}
            </div>
            <p className="type-body stg-hint">
              Applies to new imports. To re-analyze an existing match, delete
              it in the Library and import the demo again.
            </p>
          </Card>

          <Card eyebrow="Detector thresholds">
            <DataTable
              head={["threshold", "value", "unit"]}
              rows={s.thresholds.map((t) => [t.name, t.value, t.unit])}
              rowKey={(i) => s.thresholds[i].name}
            />
            <p className="type-body stg-hint">
              v0 ships these fixed defaults — tuned against real demos, not
              editable yet.
            </p>
          </Card>

          <Card eyebrow="Data">
            <p className="type-body stg-line">
              <span className="type-micro stg-label">Database</span>
              <span className="type-data stg-path">{s.db_path}</span>
            </p>
            <p className="type-body stg-line">
              <span className="type-micro stg-label">Your matches</span>
              <span className="type-data">{s.own_matches}</span>
            </p>
            <p className="type-body stg-line">
              <span className="type-micro stg-label">Reference demos</span>
              <span className="type-data">{s.corpus_demos}</span>
            </p>
          </Card>
        </div>
      )}
    </div>
  );
}
