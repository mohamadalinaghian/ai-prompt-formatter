use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Writer output schema as defined in Section 4.3.
/// Strict: deny_unknown_fields, all fields required in schema, arrays may be empty.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WriterOutput {
    /// Imperative, specific task brief (≤ 1200 chars).
    pub task: String,

    /// Concrete facts from repo/history that change execution (≤ 6 items, each ≤ 200 chars).
    #[serde(default)]
    pub context: Vec<String>,

    /// Paths from <candidates> relevant to this task (≤ 10 items).
    #[serde(default, rename = "relevant_files")]
    pub relevant_files: Vec<String>,

    /// Observable, testable criteria (1–8 items, each ≤ 200 chars).
    #[serde(default, rename = "acceptance_criteria")]
    pub acceptance_criteria: Vec<String>,

    /// Extra constraints explicitly requested by user (≤ 5 items, each ≤ 200 chars).
    #[serde(default, rename = "extra_rules")]
    pub extra_rules: Vec<String>,

    /// Extra report format lines requested by user (≤ 3 items, each ≤ 200 chars).
    #[serde(default, rename = "extra_report")]
    pub extra_report: Vec<String>,

    /// Assumptions made for ambiguous requirements (≤ 4 items, each ≤ 200 chars).
    #[serde(default)]
    pub assumptions: Vec<String>,

    /// Items known to be out of scope for this prompt (≤ 5 items, each ≤ 200 chars).
    #[serde(default)]
    pub out_of_scope: Vec<String>,

    /// Verified commands and LLM verify items (≤ 5 items, each ≤ 200 chars).
    #[serde(default)]
    pub verify: Vec<String>,

    /// Session kind (e.g. "read-only", "write-enabled").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_kind: Option<String>,

    /// Confidence report from Jev scoring of LLM-written items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confidence: Vec<ItemConfidence>,
}

/// Confidence score for a single item in a section.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
pub struct ItemConfidence {
    /// Section name (e.g., "task", "ctx_1", "scope_1", "ver_1")
    pub section: String,
    /// Index of the item within its section
    pub index: usize,
    /// Confidence score (Some(f64) if scored, None if uncheckable)
    pub score: Option<f64>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("task exceeds 1200 characters (got {0})")]
    TaskTooLong(usize),
    #[error("context exceeds 6 items (got {0})")]
    TooManyContextItems(usize),
    #[error("context item exceeds 200 characters (got {0})")]
    ContextItemTooLong(usize),
    #[error("relevant_files exceeds 10 items (got {0})")]
    TooManyRelevantFiles(usize),
    #[error("acceptance_criteria exceeds 8 items (got {0})")]
    TooManyAcceptanceCriteria(usize),
    #[error("acceptance_criteria item exceeds 200 characters (got {0})")]
    AcceptanceCriterionTooLong(usize),
    #[error("extra_rules exceeds 5 items (got {0})")]
    TooManyExtraRules(usize),
    #[error("extra_rule item exceeds 200 characters (got {0})")]
    ExtraRuleTooLong(usize),
    #[error("extra_report exceeds 3 items (got {0})")]
    TooManyExtraReports(usize),
    #[error("extra_report item exceeds 200 characters (got {0})")]
    ExtraReportTooLong(usize),
    #[error("assumptions exceeds 4 items (got {0})")]
    TooManyAssumptions(usize),
    #[error("assumption item exceeds 200 characters (got {0})")]
    AssumptionTooLong(usize),
}

impl WriterOutput {
    /// Validate all length and item constraints.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let task_len = self.task.chars().count();
        if task_len > 1200 {
            return Err(ValidationError::TaskTooLong(task_len));
        }

        if self.context.len() > 6 {
            return Err(ValidationError::TooManyContextItems(self.context.len()));
        }
        for item in &self.context {
            let len = item.chars().count();
            if len > 200 {
                return Err(ValidationError::ContextItemTooLong(len));
            }
        }

        if self.relevant_files.len() > 10 {
            return Err(ValidationError::TooManyRelevantFiles(
                self.relevant_files.len(),
            ));
        }

        if self.acceptance_criteria.len() > 8 {
            return Err(ValidationError::TooManyAcceptanceCriteria(
                self.acceptance_criteria.len(),
            ));
        }
        for item in &self.acceptance_criteria {
            let len = item.chars().count();
            if len > 200 {
                return Err(ValidationError::AcceptanceCriterionTooLong(len));
            }
        }

        if self.extra_rules.len() > 5 {
            return Err(ValidationError::TooManyExtraRules(self.extra_rules.len()));
        }
        for item in &self.extra_rules {
            let len = item.chars().count();
            if len > 200 {
                return Err(ValidationError::ExtraRuleTooLong(len));
            }
        }

        if self.extra_report.len() > 3 {
            return Err(ValidationError::TooManyExtraReports(
                self.extra_report.len(),
            ));
        }
        for item in &self.extra_report {
            let len = item.chars().count();
            if len > 200 {
                return Err(ValidationError::ExtraReportTooLong(len));
            }
        }

        if self.assumptions.len() > 4 {
            return Err(ValidationError::TooManyAssumptions(self.assumptions.len()));
        }
        for item in &self.assumptions {
            let len = item.chars().count();
            if len > 200 {
                return Err(ValidationError::AssumptionTooLong(len));
            }
        }

        Ok(())
    }

    /// Generate strict JSON schema string for OpenAI/Groq structured output.
    pub fn json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(WriterOutput);
        serde_json::to_value(schema).unwrap_or(serde_json::Value::Null)
    }
}
