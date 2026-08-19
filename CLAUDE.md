# ClutchFactor

Desktop CS2 coaching app: parses match demo files (`.dem`) and produces coaching insights backed by a 2D replay viewer — not a stats tracker.

**`PROMPT.md` is the approved build spec and engineering charter. Read it before doing anything else.** Follow its §11 process rules (this file, `docs/PROGRESS.md`, ADRs, commit/push discipline) and §13 milestones.

Stack (decided — see PROMPT.md §3): Tauri 2 shell, Rust core (demoparser2 parsing, detectors, SQLite), React + TypeScript frontend.

This file must be expanded and kept current per PROMPT.md §11.1 (exact dev commands, architecture map, conventions) starting at milestone M0.

Current state: no code yet — start at PROMPT.md §14 First Actions.
