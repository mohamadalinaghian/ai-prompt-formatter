use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use pfmt_core::{
    redact_secrets, slugify_topic, ExecutionLog, GateLogRecord, StageLogRecord, VerifierLogRecord,
    VersionLogRecord, WriterOutput,
};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

enum LogCommand {
    RecordStage(StageLogRecord),
    SetGateDecision(GateLogRecord),
    SetPayloadSent(String),
    SetWriterResult(Option<String>, Option<WriterOutput>),
    SetVerifierResult(VerifierLogRecord),
    SetAppContext {
        project_dir: Option<String>,
        session_id: Option<String>,
        sources: Vec<String>,
        context_layers: Vec<String>,
    },
    Finalize {
        output_xml: String,
        banner: Option<String>,
        degraded: bool,
        exit_code: i32,
        reply: tokio::sync::oneshot::Sender<()>,
    },
}

pub struct IncrementalLogger {
    log_path: PathBuf,
    base_dir: PathBuf,
    app: String,
    tx: tokio::sync::mpsc::UnboundedSender<LogCommand>,
    worker_handle: Option<tokio::task::JoinHandle<()>>,
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

fn flush_log_sync(log_path: &Path, log: &ExecutionLog) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(log)?;
    let mut file = File::create(log_path)?;
    file.write_all(json.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Returns `<base_dir>/YYYY-MM-DD.log` for the current local date.
pub fn daily_log_file_path(base_dir: &Path) -> PathBuf {
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    base_dir.join(format!("{}.log", date_str))
}

/// Appends one session entry to `<base_dir>/YYYY-MM-DD.log`.
/// Creates `base_dir` if missing. Returns the daily file path.
pub fn append_session_entry(
    base_dir: &Path,
    app: &str,
    session_id: Option<&str>,
    output_xml: &str,
    exit_code: i32,
    degraded: bool,
) -> Result<PathBuf, std::io::Error> {
    fs::create_dir_all(base_dir)?;
    let path = daily_log_file_path(base_dir);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    let ts = Local::now().to_rfc3339();
    let session = session_id.unwrap_or("-");
    let entry = format!(
        "[{}] app={} session={} exit={} degraded={}\n{}\n---\n",
        ts, app, session, exit_code, degraded, output_xml
    );
    file.write_all(entry.as_bytes())?;
    file.flush()?;
    Ok(path)
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
            project_dir: None,
            probe_sources: Vec::new(),
            context_layers: Vec::new(),
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

        flush_log_sync(&log_path, &initial_log)?;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<LogCommand>();
        let task_path = log_path.clone();
        let task_base_dir = base_dir.to_path_buf();

        let worker_handle = if tokio::runtime::Handle::try_current().is_ok() {
            Some(tokio::spawn(async move {
                let mut current_log = initial_log;
                while let Some(cmd) = rx.recv().await {
                    match cmd {
                        LogCommand::RecordStage(record) => {
                            current_log.stages.push(record);
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                        }
                        LogCommand::SetGateDecision(gate) => {
                            current_log.gate = Some(gate);
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                        }
                        LogCommand::SetPayloadSent(payload) => {
                            current_log.payload_sent = Some(payload);
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                        }
                        LogCommand::SetWriterResult(raw, validated) => {
                            current_log.writer_raw = raw;
                            current_log.writer_validated = validated;
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                        }
                        LogCommand::SetVerifierResult(verifier) => {
                            current_log.verifier = Some(verifier);
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                        }
                        LogCommand::SetAppContext {
                            project_dir,
                            session_id,
                            sources,
                            context_layers,
                        } => {
                            current_log.project_dir = project_dir;
                            current_log.session_id = session_id;
                            current_log.probe_sources = sources;
                            current_log.context_layers = context_layers;
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                        }
                        LogCommand::Finalize {
                            output_xml,
                            banner,
                            degraded,
                            exit_code,
                            reply,
                        } => {
                            current_log.output_xml = Some(output_xml.clone());
                            current_log.banner = banner;
                            current_log.degraded = degraded;
                            current_log.exit_code = exit_code;
                            if let Err(e) = flush_log_sync(&task_path, &current_log) {
                                eprintln!("pfmt: failed to flush log: {}", e);
                            }
                            if let Err(e) = append_session_entry(
                                &task_base_dir,
                                &current_log.app,
                                current_log.session_id.as_deref(),
                                &output_xml,
                                exit_code,
                                degraded,
                            ) {
                                eprintln!("pfmt: failed to append daily log: {}", e);
                            }
                            let _ = reply.send(());
                            break;
                        }
                    }
                }
            }))
        } else {
            None
        };

        Ok(Self {
            log_path,
            base_dir: base_dir.to_path_buf(),
            app: app.to_string(),
            tx,
            worker_handle,
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn record_stage(&mut self, record: StageLogRecord) {
        let _ = self.tx.send(LogCommand::RecordStage(record));
    }

    pub fn set_gate_decision(&mut self, gate: GateLogRecord) {
        let _ = self.tx.send(LogCommand::SetGateDecision(gate));
    }

    pub fn set_payload_sent(&mut self, payload: &str) {
        let _ = self
            .tx
            .send(LogCommand::SetPayloadSent(redact_secrets(payload)));
    }

    pub fn set_writer_result(&mut self, raw: Option<String>, validated: Option<WriterOutput>) {
        let _ = self.tx.send(LogCommand::SetWriterResult(raw, validated));
    }

    pub fn set_verifier_result(&mut self, verifier: VerifierLogRecord) {
        let _ = self.tx.send(LogCommand::SetVerifierResult(verifier));
    }

    pub fn set_app_context(
        &mut self,
        project_dir: Option<&str>,
        session_id: Option<&str>,
        sources: &[String],
        context_layers: &[String],
    ) {
        let _ = self.tx.send(LogCommand::SetAppContext {
            project_dir: project_dir.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
            sources: sources.to_vec(),
            context_layers: context_layers.to_vec(),
        });
    }

    pub async fn finalize(
        &mut self,
        output_xml: &str,
        banner: Option<String>,
        degraded: bool,
        exit_code: i32,
    ) {
        if self.worker_handle.is_none() {
            // No async worker (no Tokio runtime at init): update JSON + daily log synchronously.
            let _ = banner;
            if let Err(e) = append_session_entry(
                &self.base_dir,
                &self.app,
                None,
                output_xml,
                exit_code,
                degraded,
            ) {
                eprintln!("pfmt: failed to append daily log: {}", e);
            }
            return;
        }
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let _ = self.tx.send(LogCommand::Finalize {
            output_xml: output_xml.to_string(),
            banner,
            degraded,
            exit_code,
            reply: reply_tx,
        });
        let _ = reply_rx.await;
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.await;
        }
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
