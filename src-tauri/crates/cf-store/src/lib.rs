//! SQLite persistence (rusqlite, bundled), embedded versioned migrations (PROMPT.md §3).

pub mod migrations;
pub mod store;

pub use store::{MatchSummary, Store, StoreError};
