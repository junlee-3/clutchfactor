//! Steam profile lookup for the tracked player: persona name + avatar.
//!
//! Source is the community profile's XML view
//! (`https://steamcommunity.com/profiles/<id64>?xml=1`), which needs no API
//! key — unlike `ISteamUser/GetPlayerSummaries`, which would make every user
//! register one just to see their own face. We read exactly two fields
//! (`steamID`, `avatarFull`) and ignore the rest of the document.
//!
//! The avatar is inlined as a `data:` URI rather than left as a URL: the
//! sidebar then renders offline and the webview never reaches the network on
//! render (issue #39's last acceptance criterion). Both fields are cached in
//! the settings table under `steam_profile:<id64>` with a day's TTL.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use cf_store::Store;

const TIMEOUT: Duration = Duration::from_secs(8);

/// Re-check once a day — a rename or a new avatar is not urgent.
const TTL_SECS: u64 = 24 * 60 * 60;

/// A profile whose avatar download failed is only held briefly: a blip
/// mid-fetch must not pin the initials placeholder for a whole day.
const PARTIAL_TTL_SECS: u64 = 15 * 60;

/// Steam avatars are 184x184 JPEGs (~10 KB). A megabyte is already absurd;
/// the cap exists so a redirected or hostile response can't bloat the DB.
const AVATAR_MAX_BYTES: usize = 1024 * 1024;

/// Persona names cap at 32 characters in Steam's own UI. Clamp anyway — the
/// XML is remote data and this string goes straight into the sidebar.
const NAME_MAX_CHARS: usize = 64;

/// The avatar URL comes out of a remote document, so it is not followed
/// blindly: only Valve's own CDN hosts are fetched.
const AVATAR_HOST_SUFFIX: &str = ".steamstatic.com";

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    /// Steam persona name (`<steamID>`), not the in-game name.
    pub persona: Option<String>,
    /// `data:image/jpeg;base64,...`
    pub avatar: Option<String>,
    /// Unix seconds; 0 for a profile that never came from a live fetch.
    #[serde(default)]
    pub fetched_at: u64,
    /// The document named an avatar we failed to download — worth retrying
    /// long before the normal TTL is up.
    #[serde(default)]
    pub partial: bool,
}

impl Profile {
    pub fn is_fresh(&self) -> bool {
        let ttl = if self.partial {
            PARTIAL_TTL_SECS
        } else {
            TTL_SECS
        };
        now_secs().saturating_sub(self.fetched_at) < ttl
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn cache_key(steamid: &str) -> String {
    format!("steam_profile:{steamid}")
}

/// A cache miss and a corrupt cache row are the same thing to callers: no
/// usable profile, go and fetch one.
pub fn read_cache(store: &Store, steamid: &str) -> Option<Profile> {
    let raw = store.get_setting(&cache_key(steamid)).ok()??;
    serde_json::from_str(&raw).ok()
}

pub fn write_cache(store: &mut Store, steamid: &str, profile: &Profile) {
    if let Ok(json) = serde_json::to_string(profile) {
        let _ = store.set_setting(&cache_key(steamid), &json);
    }
}

/// A SteamID64 is 17 digits. Checked because it is interpolated into a URL.
fn valid_steamid(steamid: &str) -> bool {
    steamid.len() == 17 && steamid.bytes().all(|b| b.is_ascii_digit())
}

/// Extracts `<tag>value</tag>`, unwrapping the CDATA section Steam wraps its
/// text fields in. Deliberately not a full XML parser: two known fields out
/// of a known document beats a dependency.
fn field(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let rest = &xml[start..];
    let raw = rest[..rest.find(&close)?].trim();
    // CDATA content is literal by definition — only the plain-text branch
    // carries entities, and unescaping both would rewrite a persona that
    // genuinely contains the characters "&amp;".
    let value = match raw
        .strip_prefix("<![CDATA[")
        .and_then(|s| s.strip_suffix("]]>"))
    {
        Some(cdata) => cdata.trim().to_string(),
        None => unescape(raw),
    };
    if value.is_empty() {
        return None;
    }
    Some(value)
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        // Steam serves the XML view to unidentified clients inconsistently.
        .user_agent(concat!("ClutchFactor/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

/// Fetches persona + avatar. Errors are the caller's cue to fall back to what
/// it already has (stale cache, demo name) — never to show nothing.
pub async fn fetch(steamid: &str) -> Result<Profile, String> {
    if !valid_steamid(steamid) {
        return Err(format!("not a SteamID64: {steamid}"));
    }
    let client = http()?;
    let url = format!("https://steamcommunity.com/profiles/{steamid}?xml=1");
    let xml = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // A deleted or nonexistent profile answers with an <error> document.
    if field(&xml, "steamID64").is_none() {
        return Err("no such Steam profile".into());
    }

    let persona = field(&xml, "steamID").map(|mut n| {
        if n.chars().count() > NAME_MAX_CHARS {
            n = n.chars().take(NAME_MAX_CHARS).collect();
        }
        n
    });
    // A private profile still publishes its persona and avatar; if it ever
    // doesn't, `avatar` is simply None and the initials placeholder shows.
    let listed = field(&xml, "avatarFull");
    let avatar = match &listed {
        Some(u) => fetch_avatar(&client, u).await,
        None => None,
    };

    Ok(Profile {
        persona,
        // Promised an avatar and didn't get one: keep the persona, but let
        // this row expire quickly (see PARTIAL_TTL_SECS).
        partial: listed.is_some() && avatar.is_none(),
        avatar,
        fetched_at: now_secs(),
    })
}

/// Downloads the avatar and inlines it. A failure here is not fatal: the name
/// alone is still a better sidebar than a raw SteamID64.
async fn fetch_avatar(client: &reqwest::Client, url: &str) -> Option<String> {
    let host = url.strip_prefix("https://")?.split('/').next()?;
    if !host.ends_with(AVATAR_HOST_SUFFIX) {
        return None;
    }
    let mut res = client.get(url).send().await.ok()?.error_for_status().ok()?;
    let mime = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_ascii_lowercase())
        .filter(|m| {
            matches!(
                m.as_str(),
                "image/jpeg" | "image/png" | "image/gif" | "image/webp"
            )
        })?;
    // Enforced while streaming: `bytes()` would allocate the whole body
    // first, which is exactly what this cap exists to prevent.
    if res
        .content_length()
        .is_some_and(|n| n > AVATAR_MAX_BYTES as u64)
    {
        return None;
    }
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = res.chunk().await.ok()? {
        if body.len() + chunk.len() > AVATAR_MAX_BYTES {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    if body.is_empty() {
        return None;
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&body);
    Some(format!("data:{mime};base64,{b64}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from a real response — see the module docs for the live URL.
    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><profile>
        <steamID64>76561199228328773</steamID64>
        <steamID><![CDATA[misosoupy3]]></steamID>
        <privacyState>public</privacyState>
        <avatarFull><![CDATA[https://avatars.fastly.steamstatic.com/9e1c_full.jpg]]></avatarFull>
        <customURL><![CDATA[]]></customURL>
        </profile>"#;

    #[test]
    fn reads_persona_and_avatar() {
        assert_eq!(field(SAMPLE, "steamID").as_deref(), Some("misosoupy3"));
        assert_eq!(
            field(SAMPLE, "avatarFull").as_deref(),
            Some("https://avatars.fastly.steamstatic.com/9e1c_full.jpg")
        );
    }

    /// `<steamID>` must not be satisfied by the `<steamID64>` that precedes it.
    #[test]
    fn does_not_confuse_steamid_with_steamid64() {
        assert_eq!(
            field(SAMPLE, "steamID64").as_deref(),
            Some("76561199228328773")
        );
        assert_ne!(
            field(SAMPLE, "steamID").as_deref(),
            Some("76561199228328773")
        );
    }

    #[test]
    fn empty_and_missing_fields_are_none() {
        assert_eq!(field(SAMPLE, "customURL"), None);
        assert_eq!(field(SAMPLE, "realname"), None);
    }

    #[test]
    fn unescapes_non_cdata_text() {
        assert_eq!(
            field("<steamID>a &amp; b</steamID>", "steamID").as_deref(),
            Some("a & b")
        );
    }

    #[test]
    fn rejects_ids_that_are_not_steamid64() {
        assert!(valid_steamid("76561199228328773"));
        assert!(!valid_steamid("765611992283287"));
        assert!(!valid_steamid("../../etc/passwd"));
        assert!(!valid_steamid("7656119922832877x"));
    }

    /// Hits the network, so it stays out of `cargo test` and out of CI. Run
    /// it by hand after touching the parsing above:
    /// `cargo test --manifest-path src-tauri/Cargo.toml -p clutchfactor
    ///  -- --ignored live_profile`
    #[ignore = "hits steamcommunity.com"]
    #[tokio::test]
    async fn live_profile_still_matches_what_we_parse() {
        // The tracked player from CLAUDE.md — a public profile.
        let p = fetch("76561199228328773").await.expect("fetch");
        assert!(p.persona.is_some(), "no persona in the live document");
        let avatar = p.avatar.expect("no avatar in the live document");
        assert!(avatar.starts_with("data:image/"), "not an image data URI");
        assert!(avatar.len() > 1000, "avatar suspiciously small");
    }

    #[test]
    fn freshness_follows_the_ttl() {
        let fresh = Profile {
            fetched_at: now_secs(),
            ..Default::default()
        };
        assert!(fresh.is_fresh());
        let stale = Profile {
            fetched_at: now_secs() - TTL_SECS - 1,
            ..Default::default()
        };
        assert!(!stale.is_fresh());
        // A cache row written before this field existed must not read as fresh.
        assert!(!Profile::default().is_fresh());
    }

    /// A blip that cost us the avatar must not pin the placeholder for a day.
    #[test]
    fn a_partial_profile_expires_far_sooner() {
        let half = Profile {
            partial: true,
            fetched_at: now_secs() - PARTIAL_TTL_SECS - 1,
            ..Default::default()
        };
        assert!(!half.is_fresh());
        let whole = Profile {
            fetched_at: now_secs() - PARTIAL_TTL_SECS - 1,
            ..Default::default()
        };
        assert!(whole.is_fresh(), "a complete profile still holds for a day");
    }

    #[test]
    fn cdata_text_is_not_entity_unescaped() {
        // A persona that genuinely contains "&amp;" must survive verbatim.
        assert_eq!(
            field("<steamID><![CDATA[Tom &amp; Jerry]]></steamID>", "steamID").as_deref(),
            Some("Tom &amp; Jerry")
        );
    }
}
