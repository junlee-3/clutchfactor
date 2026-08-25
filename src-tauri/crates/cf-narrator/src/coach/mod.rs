//! The coach (docs/spec/play-ledger-and-coach.md §3): everything the model
//! sees and everything we check, as pure functions. No network here — the
//! app crate (`src-tauri/src/coach/`) owns the key, the HTTP call, the
//! cache and the fallback. Facts are grounded (`validate`), judgment is free.

pub mod parse;
pub mod prompt;
pub mod types;
pub mod validate;
