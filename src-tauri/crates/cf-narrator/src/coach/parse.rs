//! Turning the model's text into typed answers. Lenient about a markdown
//! code fence (models add them), strict about shape.

use std::fmt;

use serde::Deserialize;

use super::types::{MatchSynthesis, RoundCommentary};

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    NotJson(String),
    Shape(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NotJson(s) => write!(f, "the coach did not answer in JSON ({s})"),
            ParseError::Shape(s) => write!(f, "the coach's JSON had the wrong shape ({s})"),
        }
    }
}

impl std::error::Error for ParseError {}

/// At most 80 chars of model text, one line, no control characters — the
/// only form of the model's output that may ever reach an error message.
fn bounded(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(80)
        .collect();
    if s.chars().count() > 80 {
        out.push('…');
    }
    out
}

pub fn strip_code_fence(text: &str) -> &str {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim()
}

#[derive(Deserialize)]
struct Batch {
    rounds: Vec<RoundCommentary>,
}

fn parse_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, ParseError> {
    let body = strip_code_fence(text);
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| ParseError::NotJson(bounded(&format!("{e}; text starts: {body}"))))?;
    serde_json::from_value(value).map_err(|e| ParseError::Shape(bounded(&e.to_string())))
}

pub fn parse_round_batch(text: &str) -> Result<Vec<RoundCommentary>, ParseError> {
    let batch: Batch = parse_json(text)?;
    Ok(batch.rounds)
}

pub fn parse_synthesis(text: &str) -> Result<MatchSynthesis, ParseError> {
    parse_json(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_batch_and_tolerates_a_code_fence_and_missing_optionals() {
        let text = "```json\n{\"rounds\":[{\"round\":6,\"read\":\"ok\",\"plays\":[{\"tick\":1,\"comment\":\"c\"}]},{\"round\":7,\"read\":\"r\",\"plays\":[],\"focus\":null}]}\n```";
        let rounds = parse_round_batch(text).unwrap();
        assert_eq!(rounds.len(), 2);
        assert_eq!(rounds[0].plays[0].tick, 1);
        assert_eq!(rounds[1].focus, None);
        assert_eq!(rounds[1].why_it_mattered, None);
    }

    #[test]
    fn non_json_and_wrong_shape_are_errors_that_do_not_echo_the_whole_text() {
        let e = parse_round_batch("Sure! Here is my analysis of the round...").unwrap_err();
        assert!(matches!(e, ParseError::NotJson(_)));
        assert!(e.to_string().len() < 160);
        let e = parse_round_batch("{\"answer\": 42}").unwrap_err();
        assert!(matches!(e, ParseError::Shape(_)));
    }

    #[test]
    fn parses_synthesis() {
        let s = parse_synthesis("{\"opening\":\"o\",\"work_on\":[\"a\",\"b\"]}").unwrap();
        assert_eq!(s.work_on.len(), 2);
        assert!(parse_synthesis("{\"work_on\":[]}").is_err());
    }

    #[test]
    fn shape_errors_never_echo_the_models_text() {
        let huge_round = "A".repeat(2000);
        let text =
            format!("{{\"rounds\":[{{\"round\":\"{huge_round}\",\"read\":\"x\",\"plays\":[]}}]}}");
        let e = parse_round_batch(&text).unwrap_err();
        assert!(matches!(e, ParseError::Shape(_)));
        assert!(e.to_string().chars().count() < 160);
    }

    #[test]
    fn not_json_errors_are_bounded_even_for_quote_heavy_text() {
        let text = "\"".repeat(200);
        let e = parse_round_batch(&text).unwrap_err();
        assert!(matches!(e, ParseError::NotJson(_)));
        let s = e.to_string();
        assert!(s.chars().count() < 160);
        assert!(!s.contains("\\\""));
    }
}
