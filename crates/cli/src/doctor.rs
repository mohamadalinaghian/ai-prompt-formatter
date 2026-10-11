use pfmt_core::{resolve_api_key, Config, Profile};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

pub struct DoctorCheck {
    pub category: &'static str,
    pub name: String,
    pub status: CheckStatus,
    pub details: String,
    pub remediation: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "[PASS]",
            CheckStatus::Warn => "[WARN]",
            CheckStatus::Fail => "[FAIL]",
        }
    }
}

pub async fn run_doctor(config: &Config) -> bool {
    println!("\n🔍 Running pfmt doctor system diagnostics...\n");

    let mut checks = vec![
        check_config(),
        check_profile(),
        check_groq_key(config),
        check_jev_key(),
        check_clipboard(),
        check_macos_accessibility(),
    ];

    // 4. Agent stores
    checks.extend(check_agent_stores());

    // 5. Model reachability
    checks.push(check_groq_reachability(config).await);
    checks.push(check_jev_reachability().await);

    // Print summary table
    let mut critical_failures = 0;
    let mut warnings = 0;

    let mut current_category = "";
    for check in &checks {
        if check.category != current_category {
            current_category = check.category;
            println!("═══ {} ═══", current_category);
        }

        println!(
            "  {} {}: {}",
            check.status.badge(),
            check.name,
            check.details
        );
        if let Some(ref rem) = check.remediation {
            println!("       💡 Recommendation: {}", rem);
        }

        match check.status {
            CheckStatus::Fail => critical_failures += 1,
            CheckStatus::Warn => warnings += 1,
            CheckStatus::Pass => {}
        }
    }

    println!("\n───────────────────────────────────────────");
    if critical_failures > 0 {
        println!(
            "❌ Doctor detected {} critical failure(s) and {} warning(s).",
            critical_failures, warnings
        );
        println!("Please address the failed items above before using pfmt.");
        false
    } else if warnings > 0 {
        println!(
            "⚠️ Doctor passed with {} warning(s). Core pipeline is operational.",
            warnings
        );
        true
    } else {
        println!("✅ All doctor checks passed successfully! System is fully operational.");
        true
    }
}

fn check_config() -> DoctorCheck {
    let local = PathBuf::from("config.toml");
    let user = dirs::config_dir().map(|d| d.join("pfmt/config.toml"));

    let source = if local.exists() {
        "local config.toml"
    } else if user.as_ref().map(|p| p.exists()).unwrap_or(false) {
        "~/.config/pfmt/config.toml"
    } else {
        "embedded defaults"
    };

    DoctorCheck {
        category: "Configuration",
        name: "Config File".to_string(),
        status: CheckStatus::Pass,
        details: format!("Loaded successfully from {}", source),
        remediation: None,
    }
}

fn check_profile() -> DoctorCheck {
    let profile = Profile::load_default();
    if profile.rules.is_empty() || profile.report.is_empty() {
        DoctorCheck {
            category: "Configuration",
            name: "Default Profile".to_string(),
            status: CheckStatus::Fail,
            details: "Default profile is empty or corrupted".to_string(),
            remediation: Some(
                "Verify profiles/default.toml contains valid rules and report templates"
                    .to_string(),
            ),
        }
    } else {
        DoctorCheck {
            category: "Configuration",
            name: "Default Profile".to_string(),
            status: CheckStatus::Pass,
            details: "Loaded default rules and report templates".to_string(),
            remediation: None,
        }
    }
}

fn check_groq_key(config: &Config) -> DoctorCheck {
    match resolve_api_key(&config.llm.writer.api_key_source, "GROQ_API_KEY") {
        Ok(key) => {
            let masked = mask_secret(&key);
            DoctorCheck {
                category: "API Keys",
                name: "Groq API Key".to_string(),
                status: CheckStatus::Pass,
                details: format!("Found ({}) via {}", masked, config.llm.writer.api_key_source),
                remediation: None,
            }
        }
        Err(e) => DoctorCheck {
            category: "API Keys",
            name: "Groq API Key".to_string(),
            status: CheckStatus::Fail,
            details: format!("Not found: {}", e),
            remediation: Some(
                "Store key in Keychain via `security add-generic-password -U -s pfmt-groq -a default -w <KEY>` or set GROQ_API_KEY environment variable"
                    .to_string(),
            ),
        },
    }
}

fn check_jev_key() -> DoctorCheck {
    match resolve_api_key("keychain:pfmt-jev", "TYPESAFE_API_KEY") {
        Ok(key) => {
            let masked = mask_secret(&key);
            DoctorCheck {
                category: "API Keys",
                name: "TypeSafe Jev API Key".to_string(),
                status: CheckStatus::Pass,
                details: format!("Found ({})", masked),
                remediation: None,
            }
        }
        Err(_) => DoctorCheck {
            category: "API Keys",
            name: "TypeSafe Jev API Key".to_string(),
            status: CheckStatus::Warn,
            details: "Not configured (gate will use Groq fallback or heuristic bypass)".to_string(),
            remediation: Some(
                "Optional: Store in Keychain via `security add-generic-password -U -s pfmt-jev -a default -w <KEY>` or set TYPESAFE_API_KEY"
                    .to_string(),
            ),
        },
    }
}

fn check_clipboard() -> DoctorCheck {
    let pbcopy_ok = Command::new("which")
        .arg("pbcopy")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let pbpaste_ok = Command::new("which")
        .arg("pbpaste")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if pbcopy_ok && pbpaste_ok {
        DoctorCheck {
            category: "System Integration",
            name: "macOS Clipboard (pbcopy/pbpaste)".to_string(),
            status: CheckStatus::Pass,
            details: "Available in PATH".to_string(),
            remediation: None,
        }
    } else {
        DoctorCheck {
            category: "System Integration",
            name: "macOS Clipboard (pbcopy/pbpaste)".to_string(),
            status: CheckStatus::Fail,
            details: "pbcopy or pbpaste missing".to_string(),
            remediation: Some(
                "Ensure running on standard macOS with standard /usr/bin tools".to_string(),
            ),
        }
    }
}

fn check_macos_accessibility() -> DoctorCheck {
    let script = r#"tell application "System Events" to get name of first application process whose frontmost is true"#;
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(out) if out.status.success() => {
            let app_name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            DoctorCheck {
                category: "System Integration",
                name: "Accessibility Permissions".to_string(),
                status: CheckStatus::Pass,
                details: format!("Granted (detected front app: '{}')", app_name),
                remediation: None,
            }
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            DoctorCheck {
                category: "System Integration",
                name: "Accessibility Permissions".to_string(),
                status: CheckStatus::Fail,
                details: format!("Failed: {}", err),
                remediation: Some(
                    "Grant Accessibility permission to Terminal/iTerm/Karabiner in System Settings -> Privacy & Security -> Accessibility"
                        .to_string(),
                ),
            }
        }
        Err(e) => DoctorCheck {
            category: "System Integration",
            name: "Accessibility Permissions".to_string(),
            status: CheckStatus::Fail,
            details: format!("Failed to execute osascript: {}", e),
            remediation: Some("Ensure macOS System Events and osascript are available".to_string()),
        },
    }
}

fn check_agent_stores() -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    let stores = [
        (
            "Antigravity",
            dirs::home_dir().map(|h| h.join(".gemini/antigravity")),
        ),
        (
            "OpenCode Desktop",
            dirs::data_dir().map(|d| d.join("ai.opencode.desktop")),
        ),
        ("Freebuff", dirs::data_dir().map(|d| d.join("freebuff"))),
        ("Hermes", dirs::home_dir().map(|h| h.join(".hermes"))),
    ];

    for (name, path_opt) in stores {
        match path_opt {
            Some(path) if path.exists() => {
                checks.push(DoctorCheck {
                    category: "Agent Stores",
                    name: format!("{} Store", name),
                    status: CheckStatus::Pass,
                    details: format!("Found at {}", path.display()),
                    remediation: None,
                });
            }
            Some(path) => {
                let app_slug = name.to_lowercase().replace(' ', "");
                checks.push(DoctorCheck {
                    category: "Agent Stores",
                    name: format!("{} Store", name),
                    status: CheckStatus::Warn,
                    details: format!("Not found at {}", path.display()),
                    remediation: Some(format!(
                        "If you do not use {}, history adapter can be disabled via [apps.{}.stages.history] enabled = false",
                        name, app_slug
                    )),
                });
            }
            None => {
                checks.push(DoctorCheck {
                    category: "Agent Stores",
                    name: format!("{} Store", name),
                    status: CheckStatus::Warn,
                    details: "Could not resolve home directory".to_string(),
                    remediation: None,
                });
            }
        }
    }

    checks
}

async fn check_groq_reachability(config: &Config) -> DoctorCheck {
    let key = match resolve_api_key(&config.llm.writer.api_key_source, "GROQ_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            return DoctorCheck {
                category: "Model Reachability",
                name: "Groq Endpoint".to_string(),
                status: CheckStatus::Fail,
                details: "Cannot test reachability: API key missing".to_string(),
                remediation: Some("Configure Groq API key first".to_string()),
            };
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck {
                category: "Model Reachability",
                name: "Groq Endpoint".to_string(),
                status: CheckStatus::Fail,
                details: format!("Failed to create HTTP client: {}", e),
                remediation: None,
            };
        }
    };

    let start = Instant::now();
    let resp = client
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(key)
        .send()
        .await;

    match resp {
        Ok(res) if res.status().is_success() => {
            let latency_ms = start.elapsed().as_millis();
            DoctorCheck {
                category: "Model Reachability",
                name: "Groq Endpoint".to_string(),
                status: CheckStatus::Pass,
                details: format!("Reachable (HTTP 200 OK, latency {}ms)", latency_ms),
                remediation: None,
            }
        }
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            DoctorCheck {
                category: "Model Reachability",
                name: "Groq Endpoint".to_string(),
                status: CheckStatus::Fail,
                details: format!(
                    "HTTP error {}: {}",
                    status,
                    body.chars().take(100).collect::<String>()
                ),
                remediation: Some(
                    "Check if your Groq API key is valid and has sufficient quota".to_string(),
                ),
            }
        }
        Err(e) => DoctorCheck {
            category: "Model Reachability",
            name: "Groq Endpoint".to_string(),
            status: CheckStatus::Fail,
            details: format!("Network connection failed: {}", e),
            remediation: Some("Verify internet connectivity to api.groq.com".to_string()),
        },
    }
}

async fn check_jev_reachability() -> DoctorCheck {
    let key = match resolve_api_key("keychain:pfmt-jev", "TYPESAFE_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            return DoctorCheck {
                category: "Model Reachability",
                name: "TypeSafe Jev Endpoint".to_string(),
                status: CheckStatus::Warn,
                details: "Skipped: TYPESAFE_API_KEY not set".to_string(),
                remediation: Some(
                    "Set TYPESAFE_API_KEY to enable Jev fast probabilistic gating".to_string(),
                ),
            };
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck {
                category: "Model Reachability",
                name: "TypeSafe Jev Endpoint".to_string(),
                status: CheckStatus::Fail,
                details: format!("Failed to create HTTP client: {}", e),
                remediation: None,
            };
        }
    };

    let start = Instant::now();
    // Test a tiny probe
    let payload = serde_json::json!({
        "model": "jev-latest",
        "state": "diagnostic ping",
        "questions": {
            "ping": {
                "type": "noul",
                "instructions": "Is this a diagnostic ping test?"
            }
        }
    });

    let resp = client
        .post("https://api.typesafe.ai/v1/systemone")
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await;

    match resp {
        Ok(res) if res.status().is_success() => {
            let latency_ms = start.elapsed().as_millis();
            DoctorCheck {
                category: "Model Reachability",
                name: "TypeSafe Jev Endpoint".to_string(),
                status: CheckStatus::Pass,
                details: format!("Reachable (HTTP 200 OK, latency {}ms)", latency_ms),
                remediation: None,
            }
        }
        Ok(res) => {
            let status = res.status();
            DoctorCheck {
                category: "Model Reachability",
                name: "TypeSafe Jev Endpoint".to_string(),
                status: CheckStatus::Warn,
                details: format!("Returned HTTP status {}", status),
                remediation: Some(
                    "Check TYPESAFE_API_KEY validity or fallback to Groq decider".to_string(),
                ),
            }
        }
        Err(e) => DoctorCheck {
            category: "Model Reachability",
            name: "TypeSafe Jev Endpoint".to_string(),
            status: CheckStatus::Warn,
            details: format!("Network request failed: {}", e),
            remediation: Some("Verify internet connectivity to api.typesafe.ai".to_string()),
        },
    }
}

fn mask_secret(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.len() <= 8 {
        return "***".to_string();
    }
    let prefix = &trimmed[..4.min(trimmed.len())];
    let suffix = &trimmed[trimmed.len().saturating_sub(4)..];
    format!("{}...{}", prefix, suffix)
}
