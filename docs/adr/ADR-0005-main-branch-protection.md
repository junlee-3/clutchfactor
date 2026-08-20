# ADR-0005: Main branch protection & merge policy

Status: accepted
Date: 2026-08-20

## Context

`main` had no protection: any push landed directly, and CI was advisory — a red
`ci` run on main was discoverable only by looking. That is survivable for a solo
repo up to v0, but the charter treats a few things as load-bearing regression
gates (class-13 share, golden clip tests, `-D warnings`, the Windows build per
§10.3), and a gate nobody is forced through stops being a gate.

The repo is public and owner-operated by one person, so the usual lever —
required reviews — is unavailable: you cannot approve your own PR.

## Decision

A **repository ruleset** on `~DEFAULT_BRANCH` (not classic branch protection —
rulesets are the maintained API and show which rule blocked a push):

- **Pull request required**, with `required_approving_review_count: 0`. The PR is
  the unit that carries checks; approval count is the part that would deadlock a
  solo repo. Review-thread resolution is required, and stale reviews are
  dismissed on push, so the rule tightens for free when a second contributor
  arrives — flip the count to 1.
- **Required status checks**: `rust`, `windows-build`, `web`, pinned to the
  GitHub Actions app (`integration_id: 15368`) so a third-party app cannot post a
  same-named green check. `strict` is on: a PR must be rebased onto current main
  before it merges, so checks describe the merged result rather than a stale base.
- **Linear history**, **no force-push**, **no deletion**. Merge commits are
  disabled repo-wide, leaving squash and rebase — both preserve the conventional
  commit trail the charter relies on.
- **Bypass: repository admin (`actor_id: 5`), mode `always`.** Deliberate. The
  owner ships unsigned desktop builds from tags and needs an unblocked path when
  a release is mid-flight. The rules stay the default path, not a wall.

CI was adjusted to suit: `push` is filtered to `main` so feature commits stop
producing a second set of runs under the same check names that protection matches
on, plus a read-only default token and a concurrency group that supersedes stale
PR runs. Cancellation is deliberately **off for `main`** — every commit there has
to keep its own verdict, and a cancelled run is not a pass: it would drop the
clippy `-D warnings` gate, the class-13 share metric and the Windows build while
looking like the job merely went away.

Dependabot is **security-only** (alerts + automated fixes), no version-update
schedule. A weekly bump PR against a pinned demoparser2 git dep and a Tauri
toolchain is noise this repo cannot absorb yet.

## Consequences

- Every change to main is a PR. Day-to-day cost is `gh pr create` plus
  `gh pr merge --auto --squash` — auto-merge is enabled, so the merge fires when
  the three checks go green rather than requiring a second visit.
- `strict` means a merge to main invalidates other open PRs' checks until they
  rebase. With one contributor and rarely-concurrent PRs this is cheap; if PR
  traffic ever makes it painful, dropping `strict` is the first thing to relax.
- Required check names are now part of the public contract. Renaming a job in
  `ci.yml` silently un-enforces it — the ruleset keeps waiting on a context that
  no longer reports. Rename the job and the ruleset in the same PR, or move to a
  single aggregate `ci-ok` gate job if the job list starts changing often.
- Admin bypass means this is protection against accident, not against the owner.
  Recorded here so the choice is visible rather than assumed.
