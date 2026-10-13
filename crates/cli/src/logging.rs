use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use pfmt_core::{
    redact_secrets, slugify_topic, ExecutionLog, GateLogRecord, StageLogRecord, VerifierLogRecord,
    VersionLogRecord, WriterOutput,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct IncrementalLogger {
    log_path: PathBuf,
    current_log: ExecutionLog,
}

/// Resolves the default log directory:
/// If `./daily_logs` exists or current directory is a git repo/workspace, uses `./daily_logs`.
/// Otherwise falls back to `~/.config/pfmt/daily_logs`.
pub fn resolve_default_log_dir() -> PathBuf {
    let local = PathBuf::from("daily_logs");
    if local.exists() {
        return local;
    }
    // Check if CWD has Cargo.toml, .git, or package.json
    if Path::new(".git").exists()
        || Path::new("Cargo.toml").exists()
        || Path::new("package.json").exists()
    {
        return local;
    }
    // Global fallback
    dirs::config_dir()
        .map(|d| d.join("pfmt").join("daily_logs"))
        .unwrap_or(local)
}

impl IncrementalLogger {
    /// Initializes an incremental log file at the start of a run.
    /// File path: `<base_dir>/YYYY-MM-DD/<app>_<session>_<topic>_<HHMMSS>.json`
    pub fn init(
        base_dir: &Path,
        app: &str,
        session_id: Option<&str>,
        raw_input: &str,
        retention_days: u64,
    ) -> Result<Self, std::io::Error> {
        // 1. Clean up old logs per retention policy
        clean_expired_logs(base_dir, retention_days);

        let now = Local::now();
        let date_str = now.format("%Y-%m-%d").to_string();
        let time_str = now.format("%H%M%S").to_string();

        let day_dir = base_dir.join(&date_str);
        fs::create_dir_all(&day_dir)?;

        let topic = slugify_topic(raw_input);
        let session_part = session_id.unwrap_or("");
        let file_name = if session_part.is_empty() {
            format!("{}_{}_{}.json", app, topic, time_str)
        } else {
            format!("{}_{}_{}_{}.json", app, session_part, topic, time_str)
        };

        let log_path = day_dir.join(file_name);

        let redacted_input = redact_secrets(raw_input.trim());

        let initial_log = ExecutionLog {
            schema_version: "1.0".to_string(),
            ts: now.to_rfc3339(),
            app: app.to_string(),
            agent: app.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            topic,
            input: redacted_input,
            stages: Vec::new(),
            gate: None,
            payload_sent: None,
            writer_raw: None,
            writer_validated: None,
            verifier: None,
            output_xml: None,
            banner: None,
            degraded: false,
            exit_code: 1, // Pending / default to aborted if crashed
            versions: VersionLogRecord {
                pfmt: env!("CARGO_PKG_VERSION").to_string(),
                meta_prompt: "v1".to_string(),
                schema: "v1".to_string(),
                config_hash: "default".to_string(),
            },
        };

        let logger = Self {
            log_path,
            current_log: initial_log,
        };

        logger.flush()?;
        Ok(logger)
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn record_stage(&mut self, record: StageLogRecord) {
        self.current_log.stages.push(record);
        let _ = self.flush();
    }

    pub fn set_gate_decision(&mut self, gate: GateLogRecord) {
        self.current_log.gate = Some(gate);
        let _ = self.flush();
    }

    pub fn set_payload_sent(&mut self, payload: &str) {
        self.current_log.payload_sent = Some(redact_secrets(payload));
        let _ = self.flush();
    }

    pub fn set_writer_result(&mut self, raw: Option<String>, validated: Option<WriterOutput>) {
        self.current_log.writer_raw = raw;
        self.current_log.writer_validated = validated;
        let _ = self.flush();
    }

    pub fn set_verifier_result(&mut self, verifier: VerifierLogRecord) {
        self.current_log.verifier = Some(verifier);
        let _ = self.flush();
    }

    pub fn set_app_context(
        &mut self,
        project_dir: Option<&str>,
        session_id: Option<&str>,
        sources: &[String],
        context_layers: &[String],
    ) {
        self.current_log.project_dir = project_dir.map(|s| s.to_string());
        self.current_log.session_id = session_id.map(|s| s.to_string());
        self.current_log.probe_sources = sources.to_vec();
        self.current_log.context_layers = context_layers.to_vec();
        let _ = self.flush();
    }

    pub fn set_app_context(
        &mut self,
        project_dir: Option<&str>,
        session_id: Option<&str>,
        sources: &[String],
        context_layers: &[String],
    ) {
        self.current_log.project_dir = project_dir.map(|s| s.to_string());
        self.current_log.session_id = session_id.map(|s| s.to_string());
        self.current_log.probe_sources = sources.to_vec();
        self.current_log.context_layers = context_layers.to_vec();
        let _ = self.flush();
    }

    pub fn finalize(
        &mut self,
        output_xml: &str,
        banner: Option<String>,
        degraded: bool,
        exit_code: i32,
    ) {
        self.current_log.output_xml = Some(output_xml.to_string());
        self.current_log.banner = banner;
        self.current_log.degraded = degraded;
        self.current_log.exit_code = exit_code;
        let _ = self.flush();
    }

    fn flush(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self.current_log)?;
        let mut file = File::create(&self.log_path)?;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

/// Automatically cleans directories older than `retention_days` under base_dir
pub fn clean_expired_logs(base_dir: &Path, retention_days: u64) {
    if !base_dir.exists() || retention_days == 0 {
        return;
    }

    let today = Local::now().date_naive();
    let cutoff = today - ChronoDuration::days(retention_days as i64);

    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(dir_date) = NaiveDate::parse_from_str(dir_name, "%Y-%m-%d") {
                        if dir_date < cutoff {
                            let _ = fs::remove_dir_all(&path);
                        }
                    }
                }
            }
        }
    }
}
