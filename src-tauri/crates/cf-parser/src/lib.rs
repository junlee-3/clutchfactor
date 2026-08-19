//! demoparser2 wrapper producing normalized match data.
//!
//! Boundary rule (PROMPT.md §4): types from this crate are the ONLY interface
//! downstream crates (cf-analysis, cf-store) see — no demoparser2 types leak out.

pub mod model;
pub mod proof;
pub mod rounds;
