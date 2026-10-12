use std::sync::OnceLock;

static PATTERNS: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();

fn get_patterns() -> &'static Vec<(&'static str, &'static str)> {
    PATTERNS.get_or_init(|| {
        vec![
            // Private keys (PEM format)
            (r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----", "[REDACTED_PRIVATE_KEY]"),
            // Database URLs with passwords (e.g. postgresql://user:pass@host/db)
            (r#"([a-zA-Z0-9+.-]+://[^:]+):[^@\s"']+(@[^\s"']+)"#, "$1:[REDACTED]$2"),
            // Groq API keys (gsk_...)
            (r"gsk_[a-zA-Z0-9]{32,}", "[REDACTED_GROQ_KEY]"),
            // OpenAI API keys (sk-...)
            (r"sk-[a-zA-Z0-9]{20,}", "[REDACTED_OPENAI_KEY]"),
            // Anthropic API keys (sk-ant-...)
            (r"sk-ant-[a-zA-Z0-9_.-]{20,}", "[REDACTED_ANTHROPIC_KEY]"),
            // GitHub personal access tokens
            (r"ghp_[a-zA-Z0-9]{36}", "[REDACTED_GITHUB_TOKEN]"),
            (r"github_pat_[a-zA-Z0-9_]{50,}", "[REDACTED_GITHUB_TOKEN]"),
            // AWS Access Key ID
            (r"\bAKIA[0-9A-Z]{16}\b", "[REDACTED_AWS_KEY]"),
            // Bearer tokens
            (r"(?i)bearer\s+[a-zA-Z0-9_.-]{20,}", "Bearer [REDACTED_TOKEN]"),
            // Common credential assignments (API keys, secrets, passwords, tokens)
            (r#"(?i)(api[_-]?key|secret|password|auth_token|access_token|private_key)\s*[:=]\s*['"]?[a-zA-Z0-9_.-]{8,}['"]?"#, "$1=[REDACTED]"),
        ]
    })
}

/// Redact known secret and credential patterns from text before network transmission or logging.
pub fn redact_secrets(input: &str) -> String {
    let mut result = input.to_string();
    let patterns = get_patterns();

    for (pattern, replacement) in patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern) {
            result = re.replace_all(&result, *replacement).to_string();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a syntactically valid but fake Groq key. Assembled at runtime so that no
    /// `gsk_<32+ alnum>` literal ever sits in the source tree (GitHub push protection blocks
    /// real-key-shaped strings, even in tests).
    fn fake_groq_key() -> String {
        format!("gsk_{}", "A1b2C3d4".repeat(5))
    }

    #[test]
    fn test_redact_groq_key() {
        let key = fake_groq_key();
        let text = format!("Groq key: {key} in config");
        let redacted = redact_secrets(&text);
        assert!(!redacted.contains(&key));
        assert!(redacted.contains("[REDACTED_GROQ_KEY]"));
    }

    #[test]
    fn test_redact_openai_and_anthropic_keys() {
        let text =
            "OpenAI: sk-abc1234567890abcdef12345, Anthropic: sk-ant-api03-abcdefg1234567890-hijk";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("sk-abc1234567890"));
        assert!(!redacted.contains("sk-ant-api03"));
        assert!(redacted.contains("[REDACTED_OPENAI_KEY]"));
        assert!(redacted.contains("[REDACTED_ANTHROPIC_KEY]"));
    }

    #[test]
    fn test_redact_github_and_aws_keys() {
        let text = "ghp_123456789012345678901234567890123456 and AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("ghp_123456789012345678901234567890123456"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(redacted.contains("[REDACTED_GITHUB_TOKEN]"));
        assert!(redacted.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let text = "Authorization: Bearer mySecretToken1234567890abcdefg";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("mySecretToken"));
        assert!(redacted.contains("Bearer [REDACTED_TOKEN]"));
    }

    #[test]
    fn test_redact_database_url() {
        let text = "DATABASE_URL=postgresql://postgres:supersecretpassword@localhost:5432/mydb";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("supersecretpassword"));
        assert!(redacted.contains("postgresql://postgres:[REDACTED]@localhost:5432/mydb"));
    }

    #[test]
    fn test_redact_env_password_assignment() {
        let text = "DB_PASSWORD=\"my_super_secret_db_pass_123\"";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("my_super_secret_db_pass_123"));
        assert!(redacted.contains("DB_PASSWORD=[REDACTED]"));
    }

    #[test]
    fn test_redact_private_key() {
        let text = "prefix\n-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0...\n-----END RSA PRIVATE KEY-----\nsuffix";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("MIIEowIBAAKCAQEA0"));
        assert!(redacted.contains("[REDACTED_PRIVATE_KEY]"));
    }
}
