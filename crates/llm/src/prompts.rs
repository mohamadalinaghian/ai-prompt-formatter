use std::path::Path;

pub const EMBEDDED_META_PROMPT_V1: &str = include_str!("../../../prompts/meta_prompt.v1.md");
pub const EMBEDDED_COMPACTOR_PROMPT_V1: &str = include_str!("../../../prompts/compactor.v1.md");

pub struct PromptLoader;

impl PromptLoader {
    /// Loads meta prompt v1 from file path if present, otherwise returns embedded version.
    pub fn load_meta_prompt() -> String {
        let path = Path::new("prompts/meta_prompt.v1.md");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        EMBEDDED_META_PROMPT_V1.trim().to_string()
    }

    /// Loads compactor prompt v1 from file path if present, otherwise returns embedded version.
    pub fn load_compactor_prompt() -> String {
        let path = Path::new("prompts/compactor.v1.md");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        EMBEDDED_COMPACTOR_PROMPT_V1.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_meta_prompt() {
        let prompt = PromptLoader::load_meta_prompt();
        assert!(prompt.contains("Rewrite a developer's raw prompt into a precise brief"));
        assert!(prompt.contains("acceptance_criteria"));
    }

    #[test]
    fn test_load_compactor_prompt() {
        let prompt = PromptLoader::load_compactor_prompt();
        assert!(prompt.contains("session compactor"));
        assert!(prompt.contains("summary"));
    }
}
