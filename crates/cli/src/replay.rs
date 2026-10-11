use crate::pipeline::execute_enhance_pipeline;
use pfmt_core::{Config, ExecutionLog, Profile};
use std::fs;
use std::path::Path;
use std::time::Instant;

pub async fn run_replay(log_path: &Path, config: &Config, profile: &Profile) -> Result<(), String> {
    if !log_path.exists() {
        return Err(format!("Log file not found: {}", log_path.display()));
    }

    let content =
        fs::read_to_string(log_path).map_err(|e| format!("Failed to read log file: {}", e))?;

    let old_log: ExecutionLog = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse log file JSON: {}", e))?;

    println!("\n🔄 Replaying logged prompt execution");
    println!("Log source:  {}", log_path.display());
    println!("Original ts: {}", old_log.ts);
    println!("App / Agent: {}", old_log.app);
    println!("Topic slug:  {}", old_log.topic);
    println!("───────────────────────────────────────────");

    println!("\n[Original Input]");
    println!("{}\n", old_log.input);

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let start = Instant::now();
    let new_res = execute_enhance_pipeline(
        &old_log.input,
        &old_log.app,
        false,
        &cwd,
        config,
        profile,
        None,
    )
    .await;
    let new_latency_ms = start.elapsed().as_millis();

    println!("───────────────────────────────────────────");
    println!("📊 Comparison Summary");
    println!("───────────────────────────────────────────");
    println!(
        "Exit Code:    Before: {:<5} | After: {:<5}",
        old_log.exit_code, new_res.exit_code as i32
    );
    println!(
        "Degraded:     Before: {:<5} | After: {:<5}",
        old_log.degraded,
        !new_res.failed_stages.is_empty()
    );
    let old_latency: u64 = old_log.stages.iter().map(|s| s.ms).sum();
    println!(
        "Latency:      Before: {}ms | After: {}ms",
        old_latency, new_latency_ms
    );
    println!("Status Line:  {}", new_res.stderr_status);

    println!("\n[Replayed Output XML]");
    println!("{}", new_res.stdout_content);
    println!("───────────────────────────────────────────\n");

    Ok(())
}
