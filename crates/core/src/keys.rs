use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
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
    #[error("keychain access denied for service '{0}': {1}")]
    AccessDenied(String, String),
}

fn key_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached(key: &str) -> Option<String> {
    key_cache().lock().ok().and_then(|m| m.get(key).cloned())
}

fn store_cached(key: &str, value: &str) {
    if let Ok(mut m) = key_cache().lock() {
        m.insert(key.to_string(), value.to_string());
    }
}

/// Clears the in-process key cache. Primarily for tests.
#[allow(dead_code)]
pub fn clear_key_cache() {
    if let Ok(mut m) = key_cache().lock() {
        m.clear();
    }
}

fn read_env_nonempty(var: &str) -> Option<String> {
    if var.is_empty() {
        return None;
    }
    std::env::var(var)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Resolves an API key from a configured source (e.g. "keychain:pfmt-groq" or "env:GROQ_API_KEY")
/// with transparent fallback to `fallback_env_var` if the keychain item does not exist.
///
/// Lookup order (to avoid needless macOS Keychain prompts):
/// 1. `fallback_env_var` if set and non-empty (fast, non-interactive).
/// 2. Keychain, trying accounts `["default", $USER, "pfmt"]` in order.
///    Results are cached per-process so a single `pfmt` run hits the
///    Keychain at most once per service instead of once per model/retry.
/// 3. `fallback_env_var` again (covers the case where env was set between
///    steps, and keeps the documented keychain-first fallback working when
///    env was unset at step 1).
pub fn resolve_api_key(source: &str, fallback_env_var: &str) -> Result<String, KeyError> {
    let trimmed = source.trim();

    if let Some(service) = trimmed.strip_prefix("keychain:") {
        let service_name = service.trim();
        let cache_key = format!("keychain:{}|env:{}", service_name, fallback_env_var);
        if let Some(hit) = cached(&cache_key) {
            return Ok(hit);
        }

        // Step 1: env first — avoids a Keychain auth prompt when the user
        // already configured the env fallback.
        if let Some(val) = read_env_nonempty(fallback_env_var) {
            store_cached(&cache_key, &val);
            return Ok(val);
        }

        // Step 2: Keychain. NOTE: `Entry::new` only builds a handle and
        // almost never fails, so chaining `.or_else` on it never actually
        // tries the next account. We must call `get_password` per account.
        let username = whoami_username();
        let accounts = [String::from("default"), username, String::from("pfmt")];
        let mut last_err: Option<String> = None;
        for account in &accounts {
            match keyring::Entry::new(service_name, account) {
                Ok(entry) => match entry.get_password() {
                    Ok(password) => {
                        let pass_trimmed = password.trim().to_string();
                        if !pass_trimmed.is_empty() {
                            store_cached(&cache_key, &pass_trimmed);
                            return Ok(pass_trimmed);
                        }
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        // `NoEntry` just means "try next account". Anything
                        // else (auth prompt denied/cancelled, IO, etc.) is
                        // worth surfacing if nothing else resolves.
                        if !matches!(e, keyring::Error::NoEntry) {
                            last_err = Some(msg);
                        }
                    }
                },
                Err(e) => {
                    last_err = Some(e.to_string());
                }
            }
        }

        // Step 3: env fallback (re-check in case it appeared, or step-1
        // was skipped because fallback_env_var was empty then — no-op).
        if let Some(val) = read_env_nonempty(fallback_env_var) {
            store_cached(&cache_key, &val);
            return Ok(val);
        }

        if let Some(detail) = last_err {
            return Err(KeyError::AccessDenied(service_name.to_string(), detail));
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
        clear_key_cache();
        std::env::set_var("GROQ_TEST_FALLBACK", "gsk_test1234567890abcdefg");
        let resolved = resolve_api_key(
            "keychain:non_existent_pfmt_service_xyz",
            "GROQ_TEST_FALLBACK",
        )
        .unwrap();
        assert_eq!(resolved, "gsk_test1234567890abcdefg");
    }

    #[test]
    fn test_env_takes_precedence_without_keychain_prompt() {
        // Env-first: when the fallback env is set, resolve must return it
        // without requiring any keychain item to exist.
        clear_key_cache();
        std::env::set_var("PFMT_TEST_ENV_FIRST", "env-key-wins");
        let resolved = resolve_api_key(
            "keychain:non_existent_pfmt_service_xyz_env_first",
            "PFMT_TEST_ENV_FIRST",
        )
        .unwrap();
        assert_eq!(resolved, "env-key-wins");
        std::env::remove_var("PFMT_TEST_ENV_FIRST");
    }

    #[test]
    fn test_resolve_missing_fails_explicitly() {
        clear_key_cache();
        std::env::remove_var("COMPLETELY_MISSING_ENV_XYZ_123");
        let res = resolve_api_key(
            "keychain:completely_missing_xyz_123",
            "COMPLETELY_MISSING_ENV_XYZ_123",
        );
        assert!(
            matches!(
                res,
                Err(KeyError::NotFound(_, _)) | Err(KeyError::AccessDenied(_, _))
            ),
            "expected NotFound or AccessDenied, got {:?}",
            res
        );
    }
}
