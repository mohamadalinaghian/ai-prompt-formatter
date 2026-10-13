use pfmt_cli::pipeline::execute_enhance_pipeline;
use pfmt_core::{Config, ExitCode, Profile};
use std::path::PathBuf;
use std::time::Instant;

#[tokio::test]
async fn test_pipeline_offline_latency_and_async_logging() {
    let config = Config::load();
    let profile = Profile::load_default();
    let cwd = PathBuf::from(".");

    let prompt = "Explain how the pipeline latency test works";

    // Fast path: generic app with no large disk scan
    let start_fast = Instant::now();
    let res_fast =
        execute_enhance_pipeline(prompt, "test", true, &cwd, &config, &profile, None).await;
    let elapsed_fast = start_fast.elapsed();
    println!(
        "Offline pipeline (generic app) completed in {:?}",
        elapsed_fast
    );
    assert_eq!(res_fast.exit_code, ExitCode::Ok);
    assert!(
        elapsed_fast.as_millis() < 150,
        "Generic offline pipeline took too long: {:?}",
        elapsed_fast
    );

    // Full path with live adapter probe (Antigravity scans live transcript files)
    let start = Instant::now();
    let res =
        execute_enhance_pipeline(prompt, "Antigravity", true, &cwd, &config, &profile, None).await;
    let elapsed = start.elapsed();
    println!("Offline pipeline (Antigravity) completed in {:?}", elapsed);
    assert_eq!(res.exit_code, ExitCode::Ok);
    assert!(
        elapsed.as_millis() < 1500,
        "Antigravity offline pipeline took too long: {:?}",
        elapsed
    );

    // 3. Log file verification (channel-based actor must have flushed and finalized)
    assert!(res.log_path.is_some(), "Log path must be present");
    let log_path = res.log_path.unwrap();
    assert!(log_path.exists(), "Log file must exist on disk");

    let log_content = std::fs::read_to_string(&log_path).expect("Read log file");
    assert!(log_content.contains(r#""schema_version": "1.0""#));
    assert!(log_content.contains(r#""exit_code": 0"#));
    assert!(log_content.contains(r#""output_xml": "#));
}

#[tokio::test]
async fn test_pipeline_concurrent_stages_correctness() {
    let config = Config::load();
    let profile = Profile::load_default();
    let cwd = PathBuf::from(".");

    let prompt = "Fix bug in logging actor channel";
    let res =
        execute_enhance_pipeline(prompt, "OpenCode", true, &cwd, &config, &profile, None).await;

    assert_eq!(res.exit_code, ExitCode::Ok);
    assert!(res
        .stdout_content
        .contains("<task>\nFix bug in logging actor channel\n</task>"));
    assert!(res.stdout_content.contains("<rules>"));
    assert!(res.stdout_content.contains("<report>"));
}
