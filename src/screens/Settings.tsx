import { useState } from "react";
import { Link } from "react-router-dom";
import {
  useAppSettings,
  useSetTrackedOverride,
  useCoachStatus,
  useSetGeminiKey,
  useSetCoachModels,
  useSetCoachEnabled,
  useTestGeminiKey,
} from "../lib/queries";
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

  const coach = useCoachStatus();
  const setKey = useSetGeminiKey();
  const setModels = useSetCoachModels();
  const setEnabled = useSetCoachEnabled();
  const testKey = useTestGeminiKey();
  const [keyDraft, setKeyDraft] = useState("");
  const [roundModel, setRoundModel] = useState<string | null>(null);
  const [synthModel, setSynthModel] = useState<string | null>(null);

  async function saveKey() {
    try {
      await setKey.mutateAsync(keyDraft.trim());
      setKeyDraft("");
      // `enabled` is (not paused) && (key present): read it back after the
      // save so a key saved while the coach is paused says so instead of
      // promising a read that won't come until Resume coach.
      const fresh = await coach.refetch();
      toast.push(
        "status",
        (fresh.data?.enabled ?? true)
          ? "Gemini key saved — the coach is on. Open any match to see its read."
          : "Gemini key saved — the coach is paused; press Resume coach to use it.",
      );
    } catch (e) { toast.push("error", String(e)); }
  }
  async function removeKey() {
    try { await setKey.mutateAsync(null); toast.push("status", "Gemini key removed — back to the template captions."); }
    catch (e) { toast.push("error", String(e)); }
  }
  async function saveModels() {
    const c = coach.data;
    try {
      await setModels.mutateAsync({ roundModel: (roundModel ?? c?.round_model ?? "").trim(), synthesisModel: (synthModel ?? c?.synthesis_model ?? "").trim() });
      setRoundModel(null); setSynthModel(null);
      toast.push("status", "Coach models saved.");
    } catch (e) { toast.push("error", String(e)); }
  }
  async function test() {
    try { toast.push("status", await testKey.mutateAsync()); } catch (e) { toast.push("error", String(e)); }
  }

  const c = coach.data;

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
              Applies to new imports — and to any existing match you
              Re-analyze from the Library.
            </p>
          </Card>

          <Card eyebrow="Detector thresholds">
            <DataTable
              head={["threshold", "value", "unit"]}
              rows={s.thresholds.map((t) => [t.name, t.value, t.unit])}
              rowKey={(i) => s.thresholds[i].name}
            />
            <p className="type-body stg-hint">
              v0 ships these fixed defaults — see what each one means on{" "}
              <Link to="/watches" className="stg-hint-link">
                Watches
              </Link>
              .
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

          <Card eyebrow="Coach">
            {c && (
              <>
                <p className="type-body stg-line">
                  <span className="type-micro stg-label">Status</span>
                  <span className="type-data">
                    {c.key_source
                      ? `${c.enabled ? "on" : "paused"} · key ${c.key_hint} from ${c.key_source === "env" ? "the environment" : "Settings"}`
                      : "off — no key"}
                  </span>
                </p>
                <div className="stg-override">
                  <Input
                    mono
                    type="password"
                    placeholder="Gemini API key from Google AI Studio"
                    value={keyDraft}
                    onChange={(e) => setKeyDraft(e.target.value)}
                    aria-label="Gemini API key"
                    disabled={c.key_source === "env"}
                  />
                  <Button variant="primary" disabled={setKey.isPending || keyDraft.trim() === "" || c.key_source === "env"} onClick={() => void saveKey()}>
                    Save key
                  </Button>
                  {c.key_source === "settings" && (
                    <Button variant="danger" size="sm" disabled={setKey.isPending} onClick={() => void removeKey()}>Remove key</Button>
                  )}
                </div>
                {c.key_source === "env" && (
                  <p className="type-body stg-hint">The key comes from the CLUTCHFACTOR_GEMINI_KEY environment variable and can't be edited here.</p>
                )}
                <div className="stg-coach-row">
                  <Button variant="secondary" size="sm" disabled={!c.key_source || testKey.isPending} onClick={() => void test()}>
                    {testKey.isPending ? "Testing…" : "Test connection"}
                  </Button>
                  {c.key_source && (
                    <Button variant="secondary" size="sm" disabled={setEnabled.isPending} onClick={() => void setEnabled.mutateAsync(!c.enabled)}>
                      {c.enabled ? "Pause coach" : "Resume coach"}
                    </Button>
                  )}
                </div>
                <div className="stg-coach-models">
                  <Input mono aria-label="Per-round model" value={roundModel ?? c.round_model} onChange={(e) => setRoundModel(e.target.value)} />
                  <Input mono aria-label="Match synthesis model" value={synthModel ?? c.synthesis_model} onChange={(e) => setSynthModel(e.target.value)} />
                  <Button variant="secondary" size="sm" disabled={setModels.isPending || (roundModel === null && synthModel === null)} onClick={() => void saveModels()}>Save models</Button>
                </div>
                <p className="type-body stg-hint">
                  Per-round commentary and the match read are generated by Gemini from the facts ClutchFactor measured; every number, name and callout the coach cites is checked against those facts before it is shown. The key is stored in the local database on this computer and sent only to Google's API. A 24-round match is about five requests.
                </p>
              </>
            )}
          </Card>
        </div>
      )}
    </div>
  );
}
