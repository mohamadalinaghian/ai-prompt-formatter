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
pub use types::{ValidationError, WriterOutput};

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
    };
    render(&output, profile, degradation)
}

/// Fallback no-LLM path: wraps the raw prompt into the minimal skeleton
/// with fixed rules and report blocks from the profile without degradation banner.
pub fn no_llm_wrap(raw: &str, profile: &Profile) -> String {
    no_llm_wrap_with_degradation(raw, profile, None)
}
