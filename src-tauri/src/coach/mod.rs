//! The coach's side effects (spec §3): key resolution, the Gemini call,
//! caching and fallback. The pure half — prompts, schemas, the grounding
//! validator, parsing — lives in `cf_narrator::coach` and is what the
//! adversarial tests exercise.

pub mod gemini;
pub mod key;

use std::fmt;

/// An API key that can be used but not printed. `Debug`/`Display` never
/// show the value; DTOs only ever carry `hint()`.
#[derive(Clone)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(s: String) -> Self {
        SecretKey(s.trim().to_string())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    /// "…ab12" — the last four characters, enough to tell keys apart.
    pub fn hint(&self) -> String {
        let tail: String = self
            .0
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("…{tail}")
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(…)")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoachError {
    NoKey,
    InvalidKey,
    RateLimited,
    Offline(String),
    Server(u16),
    BadRequest(String),
    BadResponse(String),
}

impl fmt::Display for CoachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoachError::NoKey => write!(f, "No Gemini key set — add one in Settings → Coach."),
            CoachError::InvalidKey => write!(f, "Gemini rejected the key. Check it in Settings → Coach."),
            CoachError::RateLimited => write!(f, "Gemini is rate-limiting this key right now — the coach will retry later; the template captions are shown meanwhile."),
            CoachError::Offline(e) => write!(f, "Couldn't reach Gemini ({e}). The template captions are shown meanwhile."),
            CoachError::Server(s) => write!(f, "Gemini returned a server error ({s}). Try again in a minute."),
            CoachError::BadRequest(m) => write!(f, "Gemini rejected the request: {m}"),
            CoachError::BadResponse(m) => write!(f, "The coach's answer couldn't be used: {m}"),
        }
    }
}

impl std::error::Error for CoachError {}
