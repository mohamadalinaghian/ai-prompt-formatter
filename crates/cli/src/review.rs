use chrono::NaiveDate;
use pfmt_core::{ExecutionLog, StageStatus};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct StageMetrics {
    pub total_calls: usize,
    pub failures: usize,
    pub latencies_ms: Vec<u64>,
}

#[derive(Default)]
pub struct ReviewReport {
    pub total_runs: usize,
    pub exit_ok: usize,
    pub exit_degraded: usize,
    pub exit_abort: usize,
    pub stage_metrics: HashMap<String, StageMetrics>,
    pub verifier_rejections: usize,
    pub verifier_cleaned: usize,
    pub worst_cases: Vec<(u64, String, String, PathBuf)>, // (latency_ms, topic, app, file)
}

pub fn run_review(
    log_dir: &Path,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<ReviewReport, std::io::Error> {
    let from_date = from.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let to_date = to.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let mut report = ReviewReport::default();

    if !log_dir.exists() {
        return Ok(report);
    }

    let mut log_files = Vec::new();

    // Traverse daily_logs/YYYY-MM-DD/*.json
    if let Ok(day_entries) = fs::read_dir(log_dir) {
        for day in day_entries.flatten() {
            let day_path = day.path();
            if day_path.is_dir() {
                if let Some(day_str) = day_path.file_name().and_then(|s| s.to_str()) {
                    if let Ok(date) = NaiveDate::parse_from_str(day_str, "%Y-%m-%d") {
                        if let Some(f) = from_date {
                            if date < f {
                                continue;
                            }
                        }
                        if let Some(t) = to_date {
                            if date > t {
                                continue;
                            }
                        }
                    }
                }

                if let Ok(file_entries) = fs::read_dir(&day_path) {
                    for f in file_entries.flatten() {
                        let path = f.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json") {
                            log_files.push(path);
                        }
                    }
                }
            }
        }
    }

    for path in log_files {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(log) = serde_json::from_str::<ExecutionLog>(&content) {
                process_log(&log, &path, &mut report);
            }
        }
    }

    // Sort worst cases by latency descending
    report.worst_cases.sort_by_key(|a| std::cmp::Reverse(a.0));
    report.worst_cases.truncate(3);

    Ok(report)
}

fn process_log(log: &ExecutionLog, path: &Path, report: &mut ReviewReport) {
    report.total_runs += 1;
    match log.exit_code {
        0 => report.exit_ok += 1,
        10 => report.exit_degraded += 1,
        _ => report.exit_abort += 1,
    }

    let mut total_run_latency = 0;

    for stage in &log.stages {
        total_run_latency += stage.ms;
        let metrics = report.stage_metrics.entry(stage.id.clone()).or_default();
        metrics.total_calls += 1;
        if stage.status == StageStatus::Failed {
            metrics.failures += 1;
        }
        metrics.latencies_ms.push(stage.ms);
    }

    if let Some(ref v) = log.verifier {
        if v.verdict == "rejected" {
            report.verifier_rejections += 1;
        } else if v.verdict == "cleaned" || !v.dropped_files.is_empty() {
            report.verifier_cleaned += 1;
        }
    }

    report.worst_cases.push((
        total_run_latency,
        log.topic.clone(),
        log.app.clone(),
        path.to_path_buf(),
    ));
}

pub fn print_review_report(report: &ReviewReport, from: Option<&str>, to: Option<&str>) {
    println!("\n📊 pfmt Review Report");
    let range = match (from, to) {
        (Some(f), Some(t)) => format!("{} to {}", f, t),
        (Some(f), None) => format!("from {}", f),
        (None, Some(t)) => format!("up to {}", t),
        (None, None) => "all available records".to_string(),
    };
    println!(
        "Period: {}\n───────────────────────────────────────────",
        range
    );

    println!("Total Runs:       {}", report.total_runs);
    println!(
        "  - Succeeded:    {} ({:.1}%)",
        report.exit_ok,
        pct(report.exit_ok, report.total_runs)
    );
    println!(
        "  - Degraded:     {} ({:.1}%)",
        report.exit_degraded,
        pct(report.exit_degraded, report.total_runs)
    );
    println!(
        "  - Aborted:      {} ({:.1}%)",
        report.exit_abort,
        pct(report.exit_abort, report.total_runs)
    );

    println!("\nStage Performance & Reliability:");
    println!(
        "{:<12} | {:<7} | {:<12} | {:<8} | {:<8}",
        "Stage", "Calls", "Failure Rate", "Avg (ms)", "p95 (ms)"
    );
    println!(
        "{:-<12}-+-{:-<7}-+-{:-<12}-+-{:-<8}-+-{:-<8}",
        "", "", "", "", ""
    );

    let mut stage_keys: Vec<_> = report.stage_metrics.keys().cloned().collect();
    stage_keys.sort();

    for key in stage_keys {
        if let Some(m) = report.stage_metrics.get(&key) {
            let fail_pct = if m.total_calls > 0 {
                (m.failures as f64 / m.total_calls as f64) * 100.0
            } else {
                0.0
            };
            let (avg, p95) = calc_latencies(&m.latencies_ms);
            println!(
                "{:<12} | {:<7} | {:<5.1}% ({}/{:<2}) | {:<8} | {:<8}",
                key, m.total_calls, fail_pct, m.failures, m.total_calls, avg, p95
            );
        }
    }

    println!("\nVerifier Statistics:");
    println!("  - Total Rejections: {}", report.verifier_rejections);
    println!(
        "  - Hallucinated Files Cleaned: {}",
        report.verifier_cleaned
    );

    if !report.worst_cases.is_empty() {
        println!("\nWorst Cases (Slowest Runs):");
        for (i, (ms, topic, app, path)) in report.worst_cases.iter().enumerate() {
            println!(
                "  {}. {}ms | app: {} | topic: {} | log: {}",
                i + 1,
                ms,
                app,
                topic,
                path.display()
            );
        }
    }
    println!("───────────────────────────────────────────\n");
}

fn pct(n: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (n as f64 / total as f64) * 100.0
    }
}

fn calc_latencies(latencies: &[u64]) -> (u64, u64) {
    if latencies.is_empty() {
        return (0, 0);
    }
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();

    let sum: u64 = sorted.iter().sum();
    let avg = sum / sorted.len() as u64;

    let p95_idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
    let p95 = sorted[p95_idx.min(sorted.len() - 1)];

    (avg, p95)
}
