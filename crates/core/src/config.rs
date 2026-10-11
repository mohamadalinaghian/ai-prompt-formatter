use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_CONFIG_TOML: &str = r#"
[budget]
total_ms = 9000
payload_tokens_max = 2500

[llm.writer]
chain = ["openai/gpt-oss-120b", "qwen/qwen3.8-27b", "openai/gpt-oss-20b"]
api_key_source = "keychain:pfmt-groq"
timeout_ms = 6000

[llm.compactor]
model = "qwen/qwen3.8-27b"

[decider]
provider = "jev"

[stages.input]
enabled = true
on_fail = "abort"

[stages.redact]
enabled = true
on_fail = "skip"

[stages.gate]
enabled = true
on_fail = "skip"

[stages.context]
enabled = true
on_fail = "skip"

[stages.history]
enabled = true
on_fail = "skip"

[stages.writer]
enabled = true
on_fail = "skip"

[stages.verify]
enabled = true
on_fail = "skip"

[stages.render]
enabled = true
on_fail = "abort"

[stages.log]
enabled = true
on_fail = "continue"

[logs]
retention_days = 30
"#;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnFail {
    #[default]
    Skip,
    Abort,
    Continue,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct StageConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub on_fail: OnFail,
}

fn default_true() -> bool {
    true
}

impl Default for StageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_fail: OnFail::Skip,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetConfig {
    #[serde(default = "default_total_ms")]
    pub total_ms: u64,
    #[serde(default = "default_payload_tokens_max")]
    pub payload_tokens_max: usize,
}

fn default_total_ms() -> u64 {
    9000
}
fn default_payload_tokens_max() -> usize {
    2500
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            total_ms: default_total_ms(),
            payload_tokens_max: default_payload_tokens_max(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WriterConfig {
    #[serde(default = "default_writer_chain")]
    pub chain: Vec<String>,
    #[serde(default = "default_api_key_source")]
    pub api_key_source: String,
    #[serde(default = "default_writer_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_writer_chain() -> Vec<String> {
    vec![
        "openai/gpt-oss-120b".to_string(),
        "qwen/qwen3.8-27b".to_string(),
        "openai/gpt-oss-20b".to_string(),
    ]
}

fn default_api_key_source() -> String {
    "keychain:pfmt-groq".to_string()
}

fn default_writer_timeout_ms() -> u64 {
    6000
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            chain: default_writer_chain(),
            api_key_source: default_api_key_source(),
            timeout_ms: default_writer_timeout_ms(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompactorConfig {
    #[serde(default = "default_compactor_model")]
    pub model: String,
}

fn default_compactor_model() -> String {
    "qwen/qwen3.8-27b".to_string()
}

impl Default for CompactorConfig {
    fn default() -> Self {
        Self {
            model: default_compactor_model(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LlmConfig {
    #[serde(default)]
    pub writer: WriterConfig,
    #[serde(default)]
    pub compactor: CompactorConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeciderConfig {
    #[serde(default = "default_decider_provider")]
    pub provider: String,
}

fn default_decider_provider() -> String {
    "jev".to_string()
}

impl Default for DeciderConfig {
    fn default() -> Self {
        Self {
            provider: default_decider_provider(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogConfig {
    #[serde(default = "default_retention_days")]
    pub retention_days: u64,
}

fn default_retention_days() -> u64 {
    30
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            retention_days: default_retention_days(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub stages: HashMap<String, StageConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub decider: DeciderConfig,
    #[serde(default)]
    pub stages: HashMap<String, StageConfig>,
    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
    #[serde(default)]
    pub logs: LogConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

impl Config {
    pub fn from_toml_str(content: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(content)?;
        Ok(config)
    }

    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::Io(path.to_path_buf(), e))?;
        Self::from_toml_str(&content)
    }

    /// Load configuration by searching user config directory, local config, or embedded fallback.
    pub fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let user_cfg = config_dir.join("pfmt/config.toml");
            if user_cfg.exists() {
                if let Ok(cfg) = Self::load_from_path(&user_cfg) {
                    return cfg;
                }
            }
        }

        let local_cfg = PathBuf::from("config.toml");
        if local_cfg.exists() {
            if let Ok(cfg) = Self::load_from_path(&local_cfg) {
                return cfg;
            }
        }

        Self::from_toml_str(DEFAULT_CONFIG_TOML)
            .expect("embedded default config must be valid TOML")
    }

    /// Returns whether a stage is enabled, taking into account per-app overrides.
    pub fn is_stage_enabled(&self, app: &str, stage: &str) -> bool {
        if let Some(app_cfg) = self.apps.get(app) {
            if let Some(stage_cfg) = app_cfg.stages.get(stage) {
                return stage_cfg.enabled;
            }
        }
        self.stages.get(stage).map(|s| s.enabled).unwrap_or(true)
    }

    /// Returns the on_fail behavior for a stage, taking into account per-app overrides.
    pub fn stage_on_fail(&self, app: &str, stage: &str) -> OnFail {
        if let Some(app_cfg) = self.apps.get(app) {
            if let Some(stage_cfg) = app_cfg.stages.get(stage) {
                return stage_cfg.on_fail;
            }
        }
        self.stages
            .get(stage)
            .map(|s| s.on_fail)
            .unwrap_or_else(|| match stage {
                "input" | "render" => OnFail::Abort,
                "log" => OnFail::Continue,
                _ => OnFail::Skip,
            })
    }
}
