use pfmt_cli::pipeline::{execute_enhance_pipeline, StageFailureHook};
use pfmt_cli::replay::run_replay;
use pfmt_cli::review::run_review;
use pfmt_core::{Config, ExecutionLog, ExitCode, Profile};
use std::fs;
use std::path::PathBuf;

fn test_config() -> Config {
    Config::load()
}

fn test_profile() -> Profile {
    Profile::load_default()
}

#[tokio::test]
async fn test_every_run_leaves_a_log() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    // 1. Normal run
    let res1 = execute_enhance_pipeline(
        "Implement rate limiting for API endpoints",
        "Antigravity",
        true,
        &cwd,
        &config,
        &profile,
        None,
    )
    .await;
    assert_eq!(res1.exit_code, ExitCode::Ok);
    assert!(res1.log_path.is_some());
    let path1 = res1.log_path.unwrap();
    assert!(path1.exists(), "Log file must exist on disk");

    let log_content = fs::read_to_string(&path1).expect("Failed to read log file");
    let parsed: ExecutionLog = serde_json::from_str(&log_content).expect("Valid log json");
    assert_eq!(parsed.app, "Antigravity");
    assert_eq!(parsed.exit_code, 0);
    assert!(!parsed.stages.is_empty());

    // 2. Aborted run (empty input)
    let res2 =
        execute_enhance_pipeline("", "Antigravity", false, &cwd, &config, &profile, None).await;
    assert_eq!(res2.exit_code, ExitCode::Abort);
    assert!(res2.log_path.is_some());
    let path2 = res2.log_path.unwrap();
    assert!(
        path2.exists(),
        "Log file for aborted run must exist on disk"
    );

    let log2_content = fs::read_to_string(&path2).expect("Failed to read log2 file");
    let parsed2: ExecutionLog = serde_json::from_str(&log2_content).expect("Valid log2 json");
    assert_eq!(parsed2.exit_code, 1);

    // 3. Degraded run (injected failure)
    let hook = StageFailureHook {
        fail_context: true,
        ..Default::default()
    };
    let res3 = execute_enhance_pipeline(
        "Refactor database connection pool",
        "OpenCode",
        true,
        &cwd,
        &config,
        &profile,
        Some(&hook),
    )
    .await;
    assert_eq!(res3.exit_code, ExitCode::Degraded);
    assert!(res3.log_path.is_some());
    let path3 = res3.log_path.unwrap();
    assert!(
        path3.exists(),
        "Log file for degraded run must exist on disk"
    );

    let log3_content = fs::read_to_string(&path3).expect("Failed to read log3 file");
    let parsed3: ExecutionLog = serde_json::from_str(&log3_content).expect("Valid log3 json");
    assert_eq!(parsed3.exit_code, 10);
    assert!(parsed3.degraded);
}

#[tokio::test]
async fn test_review_aggregates_metrics() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    // Ensure at least one run has been logged before running review
    let _ = execute_enhance_pipeline(
        "Generate metrics for review report testing",
        "ReviewApp",
        true,
        &cwd,
        &config,
        &profile,
        None,
    )
    .await;

    let log_dir = PathBuf::from("daily_logs");
    let report = run_review(&log_dir, None, None).expect("Review run succeeds");

    assert!(report.total_runs > 0);
    // Stages like input and context should be recorded
    assert!(report.stage_metrics.contains_key("input"));
}

#[tokio::test]
async fn test_replay_on_real_log() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    // Create a run to generate a fresh log
    let res = execute_enhance_pipeline(
        "Add unit test for replay functionality",
        "Terminal",
        true,
        &cwd,
        &config,
        &profile,
        None,
    )
    .await;

    let log_file = res.log_path.expect("Log path exists");

    // Replay the run
    let replay_result = run_replay(&log_file, &config, &profile).await;
    assert!(
        replay_result.is_ok(),
        "Replay must succeed on valid log file"
    );
}

#[tokio::test]
async fn test_golden_prompt_eval_dataset() {
    let config = test_config();
    let profile = test_profile();
    let cwd = PathBuf::from(".");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let eval_file = if PathBuf::from("tests/eval/golden_prompts.json").exists() {
        PathBuf::from("tests/eval/golden_prompts.json")
    } else {
        manifest_dir.join("../../tests/eval/golden_prompts.json")
    };
    assert!(
        eval_file.exists(),
        "Golden prompt evaluation file must exist at {}",
        eval_file.display()
    );

    let content = fs::read_to_string(eval_file).expect("Read eval json");
    let prompts: Vec<serde_json::Value> = serde_json::from_str(&content).expect("Parse eval json");

    for item in prompts {
        let id = item["id"].as_str().unwrap();
        let prompt = item["prompt"].as_str().unwrap();
        let should_idempotent = item["should_be_idempotent"].as_bool().unwrap();

        let res = execute_enhance_pipeline(
            prompt,
            "Antigravity",
            true, // use no_llm for fast, deterministic regression testing
            &cwd,
            &config,
            &profile,
            None,
        )
        .await;

        if should_idempotent {
            assert_eq!(res.exit_code, ExitCode::Ok);
            assert_eq!(res.stderr_status, "unchanged");
        } else {
            assert_eq!(res.exit_code, ExitCode::Ok);
            assert!(
                res.stdout_content.contains("<task>"),
                "Prompt {} must render <task>",
                id
            );
            assert!(
                res.stdout_content.contains("<rules>"),
                "Prompt {} must render <rules>",
                id
            );
            assert!(
                res.stdout_content.contains("<report>"),
                "Prompt {} must render <report>",
                id
            );
        }
    }
}
