use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("API key not found in keychain (service '{0}') and fallback env '{1}' is unset")]
    NotFound(String, String),
    #[error("environment variable '{0}' is not set")]
    EnvVarNotSet(String),
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("empty or invalid key source format '{0}'")]
    InvalidSource(String),
}

/// Resolves an API key from a configured source (e.g. "keychain:pfmt-groq" or "env:GROQ_API_KEY")
/// with transparent fallback to `fallback_env_var` if the keychain item does not exist.
pub fn resolve_api_key(source: &str, fallback_env_var: &str) -> Result<String, KeyError> {
    let trimmed = source.trim();

    if let Some(service) = trimmed.strip_prefix("keychain:") {
        let service_name = service.trim();
        // Attempt retrieving from macOS Keychain
        // Default username in macOS generic passwords for cli tools is typically the current user or "pfmt"
        let username = whoami_username();
        let entry = keyring::Entry::new(service_name, &username)
            .or_else(|_| keyring::Entry::new(service_name, "pfmt"));

        if let Ok(entry) = entry {
            if let Ok(password) = entry.get_password() {
                let pass_trimmed = password.trim();
                if !pass_trimmed.is_empty() {
                    return Ok(pass_trimmed.to_string());
                }
            }
        }

        // Fallback to environment variable if keychain fails or is empty
        if !fallback_env_var.is_empty() {
            if let Ok(val) = std::env::var(fallback_env_var) {
                let val_trimmed = val.trim();
                if !val_trimmed.is_empty() {
                    return Ok(val_trimmed.to_string());
                }
            }
        }

        return Err(KeyError::NotFound(
            service_name.to_string(),
            fallback_env_var.to_string(),
        ));
    }

    if let Some(env_var) = trimmed.strip_prefix("env:") {
        let var_name = env_var.trim();
        return std::env::var(var_name)
            .map(|v| v.trim().to_string())
            .map_err(|_| KeyError::EnvVarNotSet(var_name.to_string()));
    }

    // Direct env var reference or fallback
    if let Ok(val) = std::env::var(trimmed) {
        let val_trimmed = val.trim();
        if !val_trimmed.is_empty() {
            return Ok(val_trimmed.to_string());
        }
    }

    if !fallback_env_var.is_empty() {
        if let Ok(val) = std::env::var(fallback_env_var) {
            let val_trimmed = val.trim();
            if !val_trimmed.is_empty() {
                return Ok(val_trimmed.to_string());
            }
        }
    }

    Err(KeyError::InvalidSource(source.to_string()))
}

fn whoami_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "pfmt".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_from_env_direct() {
        std::env::set_var("TEST_PFMT_KEY", "secret-test-key-123");
        let resolved = resolve_api_key("env:TEST_PFMT_KEY", "FALLBACK_VAR").unwrap();
        assert_eq!(resolved, "secret-test-key-123");
    }

    #[test]
    fn test_resolve_keychain_fallback_to_env() {
        std::env::set_var("GROQ_TEST_FALLBACK", "gsk_test1234567890abcdefg");
        let resolved = resolve_api_key(
            "keychain:non_existent_pfmt_service_xyz",
            "GROQ_TEST_FALLBACK",
        )
        .unwrap();
        assert_eq!(resolved, "gsk_test1234567890abcdefg");
    }

    #[test]
    fn test_resolve_missing_fails_explicitly() {
        let res = resolve_api_key(
            "keychain:completely_missing_xyz_123",
            "COMPLETELY_MISSING_ENV_XYZ_123",
        );
        assert!(matches!(res, Err(KeyError::NotFound(_, _))));
    }
}
