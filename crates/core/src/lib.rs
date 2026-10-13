pub mod banner;
pub mod config;
pub mod keys;
pub mod log;
pub mod profile;
pub mod redact;
pub mod render;
pub mod types;

pub use banner::{Degradation, ExitCode};
pub use config::{AppConfig, BudgetConfig, Config, ConfigError, OnFail, StageConfig};
pub use keys::{clear_key_cache, resolve_api_key, KeyError};
pub use log::{
    slugify_topic, ExecutionLog, GateLogRecord, StageLogRecord, StageStatus, VerifierLogRecord,
    VersionLogRecord,
};
pub use profile::{Profile, ProfileError};
pub use redact::redact_secrets;
pub use render::{render, xml_escape};
pub use types::{ValidationError, WriterOutput, ItemConfidence};

/// Context gathered from the frontmost app (probe stage).
/// Every later stage uses whichever fields exist; missing fields are simply omitted.
#[derive(Debug, Clone)]
pub struct AppContext {
    pub app_name: String,
    pub project_dir: Option<std::path::PathBuf>,
    pub session_id: Option<String>,
    pub sources: Vec<String>,
}

/// Idempotence check as defined in Section 4.1:
/// If the input already contains the `<task>` and `<acceptance_criteria>` sections
/// of this format, return it unchanged (exit 0, status "unchanged").
pub fn is_idempotent(input: &str) -> bool {
    input.contains("<task>") && input.contains("<acceptance_criteria>")
}

/// Fallback no-LLM path: wraps the raw prompt into the minimal skeleton
/// with fixed rules and report blocks from the profile, optionally with a degradation banner.
pub fn no_llm_wrap_with_degradation(
    raw: &str,
    profile: &Profile,
    degradation: Option<&Degradation>,
) -> String {
    let output = WriterOutput {
        task: raw.trim().to_string(),
        context: Vec::new(),
        relevant_files: Vec::new(),
        acceptance_criteria: Vec::new(),
        extra_rules: Vec::new(),
        extra_report: Vec::new(),
        assumptions: Vec::new(),
        out_of_scope: Vec::new(),
        verify: Vec::new(),
        confidence: Vec::new(),
    };
    render(&output, profile, degradation, None, 0.6)
}

/// Fallback no-LLM path: wraps the raw prompt into the minimal skeleton
/// with fixed rules and report blocks from the profile without degradation banner.
pub fn no_llm_wrap(raw: &str, profile: &Profile) -> String {
    no_llm_wrap_with_degradation(raw, profile, None)
}
