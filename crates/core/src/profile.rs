use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const EMBEDDED_DEFAULT_PROFILE: &str = r#"[rules]
text = """- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- For other ambiguity, pick the most reasonable option and list it in the report."""

[report]
text = """KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🎯 **Key Changes**: (bullet points, max 1 line each)
2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)
3. 📋 **Remaining Tasks**: (what is left to do, actionable items)"""
"#;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct SectionConfig {
    pub text: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct ProfileToml {
    #[serde(default)]
    pub kind: Option<String>,
    pub rules: SectionConfig,
    pub report: SectionConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub kind: Option<String>,
    pub rules: String,
    pub report: String,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("failed to read profile at {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("failed to parse profile TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

impl Profile {
    pub fn new(rules: impl Into<String>, report: impl Into<String>) -> Self {
        Self {
            kind: None,
            rules: rules.into(),
            report: report.into(),
        }
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn from_toml_str(toml_content: &str) -> Result<Self, ProfileError> {
        let parsed: ProfileToml = toml::from_str(toml_content)?;
        Ok(Self {
            kind: parsed.kind,
            rules: parsed.rules.text.trim().to_string(),
            report: parsed.report.text.trim().to_string(),
        })
    }

    pub fn load_from_path(path: &Path) -> Result<Self, ProfileError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ProfileError::Io(path.to_path_buf(), e))?;
        Self::from_toml_str(&content)
    }

    /// Load default profile: checks ~/.config/pfmt/profiles/default.toml,
    /// then ./profiles/default.toml, then embedded fallback.
    pub fn load_default() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let user_profile = config_dir.join("pfmt/profiles/default.toml");
            if user_profile.exists() {
                if let Ok(profile) = Self::load_from_path(&user_profile) {
                    return profile;
                }
            }
        }

        let local_profile = PathBuf::from("profiles/default.toml");
        if local_profile.exists() {
            if let Ok(profile) = Self::load_from_path(&local_profile) {
                return profile;
            }
        }

        Self::from_toml_str(EMBEDDED_DEFAULT_PROFILE)
            .expect("embedded default profile must be valid TOML")
    }
}
