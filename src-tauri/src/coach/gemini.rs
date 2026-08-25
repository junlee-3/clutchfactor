//! The one place that talks to Gemini (REST `generateContent`, verified
//! 2026-08-25). Key in the `x-goog-api-key` header, never in the URL.

use std::time::Duration;

use serde_json::{json, Value};

use super::{CoachError, SecretKey};

const ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const TIMEOUT: Duration = Duration::from_secs(45);
const TEMPERATURE: f64 = 0.4;

#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    pub text: String,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
}

pub struct GeminiClient {
    http: reqwest::Client,
    key: SecretKey,
}

impl GeminiClient {
    pub fn new(key: SecretKey) -> Result<Self, CoachError> {
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|e| CoachError::Offline(e.to_string()))?;
        Ok(GeminiClient { http, key })
    }

    pub async fn generate_json(
        &self,
        model: &str,
        system: &str,
        user: &str,
        schema: &Value,
    ) -> Result<Generated, CoachError> {
        let url = format!("{ENDPOINT}/{model}:generateContent");
        let resp = self
            .http
            .post(&url)
            .header("x-goog-api-key", self.key.expose())
            .header("content-type", "application/json")
            .json(&build_body(system, user, schema))
            .send()
            .await
            .map_err(|e| CoachError::Offline(short(&e.to_string())))?;
        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| CoachError::Offline(short(&e.to_string())))?;
        if !(200..300).contains(&status) {
            return Err(map_failure(status, &body));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|e| CoachError::BadResponse(format!("not JSON: {e}")))?;
        extract_text(&value)
    }
}

pub fn build_body(system: &str, user: &str, schema: &Value) -> Value {
    json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": [{ "role": "user", "parts": [{ "text": user }] }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": schema,
            "temperature": TEMPERATURE
        }
    })
}

/// Google's error bodies are `{"error": {"message": …, "status": …}}`.
fn error_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Trim an error to one short line — and never let a key-looking token
/// through (defence in depth: Google's messages don't echo keys, but a
/// proxy or a future SDK might). Redacts any whitespace-delimited token of
/// 20+ alphanumeric/`-_.` chars: real Gemini keys run ~39 chars, well clear
/// of this bound.
fn short(s: &str) -> String {
    const REDACT_MIN_LEN: usize = 20;
    let one: String = s.lines().next().unwrap_or("").chars().take(120).collect();
    one.split_whitespace()
        .map(|w| {
            if w.len() >= REDACT_MIN_LEN
                && w.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            {
                "[redacted]"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn map_failure(status: u16, body: &str) -> CoachError {
    let msg = error_message(body);
    match status {
        401 | 403 => CoachError::InvalidKey,
        400 if msg.to_lowercase().contains("api key") => CoachError::InvalidKey,
        429 => CoachError::RateLimited,
        500..=599 => CoachError::Server(status),
        _ => CoachError::BadRequest(short(if msg.is_empty() { body } else { &msg })),
    }
}

pub fn extract_text(body: &Value) -> Result<Generated, CoachError> {
    let cand = body["candidates"]
        .get(0)
        .ok_or_else(|| CoachError::BadResponse("no candidates".into()))?;
    let text = cand["content"]["parts"]
        .get(0)
        .and_then(|p| p["text"].as_str())
        .ok_or_else(|| {
            let reason = cand["finishReason"].as_str().unwrap_or("unknown");
            CoachError::BadResponse(format!("no text (finishReason {reason})"))
        })?;
    let usage = &body["usageMetadata"];
    Ok(Generated {
        text: text.to_string(),
        prompt_tokens: usage["promptTokenCount"].as_u64().unwrap_or(0) as u32,
        output_tokens: usage["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coach::{CoachError, SecretKey};
    use serde_json::json;

    #[test]
    fn body_has_system_instruction_user_content_and_json_config() {
        let b = build_body("persona", "facts", &json!({"type": "object"}));
        assert_eq!(b["systemInstruction"]["parts"][0]["text"], "persona");
        assert_eq!(b["contents"][0]["role"], "user");
        assert_eq!(b["contents"][0]["parts"][0]["text"], "facts");
        assert_eq!(
            b["generationConfig"]["responseMimeType"],
            "application/json"
        );
        assert_eq!(b["generationConfig"]["responseSchema"]["type"], "object");
        assert_eq!(b["generationConfig"]["temperature"], 0.4);
    }

    #[test]
    fn failures_map_to_the_right_error_and_never_echo_a_key() {
        let key = "TESTKEY-not-a-real-key-9999";
        assert!(matches!(map_failure(401, ""), CoachError::InvalidKey));
        assert!(matches!(map_failure(403, ""), CoachError::InvalidKey));
        assert!(matches!(
            map_failure(
                400,
                "{\"error\":{\"message\":\"API key not valid. Please pass a valid API key.\"}}"
            ),
            CoachError::InvalidKey
        ));
        assert!(matches!(map_failure(429, ""), CoachError::RateLimited));
        assert!(matches!(map_failure(503, ""), CoachError::Server(503)));
        let e = map_failure(
            400,
            &format!("{{\"error\":{{\"message\":\"bad schema for {key}\"}}}}"),
        );
        assert!(matches!(e, CoachError::BadRequest(_)));
        for e in [
            CoachError::NoKey,
            CoachError::InvalidKey,
            CoachError::RateLimited,
            CoachError::Offline("dns".into()),
            CoachError::Server(500),
            map_failure(400, &format!("x {key} y")),
            CoachError::BadResponse("blocked".into()),
        ] {
            assert!(!e.to_string().contains(key), "{e}");
            assert!(!e.to_string().contains("TESTKEY"), "{e}");
        }
        let _ = SecretKey::new(key.to_string());
    }

    #[test]
    fn extract_text_reads_the_first_candidate_and_usage() {
        let body = json!({"candidates":[{"content":{"parts":[{"text":"{\"rounds\":[]}"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":120,"candidatesTokenCount":30,"totalTokenCount":150}});
        let g = extract_text(&body).unwrap();
        assert_eq!(g.text, "{\"rounds\":[]}");
        assert_eq!((g.prompt_tokens, g.output_tokens), (120, 30));
        let blocked = json!({"candidates":[{"finishReason":"SAFETY"}]});
        assert!(matches!(
            extract_text(&blocked),
            Err(CoachError::BadResponse(_))
        ));
        assert!(matches!(
            extract_text(&json!({})),
            Err(CoachError::BadResponse(_))
        ));
    }
}
