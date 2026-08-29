# Death Taxonomy & Rule Families — spec addendum

> Owner-supplied spec (2026-08-19), adopted as part of the product spec alongside `PROMPT.md`.
> PROMPT.md §5's detectors D1–D6 remain the milestone deliverables; this document defines the
> *shape* the death/duel analysis takes (15-class taxonomy + H-family rules) and adds a core
> requirement: **cross-demo habit tracking**. Where this doc and PROMPT.md §5 overlap, this doc
> is the more detailed and governing one for death classification.

## Integration notes (engineering, 2026-08-19)

- **Mapping to PROMPT.md detectors:** D1 (untraded/isolated deaths) ⇒ H2 family + classes 6/7/9.
  D2 (flash effectiveness) ⇒ H5/H6 (`H6_FLASH_SELF_OR_TEAM`, `H5_NO_FLASH_REACTION`) + class 3.
  D3 (utility usage/waste) ⇒ H3/H16/H6. D4 (trade discipline/entry) ⇒ H2 + H6_DRY_ENTRY + H14.
  D5 (timing/rotation) ⇒ H1/H11. D6 (positional baseline) unchanged, complemented by H8.
- **Milestone impact:** M1 schema must include `death_class` + rule-flag storage (see Storage below)
  and cross-demo keys. M3 = taxonomy MVP: classes reachable without geometry (1,2,3,4,5,6,7,9,13,14,15
  via H2/H3/H16/H4-Tier-1/H5-subset) + rule engine with rules-as-data. M4 adds pattern promotion
  (cross-demo habits) to the Match Report + narrator. M6 Trends renders class shares & habit trends.
- **Evidence contract holds:** every classified death and every fired rule is an `Insight` with
  `EvidenceRef` — the taxonomy adds `class_id`/`rule_id`/`confidence`/`secondary_tags` to the payload,
  it does not replace the contract.
- **Preventability grouping (UI-facing):** classes 1–12 = preventable (each with a specific fix),
  13 = fair loss (mechanics — explicitly good news), 14 = hygiene (not a coaching moment),
  15 = unclassified (must stay near-empty). The Match Report groups the death pie this way.
- **Cross-demo habit engine (owner requirement, core not optional):** mistakes recur across games;
  the product must (a) classify each death per demo, (b) give per-death feedback, and (c) promote
  *patterns* across demos — "bad habit X, seen in N of your last M matches, here's the fix" —
  with replayable evidence from multiple demos. H4_REPEAT_HOTSPOT and H8 are natively cross-demo;
  class shares and per-rule rates trend across matches. Promotion thresholds live in DetectorConfig
  like all other tunables. Severity ranking becomes severity × confidence × recurrence-across-demos.
- **§5 parser facts below** were verified by the owner against demoparser2/awpy on real demos.
  Per PROMPT.md ground rule 3 we still re-verify each fact against our pinned rev when we first
  build on it (they inform design now; golden tests enforce them at build time).

---

## 1. Death taxonomy — 15 classes

Every death of the tracked player gets **exactly one** primary class, assigned by priority order, plus secondary tags recording every other rule that also fired.

The premise: *"how you die"* is more actionable than any composite rating, and it costs nothing beyond rules you already need.

| # | Class | Source rule |
|---|---|---|
| 1 | Caught in utility animation | H3 |
| 2 | Caught in grenade/incendiary damage (no duel) | H16 |
| 3 | Blinded / flashed out | H5 |
| 4 | Caught reloading or unscoped | H3 |
| 5 | No-engagement death (wallbang / shot through smoke, never saw attacker) | H4 |
| 6 | Isolated & untradeable | H2 |
| 7 | Baited / unsupported trade attempt | H2 |
| 8 | Over-peek in man disadvantage | H1 |
| 9 | Crossfire death (killed by second enemy mid-duel) | H4 |
| 10 | Lost angle-advantage duel (wide peek) | H4 |
| 11 | Pushed without info | H6 |
| 12 | Off-angle / repeat-hotspot death | H8 |
| 13 | **Outaimed in fair duel** | fallback — *explicitly good to see* |
| 14 | Fall damage / self-inflicted | event-derived, no rule |
| 15 | Unclassified | fallback |

### Why the order is the spec

The priority order encodes *what actually caused the death*. Four principles:

1. **Cause of damage outranks context.** If a molotov killed you (2), your spacing (6) is a footnote.
2. **"You never had a duel" outranks "you lost the duel."** Classes 2 and 5 both mean no engagement happened. They must sit above every duel-loss class, or a wallbang gets filed as "outaimed."
3. **Intent-correct errors rank below intent-wrong ones.** Class 7 (baited trade) sits *below* 6 (isolated) because contesting a trade is the right instinct executed without support. Severity must reflect that — see H2 below.
4. **14 exists purely for hygiene.** Fall damage and own-utility deaths aren't coaching moments. Without an explicit class they pollute 13 and understate the player's real fair-duel share.

### Class 13 is the load-bearing one

"Outaimed in fair duel" is deliberately framed as good news. A coach who calls *every* death a mistake gets ignored; showing *"31 % of your deaths were fair duels you lost on mechanics"* is what makes the other 69 % credible.

But it is a **fallthrough, not a judgement** — it means classes 1–12 all failed to match. So it is only as trustworthy as those classes are complete, and **every class you haven't built yet inflates it.**

Practical consequences:

- **Track class-13 share as a CI metric.** A jump after a rule change means you broke a classifier above it. It's the most likely regression this design has.
- **If you ship with classes missing, say so in the UI.** Otherwise the app confidently tells the user most of their deaths were fair losses.
- **Class 15 should be near-empty** on real demos. If it isn't, the taxonomy has a hole — a design bug, not a tuning problem.
- **Class 14 must not be zero** across a corpus. Fall/self deaths are rare but real; zero means you aren't detecting them and they're hiding in 13.

### Storage

One row per death in a derived table — not a flag (flags are per-rule; a death has exactly one class):

```
death_class : demo_id, round_num, tick, victim_steamid,
              class_id u8,            -- 1-15
              class_source TEXT,      -- claiming rule id, or 'fallthrough'
              secondary_tags TEXT[],  -- every other rule that fired on this death
              confidence f32          -- inherited from the claiming rule
```

`secondary_tags` makes the pie chart drillable without re-running rules: a death classed `6 (isolated)` that also tripped `H3_WASTED_UTILITY` keeps both facts.

---

## 2. Required rule families

> **Rule ids are load-bearing** — flags persist `rule_id`, patterns key on it, golden tests reference it by name. Never renumber a family to tidy the list. `H16` is numbered after H15 because it was added later; it is **required**, not optional.

### H1 — Man-Count Discipline

The man count tells you which mistake you're about to make. Up bodies, the only way to lose is to give the advantage back. Down bodies, the only way to win is to buy time and isolate duels — and players do the opposite, taking the hero peek that ends the round.

| Rule | Definition |
|---|---|
| `H1_GREEDY_PEEK` | `man_advantage >= 2`, `advantage_age_s > 3.0`, player initiated contact |
| `H1_CROSSFIRE_ABANDON` | in advantage, `nearest_teammate_dist` grew 800 → >1400 within 5 s |
| `H1_SOLO_INFO_PEEK` | in advantage, `nearest_teammate_dist > 1500` at first enemy spot |
| `H1_ADVANTAGE_TIME_WASTE` | T-side, `man_advantage >= 2`, no plant, `time_remaining_s < 35`, team not moving to site |
| `H1_LAST_ALIVE_GREED` | died last alive with a man advantage still on the board |
| `H1_DESPERATION_PEEK` | `man_advantage <= -1`, player initiated, **and the clock did not demand it** → **class 8** |

**Headline metric — Advantage Conversion Rate:** `rounds won | team held a man advantage`, with the personal split *"you died first in the advantage in N of them"*.

**Suppression matters more on the disadvantage rule than the advantage one.** Being down bodies legitimately forces aggression far more often, so a rule that flags every disadvantage death is wrong most of the time and destroys trust. Suppress `H1_DESPERATION_PEEK` when:

- CT-side, bomb planted, `time_remaining_s < 12` — you have to take the duel
- T-side, no plant, `time_remaining_s < 20` — same
- You were last alive (that's a clutch, judged by H10)
- `traded_within_s <= 2.0` — the peek worked as intended
- The enemy was already spotted *and* pushing you — you were defending, not initiating

Target case after gating: 3v4, 40 seconds left, nothing forcing contact, walked into the open looking for the equaliser.

### H2 — Trade Spacing & Line-of-Sight

Most ranked deaths aren't lost duels, they're duels taken where nobody could punish the winner.

At each death tick:
```
nearest_teammate_dist = min over alive teammates of ‖p_self − p_mate‖
teammate_los          = ∃ alive teammate with LOS to killer position
traded_within_s       = (t_killer_death − t_death) if killer dies, else NULL
```

| Rule | Definition |
|---|---|
| `H2_ISOLATED_DEATH` | `dist > 800` AND `teammate_los == false` AND not traded within 2.0 s → **class 6** |
| `H2_FAILED_TRADE` | you were within 800 u of a teammate's death and did **not** commit within 2.0 s — catches passive players |
| `H2_BAITED_TRADE` | teammate died within the trade window; you *did* commit; you died committing; **no other teammate was in trade range of you** → **class 7** |
| `H2_STACKED` | `nearest_teammate_dist < 150` for >4 s in the open — two players in one nade |
| `H2_NO_CONTACT` | `time_since_teammate_contact_s > 12` at death |

**`H2_BAITED_TRADE` is the one rule here that is not primarily the player's fault, and the engine must say so.** It's the exact complement of `H2_FAILED_TRADE` — same three-player situation, seen from the player who did the right thing.

- **Cap its severity well below `H2_ISOLATED_DEATH`.** Weighted equally, the player learns to stop trading, which is strictly worse play.
- **Evidence must name the teammate who didn't follow** (id, distance, callout at your death). Without that the caption reads as blame; with it, it reads as *"you were third man in a two-man fight."*
- **Never promote it to a habit alone.** If it and `H2_FAILED_TRADE` both fire often, the pattern is a *team* spacing problem — the caption should say that, not coach the user to change their own behaviour.

Two refinements worth building: separate *trade availability* from *trade execution* (only charge the user for the former), and weight vertical distance more heavily — 800 u apart with a 200 u z-difference is not tradeable spacing.

### H3 — Utility Vulnerability

The cheapest deaths in the game. Pure habit, trivially fixable, poorly tracked elsewhere. This family is *your* utility making *you* vulnerable.

| Rule | Definition |
|---|---|
| `H3_DIED_WITH_NADE_OUT` | `active_weapon ∈ grenades` at death → **class 1** |
| `H3_DIED_MID_SWITCH` | `t_death − t_last_weapon_switch < 0.3 s` → **class 1** |
| `H3_DIED_RELOADING` | `RELOADING` set at death → **class 4** |
| `H3_DIED_SCOPED_CLOSE` | `SCOPED` set AND `nearest_enemy_dist < 600` → **class 4** |
| `H3_DIED_DEFUSING_NO_COVER` | `DEFUSING` set AND no teammate with LOS to site entrance |
| `H3_NADE_OUT_IN_OPEN` | grenade equipped AND `in_open` AND `speed > 200` for >1.5 s — **precursor rule, fires without a death** |
| `H3_WASTED_UTILITY` | died with `util_mask != 0` — *"you died holding 2 flashes and a smoke in 11 of 24 rounds"* |

**Headline metric — Vulnerable Death %:** share of deaths where you couldn't shoot back. Ranked players sit at 15–25 %; good players under 10 %.

### H4 — Peeking Geometry & Exposure

Whoever is further from the corner sees the other first. Built in three tiers of geometric fidelity.

**Tier 1 (MVP) — spotted-flag differential, no geometry:**
```
spot_delta = t_first_spotted_by_any_enemy − t_first_spotted_any_enemy
```
- `spot_delta < −0.15 s` → you saw them first and still lost → mechanical problem
- `spot_delta > +0.15 s` → they saw you first → positioning problem

That split alone resolves the most common misdiagnosis in ranked ("I need better aim" when the real problem is exposure). Rules: `H4_PEEKED_INTO_PREAIM`, `H4_LOST_DUEL_SEEN_FIRST`, `H4_WON_INFO_LOST_DUEL`.

⚠️ **This is any-enemy, not this-enemy — see §5.1.** Gate Tier-1 rules on duel cleanliness (one enemy plausibly in contact); route multi-enemy fights to `H4_CAUGHT_IN_CROSSFIRE` instead, and never write "they saw you first" in a caption when you can't name who "they" is.

Two taxonomy classes source here, both cheap and MVP-available:

| Rule | Definition |
|---|---|
| `H4_KILLED_WITHOUT_CONTACT` | `thru_smoke` OR `penetrated > 0` OR (never spotted killer in prior 2 s AND fired 0 shots at them) → **class 5** |
| `H4_CAUGHT_IN_CROSSFIRE` | engaged with enemy A within prior 2.0 s, killer is enemy B ≠ A, angle A→you→B > 45° → **class 9** |

`H4_KILLED_WITHOUT_CONTACT` needs a careful caption: it is not "you were outplayed", it's "you stood in a line someone pre-fires for free". Cross-reference H8 — a *repeat* smoke/wallbang death at the same spot is the strongest version and the only one worth calling a habit.

**Tier 2 — kinematic:** `H4_WIDE_PEEK_HELD_ANGLE` (exposure distance > 120 u before firing), `H4_PEEK_WHILE_MOVING` (`speed > 60` at first shot), `H4_CROSSHAIR_PLACEMENT` (median angular delta to enemy head at first mutual LOS).
**Tier 3 (Phase 4) — true angle advantage:** requires raycasting against map collision geometry.
**Also:** `H4_REPEAT_HOTSPOT` — DBSCAN over death positions, fires at ≥3 deaths within 250 u across ≥2 demos. *"You've died at Mirage Palace 5 times this week"* — needs no geometry at all.

### H5 — Audio-Cued Misplay

The information was there and free; the player didn't act on it.

Audibility model (MVP): `audible = ‖p_src − p_lis‖ < radius(sound_type)`. Running footsteps ~1100 u, walking 0 (silent by design), flash pin-pull ~1200, grenade bounce ~1500, reload ~800, defuse ~1000, plant ~1500.

Occlusion is ignored in MVP, which over-reports audibility → over-reports "you ignored a cue". **Compensate by capping confidence at 0.6 for the whole family** and requiring a clear reaction failure, not merely a non-reaction.

| Rule | Definition |
|---|---|
| `H5_NO_FLASH_REACTION` | enemy pin-pull audible, LOS toward throw origin, `\|Δyaw\| < 20°` in following 0.5 s, ends up `flash_alpha > 180` → **class 3** |
| `H5_WALKED_INTO_HEARD_ENEMY` | enemy footsteps audible within 1100 u in last 1.5 s, continued toward them at `speed > 200` without pre-aim, died |
| `H5_MISSED_JUMPTHROW_CUE` | enemy jump-throw audible, remained in detonation radius |
| `H5_FIRE_DISCIPLINE` | took >20 molotov damage having been in fire >1.0 s |
| `H5_OWN_NOISE` | ran (audible) into a held angle with an enemy within 1100 u — **you gave them the cue** |

`H5_OWN_NOISE` is the reciprocal nobody builds, and it's frequently the actual cause of *"why do they always know where I am."*

### H16 — Utility Damage Exposure *(required — sources class 2)*

Dying to a grenade you never contested is the purest avoidable death in the game. There was no duel to lose.

**Distinct from H3, deliberately.** H3 is your utility making you vulnerable; H16 is their utility killing you outright. Opposite direction, different fix — a player told "stop dying to utility" needs to know which one they're doing.

| Rule | Definition |
|---|---|
| `H16_DIED_TO_UTILITY_NO_DUEL` | killer weapon ∈ {hegrenade, inferno, molotov} AND 0 shots fired in prior 3 s AND no enemy spotted in prior 2 s → **class 2** |
| `H16_FIRE_LINGER` | >20 fire damage after >1.0 s in fire (same event as `H5_FIRE_DISCIPLINE` — keep that id, this is its taxonomy-facing alias) |
| `H16_NADE_STACK` | took HE/molly damage within 150 u of a teammate hit by the same grenade — damage-side twin of `H2_STACKED` |
| `H16_PREDICTABLE_UTIL_SPOT` | ≥40 utility damage within 250 u of ≥2 prior utility-damage events this session |

**Volume warning:** measured on a real 130-kill match, deaths where the killing weapon was utility: **1 in 130 (0.8 %)**. So class 2 will almost never clear pattern promotion on its own — expect it as a taxonomy slice, not a habit callout. Its false-positive rate is also unmeasurable on small samples; ship it silent-biased until the corpus exists.

`H16_PREDICTABLE_UTIL_SPOT` is why the family earns its place: *damage* events are far more common than utility *deaths*, so "you keep eating the same molotov" has real volume even when the death class doesn't.

---

## 3. Additional families (referenced by the taxonomy)

- **H6 — Information & Utility Economy** ★★★★★ — `H6_PUSH_WITHOUT_INFO` (**class 11**), `H6_DRY_ENTRY`, `H6_UNUSED_UTIL_AT_ROUND_END`, `H6_FLASH_SELF_OR_TEAM`
- **H8 — Positional Repetition & Predictability** ★★★★☆ — **class 12**; entropy of your position at freeze-end across a half. *"You held CT Mirage from Ticket 9 rounds out of 12"* — invisible to you, obvious to the enemy
- **H7** Economy discipline · **H9** Post-plant & retake · **H10** Clutch behaviour · **H11** Rotation timing · **H12** Reload & ammo · **H13** Movement quality · **H14** Opening duel profile · **H15** Bomb discipline

---

## 4. Design principles behind all of it

1. **Bias every approximation toward silence.** A false negative is a missed lesson; a false positive is the user losing trust. Where geometry is approximated, arrange it so uncertainty *suppresses* a flag.
2. **Every rule emits `confidence`.** Rules on approximate LOS or the audio model cap around 0.6–0.7. Rank by `severity × confidence`.
3. **Rules are data, not code** — declarative thresholds + a predicate over columns, loaded from YAML at startup. Tunable, serialisable, testable, vectorisable.
4. **Thresholds in seconds and world units, never ticks.**
5. **Every rule needs a golden test** — one hand-verified clip per rule, asserted in CI. Threshold tuning breaks other rules silently otherwise.
6. **Precision over recall, always.** Hand-review 30 flags per rule per release; anything above ~10 % false-positive gets tightened or downweighted before shipping.

---

## 5. Parser facts verified against real demos

These were tested directly against demoparser2/awpy on real match demos, not assumed from docs. They contradict things a reasonable person would design around.

### 5.1 `spotted_by_mask` does not exist

The natural design is a bitmask of *which* enemies can see you. **It isn't available.** Every plausible name was tried — `spotted_by_mask`, `spotted_by`, `spotter_mask`, `is_spotted_by_mask`, `entity_spotted_mask` — and all are silently dropped (no error, just missing from the output).

The only visibility field is **`spotted`, a plain boolean**: *am I seen by at least one enemy.*

Consequence: `spot_delta` is *any-enemy*, not *this-enemy*. Exact in a clean 1v1; degrades in multi-enemy fights, where "an enemy saw you first" may be a different player from the one who killed you. Gate it, cap confidence, or route to a crossfire classification. True per-opponent visibility needs raycasting.

### 5.2 Filter `is_alive` before touching any per-tick field

Naive whole-column null rates look alarming and are entirely misleading:

| Field | Null while **alive** | Null while **dead** |
|---|---|---|
| `spotted` | **0 %** | 0–92 % (varies per player) |
| `weapon_name` | **0 %** | 100 % |
| `active_weapon` | **0 %** | — |

All three are fully reliable for every tick a player is alive — which is the only window any death/duel rule cares about. The nulls are dead-spectator ticks. Whole-column rates (`spotted` 0–9 %, `weapon_name` 23–36 %) mean nothing.

### 5.3 `weapon_switch` is not a first-class event — derive it

Diff `active_weapon` per player between consecutive ticks, once, at cache-write time. Verified across 16 demos: yields **2,404–10,118 switches per demo** — believable, not zero, not noise. `active_weapon` is never null while alive, so detection is solid. H3's 0.3 s rule depends on this.

### 5.4 Signal volumes measured on a real 130-kill demo

| Signal | Field | Rate | Consequence |
|---|---|---|---|
| Shot through smoke | `thru_smoke` | 15/130 (11.5 %) | class 5 has real volume — build it |
| Wallbang | `penetrated` | 5/130 (3.8 %) | class 5 confirmed, mostly smoke not walls |
| Killed by utility | `weapon ∈ {hegrenade, inferno}` | 1/130 (0.8 %) | class 2 is rare — taxonomy slice, not a habit |
| Attacker was blind | `attacker_blind` | 0/130 | sparse — don't build anything load-bearing on it |

Also note `weapon` carries non-player killers such as `planted_c4`, and `attacker_id` may be null, self, or a teammate — all of which feed class 14 and must be classified out explicitly.

### 5.5 Nav mesh yes, callouts no

`awpy get navs` / `awpy get maps` give pre-parsed nav mesh JSON + radar images per map, no VPK work needed. Verified by rendering overlays for dust2/mirage/inferno/nuke/ancient/anubis/train — all correct. `de_cache` is **not** in the current bundle.

**Callout names are a different story.** CS2 moved them out of the nav file into `env_cs_place` entities inside the compiled map (VMap/DMX in VPKs). `awpy` has no code path for this at all. Extracting them needs the actual CS2 game files (Windows/Linux only — not extractable on macOS).

Not a rule-engine blocker: callout *names* are only used for human-readable evidence text ("died at Palace"), never as rule-math input — that runs on nav area ids and centroid distance. It blocks readable captions, not detection.

> Engineering note on §5.5: demoparser2 also exposes `last_place_name` as a per-tick player prop
> (PROMPT.md §6.2) which may cover readable location captions without any VPK extraction —
> verify which source is better for captions at M2/M3.

---

## 5A. Implementation addenda (M3, 2026-08-19 — additions only, no renames)

- New rule ids added per §2's "additions allowed" rule: `H5_DIED_FLASHED` (sources class 3
  until the audio-model H5 rules land), `H6_DEAD_TIME_SMOKE`, `H6_UNUSED_UTIL_AT_ROUND_END`
  (round-end holding; H3_WASTED_UTILITY covers died-holding), `H6_UTIL_TEAM_DAMAGE`,
  `H14_DIED_SELF_OR_WORLD` (class-14 source, event-derived).
- M3 ships classes 1–7, 9, 13–15; classes 8/10/11/12 are reserved (H1, H4-Tier-2/3,
  H6-info, H8) and labeled "[not built]" in tooling output per §1's honesty rule.
- Parser facts learned: the death-tick inventory sample is always empty (items drop on
  death) — inventories are sampled ~0.25 s pre-death, and empty samples are treated as the
  death artifact (a living player always holds a knife).
- **V1.6 (2026-08-29, additions only):** `H1_DESPERATION_PEEK` (class 8) and
  `H4_WIDE_PEEK_HELD_ANGLE` (class 10) ship as **kinematic** rules — no geometry, no
  `spotted` flag (§5.1). H1's "player initiated" is: over a 2 s window the tracked player
  closed ≥ 150 u on the killer, walked ≥ 150 u, and covered at least as much ground as the
  killer did (a killer who moved more was the one pushing → silent, which is this table's
  "enemy already pushing you" suppression). Its clock test reads the bomb timer once the
  bomb is down and the round clock otherwise, and stays silent when the derived time has
  already run out. H4 Tier 2's "whoever is further from the corner sees the other first"
  becomes: the victim covered ≥ 120 u toward a killer who covered ≤ 60 u, still ≥ 150 u
  apart at the death, with a shot or damage from the victim. Both emit `confidence` 0.6
  (§4.2's cap on approximations). Class 12 is now the only reserved class.

## 6. Cross-demo habit tracking (owner requirement, verbatim intent)

The coaching must be cross-demo, not single-demo. If a mistake repeats across games, the app tells
the user what bad habit they repeat and how to fix it. Pipeline: parse demo → classify each death
(preventable / unpreventable etc. via the taxonomy) → per-mistake feedback in that demo → and a
cross-demo analysis layer that tracks bad habits/mistakes over multiple games, with evidence links
into each contributing match.
