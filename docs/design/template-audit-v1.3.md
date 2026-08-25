# Template actionability audit — V1.3

**Status: done (2026-08-26).** V1.3 DoD item "template actionability audit" (PROMPT-V1.md §4/§7). Guarded by `every_insight_template_carries_a_concrete_action` in `src-tauri/crates/cf-narrator/src/lib.rs`.

## The bar

PROMPT.md §8: templates are written "like a good coach talks: specific, actionable, no filler". The V1 charter's voice rules bind (numbers first, then the fix; no exclamation marks; never scold; "unusual, not wrong" for D6; no economy/buy coaching). For this audit *actionable* means the body names a concrete next action the player can rehearse in the next match — not a diagnosis alone, and not a place to look. The V1.3 coach uses these narrations as its fallback and as synthesis inputs, so every one must stand on its own.

## Method

- Enumerated every `insight.detector` arm of `templates::narrate` (15 arms + the unknown-detector fallback), every `narrate_habit` arm (6 + the generic arm), `summarize`, and the rail's `what_to_practise` lines (5 rules).
- One realistic sample `Insight` per arm, plus one per phrasing variant and coaching branch where an arm has more than one (`pick` hashes (detector, round, count); counts were chosen to land on each side). Values mirror the crate's existing exact tests: Mirage, 10-13 loss, 19 deaths, teammate "Riku". Sample bodies below are the rendered output, not paraphrases.
- Guard test (verbatim from the task brief): the lowercase body, split on non-alphanumerics, must contain one of `hold arrive throw wait swing trade rotate check peek stay flash smoke walk clear push step keep pair follow drop commit move save use count call let play take pull`, and must not contain `!`. Word-level matching means "re-peek" counts and "re-peeking" does not. The list was not extended.
- The guard iterates `narrate` only (as specified). Habits, the summary and the rail lines are hand-audited here with the same verb rule.
- Ratings: **yes** = names a move to rehearse · **weak** = the move is implied, stated as a standard, or is a review action · **no** = diagnosis only.
- RED on first run (before any wording change): `H3_VULNERABLE_DEATHS/b`. Hand audit additionally flagged `H2_BAITED_TRADE` (weak), the `H2_FAILED_TRADE` and `H3_WASTED_UTILITY` habits (weak), and a buy-coaching clause in `H6_DEAD_TIME_SMOKE`.

## `templates::narrate` — one row per arm (sub-rows per phrasing variant / branch)

| Detector (variant) | Sample title | Sample body (after) | Actionable? | Fix applied |
|---|---|---|---|---|
| `H2_ISOLATED_DEATH/a` | Died isolated 5 times | You died isolated 5 times with no teammate close enough to punish the kill — rounds 4, 7, 12, 15 and 1 more. Take those duels one angle closer to a teammate: arrive together, or hold until someone can trade you. | yes | none — "arrive together, or hold until someone can trade you" |
| `H2_ISOLATED_DEATH/b` | 9 deaths nobody could trade | Nobody was in range to trade you on 9 of your deaths — rounds 2, 6, 11, 14 and 5 more. Before you take the duel, know who re-peeks for you; if the answer is nobody, hold the angle and make them come to you. | yes | none — "hold the angle and make them come to you" |
| `H2_FAILED_TRADE/a` | 2 trades you were in range for | A teammate died inside trade range of you twice and you didn't take the re-peek — rounds 5 and 9. The two seconds after his death are the cheapest kill in the round: move on the sound, not after it. | yes | none — "move on the sound, not after it" |
| `H2_FAILED_TRADE/b` | Missed 3 trades in range | You were close enough to trade 3 teammate deaths and stayed on your angle — rounds 5, 9 and 14. Keep your crosshair where he is fighting so the trade is one step, not a repositioning job. | yes | none — "keep your crosshair where he is fighting" |
| `H2_FAILED_TRADE/team-pattern` | Missed 3 trades in range | You were close enough to trade 3 teammate deaths and stayed on your angle — rounds 5, 9 and 14. Keep your crosshair where he is fighting so the trade is one step, not a repositioning job. Baited trades are recurring too, so this is a team spacing problem: decide who the second man is before the round, not during it. | yes | none — team clause adds "decide who the second man is before the round" |
| `H2_BAITED_TRADE` | You traded in, nobody followed | You committed to the trade and the follow-up never came — 4 times, rounds 3, 8, 11 and 16. You were the only one who re-peeked; that is a team spacing problem, not a reason to stop trading: keep re-peeking and call the swing so the second man leaves with you. | weak → yes | body ended on "not a reason to stop trading" — what not to change, nothing to rehearse. Added "keep re-peeking and call the swing so the second man leaves with you" (mirrors the habit line; death-taxonomy §2 H2: never coach out of the trade) |
| `H2_BAITED_TRADE/named` | You traded in, nobody followed | You committed to the trade and the follow-up never came — twice, rounds 3 and 8. You were the only one who re-peeked — Riku was nearest and stayed put; that is a team spacing problem, not a reason to stop trading: keep re-peeking and call the swing so the second man leaves with you. | weak → yes | same fix; the named non-follower clause is unchanged |
| `H2_BAITED_TRADE/team-pattern` | You traded in, nobody followed | You committed to the trade and the follow-up never came — 4 times, rounds 3, 8, 11 and 16. You were the only one who re-peeked; that is a team spacing problem, not a reason to stop trading: keep re-peeking and call the swing so the second man leaves with you. Failed trades are recurring on your side too — the whole unit is arriving one man at a time. | weak → yes | same fix; the action now sits before the team clause, body stays at 3 sentences |
| `H3_VULNERABLE_DEATHS/a` | 6 of 19 deaths with no way to fight back | 6 of your 19 deaths (32%) came while you couldn't fight back — mid-throw, reloading or swapping weapons. Do that work behind cover: step off the angle first, then throw or reload. | yes | none — "step off the angle first, then throw or reload" |
| `H3_VULNERABLE_DEATHS/b` | Caught mid-animation in 7 deaths | You were mid-animation — throwing, reloading, swapping — for 7 of your 19 deaths (37%). The nade and the reload each cost you a second: step behind cover first and spend it where nobody has a line on you. | no → yes | guard RED: "spend it where nobody has a line on you" names a place, not a move. Now "step behind cover first and spend it where nobody has a line on you" |
| `H3_WASTED_UTILITY` | Died with unused utility 5 times | You died with grenades still unused in your inventory in 5 of your 19 deaths — most often a smoke. Utility you carry into your own death is utility you paid for and never used: throw it into the fight you are already in. | yes | none — "throw it into the fight you are already in" |
| `H4_KILLED_WITHOUT_CONTACT/a` | 4 deaths without a duel | You were killed through smoke twice and through a wall twice — 4 deaths where you never got to fight. Those are lines the enemy sprays for free: cross the gap wide, or hold from a spot they don't pre-fire first. | yes | none — "cross the gap wide, or hold from a spot they don't pre-fire first" |
| `H4_KILLED_WITHOUT_CONTACT/b` | Killed through smoke and walls 5 times | 5 of your deaths never became a duel — 3 through smoke and 2 through a wall. Change where you stand rather than how you aim: step off the common spray line before you hold it. | yes | none — "step off the common spray line before you hold it" |
| `H4_KILLED_WITHOUT_CONTACT/smoke-only` | 3 deaths without a duel | You were killed through smoke 3 times without ever getting to fight. Those are lines the enemy sprays for free: cross the gap wide, or hold from a spot they don't pre-fire first. | yes | none — single-medium phrasing, same coaching line as /a |
| `H4_CAUGHT_IN_CROSSFIRE` | Caught in crossfire 3 times | You were mid-duel with one enemy and killed by a second from another angle 3 times. Clear the off-angle before you commit, or take the fight from where only one of them has a line on you. | yes | none — "clear the off-angle before you commit" |
| `H16_UTILITY_EXPOSURE` | Enemy utility cost you 2 deaths | Enemy grenades killed you twice with no duel involved, and you took 87 damage standing in fire across 3 episodes. Move on the first tick of fire damage — the exit is always cheaper than standing in it. | yes | none — "move on the first tick of fire damage" |
| `H16_UTILITY_EXPOSURE/fire-only` | You keep standing in fire | You took 120 damage standing in fire across 4 episodes. Move on the first tick of fire damage — the exit is always cheaper than standing in it. | yes | none — same line, deaths clause dropped |
| `D2_FLASH_EFFECTIVENESS/self-flash` | 9 flashes, 4 blinded an enemy | You threw 9 flashes: 4 blinded an enemy, 3 caught a teammate and 2 led to a kill. Flash for the man entering, not for yourself: throw it over cover from behind him and let him move on the pop. | yes | none — "throw it over cover from behind him and let him move on the pop" |
| `D2_FLASH_EFFECTIVENESS/team-heavy` | 8 flashes, 2 blinded an enemy | You threw 8 flashes: 2 blinded an enemy, 5 caught a teammate and 1 led to a kill. More of them landed on your own team than on the enemy — line the flash up over cover and agree who entries before you throw. | yes | none — "line the flash up over cover and agree who entries before you throw" |
| `D2_FLASH_EFFECTIVENESS/good-rate` | 8 flashes, 7 blinded an enemy | You threw 8 flashes: 7 blinded an enemy and 4 led to a kill. That is a rate worth keeping — throw from behind the man entering and make sure someone moves on every pop. | yes | none — reinforces a good rate with the same rehearsable throw |
| `D2_FLASH_EFFECTIVENESS/plain` | 9 flashes, 3 blinded an enemy | You threw 9 flashes: 3 blinded an enemy and 1 led to a kill. Throw from behind the man entering and over cover, so the flash pops where he is already looking. | yes | none — "throw from behind the man entering and over cover" |
| `H6_UTIL_TEAM_DAMAGE` | Your utility hurt teammates 3 times | Your grenades did 96 damage to your own team across 3 throws, most of it on Riku. Call the nade before it leaves your hand and wait for the lane to clear — that HP comes straight out of the next duel. | yes | none — "call the nade before it leaves your hand and wait for the lane to clear" |
| `H6_UNUSED_UTIL_AT_ROUND_END` | Ended 5 rounds holding utility | You finished 5 rounds alive with 2 or more grenades unthrown. Utility has no value once the round ends: spend the smoke on the timing you already committed to, or the flash on the last angle you take. | yes | none — "spend the smoke on the timing you already committed to, or the flash on the last angle you take" |
| `H6_DEAD_TIME_SMOKE` | 3 smokes thrown after the round | 3 of your smokes went out after the round had already ended. That is utility you paid for and never used — throw it while the round is still live, on the crossing or the retake you are about to make. | yes (voice fix) | "or keep the money for a rifle" was buy coaching (PROMPT-V1: no economy/buy coaching in v1). Now "throw it while the round is still live, on the crossing or the retake you are about to make". Note: no match-level insight is emitted for this rule today (flag only; the rail narrates it) — the arm is kept for when one is |
| `D4_ENTRY_PROFILE` | 6 entries, 2 won | You took first contact on 6 of your team's 14 entries and won 2 of them. 4 of those went in unsupported and 3 went untraded. Don't take the first duel until the flash or the second man is with you — an entry alone is a coin flip you are paying for. | yes | none — "don't take the first duel until the flash or the second man is with you" |
| `D4_ENTRY_PROFILE/clean` | 5 entries, 4 won | You took first contact on 5 of your team's 12 entries and won 4 of them. Keep the flash and the second man attached to every one of them — an entry alone is a coin flip you are paying for. | yes | none — reinforces: "keep the flash and the second man attached to every one of them" |
| `D5_TIMING/all` | Early deaths and slow rotations | You died on early aggression in 4 rounds, rotated late 3 times and pushed without info twice. Take space after first contact tells you where they aren't — and rotate on the call, not after the site falls. | yes | none — "take space after first contact tells you where they aren't — and rotate on the call" |
| `D5_TIMING/early-only` | Dying early in the round | You died on early aggression in 3 rounds. Take space after first contact tells you where they aren't, not before. | yes | none — "take space after first contact tells you where they aren't, not before" |
| `D5_TIMING/slow-only` | Rotating late | You rotated late 3 times. Rotate on the call, not after the site falls. | yes | none — "rotate on the call, not after the site falls" |
| `D6_UNUSUAL_POSITIONING` | Unusual T-side positioning — 3 rounds | Reference players rarely hold the spot you took at mid-round on T — 3 rounds this match. This measures unusual, not wrong: check the heatmap for where they set up instead. | yes | none — "check the heatmap for where they set up instead" is the right-sized action for a measure of unusualness (D6 honesty rule: unusual, not wrong) |
| `_` (unknown detector) | Off angle habit | Flagged 3 times this match. | n/a | none — a readable stub for a detector that ships before its template ("never empty, never a lie"), not coaching. Unreachable today: every detector cf-analysis emits (`h2.rs`, `flash_util.rs` aggregates, D2/D4/D5/D6, H3/H4/H16) has an arm. Excluded from the guard with a comment |

Not sampled: the `D5_TIMING` branch with no early/slow/blind counts ("Let the round tell you where the space is before you take it, and rotate on the call rather than after it") — unreachable under the D5 gate (early + slow ≥ 2) and an all-zero insight is not real data; it carries `let`/`take`/`rotate` regardless.

## `narrate_habit` — one row per arm

| Rule | Sample title | Sample body (after) | Actionable? | Fix applied |
|---|---|---|---|---|
| `H2_ISOLATED_DEATH` | Habit: isolated deaths | You died isolated in 4 of your last 5 matches — 17 times in all, most often at Catwalk (5) and Underpass (3). This is the first habit to fix: pick fights a teammate can re-peek within two seconds. | yes | none — "pick fights a teammate can re-peek within two seconds" |
| `H2_FAILED_TRADE` | Habit: missed trades | You left trades on the table in 3 of your last 5 matches — 9 times in all. Standing near a teammate is not support; re-peeking within two seconds of his death is — move on the sound, not after it. | weak → yes | "re-peeking within two seconds of his death is" stated the standard, not the move. Appended "— move on the sound, not after it" |
| `H2_BAITED_TRADE` | Habit: nobody follows your trade | You were the only one who committed to the trade in 3 of your last 5 matches — 9 times in all. Failed trades are recurring on your side in the same window, so this is a team spacing problem, not a habit to unlearn: keep re-peeking, and fix the timing so the second man leaves with you. | yes | none — "keep re-peeking, and fix the timing so the second man leaves with you" |
| `H3_WASTED_UTILITY` | Habit: dying with unused utility | You died with grenades still unused in your inventory in 3 of your last 10 matches — 22 times in all. Make it a rule: throw the nades before the fight starts, not once you are in it. | weak → yes | "nades leave your hand before the fight starts" was declarative. Now "throw the nades before the fight starts, not once you are in it" |
| `H4_KILLED_WITHOUT_CONTACT` | Habit: killed without a duel | Smoke and wallbang deaths caught you in 3 of your last 5 matches — 8 times in all, most often at Mid (4). You keep holding lines that get sprayed blind — take one step off the common spot before you set up. | yes | none — "take one step off the common spot before you set up" |
| `H4_REPEAT_HOTSPOT` | Repeat hotspot: A site on Mirage | You have died 5 times at A site on Mirage across 2 matches. They know that angle better than you do — hold it from a different position, or stop taking that fight. | yes | none — "hold it from a different position, or stop taking that fight" |
| `_` generic (sampled as H11_SLOW_ROTATION) | Habit: slow rotation | Slow rotation recurred in 3 of your last 5 matches — 8 times in all. A mistake that repeats across matches is a habit: watch the clips together and find what they share. | weak (by design) | generic arm for any rule id without a hand-written habit; "watch the clips together and find what they share" is a review action, not an in-game one, and the narrator cannot name a move for a rule it does not know. Reachable: `get_habits` promotes every flagged rule id. No wording change (anything added would be filler) — see follow-ups |

## `summarize`

| Path | Sample title | Sample body | Actionable? | Fix applied |
|---|---|---|---|---|
| match summary | Mirage, 10-13 loss | You lost 10-13 on Mirage and died 19 times. 32% of your deaths were fair duels you lost on mechanics — the rest had a fixable cause. Deaths are the biggest group at 3 of the 5 insights, so start there. | n/a (report header) | none — result, class-13 share and where to start; the rehearsable actions live in the insights it points at. "Start there" is the summary's whole job |

## Rail `what_to_practise` (`src-tauri/crates/cf-narrator/src/rail.rs`)

| Rule | Sample line | Actionable? | Fix applied |
|---|---|---|---|
| `H2_ISOLATED_DEATH` | Before you take a fight at Catwalk, know who is close enough to trade you. | yes | none |
| `H2_FAILED_TRADE` | You were in trade range when Takenouchi died — move on the sound, not after it. | yes | none |
| `H14_UNSUPPORTED_ENTRY (opponent named)` | You took that entry on UncleBubbles alone — get the flash or the second man on you before you commit. | yes | none |
| `H14_UNSUPPORTED_ENTRY (no opponent)` | Don't take that entry alone — get the flash or the second man on you before you commit. | yes | none |
| `H6_PUSH_WITHOUT_INFO` | You pushed 900 u from spawn with no read on the site — wait for a call before committing. | yes | none |
| `H11_EARLY_AGGRESSIVE_DEATH` | You died 8 s into the round, 750 u from spawn with nobody close enough to trade — take a slower entry or bring a teammate with you. | yes | none |

`rail.rs` needed no wording change and is untouched by this audit.

## Wording changes (before → after)

All in `src-tauri/crates/cf-narrator/src/templates.rs`; wording only — no logic, facts keys or rule ids changed. Three exact-string tests in `lib.rs` updated to match.

1. `H2_BAITED_TRADE` (all variants): `…not a reason to stop trading.` → `…not a reason to stop trading: keep re-peeking and call the swing so the second man leaves with you.`
2. `H3_VULNERABLE_DEATHS` variant b: `…cost you a second: spend it where nobody has a line on you.` → `…cost you a second: step behind cover first and spend it where nobody has a line on you.`
3. `H6_DEAD_TIME_SMOKE`: `…throw it while the round is still live, or keep the money for a rifle.` → `…throw it while the round is still live, on the crossing or the retake you are about to make.`
4. Habit `H2_FAILED_TRADE`: `…re-peeking within two seconds of his death is.` → `…re-peeking within two seconds of his death is — move on the sound, not after it.`
5. Habit `H3_WASTED_UTILITY`: `Make it a rule: nades leave your hand before the fight starts, not once you are in it.` → `Make it a rule: throw the nades before the fight starts, not once you are in it.`
6. `templates.rs` header: added the house rule "every body names a concrete action the player can rehearse" pointing at the guard test.

The coach's `STYLE_VERSION` is untouched: templates reach the coach only as `SynthesisInput.insights`, and the cache hash covers the prompt text.

## Follow-ups (out of wording-only scope)

- Hand-written habit lines for the other promotable rule ids that currently fall to the generic habit arm (`H5_DIED_FLASHED`, `H6_FLASH_SELF_OR_TEAM`, `H6_PUSH_WITHOUT_INFO`, `H11_SLOW_ROTATION`, `H11_EARLY_AGGRESSIVE_DEATH`, `H14_UNSUPPORTED_ENTRY`, `H16_FIRE_LINGER`) — new match arms, so a separate change.
- `H6_DEAD_TIME_SMOKE` has a template arm but no match-level insight emitter in `cf-analysis` (flag only); decide whether to aggregate it or drop the arm.
- The guard covers `narrate`; extending the same verb rule to `narrate_habit` and `what_to_practise` would be a small follow-on test.
