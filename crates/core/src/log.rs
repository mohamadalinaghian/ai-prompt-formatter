use crate::redact::redact_secrets;
use crate::types::WriterOutput;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Ok,
    Failed,
    Skipped,
    Disabled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StageLogRecord {
    pub id: String,
    pub status: StageStatus,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_in: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_out: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StageLogRecord {
    pub fn new(id: &str, status: StageStatus, ms: u64) -> Self {
        Self {
            id: id.to_string(),
            status,
            ms,
            model: None,
            tokens_in: None,
            tokens_out: None,
            error: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_tokens(mut self, tokens_in: usize, tokens_out: usize) -> Self {
        self.tokens_in = Some(tokens_in);
        self.tokens_out = Some(tokens_out);
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateLogRecord {
    pub needs_history: bool,
    pub already_structured: bool,
    pub injection_risk: bool,
    pub extra_rules_requested: bool,
    pub extra_report_requested: bool,
    pub allowed_candidates: Vec<String>,
    pub probabilities: HashMap<String, f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifierLogRecord {
    pub verdict: String, // "passed" | "cleaned" | "rejected"
    pub dropped_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionLogRecord {
    pub pfmt: String,
    pub meta_prompt: String,
    pub schema: String,
    pub config_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionLog {
    pub schema_version: String,
    pub ts: String,
    pub app: String,
    pub agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Probed project directory and how it was resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
    /// Source references for the probed fields.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub probe_sources: Vec<String>,
    /// Which context layers were actually used.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context_layers: Vec<String>,
    pub topic: String,
    pub input: String,
    pub stages: Vec<StageLogRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateLogRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_sent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writer_raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writer_validated: Option<WriterOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier: Option<VerifierLogRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_xml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    pub degraded: bool,
    pub exit_code: i32,
    pub versions: VersionLogRecord,
}

/// Computes a ≤40 char URL/filesystem-safe slug from the prompt, falling back to "untitled".
pub fn slugify_topic(input: &str) -> String {
    let clean = redact_secrets(input.trim());
    let first_line = clean.lines().next().unwrap_or("untitled").trim();
    let mut slug = String::new();
    for ch in first_line.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_')
            && !slug.ends_with('-')
            && !slug.is_empty()
        {
            slug.push('-');
        }
        if slug.len() >= 40 {
            break;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_topic() {
        assert_eq!(
            slugify_topic("Add health check endpoint"),
            "add-health-check-endpoint"
        );
        assert_eq!(
            slugify_topic("   Fix: bug in parser / lexer   "),
            "fix-bug-in-parser-lexer"
        );
        assert_eq!(
            slugify_topic("A very long prompt description that definitely exceeds forty characters in total length"),
            "a-very-long-prompt-description-that-defi"
        );
        assert_eq!(slugify_topic(""), "untitled");
        assert_eq!(slugify_topic("!!! @@@ ### $$$"), "untitled");
    }
}
