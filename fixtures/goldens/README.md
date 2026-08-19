# Golden snapshots — provenance & hand-validation

Golden JSONs here are committed; the source demos are gitignored (see `fixtures/README.md`).

| Golden | Source demo (gitignored) | Origin | Validated against |
|---|---|---|---|
| `navi-javelins-mirage.proof.json` | `fixtures/public/navi-javelins-vs-9-pandas-fearless-m1-mirage.dem` | demofile-net public test bundle (`demofile-net-demos-9.zip`, R2 bucket used by their MIT-licensed CI) | (1) demofile-net's committed, maintainer-verified snapshot `RoundStartEnd([v13963…]).txt`: **23 round_end events — 15 × winner side 2 (T), 8 × winner side 3 (CT)**; (2) HLTV/egamersworld: NAVI Javelins beat 9 Pandas Fearless 2:0 on 2023-10-19 (ESL Impact League S4 Europe), map 1 Mirage. |

Validation performed: (fill in when the golden is generated — winner sequence + count must match (1) exactly; kill feed spot-checks noted below).
