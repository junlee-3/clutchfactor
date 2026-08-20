# ADR-0005: Main branch protection & merge policy

Status: accepted
Date: 2026-08-20

## Context

`main` had no protection: pushes landed directly and a red CI run was
discoverable only by looking. The charter's regression gates only gate if
something forces every change through them. Solo repo, so required reviews
are unavailable (you cannot approve your own PR).

## Decision

A repository ruleset on the default branch (rulesets over classic protection:
maintained API, shows which rule blocked):

- **PR required**, zero approvals (anything more deadlocks a solo repo);
  review threads must be resolved; stale reviews dismissed on push.
- **Required status checks — exactly the `ci.yml` job names `rust`,
  `windows-build`, `web`** — pinned to the GitHub Actions app so a
  third-party app can't post a same-named green check. `strict` on: PRs
  rebase onto current main before merging.
- **Linear history, no force-push, no deletion**; merge commits disabled
  (squash/rebase keep the conventional-commit trail).
- **Admin bypass, mode `always` — deliberate**: the owner needs an unblocked
  path mid-release. The rules are the default path, not a wall; bypass use
  should be called out in the commit message.

CI adjusted to suit: `push` filtered to main; read-only default token;
per-PR concurrency groups supersede stale PR runs, per-SHA groups on main so
no main run is ever cancelled or evicted — every main commit keeps its own
verdict. Dependabot is security-only.

## Consequences

- Day-to-day: branch → `gh pr create` → `gh pr merge --auto --squash` →
  verify the merge actually fired (auto-merge stalls silently on `BEHIND`).
- `strict` invalidates other open PRs' checks on each merge — cheap solo;
  relax it first if PR traffic grows.
- The three required check names are public contract: renaming a `ci.yml`
  job silently un-enforces it — rename job and ruleset in the same PR.
- Bypass means protection against accident, not against the owner.
