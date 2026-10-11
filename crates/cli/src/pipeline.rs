use pfmt_context::{
    build_writer_payload, collect_repo_summary, extract_candidate_files, PayloadInput, SessionMode,
};
use pfmt_core::{
    is_idempotent, no_llm_wrap_with_degradation, redact_secrets, render, resolve_api_key, Config,
    Degradation, ExitCode, OnFail, Profile, WriterOutput,
};
use pfmt_llm::{
    Decider, GateDecisions, GroqClient, GroqJsonDecider, JevDecider, PromptLoader, Verifier,
};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineResult {
    pub exit_code: ExitCode,
    pub stdout_content: String,
    pub stderr_status: String,
    pub failed_stages: Vec<String>,
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
    // Stage 1: Input stage
    // ──────────────────────────────────────────
    let trimmed_input = raw_input.trim();
    if trimmed_input.is_empty() || hooks.fail_input {
        let on_fail = config.stage_on_fail(app, "input");
        if on_fail == OnFail::Abort {
            return PipelineResult {
                exit_code: ExitCode::Abort,
                stdout_content: String::new(),
                stderr_status: "empty input: nothing to do".to_string(),
                failed_stages: vec!["input (empty input or failed read)".to_string()],
            };
        } else {
            failed_stages.push("input (empty input)".to_string());
        }
    }

    // ──────────────────────────────────────────
    // Stage 2: Idempotence check
    // ──────────────────────────────────────────
    if is_idempotent(trimmed_input) {
        return PipelineResult {
            exit_code: ExitCode::Ok,
            stdout_content: raw_input.to_string(),
            stderr_status: "unchanged".to_string(),
            failed_stages: Vec::new(),
        };
    }

    // ──────────────────────────────────────────
    // Stage 3: Redact stage
    // ──────────────────────────────────────────
    let mut processed_prompt = trimmed_input.to_string();
    let mut skip_network = force_no_llm;

    if config.is_stage_enabled(app, "redact") {
        if hooks.fail_redact {
            let on_fail = config.stage_on_fail(app, "redact");
            if on_fail == OnFail::Abort {
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: redaction stage failed".to_string(),
                    failed_stages: vec!["redact (injected failure)".to_string()],
                };
            }
            failed_stages.push("redact (redaction error: network skipped)".to_string());
            skip_network = true;
        } else {
            processed_prompt = redact_secrets(&processed_prompt);
        }
    }

    // ──────────────────────────────────────────
    // Check global time budget
    // ──────────────────────────────────────────
    if start_time.elapsed() >= total_budget {
        failed_stages.push("timeout (total time budget exceeded)".to_string());
        skip_network = true;
    }

    // ──────────────────────────────────────────
    // Stage 4: Context stage
    // ──────────────────────────────────────────
    let mut repo_summary = None;
    let mut candidate_files = Vec::new();

    if config.is_stage_enabled(app, "context") {
        if hooks.fail_context {
            let on_fail = config.stage_on_fail(app, "context");
            if on_fail == OnFail::Abort {
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: context collection failed".to_string(),
                    failed_stages: vec!["context (injected failure)".to_string()],
                };
            }
            failed_stages.push("context (repo inspection failed)".to_string());
        } else {
            let summary = collect_repo_summary(cwd);
            let candidates = extract_candidate_files(&processed_prompt, cwd);
            repo_summary = Some(summary);
            candidate_files = candidates;
        }
    }

    // ──────────────────────────────────────────
    // Stage 5: History stage
    // ──────────────────────────────────────────
    let cached_history: Option<String> = None;
    if config.is_stage_enabled(app, "history") && hooks.fail_history {
        let on_fail = config.stage_on_fail(app, "history");
        if on_fail == OnFail::Abort {
            return PipelineResult {
                exit_code: ExitCode::Abort,
                stdout_content: String::new(),
                stderr_status: "ABORT: history stage failed".to_string(),
                failed_stages: vec!["history (injected failure)".to_string()],
            };
        }
        failed_stages.push("history (adapter failed / db locked)".to_string());
    }

    // ──────────────────────────────────────────
    // Stage 6: Gate stage (Jev / Groq decider)
    // ──────────────────────────────────────────
    let is_continue = cached_history.is_some();
    let mode = if is_continue {
        SessionMode::Continue
    } else {
        SessionMode::Fresh
    };

    let mut filtered_candidates = candidate_files.clone();
    let mut _needs_history = is_continue;

    if !skip_network && config.is_stage_enabled(app, "gate") {
        let remaining_budget = total_budget.saturating_sub(start_time.elapsed());
        if remaining_budget.is_zero() {
            failed_stages.push("gate (time budget exceeded)".to_string());
        } else if hooks.fail_gate {
            let on_fail = config.stage_on_fail(app, "gate");
            if on_fail == OnFail::Abort {
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: gate stage failed".to_string(),
                    failed_stages: vec!["gate (injected failure)".to_string()],
                };
            }
            failed_stages.push("gate (decider failure: heuristic fallback)".to_string());
            // Heuristic fallback: all candidates up to cap 10
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
            match decider_result {
                Ok(decisions) => {
                    _needs_history = decisions.needs_history;
                    filtered_candidates = decisions.filtered_candidates;
                }
                Err(e) => {
                    let on_fail = config.stage_on_fail(app, "gate");
                    if on_fail == OnFail::Abort {
                        return PipelineResult {
                            exit_code: ExitCode::Abort,
                            stdout_content: String::new(),
                            stderr_status: format!("ABORT: gate failed: {}", e),
                            failed_stages: vec![format!("gate ({})", e)],
                        };
                    }
                    failed_stages.push(format!("gate ({})", e));
                    // Heuristic fallback: all candidates up to cap 10
                    filtered_candidates = candidate_files.into_iter().take(10).collect();
                }
            }
        }
    } else {
        // Gate skipped or disabled
        filtered_candidates = candidate_files.into_iter().take(10).collect();
    }

    // ──────────────────────────────────────────
    // Stage 7: Writer stage (Groq model chain)
    // ──────────────────────────────────────────
    let mut writer_output: Option<WriterOutput> = None;
    let mut writer_model_used: Option<String> = None;

    if !skip_network && config.is_stage_enabled(app, "writer") {
        let remaining_budget = total_budget.saturating_sub(start_time.elapsed());
        if remaining_budget.is_zero() {
            failed_stages.push("writer (total time budget exceeded)".to_string());
        } else if hooks.fail_writer {
            let on_fail = config.stage_on_fail(app, "writer");
            if on_fail == OnFail::Abort {
                return PipelineResult {
                    exit_code: ExitCode::Abort,
                    stdout_content: String::new(),
                    stderr_status: "ABORT: writer stage failed".to_string(),
                    failed_stages: vec!["writer (injected failure)".to_string()],
                };
            }
            failed_stages.push("writer (all models failed / timeout)".to_string());
        } else {
            let payload_input = PayloadInput {
                agent: app,
                mode,
                repo_summary: repo_summary.as_deref(),
                candidates: &filtered_candidates,
                history_summary: cached_history.as_deref(),
                history_turns: &[],
                user_prompt: &processed_prompt,
                max_tokens: config.budget.payload_tokens_max,
            };
            let payload = build_writer_payload(&payload_input);

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
                            writer_output = Some(output);
                            writer_model_used = Some(model_name);
                        }
                        Err(e) => {
                            let on_fail = config.stage_on_fail(app, "writer");
                            if on_fail == OnFail::Abort {
                                return PipelineResult {
                                    exit_code: ExitCode::Abort,
                                    stdout_content: String::new(),
                                    stderr_status: format!("ABORT: writer failed: {}", e),
                                    failed_stages: vec![format!("writer ({})", e)],
                                };
                            }
                            failed_stages.push(format!("writer ({})", e));
                        }
                    }
                }
                Err(e) => {
                    let on_fail = config.stage_on_fail(app, "writer");
                    if on_fail == OnFail::Abort {
                        return PipelineResult {
                            exit_code: ExitCode::Abort,
                            stdout_content: String::new(),
                            stderr_status: format!("ABORT: Groq API key not found: {}", e),
                            failed_stages: vec![format!("writer (key resolution: {})", e)],
                        };
                    }
                    failed_stages.push(format!("writer (key resolution: {})", e));
                }
            }
        }
    }

    // ──────────────────────────────────────────
    // Stage 8: Verify stage
    // ──────────────────────────────────────────
    if let Some(ref mut out) = writer_output {
        if config.is_stage_enabled(app, "verify") {
            if hooks.fail_verify {
                let on_fail = config.stage_on_fail(app, "verify");
                if on_fail == OnFail::Abort {
                    return PipelineResult {
                        exit_code: ExitCode::Abort,
                        stdout_content: String::new(),
                        stderr_status: "ABORT: verify stage failed".to_string(),
                        failed_stages: vec!["verify (injected failure)".to_string()],
                    };
                }
                failed_stages.push("verify (output unverified)".to_string());
            } else {
                match Verifier::verify_and_clean(out.clone(), &filtered_candidates) {
                    Ok(cleaned) => {
                        *out = cleaned;
                    }
                    Err(e) => {
                        let on_fail = config.stage_on_fail(app, "verify");
                        if on_fail == OnFail::Abort {
                            return PipelineResult {
                                exit_code: ExitCode::Abort,
                                stdout_content: String::new(),
                                stderr_status: format!("ABORT: verifier rejected output: {}", e),
                                failed_stages: vec![format!("verify ({})", e)],
                            };
                        }
                        failed_stages.push(format!("verify ({})", e));
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────
    // Stage 9: Render stage
    // ──────────────────────────────────────────
    if hooks.fail_render {
        let on_fail = config.stage_on_fail(app, "render");
        if on_fail == OnFail::Abort {
            return PipelineResult {
                exit_code: ExitCode::Abort,
                stdout_content: String::new(),
                stderr_status: "ABORT: render stage failed (bug)".to_string(),
                failed_stages: vec!["render (injected bug)".to_string()],
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

    let rendered_xml = match writer_output {
        Some(ref out) => render(out, profile, degradation.as_ref()),
        None => no_llm_wrap_with_degradation(&processed_prompt, profile, degradation.as_ref()),
    };

    // ──────────────────────────────────────────
    // Result determination & Exit code
    // ──────────────────────────────────────────
    if let Some(ref deg) = degradation {
        let stderr = format!("{} (app={})", deg.stderr_status(), app);
        PipelineResult {
            exit_code: ExitCode::Degraded,
            stdout_content: rendered_xml,
            stderr_status: stderr,
            failed_stages,
        }
    } else {
        let model_label = writer_model_used.as_deref().unwrap_or("no-LLM");
        let stderr = format!("OK (enhanced via {}, app={})", model_label, app);
        PipelineResult {
            exit_code: ExitCode::Ok,
            stdout_content: rendered_xml,
            stderr_status: stderr,
            failed_stages: Vec::new(),
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
