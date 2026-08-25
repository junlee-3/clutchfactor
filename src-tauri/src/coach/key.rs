//! Where the key comes from (spec §3): the `CLUTCHFACTOR_GEMINI_KEY` env var
//! overrides the Settings value; in debug builds the repo-root `env.local`
//! (gitignored) seeds that env var when unset. The value is never logged.

use cf_store::Store;

use super::SecretKey;

pub const ENV_KEY: &str = "CLUTCHFACTOR_GEMINI_KEY";
pub const SETTING_KEY: &str = "gemini_api_key";
pub const SETTING_ENABLED: &str = "coach_enabled";
pub const SETTING_ROUND_MODEL: &str = "coach_round_model";
pub const SETTING_SYNTHESIS_MODEL: &str = "coach_synthesis_model";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    Env,
    Settings,
}

impl KeySource {
    pub fn as_str(self) -> &'static str {
        match self {
            KeySource::Env => "env",
            KeySource::Settings => "settings",
        }
    }
}

pub fn resolve_key(store: &Store) -> Result<Option<(SecretKey, KeySource)>, String> {
    if let Ok(v) = std::env::var(ENV_KEY) {
        if !v.trim().is_empty() {
            return Ok(Some((SecretKey::new(v), KeySource::Env)));
        }
    }
    match store.get_setting(SETTING_KEY).map_err(|e| e.to_string())? {
        Some(v) if !v.trim().is_empty() => Ok(Some((SecretKey::new(v), KeySource::Settings))),
        _ => Ok(None),
    }
}

pub fn coach_enabled(store: &Store) -> Result<bool, String> {
    Ok(store
        .get_setting(SETTING_ENABLED)
        .map_err(|e| e.to_string())?
        .as_deref()
        != Some("0"))
}

/// The value of a `GEMINI_API_KEY=…` line (optionally quoted), if present
/// and non-empty. Only that key is read — nothing else in the file matters.
pub fn parse_env_local(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim().trim_start_matches("export ").trim();
        let (k, v) = line.split_once('=')?;
        if k.trim() != "GEMINI_API_KEY" {
            return None;
        }
        let v = v.trim().trim_matches('"').trim_matches('\'').trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// Debug builds only: seed `CLUTCHFACTOR_GEMINI_KEY` from `<repo>/env.local`
/// when the env var is unset. Compiled out of release builds — a shipped
/// app never reads a developer file.
#[cfg(debug_assertions)]
pub fn load_dev_env_local() {
    if std::env::var(ENV_KEY)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return;
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../env.local");
    if let Ok(text) = std::fs::read_to_string(path) {
        if let Some(v) = parse_env_local(&text) {
            std::env::set_var(ENV_KEY, v);
            eprintln!("coach: dev key loaded from env.local");
        }
    }
}

#[cfg(not(debug_assertions))]
pub fn load_dev_env_local() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_local_reads_only_the_gemini_key_line() {
        let t = "# comment\nOTHER=1\nGEMINI_API_KEY=abc-def_123\nexport FOO=bar\n";
        assert_eq!(parse_env_local(t).as_deref(), Some("abc-def_123"));
        assert_eq!(
            parse_env_local("GEMINI_API_KEY=\"quoted-value\"\n").as_deref(),
            Some("quoted-value")
        );
        assert_eq!(parse_env_local("GEMINI_API_KEY=\n"), None);
        assert_eq!(parse_env_local("NOPE=1\n"), None);
    }

    #[test]
    fn secret_key_never_prints_its_value() {
        let k = crate::coach::SecretKey::new("TESTKEY-not-a-real-key-1234".to_string());
        assert_eq!(format!("{k:?}"), "SecretKey(…)");
        assert_eq!(k.hint(), "…1234");
        assert_eq!(k.expose(), "TESTKEY-not-a-real-key-1234");
    }
}
