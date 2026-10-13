use crate::logging::IncrementalLogger;
use pfmt_context::{
    build_writer_payload, collect_repo_summary, extract_candidate_files, PayloadInput, SessionMode,
};
use pfmt_core::{
    is_idempotent, redact_secrets, render_with_template, resolve_api_key, AppContext, Config,
    Degradation, ExitCode, GateLogRecord, OnFail, Profile, ReportTemplate, StageLogRecord,
    StageStatus, VerifierLogRecord, WriterOutput,
};
use pfmt_llm::{
    ConfidenceChecker, ConfidenceItem, Decider, GateDecisions, GroqClient, GroqJsonDecider,
    ItemConfidence, JevConfidence, JevDecider, PromptLoader, Verifier,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineResult {
    pub exit_code: ExitCode,
    pub stdout_content: String,
    pub stderr_status: String,
    pub failed_stages: Vec<String>,
    pub log_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct StageFailureHook {
    pub fail_input: bool,
    pub fail_redact: bool,
    pub fail_context: bool,
    pub fail_history: bool,
    pub fail_gate: bool,
    pub fail_writer: bool,
    pub fail_verify: bool,
    pub fail_confidence: bool,
    pub fail_render: bool,
    pub force_timeout: bool,
}

pub async fn execute_enhance_pipeline(
    raw_input: &str,
    app: &str,
    force_no_llm: bool,
    cwd: &Path,
    config: &Config,
    profile: &Profile,
    hooks: Option<&StageFailureHook>,
) -> PipelineResult {
    let start_time = Instant::now();
    let hooks = hooks.cloned().unwrap_or_default();

    let total_budget = if hooks.force_timeout {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(config.budget.total_ms)
    };

    let mut failed_stages: Vec<String> = Vec::new();

    // ──────────────────────────────────────────
    // Incremental Logger initialization (at start)
    // ──────────────────────────────────────────
    let log_dir = crate::logging::resolve_default_log_dir();
    let mut logger = if config.is_stage_enabled(app, "log") {
        IncrementalLogger::init(&log_dir, app, None, raw_input, config.logs.retention_days).ok()
    } else {
        None
    };

    // ──────────────────────────────────────────
    // Stage 1: Input stage
    // ──────────────────────────────────────────
    let stage_start = Instant::now();
    let trimmed_input = raw_input.trim();
    if trimmed_input.is_empty() || hooks.fail_input {
        let elapsed = stage_start.elapsed().as_millis() as u64;
        if let Some(ref mut l) = logger {
            l.record_stage(
                StageLogRecord::new("input", StageStatus::Failed, elapsed)
                    .with_error("empty input"),
            );
            l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
        }

        let on_fail = config.stage_on_fail(app, "input");
        if on_fail == OnFail::Abort {
            return PipelineResult {
                exit_code: ExitCode::Abort,
                stdout_content: String::new(),
                stderr_status: "empty input: nothing to do".to_string(),
                failed_stages: vec!["input (empty input or failed read)".to_string()],
                log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
            };
        } else {
            failed_stages.push("input (empty input)".to_string());
        }
    } else if let Some(ref mut l) = logger {
        l.record_stage(StageLogRecord::new(
            "input",
            StageStatus::Ok,
            stage_start.elapsed().as_millis() as u64,
        ));
    }

    // ──────────────────────────────────────────
    // Stage 2: Idempotence check
    // ──────────────────────────────────────────
    if is_idempotent(trimmed_input) {
        if let Some(ref mut l) = logger {
            l.finalize(raw_input, None, false, ExitCode::Ok.as_i32())
                .await;
        }
        return PipelineResult {
            exit_code: ExitCode::Ok,
            stdout_content: raw_input.to_string(),
            stderr_status: "unchanged".to_string(),
            failed_stages: Vec::new(),
            log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
        };
    }

    // ──────────────────────────────────────────
    // Stage 3: Redact stage
    // ──────────────────────────────────────────
    let stage_start = Instant::now();
    let mut processed_prompt = trimmed_input.to_string();
    let mut skip_network = force_no_llm;

    if config.is_stage_enabled(app, "redact") {
        if hooks.fail_redact {
            let elapsed = stage_start.elapsed().as_millis() as u64;
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new("redact", StageStatus::Failed, elapsed)
                        .with_error("injected failure"),
                );
            }

            let on_fail = config.stage_on_fail(app, "redact");
            if on_fail == OnFail::Abort {
                if let Some(ref mut l) = logger {
                    l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                }
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: redaction stage failed".to_string(),
                    failed_stages: vec!["redact (injected failure)".to_string()],
                    log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                };
            }
            failed_stages.push("redact (redaction error: network skipped)".to_string());
            skip_network = true;
        } else {
            processed_prompt = redact_secrets(&processed_prompt);
            let elapsed = stage_start.elapsed().as_millis() as u64;
            if let Some(ref mut l) = logger {
                l.record_stage(StageLogRecord::new("redact", StageStatus::Ok, elapsed));
            }
        }
    } else if let Some(ref mut l) = logger {
        l.record_stage(StageLogRecord::new("redact", StageStatus::Disabled, 0));
    }

    // ──────────────────────────────────────────
    // Stage 4: Probe stage
    // ──────────────────────────────────────────
    let probe_start = Instant::now();
    let probe_enabled = config.is_stage_enabled(app, "probe");
    let app_context = if probe_enabled {
        let ctx = crate::probe::resolve_probe(app, cwd);
        let elapsed = probe_start.elapsed().as_millis() as u64;
        if let Some(ref mut l) = logger {
            l.record_stage(StageLogRecord::new("probe", StageStatus::Ok, elapsed));
        }
        ctx
    } else {
        if let Some(ref mut l) = logger {
            l.record_stage(StageLogRecord::new("probe", StageStatus::Disabled, 0));
        }
        AppContext {
            app_name: app.to_string(),
            project_dir: None,
            session_id: None,
            sources: Vec::new(),
        }
    };

    // Log AppContext (probe result) and which context layers will be used
    if let Some(ref mut l) = logger {
        let mut layers_used: Vec<String> = Vec::new();
        if config.is_stage_enabled(app, "context") {
            layers_used.push("context".to_string());
        }
        if config.is_stage_enabled(app, "history") {
            layers_used.push("history".to_string());
        }
        l.set_app_context(
            app_context
                .project_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .as_deref(),
            app_context.session_id.as_deref(),
            &app_context.sources,
            &layers_used,
        );
    }

    // ──────────────────────────────────────────
    // Check global time budget
    // ──────────────────────────────────────────
    if start_time.elapsed() >= total_budget {
        failed_stages.push("timeout (total time budget exceeded)".to_string());
        skip_network = true;
    }

    // ──────────────────────────────────────────
    // Stage 4 & 5 & Template: Concurrent Context, History (Analyzer cache), and Template Selection
    // ──────────────────────────────────────────
    let is_context_enabled = config.is_stage_enabled(app, "context");
    let is_history_enabled = config.is_stage_enabled(app, "history");
    let fail_context = hooks.fail_context;
    let fail_history = hooks.fail_history;
    let target_repo = app_context.project_dir.as_deref().unwrap_or(cwd);
    let target_repo_buf = target_repo.to_path_buf();
    let target_repo_for_context = target_repo_buf.clone();
    let target_repo_for_history = target_repo_buf;
    let prompt_for_context = processed_prompt.clone();
    let app_for_history = app.to_string();
    let session_id_opt = app_context.session_id.clone();

    let context_fut = async move {
        if !is_context_enabled {
            return (StageStatus::Disabled, None, Vec::new(), 0u64, None);
        }
        if fail_context {
            return (
                StageStatus::Failed,
                None,
                Vec::new(),
                0u64,
                Some("injected failure".to_string()),
            );
        }
        let start = Instant::now();
        let summary = collect_repo_summary(&target_repo_for_context);
        let candidates = extract_candidate_files(&prompt_for_context, &target_repo_for_context);
        let elapsed = start.elapsed().as_millis() as u64;
        (StageStatus::Ok, Some(summary), candidates, elapsed, None)
    };

    let history_fut = async move {
        if !is_history_enabled {
            return (StageStatus::Disabled, None, None, 0u64, None);
        }
        if fail_history {
            return (
                StageStatus::Failed,
                None,
                None,
                0u64,
                Some("injected failure".to_string()),
            );
        }
        let start = Instant::now();
        let mut cached_history = None;
        let mut history_src = None;
        if let Ok(db) = pfmt_history::HistoryDb::open_default() {
            let path_str = target_repo_for_history.to_str();
            if let Some(ref sid) = session_id_opt {
                cached_history = db.get_session_summary(&app_for_history, sid).ok().flatten();
                if cached_history.is_some() {
                    history_src = Some(sid.clone());
                }
            }
            if cached_history.is_none() {
                cached_history = db
                    .get_latest_summary(&app_for_history, path_str)
                    .ok()
                    .flatten();
                if cached_history.is_some() {
                    history_src = Some("project-fallback".to_string());
                }
            }
        }
        let elapsed = start.elapsed().as_millis() as u64;
        (StageStatus::Ok, cached_history, history_src, elapsed, None)
    };

    let template_fut = async { ReportTemplate::resolve(config, profile, profile.kind.as_deref()) };

    let (
        (context_status, repo_summary, candidate_files, context_elapsed, _context_err),
        (history_status, cached_history, history_src, history_elapsed, _history_err),
        _base_template,
    ) = tokio::join!(context_fut, history_fut, template_fut);

    // Process Context Stage outcome
    match context_status {
        StageStatus::Failed => {
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new("context", StageStatus::Failed, context_elapsed)
                        .with_error("injected failure"),
                );
            }

            let on_fail = config.stage_on_fail(app, "context");
            if on_fail == OnFail::Abort {
                if let Some(ref mut l) = logger {
                    l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                }
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: context collection failed".to_string(),
                    failed_stages: vec!["context (injected failure)".to_string()],
                    log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                };
            }
            failed_stages.push("context (repo inspection failed)".to_string());
        }
        StageStatus::Ok => {
            if let Some(ref mut l) = logger {
                l.record_stage(StageLogRecord::new(
                    "context",
                    StageStatus::Ok,
                    context_elapsed,
                ));
            }
        }
        StageStatus::Disabled => {
            if let Some(ref mut l) = logger {
                l.record_stage(StageLogRecord::new("context", StageStatus::Disabled, 0));
            }
        }
        _ => {}
    }

    // Process History Stage outcome
    match history_status {
        StageStatus::Failed => {
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new("history", StageStatus::Failed, history_elapsed)
                        .with_error("injected failure"),
                );
            }

            let on_fail = config.stage_on_fail(app, "history");
            if on_fail == OnFail::Abort {
                if let Some(ref mut l) = logger {
                    l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                }
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: history stage failed".to_string(),
                    failed_stages: vec!["history (injected failure)".to_string()],
                    log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                };
            }
            failed_stages.push("history (adapter failed / db locked)".to_string());
        }
        StageStatus::Ok => {
            if let Some(ref mut l) = logger {
                l.record_stage(StageLogRecord::new(
                    "history",
                    StageStatus::Ok,
                    history_elapsed,
                ));
            }
        }
        StageStatus::Disabled => {
            if let Some(ref mut l) = logger {
                l.record_stage(StageLogRecord::new("history", StageStatus::Disabled, 0));
            }
        }
        _ => {}
    }

    // ──────────────────────────────────────────
    // Stage 6: Gate stage (Jev / Groq decider)
    // ──────────────────────────────────────────
    let stage_start = Instant::now();
    let is_continue = cached_history.is_some();
    let mode = if is_continue {
        SessionMode::Continue
    } else {
        SessionMode::Fresh
    };

    let mut filtered_candidates = candidate_files.clone();
    let mut _needs_history = is_continue;
    let mut session_kind = pfmt_llm::classify_session_kind_heuristic(&processed_prompt).to_string();

    if !skip_network && config.is_stage_enabled(app, "gate") {
        let remaining_budget = total_budget.saturating_sub(start_time.elapsed());
        if remaining_budget.is_zero() {
            failed_stages.push("gate (time budget exceeded)".to_string());
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new(
                        "gate",
                        StageStatus::Failed,
                        stage_start.elapsed().as_millis() as u64,
                    )
                    .with_error("time budget exceeded"),
                );
            }
        } else if hooks.fail_gate {
            let elapsed = stage_start.elapsed().as_millis() as u64;
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new("gate", StageStatus::Failed, elapsed)
                        .with_error("injected failure"),
                );
            }

            let on_fail = config.stage_on_fail(app, "gate");
            if on_fail == OnFail::Abort {
                if let Some(ref mut l) = logger {
                    l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                }
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: gate stage failed".to_string(),
                    failed_stages: vec!["gate (injected failure)".to_string()],
                    log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                };
            }
            failed_stages.push("gate (decider failure: heuristic fallback)".to_string());
            filtered_candidates = candidate_files.into_iter().take(10).collect();
        } else {
            let gate_timeout_ms = remaining_budget.as_millis().min(2500) as u64;
            let decider_result = run_decider(
                &processed_prompt,
                &candidate_files,
                is_continue,
                config,
                gate_timeout_ms,
            )
            .await;
            let elapsed = stage_start.elapsed().as_millis() as u64;
            match decider_result {
                Ok(decisions) => {
                    _needs_history = decisions.needs_history;
                    filtered_candidates = decisions.filtered_candidates.clone();
                    session_kind = decisions.session_kind.clone();
                    if let Some(ref mut l) = logger {
                        l.set_gate_decision(GateLogRecord {
                            needs_history: decisions.needs_history,
                            already_structured: decisions.already_structured,
                            injection_risk: decisions.injection_risk,
                            extra_rules_requested: decisions.extra_rules_requested,
                            extra_report_requested: decisions.extra_report_requested,
                            session_kind: Some(session_kind.clone()),
                            allowed_candidates: decisions.filtered_candidates,
                            probabilities: decisions.probabilities,
                        });
                        l.record_stage(StageLogRecord::new("gate", StageStatus::Ok, elapsed));
                    }
                }
                Err(e) => {
                    if let Some(ref mut l) = logger {
                        l.record_stage(
                            StageLogRecord::new("gate", StageStatus::Failed, elapsed)
                                .with_error(&e),
                        );
                    }
                    let on_fail = config.stage_on_fail(app, "gate");
                    if on_fail == OnFail::Abort {
                        if let Some(ref mut l) = logger {
                            l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                        }
                        return PipelineResult {
                            exit_code: ExitCode::Abort,
                            stdout_content: String::new(),
                            stderr_status: format!("ABORT: gate failed: {}", e),
                            failed_stages: vec![format!("gate ({})", e)],
                            log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                        };
                    }
                    failed_stages.push(format!("gate ({})", e));
                    filtered_candidates = candidate_files.into_iter().take(10).collect();
                }
            }
        }
    } else {
        filtered_candidates = candidate_files.into_iter().take(10).collect();
        if let Some(ref mut l) = logger {
            let status = if skip_network {
                StageStatus::Skipped
            } else {
                StageStatus::Disabled
            };
            l.record_stage(StageLogRecord::new("gate", status, 0));
        }
    }

    // ──────────────────────────────────────────
    // Stage 7: Writer stage (Groq model chain)
    // ──────────────────────────────────────────
    let stage_start = Instant::now();
    let mut writer_output: Option<WriterOutput> = None;
    let mut writer_model_used: Option<String> = None;

    if !skip_network && config.is_stage_enabled(app, "writer") {
        let remaining_budget = total_budget.saturating_sub(start_time.elapsed());
        if remaining_budget.is_zero() {
            failed_stages.push("writer (total time budget exceeded)".to_string());
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new(
                        "writer",
                        StageStatus::Failed,
                        stage_start.elapsed().as_millis() as u64,
                    )
                    .with_error("total time budget exceeded"),
                );
            }
        } else if hooks.fail_writer {
            let elapsed = stage_start.elapsed().as_millis() as u64;
            if let Some(ref mut l) = logger {
                l.record_stage(
                    StageLogRecord::new("writer", StageStatus::Failed, elapsed)
                        .with_error("injected failure"),
                );
            }

            let on_fail = config.stage_on_fail(app, "writer");
            if on_fail == OnFail::Abort {
                if let Some(ref mut l) = logger {
                    l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                }
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: writer stage failed".to_string(),
                    failed_stages: vec!["writer (injected failure)".to_string()],
                    log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                };
            }
            failed_stages.push("writer (all models failed / timeout)".to_string());
        } else {
            let payload_input = PayloadInput {
                agent: app,
                mode,
                session_id: history_src.as_deref().or(app_context.session_id.as_deref()),
                repo_summary: repo_summary.as_deref(),
                candidates: &filtered_candidates,
                history_summary: cached_history.as_deref(),
                history_turns: &[],
                user_prompt: &processed_prompt,
                max_tokens: config.budget.payload_tokens_max,
            };
            let payload = build_writer_payload(&payload_input);

            if let Some(ref mut l) = logger {
                l.set_payload_sent(&payload.formatted);
            }

            let meta_prompt = PromptLoader::load_meta_prompt();

            match resolve_api_key(&config.llm.writer.api_key_source, "GROQ_API_KEY") {
                Ok(api_key) => {
                    let writer_timeout_ms = remaining_budget
                        .as_millis()
                        .min(config.llm.writer.timeout_ms as u128)
                        as u64;
                    let client = GroqClient::new(
                        api_key,
                        config.llm.writer.chain.clone(),
                        writer_timeout_ms,
                    );

                    match client
                        .execute_writer_chain(&meta_prompt, &payload.formatted)
                        .await
                    {
                        Ok((output, model_name)) => {
                            let elapsed = stage_start.elapsed().as_millis() as u64;
                            if let Some(ref mut l) = logger {
                                l.set_writer_result(None, Some(output.clone()));
                                l.record_stage(
                                    StageLogRecord::new("writer", StageStatus::Ok, elapsed)
                                        .with_model(&model_name)
                                        .with_tokens(
                                            payload.estimated_tokens,
                                            output.task.len() / 4,
                                        ),
                                );
                            }
                            let mut output = output;
                            output.session_kind = Some(session_kind.clone());
                            writer_output = Some(output);
                            writer_model_used = Some(model_name);
                        }
                        Err(e) => {
                            let elapsed = stage_start.elapsed().as_millis() as u64;
                            if let Some(ref mut l) = logger {
                                l.record_stage(
                                    StageLogRecord::new("writer", StageStatus::Failed, elapsed)
                                        .with_tokens(payload.estimated_tokens, 0)
                                        .with_error(e.to_string()),
                                );
                            }
                            let on_fail = config.stage_on_fail(app, "writer");
                            if on_fail == OnFail::Abort {
                                if let Some(ref mut l) = logger {
                                    l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                                }
                                return PipelineResult {
                                    exit_code: ExitCode::Abort,
                                    stdout_content: String::new(),
                                    stderr_status: format!("ABORT: writer failed: {}", e),
                                    failed_stages: vec![format!("writer ({})", e)],
                                    log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                                };
                            }
                            failed_stages.push(format!("writer ({})", e));
                        }
                    }
                }
                Err(e) => {
                    let elapsed = stage_start.elapsed().as_millis() as u64;
                    if let Some(ref mut l) = logger {
                        l.record_stage(
                            StageLogRecord::new("writer", StageStatus::Failed, elapsed)
                                .with_error(e.to_string()),
                        );
                    }
                    let on_fail = config.stage_on_fail(app, "writer");
                    if on_fail == OnFail::Abort {
                        if let Some(ref mut l) = logger {
                            l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                        }
                        return PipelineResult {
                            exit_code: ExitCode::Abort,
                            stdout_content: String::new(),
                            stderr_status: format!("ABORT: Groq API key not found: {}", e),
                            failed_stages: vec![format!("writer (key resolution: {})", e)],
                            log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                        };
                    }
                    failed_stages.push(format!("writer (key resolution: {})", e));
                }
            }
        }
    } else if let Some(ref mut l) = logger {
        let status = if skip_network {
            StageStatus::Skipped
        } else {
            StageStatus::Disabled
        };
        l.record_stage(StageLogRecord::new("writer", status, 0));
    }

    // ──────────────────────────────────────────
    // Stage 8b: Confidence stage (Jev scoring)
    // ──────────────────────────────────────────
    let mut confidence_report: Option<Vec<ItemConfidence>> = None;
    let confidence_stage_start = Instant::now();
    if !skip_network {
        if let Some(ref mut out) = writer_output {
            if config.is_stage_enabled(app, "confidence") {
                if hooks.fail_confidence {
                    let elapsed = confidence_stage_start.elapsed().as_millis() as u64;
                    if let Some(ref mut l) = logger {
                        l.record_stage(
                            StageLogRecord::new("confidence", StageStatus::Failed, elapsed)
                                .with_error("injected failure"),
                        );
                    }

                    let on_fail = config.stage_on_fail(app, "confidence");
                    if on_fail == OnFail::Abort {
                        if let Some(ref mut l) = logger {
                            l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                        }
                        return PipelineResult {
                            exit_code: ExitCode::Abort,
                            stdout_content: String::new(),
                            stderr_status: "ABORT: confidence stage failed".to_string(),
                            failed_stages: vec!["confidence (injected failure)".to_string()],
                            log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                        };
                    }
                    failed_stages.push("confidence (Jev scoring failed)".to_string());
                } else {
                    let remaining_budget = total_budget.saturating_sub(start_time.elapsed());
                    if remaining_budget.is_zero() {
                        failed_stages.push("confidence (time budget exceeded)".to_string());
                        if let Some(ref mut l) = logger {
                            l.record_stage(
                                StageLogRecord::new(
                                    "confidence",
                                    StageStatus::Failed,
                                    confidence_stage_start.elapsed().as_millis() as u64,
                                )
                                .with_error("time budget exceeded"),
                            );
                        }
                    } else {
                        let confidence_timeout_ms = remaining_budget.as_millis().min(3000) as u64;
                        match run_confidence_check(
                            &processed_prompt,
                            &app_context,
                            &repo_summary,
                            &cached_history,
                            &filtered_candidates,
                            out,
                            config.confidence.threshold,
                            confidence_timeout_ms,
                        )
                        .await
                        {
                            Ok(report) => {
                                let elapsed = confidence_stage_start.elapsed().as_millis() as u64;
                                confidence_report = Some(report);
                                if let Some(ref mut l) = logger {
                                    l.record_stage(StageLogRecord::new(
                                        "confidence",
                                        StageStatus::Ok,
                                        elapsed,
                                    ));
                                }
                            }
                            Err(e) => {
                                let elapsed = confidence_stage_start.elapsed().as_millis() as u64;
                                if let Some(ref mut l) = logger {
                                    l.record_stage(
                                        StageLogRecord::new(
                                            "confidence",
                                            StageStatus::Failed,
                                            elapsed,
                                        )
                                        .with_error(&e),
                                    );
                                }
                                let on_fail = config.stage_on_fail(app, "confidence");
                                if on_fail == OnFail::Abort {
                                    if let Some(ref mut l) = logger {
                                        l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                                    }
                                    return PipelineResult {
                                        exit_code: ExitCode::Abort,
                                        stdout_content: String::new(),
                                        stderr_status: format!("ABORT: confidence failed: {}", e),
                                        failed_stages: vec![format!("confidence ({})", e)],
                                        log_path: logger
                                            .as_ref()
                                            .map(|l| l.log_path().to_path_buf()),
                                    };
                                }
                                failed_stages.push(format!("confidence ({})", e));
                            }
                        }
                    }
                }
            }
        } else if let Some(ref mut l) = logger {
            l.record_stage(StageLogRecord::new("confidence", StageStatus::Disabled, 0));
        }
    } else if let Some(ref mut l) = logger {
        l.record_stage(StageLogRecord::new("confidence", StageStatus::Skipped, 0));
    }

    // ──────────────────────────────────────────
    // Stage 9: Verify stage
    // ──────────────────────────────────────────
    let stage_start = Instant::now();
    if let Some(ref mut out) = writer_output {
        if config.is_stage_enabled(app, "verify") {
            if hooks.fail_verify {
                let elapsed = stage_start.elapsed().as_millis() as u64;
                if let Some(ref mut l) = logger {
                    l.record_stage(
                        StageLogRecord::new("verify", StageStatus::Failed, elapsed)
                            .with_error("injected failure"),
                    );
                }

                let on_fail = config.stage_on_fail(app, "verify");
                if on_fail == OnFail::Abort {
                    if let Some(ref mut l) = logger {
                        l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                    }
                    return PipelineResult {
                        exit_code: ExitCode::Abort,
                        stdout_content: String::new(),
                        stderr_status: "ABORT: verify stage failed".to_string(),
                        failed_stages: vec!["verify (injected failure)".to_string()],
                        log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                    };
                }
                failed_stages.push("verify (output unverified)".to_string());
            } else {
                let initial_files = out.relevant_files.clone();
                match Verifier::verify_and_clean(out.clone(), &filtered_candidates) {
                    Ok(cleaned) => {
                        let elapsed = stage_start.elapsed().as_millis() as u64;
                        let dropped: Vec<String> = initial_files
                            .into_iter()
                            .filter(|f| !cleaned.relevant_files.contains(f))
                            .collect();
                        let verdict = if dropped.is_empty() {
                            "passed".to_string()
                        } else {
                            "cleaned".to_string()
                        };
                        if let Some(ref mut l) = logger {
                            l.set_verifier_result(VerifierLogRecord {
                                verdict,
                                dropped_files: dropped,
                                error: None,
                            });
                            l.record_stage(StageLogRecord::new("verify", StageStatus::Ok, elapsed));
                        }
                        *out = cleaned;
                    }
                    Err(e) => {
                        let elapsed = stage_start.elapsed().as_millis() as u64;
                        if let Some(ref mut l) = logger {
                            l.set_verifier_result(VerifierLogRecord {
                                verdict: "rejected".to_string(),
                                dropped_files: Vec::new(),
                                error: Some(e.to_string()),
                            });
                            l.record_stage(
                                StageLogRecord::new("verify", StageStatus::Failed, elapsed)
                                    .with_error(e.to_string()),
                            );
                        }
                        let on_fail = config.stage_on_fail(app, "verify");
                        if on_fail == OnFail::Abort {
                            if let Some(ref mut l) = logger {
                                l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
                            }
                            return PipelineResult {
                                exit_code: ExitCode::Abort,
                                stdout_content: String::new(),
                                stderr_status: format!("ABORT: verifier rejected output: {}", e),
                                failed_stages: vec![format!("verify ({})", e)],
                                log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
                            };
                        }
                        failed_stages.push(format!("verify ({})", e));
                    }
                }
            }
        } else if let Some(ref mut l) = logger {
            l.record_stage(StageLogRecord::new("verify", StageStatus::Disabled, 0));
        }
    }

    // ──────────────────────────────────────────
    // Stage 9: Render stage
    // ──────────────────────────────────────────
    let stage_start = Instant::now();
    if hooks.fail_render {
        let on_fail = config.stage_on_fail(app, "render");
        if on_fail == OnFail::Abort {
            if let Some(ref mut l) = logger {
                l.finalize("", None, false, ExitCode::Abort.as_i32()).await;
            }
            return PipelineResult {
                exit_code: ExitCode::Abort,
                stdout_content: String::new(),
                stderr_status: "ABORT: render stage failed (bug)".to_string(),
                failed_stages: vec!["render (injected bug)".to_string()],
                log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
            };
        }
    }

    let degradation = if !failed_stages.is_empty() {
        let result_msg = if writer_output.is_none() {
            "your raw prompt was wrapped in the default skeleton and NOT enhanced."
        } else {
            "output unverified by Jev or generated with partial context."
        };
        Some(Degradation::new(failed_stages.clone(), result_msg))
    } else {
        None
    };

    let template = ReportTemplate::resolve(config, profile, Some(&session_kind));
    let rendered_xml = match writer_output {
        Some(ref out) => render_with_template(
            out,
            &template,
            degradation.as_ref(),
            confidence_report.as_deref(),
            config.confidence.threshold,
        ),
        None => {
            let out = WriterOutput {
                task: processed_prompt.clone(),
                context: Vec::new(),
                relevant_files: Vec::new(),
                acceptance_criteria: Vec::new(),
                extra_rules: Vec::new(),
                extra_report: Vec::new(),
                assumptions: Vec::new(),
                out_of_scope: Vec::new(),
                verify: Vec::new(),
                session_kind: Some(session_kind.clone()),
                confidence: Vec::new(),
            };
            render_with_template(
                &out,
                &template,
                degradation.as_ref(),
                None,
                config.confidence.threshold,
            )
        }
    };

    let elapsed = stage_start.elapsed().as_millis() as u64;
    if let Some(ref mut l) = logger {
        l.record_stage(StageLogRecord::new("render", StageStatus::Ok, elapsed));
    }

    // ──────────────────────────────────────────
    // Result determination & Exit code
    // ──────────────────────────────────────────
    let is_degraded = !failed_stages.is_empty();
    let exit_code = if is_degraded {
        ExitCode::Degraded
    } else {
        ExitCode::Ok
    };

    if let Some(ref mut l) = logger {
        l.finalize(
            &rendered_xml,
            degradation.as_ref().map(|d| d.render_banner()),
            is_degraded,
            exit_code.as_i32(),
        )
        .await;
    }

    if let Some(ref deg) = degradation {
        let stderr = format!("{} (app={})", deg.stderr_status(), app);
        PipelineResult {
            exit_code: ExitCode::Degraded,
            stdout_content: rendered_xml,
            stderr_status: stderr,
            failed_stages,
            log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
        }
    } else {
        let model_label = writer_model_used.as_deref().unwrap_or("no-LLM");
        let stderr = format!("OK (enhanced via {}, app={})", model_label, app);
        PipelineResult {
            exit_code: ExitCode::Ok,
            stdout_content: rendered_xml,
            stderr_status: stderr,
            failed_stages: Vec::new(),
            log_path: logger.as_ref().map(|l| l.log_path().to_path_buf()),
        }
    }
}

async fn run_decider(
    prompt: &str,
    candidates: &[String],
    is_continue: bool,
    config: &Config,
    timeout_ms: u64,
) -> Result<GateDecisions, String> {
    if config.decider.provider == "jev" {
        if let Ok(key) = resolve_api_key("keychain:pfmt-jev", "TYPESAFE_API_KEY") {
            let decider = JevDecider::new(key, timeout_ms);
            return decider
                .decide(prompt, candidates, is_continue)
                .await
                .map_err(|e| format!("jev: {}", e));
        }
    }

    // Fallback to Groq decider if configured or Jev unavailable
    if let Ok(key) = resolve_api_key(&config.llm.writer.api_key_source, "GROQ_API_KEY") {
        let model = config
            .llm
            .writer
            .chain
            .last()
            .cloned()
            .unwrap_or_else(|| "openai/gpt-oss-20b".to_string());
        let decider = GroqJsonDecider::new(key, model, timeout_ms);
        return decider
            .decide(prompt, candidates, is_continue)
            .await
            .map_err(|e| format!("groq decider: {}", e));
    }

    Err("no working decider API key available".to_string())
}

#[allow(clippy::too_many_arguments)]
async fn run_confidence_check(
    user_prompt: &str,
    app_context: &AppContext,
    repo_summary: &Option<String>,
    cached_history: &Option<String>,
    candidates: &[String],
    writer_output: &WriterOutput,
    threshold: f64,
    timeout_ms: u64,
) -> Result<Vec<ItemConfidence>, String> {
    // Build the state (evidence) that the writer saw, redacted
    let mut state_parts = Vec::new();
    state_parts.push(format!("User prompt: {}", user_prompt));

    if let Some(repo) = repo_summary {
        state_parts.push(format!("Repo: {}", repo));
    }
    if let Some(history) = cached_history {
        state_parts.push(format!("History: {}", history));
    }
    if !candidates.is_empty() {
        state_parts.push(format!("Candidates: {}", candidates.join(", ")));
    }
    if let Some(project_dir) = &app_context.project_dir {
        state_parts.push(format!("Project dir: {}", project_dir.display()));
    }
    if let Some(session_id) = &app_context.session_id {
        state_parts.push(format!("Session: {}", session_id));
    }

    let state = state_parts.join("\n\n");

    // Collect LLM-written items to check
    let mut items = Vec::new();

    // Task
    items.push(ConfidenceItem {
        id: "task".to_string(),
        text: writer_output.task.clone(),
        section: "task".to_string(),
        index: 0,
    });

    // Acceptance criteria
    for (i, ac) in writer_output.acceptance_criteria.iter().enumerate() {
        items.push(ConfidenceItem {
            id: format!("ac_{}", i),
            text: ac.clone(),
            section: "acceptance_criteria".to_string(),
            index: i,
        });
    }

    // Context
    for (i, ctx) in writer_output.context.iter().enumerate() {
        items.push(ConfidenceItem {
            id: format!("ctx_{}", i),
            text: ctx.clone(),
            section: "context".to_string(),
            index: i,
        });
    }

    // Scope (relevant_files + out_of_scope)
    for (i, rf) in writer_output.relevant_files.iter().enumerate() {
        items.push(ConfidenceItem {
            id: format!("scope_{}", i),
            text: rf.clone(),
            section: "scope".to_string(),
            index: i,
        });
    }
    for (i, oos) in writer_output.out_of_scope.iter().enumerate() {
        items.push(ConfidenceItem {
            id: format!("scope_{}", writer_output.relevant_files.len() + i),
            text: oos.clone(),
            section: "scope".to_string(),
            index: i,
        });
    }

    // Verify
    for (i, ver) in writer_output.verify.iter().enumerate() {
        items.push(ConfidenceItem {
            id: format!("ver_{}", i),
            text: ver.clone(),
            section: "verify".to_string(),
            index: i,
        });
    }

    // Resolve Jev API key
    let key = resolve_api_key("keychain:pfmt-jev", "TYPESAFE_API_KEY")
        .map_err(|e| format!("Jev API key missing: {}", e))?;

    let checker = JevConfidence::new(key, timeout_ms).with_threshold(threshold);
    checker
        .check_confidence(&state, &items)
        .await
        .map_err(|e| format!("confidence check: {}", e))
}
