# V1.5 error / empty / loading audit

Task 4 (`feat/v1.5-audit`): three honest states on every screen, a
route-level `ErrorBoundary`, and the dev-only `VITE_FAIL_IPC` switch that
provokes each error branch without touching the Rust side. Per the
controller's ruling for this task, the app was not run — verification here
is code review only; the in-app check (Step 6 of the task brief, screenshots
under `docs/design/walkthrough-v1.5/audit/`) is deferred to the controller.

How to provoke each error state, once in-app verification happens:
`VITE_FAIL_IPC=list_matches|get_trends|corpus_status|get_app_settings|get_detector_catalog|get_match_report|get_match_detail|get_round_ticks pnpm tauri dev`.

| Screen | Loading | Empty | Error | How verified |
|---|---|---|---|---|
| Library | `Skeleton kind="rows" count={6}` (`.library-row-skeleton`, 90px rows — final row height) | `EmptyState` "No matches yet" → Import demos | `EmptyState` "Couldn't load your matches", body `errorMessage(matches.error)`, Retry → `matches.refetch()`. Provoke: `VITE_FAIL_IPC=list_matches` | code review; in-app check pending (controller) |
| Report | `Skeleton kind="block"/"card"` header+lead+cards at final size (`.report-header-skeleton` 56px, `.report-lead-skeleton` 88px) | `EmptyState` "Match not found" → Back to library (deleted/missing id, e.g. `/report/999`); nested "Nothing recurring to coach" / "No habits yet" (no action — informational) | `EmptyState` "Couldn't load this report", body `errorMessage(report.error)`, Retry → `report.refetch()`, checked **before** the `!report.data` "Match not found" branch. Provoke: `VITE_FAIL_IPC=get_match_report` | code review; in-app check pending (controller) |
| Trends | `Skeleton kind="rows"/"block"` ribbon+stat+line+rules at final size | `EmptyState` "Not enough matches yet" → Go to library | `EmptyState` "Couldn't load Trends", body `errorMessage(trends.error)`, Retry → `trends.refetch()`, checked before the "not enough matches" branch (so a fetch error is never mistaken for a thin library). Provoke: `VITE_FAIL_IPC=get_trends` | code review; in-app check pending (controller) |
| Corpus | `Skeleton kind="block"` (`.cps-loading-block`, 420px — final viewer height) | `EmptyState` "No reference demos yet" → Add pro demos | `EmptyState` "Couldn't load the corpus", body `errorMessage(status.error)`, Retry → `status.refetch()`, checked before the "no reference demos" branch. Provoke: `VITE_FAIL_IPC=corpus_status` | code review; in-app check pending (controller) |
| Settings | `Skeleton kind="card" count={3}` (`.stg-card-skeleton`, 140px — final card height) | n/a (Settings always has content once loaded — a fresh install still shows the cards with placeholder copy) | `EmptyState` "Couldn't load Settings", body `errorMessage(settings.error)`, Retry → `settings.refetch()` — previously this screen rendered only the bare `<h1>` on a failed fetch. Provoke: `VITE_FAIL_IPC=get_app_settings` | code review; in-app check pending (controller) |
| Replay | `Skeleton kind="block"/"rows"` header/round-strip/well/side/coach-rail at final size (`.rpl-well-skeleton` min-height 420px) | `EmptyState` "Match not found" (deleted/missing id, e.g. `/replay/999`) or "No radar calibration yet" → Back to library | Two independent branches, both title "Couldn't load this replay": (1) `detail.isError` — whole-screen `EmptyState`, Retry → `detail.refetch()`, checked before `!d`/`!mapCal`. Provoke: `VITE_FAIL_IPC=get_match_detail`. (2) `ticks.isError` — the per-round tick fetch, rendered in the `.rpl-round-error` slot (same footprint as the loading skeleton) so a round switch failure doesn't collapse the header/round-strip above it; Retry → `ticks.refetch()`. Provoke: `VITE_FAIL_IPC=get_round_ticks`. The pre-existing `reviewsError` path (coach rail) is unchanged — it hides the rail rather than blocking the tape, which is intentional (the round is still watchable without the coach's read). | code review; in-app check pending (controller) |
| Watches | `Skeleton kind="card" count={4}` | n/a (the catalog is static reference content — "cannot be empty" per the brief) | `EmptyState` "Couldn't load the catalog", body `errorMessage(cat.error)`, Retry → `cat.refetch()` — this screen already had an error branch (`cat.isError \|\| !cat.data`); it's now aligned to the same `EmptyState` + Retry pattern as every other screen instead of a bare paragraph with `String(cat.error)`. Provoke: `VITE_FAIL_IPC=get_detector_catalog` | code review; in-app check pending (controller) |

## Route-level ErrorBoundary

`src/components/ui/ErrorBoundary.tsx` (class component) catches a render
exception in any routed screen and shows `EmptyState` "Something in this
screen broke", body `<error message> — reload the screen, or go back to
the Library.` (the "go back to the Library" segment is a `Link to="/"`,
per the controller's ruling that `EmptyState.body` accepts a `ReactNode` —
this closes the brief's "second action" note without adding a second
button), and a "Reload the screen" action that clears the caught error.
`console.error("screen crashed", error, componentStack)` on catch — the
boundary never swallows the error.

`App.tsx`'s `RouteBoundary` wraps every routed screen element (`Library`,
`Trends`, `Watches`, `Settings`, `Corpus`, `Report`, `Replay`, and the
catch-all `NotFound`) with `<ErrorBoundary resetKey={pathname}>`, keyed on
`useLocation().pathname` — navigating away from a crashed screen always
resets the boundary, so a stale crash never survives a route change.

Verified: code review only (component logic, `resetKey` reset behavior on
`componentDidUpdate`, and the `App.tsx` wiring were read end to end); no
render exception was provoked in a running instance for this task per the
owner-activity guard. In-app check pending (controller).

## Skeletons — final-size check

Every screen's loading skeleton was already backed by a dedicated CSS rule
sized to the real content's footprint before this task (heights such as
`.library-row-skeleton` 90px, `.report-header-skeleton` 56px,
`.rpl-well-skeleton` min-height 420px, `.cps-loading-block` 420px,
`.stg-card-skeleton` 140px, `.trends-*-skeleton` per-section heights). None
were a bare "Loading…" sentence. **No skeleton was changed in this task** —
the one new loading-adjacent CSS addition is `.rpl-round-error`, which
gives the Replay round-ticks *error* state the same footprint as its
loading skeleton (`flex: 1; min-height: 420px`) so a round-switch failure
doesn't shift the header/round-strip above it; it is not itself a skeleton.

## VITE_FAIL_IPC

`src/lib/ipc.ts`'s `call()` checks
`import.meta.env.DEV && import.meta.env.VITE_FAIL_IPC === cmd` before doing
anything else and throws `Error("forced failure: <cmd>")` — `invoke` is
never called for the forced command. Guarded by `import.meta.env.DEV`, so
it is inert (and the env var read compiles away) in release builds.
`src/vite-env.d.ts` declares `VITE_FAIL_IPC?: string` on `ImportMetaEnv` via
declaration merging with `vite/client`'s own interface. Test:
`src/lib/ipc.test.ts` — "forces a rejection for the command named by
VITE_FAIL_IPC, in dev, without calling invoke" (`vi.stubEnv` /
`vi.unstubAllEnvs`).
