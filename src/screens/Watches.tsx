import { useMemo } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { useDetectorCatalog } from "../lib/queries";
import { STAT_KEYS, STAT_TITLES, type StatKey } from "../lib/statFormat";
import { Card } from "../components/ui/Card";
import { Chip } from "../components/ui/Chip";
import { DataTable } from "../components/ui/DataTable";
import { Segmented } from "../components/ui/Segmented";
import { Skeleton } from "../components/ui/Skeleton";

/** The honesty screen (PROMPT-V1 §6 "What your coach watches"): every rule
 *  in plain language with its live thresholds, the taxonomy with what is
 *  and isn't built, and what the engine cannot see. `?stat=` narrows to the
 *  rules behind one number — the target of every stat link in the app. */
export function Watches() {
  const [params, setParams] = useSearchParams();
  const stat = (STAT_KEYS as readonly string[]).includes(params.get("stat") ?? "")
    ? (params.get("stat") as StatKey)
    : null;
  const cat = useDetectorCatalog();
  const entries = useMemo(
    () => (cat.data?.entries ?? []).filter((e) => !stat || e.stat_links.includes(stat)),
    [cat.data, stat],
  );
  const families = useMemo(() => [...new Set(entries.map((e) => e.family))], [entries]);

  if (cat.isLoading) {
    return (
      <div className="wat-shell">
        <h1 className="type-display">What your coach watches</h1>
        <div role="status" aria-label="Loading the detector catalog">
          <Skeleton kind="card" count={4} />
        </div>
      </div>
    );
  }
  if (cat.isError || !cat.data) {
    return (
      <div className="wat-shell">
        <h1 className="type-display">What your coach watches</h1>
        <p className="type-body">Couldn't load the catalog — {String(cat.error)}</p>
      </div>
    );
  }

  return (
    <div className="wat-shell">
      <div className="wat-head">
        <h1 className="type-display">What your coach watches</h1>
        <p className="type-body wat-intro">
          Every rule the engine runs, in plain language, with the thresholds it uses right now.
          Approximations stay silent rather than guess.
        </p>
        <Segmented
          value={stat ?? "all"}
          onChange={(v) => setParams(v === "all" ? {} : { stat: v })}
          options={[
            { value: "all", label: "All" },
            ...STAT_KEYS.map((k) => ({ value: k, label: STAT_TITLES[k] })),
          ]}
          ariaLabel="Filter by stat"
        />
        {stat && (
          <p className="type-body wat-filter">
            Rules behind <strong>{STAT_TITLES[stat]}</strong> · <Link to="/watches">show all</Link>
          </p>
        )}
      </div>
      {families.map((fam) => (
        <Card key={fam}>
          <h2 className="type-micro wat-family">{fam}</h2>
          <ul className="wat-list">
            {entries
              .filter((e) => e.family === fam)
              .map((e) => (
                <li key={e.id} id={e.id} className="wat-entry">
                  <div className="wat-entry-head">
                    <h3 className="type-heading">{e.title}</h3>
                    <code className="type-micro wat-id">{e.id}</code>
                    {e.class_id !== null && (
                      <Chip className="wat-class">class {e.class_id}</Chip>
                    )}
                  </div>
                  <p className="type-body">{e.watches_for}</p>
                  <p className="type-body wat-thresholds">
                    <span className="type-micro">Counts when</span> {e.thresholds}
                  </p>
                  <p className="type-body wat-example">
                    <span className="type-micro">Reads as</span> “{e.example}”
                  </p>
                  {e.stat_links.length > 0 && (
                    <p className="type-micro wat-links">
                      Feeds {e.stat_links.map((k) => STAT_TITLES[k as StatKey] ?? k).join(", ")}
                    </p>
                  )}
                </li>
              ))}
          </ul>
        </Card>
      ))}
      {entries.length === 0 && (
        <Card eyebrow="Nothing to show">
          <p className="type-body wat-empty">
            No rule feeds this number directly — it is counted straight from the events.{" "}
            <Link to="/watches">See every rule</Link>.
          </p>
        </Card>
      )}
      {!stat && (
        <>
          <Card>
            <h2 className="type-micro wat-family">Death classes</h2>
            <DataTable
              head={["#", "class", "source", "built"]}
              rows={cat.data.classes.map((c) => [
                String(c.id),
                c.name,
                c.source,
                c.built ? "yes" : (
                  <span className="wat-notbuilt" title={c.why_not ?? ""}>
                    not yet — {c.why_not}
                  </span>
                ),
              ])}
              rowKey={(i) => String(cat.data!.classes[i].id)}
            />
          </Card>
          <Card>
            <h2 className="type-micro wat-family">What the engine cannot see</h2>
            <ul className="wat-cannot">
              {cat.data.cannot_see.map(([t, s]) => (
                <li key={t}>
                  <strong>{t}.</strong> {s}
                </li>
              ))}
            </ul>
          </Card>
        </>
      )}
    </div>
  );
}
