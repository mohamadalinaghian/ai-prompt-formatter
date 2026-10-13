use pfmt_cli::pipeline::{execute_enhance_pipeline, StageFailureHook};
use pfmt_core::{Config, ExitCode, Profile};
use std::path::PathBuf;

fn test_profile() -> Profile {
    Profile::load_default()
}

fn test_config() -> Config {
    Config::load()
}

#[tokio::test]
async fn test_stage_input_empty_aborts() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let res =
        execute_enhance_pipeline("", "Antigravity", false, &cwd, &config, &profile, None).await;

    assert_eq!(res.exit_code, ExitCode::Abort);
    assert!(res.stdout_content.is_empty());
    assert!(res.stderr_status.contains("empty input: nothing to do"));
}

#[tokio::test]
async fn test_stage_idempotence_unchanged() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let input = "<task>\nExisting task\n</task>\n<acceptance_criteria>\n1. criteria\n</acceptance_criteria>";
    let res =
        execute_enhance_pipeline(input, "Antigravity", false, &cwd, &config, &profile, None).await;

    assert_eq!(res.exit_code, ExitCode::Ok);
    assert_eq!(res.stdout_content, input);
    assert_eq!(res.stderr_status, "unchanged");
    assert!(res.failed_stages.is_empty());
}

#[tokio::test]
async fn test_stage_forced_no_llm_clean_xml() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let input = "Add a dark mode toggle button to the settings page";
    let res =
        execute_enhance_pipeline(input, "Antigravity", true, &cwd, &config, &profile, None).await;

    assert_eq!(res.exit_code, ExitCode::Ok);
    assert!(res
        .stdout_content
        .contains("<task>\nAdd a dark mode toggle button to the settings page\n</task>"));
    assert!(res.stdout_content.contains("<rules>"));
    assert!(res.stdout_content.contains("<report>"));
    assert!(!res.stdout_content.contains("PFMT DEGRADED OUTPUT"));
    assert!(res.stderr_status.starts_with("OK"));
    assert!(res.failed_stages.is_empty());
}

#[tokio::test]
async fn test_stage_fail_redact_degrades_to_no_llm() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_redact: true,
        ..Default::default()
    };

    let input = "Fix database query with secret token";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res.stdout_content.contains("Failed stages: redact"));
    assert!(res
        .stdout_content
        .contains("<task>\nFix database query with secret token\n</task>"));
    assert!(res.stderr_status.contains("DEGRADED:"));
}

#[tokio::test]
async fn test_stage_fail_context_degrades_with_banner() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_context: true,
        ..Default::default()
    };

    let input = "Implement new feature";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        true,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res.stdout_content.contains("Failed stages: context"));
    assert!(res
        .stdout_content
        .contains("<task>\nImplement new feature\n</task>"));
}

#[tokio::test]
async fn test_stage_fail_history_degrades_with_banner() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_history: true,
        ..Default::default()
    };

    let input = "Continue previous discussion";
    let res = execute_enhance_pipeline(
        input,
        "OpenCode",
        true,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res.stdout_content.contains("Failed stages: history"));
    assert!(res
        .stdout_content
        .contains("<task>\nContinue previous discussion\n</task>"));
}

#[tokio::test]
async fn test_stage_fail_gate_degrades_with_banner() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_gate: true,
        ..Default::default()
    };

    let input = "Analyze files in repo";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res.stdout_content.contains("Failed stages: gate"));
}

#[tokio::test]
async fn test_stage_fail_writer_degrades_to_no_llm_skeleton() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_writer: true,
        ..Default::default()
    };

    let input = "Update documentation";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res
        .stdout_content
        .contains("Failed stages: writer (all models failed / timeout)"));
    assert!(res
        .stdout_content
        .contains("Result: your raw prompt was wrapped in the default skeleton and NOT enhanced."));
    assert!(res
        .stdout_content
        .contains("<task>\nUpdate documentation\n</task>"));
}

#[tokio::test]
async fn test_stage_fail_verify_marks_unverified_banner() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_verify: true,
        ..Default::default()
    };

    let input = "Run unit tests";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res.stdout_content.contains("Failed stages: verify"));
}

#[tokio::test]
async fn test_stage_fail_render_aborts() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_render: true,
        ..Default::default()
    };

    let input = "Broken render test";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Abort);
    assert!(res.stdout_content.is_empty());
    assert!(res
        .stderr_status
        .contains("ABORT: render stage failed (bug)"));
}

#[tokio::test]
async fn test_stage_timeout_degrades_to_no_llm() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        force_timeout: true,
        ..Default::default()
    };

    let input = "Task that exceeded time budget";
    let res = execute_enhance_pipeline(
        input,
        "Antigravity",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Degraded);
    assert!(res
        .stdout_content
        .contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(res
        .stdout_content
        .contains("timeout (total time budget exceeded)"));
    assert!(res
        .stdout_content
        .contains("<task>\nTask that exceeded time budget\n</task>"));
}

#[tokio::test]
async fn test_per_app_stage_override_abort() {
    let mut config = test_config();
    // Add custom app config where context stage aborts instead of skips
    let mut app_cfg = pfmt_core::AppConfig::default();
    app_cfg.stages.insert(
        "context".to_string(),
        pfmt_core::StageConfig {
            enabled: true,
            on_fail: pfmt_core::OnFail::Abort,
        },
    );
    config.apps.insert("StrictApp".to_string(), app_cfg);

    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let hook = StageFailureHook {
        fail_context: true,
        ..Default::default()
    };

    let res = execute_enhance_pipeline(
        "Test strict abort",
        "StrictApp",
        false,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;

    assert_eq!(res.exit_code, ExitCode::Abort);
    assert!(res.stdout_content.is_empty());
    assert!(res
        .stderr_status
        .contains("ABORT: context collection failed"));
}

#[tokio::test]
async fn test_stage_history_cached_summary_integration() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from("/Users/test/pipeline_history_test");

    if let Ok(db) = pfmt_history::HistoryDb::open_default() {
        let pk = db
            .upsert_session(
                "TestIntegrationApp",
                "test-history-sess",
                Some("TestIntegrationApp"),
                Some("test-model"),
                cwd.to_str(),
                None,
                None,
            )
            .expect("upsert failed");
        db.update_summary(
            pk,
            "Prior task: created SQLite migrations.",
            "test-model",
            "v1",
        )
        .expect("update summary failed");
    }

    // Run pipeline with no_llm=true (forced no-llm) to verify it reads history and proceeds
    let res = execute_enhance_pipeline(
        "Build new endpoint",
        "TestIntegrationApp",
        true,
        &cwd,
        &config,
        &profile,
        None,
    )
    .await;

    // Should succeed cleanly with exit code 0 and no history stage failures
    assert_eq!(res.exit_code, ExitCode::Ok);
    assert!(!res.failed_stages.iter().any(|s| s.starts_with("history")));
}
