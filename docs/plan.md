# pfmt Implementation Plan: Dynamic Context + Jev Confidence

> Hand this file to the coding agent. Work **one step at a time, in order**. Do not start step N+1 until step N's exit check passes and you have posted its report.

## 0. Goal

Hotkey fires ? `pfmt enhance` probes the frontmost app (app name, project dir, session id) ? gathers whatever context exists (session summary, repo data, agent-doc summaries) ? LLM writes the brief ? Jev (TypeSafe) scores every LLM-written item ? output is rendered in this exact order, with low-confidence items visibly marked:

```xml
<task></task>
<acceptance_criteria></acceptance_criteria>
<context></context>
<scope></scope>
<rules></rules>
<verify></verify>
<report></report>
```

Side functions (run by `pfmt watch`, never blocking `enhance`): session summarizer (exists), agent-doc summarizer (new). Every run is logged for future upgrades.

## 1. Global rules (apply to EVERY step)

**Anti-hallucination**
1. Read every file **fully** before editing it. Edit only files listed in the step.
2. Never invent APIs, crate methods, CLI flags, JSON fields, or macOS behaviour. Verify: crate version in `Cargo.lock` ? source in `~/.cargo/registry` or `cargo doc`. For `osascript`/`lsof`/`pgrep`, run the command on this machine and paste the real output in your report.
3. If something is unknown or ambiguous: **stop and ask**. Do not guess, do not stub with fake data.
4. No mock data, no swallowed errors, no `unwrap_or(<default>)` that hides a failure. Failures must be explicit and degrade per `[stages.*] on_fail`.
5. Every claim in your report must cite `file:line` or pasted command output.

**Workflow**
6. One step = one commit. Commit message: `step N: <title>`.
7. After each step run, in order, and paste results:
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace -- --test-threads=1`
8. Never weaken, skip, or delete an existing test to make it pass. Only change tests a step explicitly names.
9. Follow existing patterns (e.g. `StageFailureHook` in `crates/cli/tests/pipeline_stages_test.rs` for forced-failure tests).
10. Ask before: destructive actions, changes to existing DB tables, public API changes not listed here.
11. Preserve repo invariants from `AGENTS.md`: exit codes (0 ok / 10 degraded / 1 abort), `redact_secrets` before any network send or log, candidate-path jailing, verifier dropping files not in candidates, `enhance` never blocking on compaction.
12. Do not "fix" `live_smoke_test.rs` to require network.

**Stop conditions** (stop and report, do not continue): exit check fails; a needed fact cannot be verified; a step needs a file not listed; baseline regressions you cannot explain.

## 2. Progress tracker

- [ ] Step 0: Baseline
- [x] Step 1: Probe stage
- [x] Step 2: Schema and render
- [x] Step 3: Context layers and docs summarizer
- [x] Step 4: Wire gate flags
- [x] Step 5: Jev confidence check
- [ ] Step 6: Logging, shell, and review fixes

## 3. Per-step report template

Post this after each step (keep it short):

```
STEP N: <title>
Files changed: <list>
Commands run + result: fmt / clippy / test (pass/fail counts)
Verified facts (with file:line or command output): ...
Assumptions made: ...
Deviations from plan: ...
Open questions: ...
```

---

## Step 0: Baseline

**Goal:** know what already fails before you touch anything.

**Do**
- Run fmt, clippy, and `cargo test --workspace -- --test-threads=1`.
- Record failing tests and why (known suspects: tests with hardcoded `/Users/mohamad_mac/...` paths in `crates/context/tests/real_repos_test.rs` and `crates/llm/tests/live_smoke_test.rs`; pipeline tests with `no_llm=false` that may hit live Groq; `test_stage_history_cached_summary_integration` writes to the real `history.db`).

**Do not:** modify any file.

**Exit check:** a written baseline list. Failures here are not yours to fix unless a later step says so.

---

## Step 1: Probe stage

**Goal:** resolve `AppContext` from the frontmost app; every later stage uses whichever fields exist.

**Files:** `crates/core/src/lib.rs` (+ new `crates/core/src/probe.rs`), `crates/history/src/adapter.rs`, `crates/history/src/adapters/antigravity.rs`, `crates/cli/src/main.rs`, `crates/cli/src/pipeline.rs`, `crates/core/src/config.rs`, `crates/cli/tests/pipeline_stages_test.rs`.

**Changes**
1. Core: add
   ```rust
   pub struct AppContext {
       pub app_name: String,
       pub project_dir: Option<PathBuf>,
       pub session_id: Option<String>,
       pub sources: Vec<String>, // how each field was resolved, for logs
   }
   ```
2. History: add `AgentAdapter::resolve_active(&self) -> Option<ActiveSession>` with a **default `None`**. Implement it **only** for Antigravity (latest transcript + existing `Cwd`/`AbsolutePath` inference). OpenCode, Freebuff, Hermes stay `None` until their on-disk formats are verified (out of scope here).
3. Terminal apps (Terminal, iTerm2, Ghostty, Warp, kitty, alacritty): resolve cwd of the tty's foreground process via `lsof -a -d cwd -Fn -p <pid>`. **Run it manually first** and paste the output; if it does not behave as expected, stop and ask.
4. CLI: `enhance` gains `--cwd <path>` and `--session-id <id>` overrides (override beats probe; also makes tests hermetic).
5. Pipeline: add a `probe` stage before `context`. Config `[stages.probe] enabled = true, on_fail = "skip"`. A probe failure or missing field **degrades silently** (no banner, no exit 10); it is logged.
6. Pipeline uses `AppContext.project_dir` instead of the process cwd for context/candidates/history lookups; if `None`, those layers are skipped.

**Do not:** change `WriterOutput`, render, or prompts. Do not guess project paths for non-Antigravity apps.

**Exit check**
- Unit tests for each resolver using temp fixtures.
- `--cwd` override test via pipeline.
- Manual run against a real Antigravity session prints a correct `AppContext` (paste it).
- Existing tests still pass.

---

## Step 2: Schema and render

**Goal:** new section order and fields; `relevant_files` finally reaches the output.

**Files:** `crates/core/src/types.rs`, `crates/core/src/render.rs`, `crates/core/src/lib.rs`, `crates/llm/src/verifier.rs`, `crates/llm/src/groq.rs` (schema only), `crates/core/tests/golden_test.rs`, `crates/core/tests/lib_test.rs`, `crates/llm/tests/local_stub_test.rs`, `tests/eval/golden_prompts.json` (only if needed).

**Changes**
1. `WriterOutput`: add `out_of_scope: Vec<String>` and `verify: Vec<String>` (`#[serde(default)]`, `deny_unknown_fields` kept). Add limits and matching `ValidationError` variants:
   - `out_of_scope`: � 5 items, each � 200 chars
   - `verify`: � 5 items, each � 200 chars
2. `render`: output order is `task`, `acceptance_criteria`, `context`, `scope`, `rules`, `verify`, `report`.
   - `scope` = `relevant_files` + `out_of_scope` (omit if both empty).
   - `verify` = detected `RepoStack` test + lint commands (deterministic, always when known) + LLM `verify` items (omit if empty).
   - Empty sections are omitted; `task`, `rules`, `report` always render.
   - `render` needs the structured `RepoStack` (or a `Vec<String>` of commands) instead of a string. Choose the smallest API change and list it in your report.
3. Verifier: drop any LLM `verify` item that looks like a shell command and is not in the detected command set (log the drop like `dropped_files`).
4. JSON schema (`WriterOutput::json_schema`): post-process so **every property is in `required`** and `additionalProperties` is `false`, while keeping `serde(default)` for parsing. Reason: strict `json_schema` mode currently likely returns 400 and wastes a call per model.
5. Update golden/lib/stub fixtures to the new order and fields. **Show me the golden diff in your report.**

**Do not:** change idempotence detection (`<task>` + `<acceptance_criteria>`), prompts, or pipeline stage order.

**Exit check**
- Golden tests pass with the new order; empty prompt still aborts (exit 1).
- Verifier test: hallucinated shell command in `verify` is dropped.
- Schema test: all properties appear in `required`.

---

## Step 3: Context layers and docs summarizer

**Goal:** session, repo, and agent-doc summaries, each optional, each with provenance.

**Files:** `crates/history/src/db.rs`, new `crates/history/src/docs.rs`, `crates/history/src/watcher.rs`, `crates/history/src/compactor.rs` (only if reused), `crates/history/src/lib.rs`, `crates/context/src/payload.rs`, `crates/cli/src/pipeline.rs`, `prompts/meta_prompt.v2.md` (new), `crates/llm/src/prompts.rs`, related tests.

**Changes**
1. Session lookup: by `(app, session_id)`; fall back to `(app, project_dir)` **only when no session_id was probed**. `is_continue` is true **only** on a session_id match (the project-only fallback is injected but marked `src="project-fallback"`).
2. Repo summary: from `AppContext.project_dir`; skipped when `None`.
3. DB: `CREATE TABLE IF NOT EXISTS doc_summaries(path TEXT, hash TEXT, summary TEXT, model TEXT, updated_at TEXT, PRIMARY KEY(path, hash))`. **Do not alter existing tables.**
4. Watcher: add a doc summarizer for `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `README.md` in each known `project_dir`. Size-capped, passed through `redact_secrets`, re-run only when the content hash changes. `enhance` only **reads** `doc_summaries`.
5. Payload: wrap each block with provenance, e.g. `<history src="session_id">`, `<history src="project-fallback">`, `<docs>`, `<repo>`. Replace positional `sections[3]`/`sections[last]` indexing with named fields (fixes the panic/overwrite when `repo` or `candidates` are absent and the budget is exceeded). Budget order when over: drop docs ? drop history turns ? truncate summary ? truncate prompt.
6. Prompts: create `prompts/meta_prompt.v2.md` (keep v1). New rule: **every item in `context`, `scope`, and `verify` must come from the given tags; otherwise leave it empty and put the guess in `assumptions`.** Update `PromptLoader` to load v2 and its tests; `include_str!` path included.

**Do not:** let `enhance` call a summarizer or wait on a job.

**Exit check**
- Payload tests with `repo`/`candidates` absent and budget exceeded: no panic, user prompt intact.
- Test: no `<repo>` when `project_dir` is `None`.
- Test: `is_continue` false when no session_id match.
- Doc summarizer test: unchanged hash does not re-summarize.

---

## Step 4: Wire gate flags

**Goal:** the gate decisions actually change behaviour.

**Files:** `crates/cli/src/pipeline.rs`, `crates/llm/src/decider.rs`, `crates/cli/tests/pipeline_stages_test.rs`.

**Changes**
1. `needs_history == false` ? drop history from the payload.
2. `already_structured == true` ? return input unchanged (exit 0, status `unchanged`).
3. `injection_risk == true` ? skip the writer; use the no-LLM wrap with a degradation banner naming the reason (exit 10).
4. `run_decider`: if Jev errors, fall back to the Groq decider (today it returns the Jev error without trying Groq).
5. `GroqJsonDecider` must use a **Groq-hosted** model, not the `:free` OpenRouter tail of the chain. Pick the first non-`is_openrouter_model` entry; if none, return an explicit error.

**Exit check:** one stub test per flag (pattern: `StageFailureHook` / local stub server); Jev-error ? Groq fallback test.

---

## Step 5: Jev confidence check

**Goal:** Jev scores every LLM-written item; low-confidence items are visibly marked in the final output.

**Files:** new `crates/llm/src/confidence.rs`, `crates/llm/src/lib.rs`, `crates/core/src/types.rs` (or new `confidence.rs` in core), `crates/core/src/render.rs`, `crates/core/src/log.rs`, `crates/core/src/config.rs`, `crates/cli/src/pipeline.rs`, `crates/cli/src/review.rs`, tests.

**Changes**
1. Trait in `crates/llm`: `ConfidenceChecker`, with `JevConfidence` impl calling the same Jev endpoint (`noul` questions).
2. Questions: one per LLM-written item:
   - ids: `task`, `ac_<i>`, `ctx_<i>`, `scope_<i>` (relevant_files + out_of_scope), `ver_<i>`.
   - `state` = the same evidence the writer saw (redacted): user prompt, repo, history, docs, candidates.
   - instruction: `"Is this statement directly supported by the state? Statement: <item>"`.
   - For `task`, instead: `"Does this task preserve the intent of the user prompt without adding requirements? Task: <task>"`.
   - Cap at 30 questions; truncate extras and mark the truncated items `unchecked`.
3. **Do not check** `rules` and `report` (profile text, deterministic).
4. Config: `[confidence] threshold = 0.6`; `[stages.confidence] enabled = true, on_fail = "skip"`. Add defaults in `DEFAULT_CONFIG_TOML` and the struct defaults.
5. Score handling: a missing answer or API error is `unchecked` � **never** treated as 0.0 (do not reuse the `unwrap_or(0.0)` path from `JevDecider::extract_prob`).
6. Core type:
   ```rust
   pub struct ConfidenceReport { pub items: Vec<ItemConfidence> }
   pub struct ItemConfidence { pub section: String, pub index: usize, pub score: Option<f64> }
   ```
   `render` takes `Option<&ConfidenceReport>`.
7. Render markers:
   - score < threshold ? item line prefixed `??[low confidence] `
   - `score == None` ? `??[unchecked] `
   - A short notice above `<task>` lists affected sections, e.g. `?? PFMT REVIEW: low confidence in: context, verify. Check marked lines before sending.`
   - For `task` (a single block), prefix the first line.
   - Markers are XML-escape safe and must not break idempotence detection.
8. Jev failure/missing key ? stage failure ? `failed_stages` gets `confidence (...)` ? exit 10 with the existing banner; all LLM-written items marked `unchecked`.
9. Log: add optional `confidence` to `ExecutionLog` (`skip_serializing_if = "Option::is_none"`, so old logs still parse). `review` prints the low-confidence rate per section.

**Do not:** let Jev rewrite or delete content; it only marks. Do not send unredacted text to Jev.

**Exit check**
- Stub Jev with mixed scores ? correct markers and correct section list in the notice.
- Missing key ? `unchecked` markers + exit 10.
- Marked output re-fed to `enhance` ? `unchanged` (idempotence intact).
- `replay` and `review` still parse old and new logs.

---

## Step 6: Logging, shell, and review fixes

**Files:** `crates/core/src/log.rs`, `crates/cli/src/logging.rs`, `crates/cli/src/pipeline.rs`, `scripts/ai_run.sh`, `scripts/karabiner/pfmt.json`, `crates/llm/src/groq.rs`, `config.toml`, `crates/history/src/watcher.rs`, `tests/test_ai_run.sh`.

**Changes**
1. Log: record the `AppContext` (with `sources`) and which context layers were used.
2. `ai_run.sh`:
   - call `"$PFMT_BIN" enhance ...` (not `pfmt-cli`), and **drop `--no-llm`**
   - move `trap cleanup` to after `acquire_lock` succeeds
   - terminal apps use Ctrl+A / Ctrl+C (per `docs/research/macos.md`); GUI apps keep Cmd+A / Cmd+C
   - restore the previous clipboard after a successful paste
3. Hotkey: set `pfmt.json` to Cmd+Shift+Space (matches actual Karabiner file; overrides earlier proposal of Ctrl+Opt+P).
4. Key leak: in `resolve_endpoint_and_key`, remove the Groq-key fallback for OpenRouter models. If the OpenRouter key is missing, skip that model with an explicit error recorded in the chain errors.
5. Outer `tokio::time::timeout` around the whole pipeline using `budget.total_ms`; on timeout degrade to no-LLM.
6. `config.toml`: replace `groq-compactor-placeholder` with a real model from the chain (`qwen/qwen3.8-27b`).
7. Watcher: only enqueue a compact job when `source_cursor` changed since the last summary; add a backoff for failed jobs; scan sessions once per tick.
8. Normalize app names to one slug function used by both the watcher and the pipeline, so per-app overrides match.

**Exit check:** `cargo build && bash tests/test_ai_run.sh`; full workspace suite; a test showing no OpenRouter request carries the Groq key.

---

## 4. Assumptions (confirmed by the user unless noted)

- Section order is exactly as in the template.
- Idempotence still keys on `<task>` + `<acceptance_criteria>`.
- Empty `scope`/`verify` sections are omitted; `verify` always includes stack commands when known.
- Missing project/session info degrades silently (no banner).
- *Confirmed:* `pfmt.json` uses `Cmd+Shift+Space`; `AGENTS.md` and `install.sh` mention `Ctrl+Opt+P` — this discrepancy is preserved (do not change hotkey binding).

## 5. Known limits

- Jev can only judge whether a claim is **supported by the input**; it cannot tell whether a supported claim is correct.
- OpenCode, Freebuff, and Hermes `resolve_active` stay `None` until their stores are researched; they get app-level context only.

## 6. Definition of done

- All 7 steps checked off with reports.
- fmt, clippy, and the full workspace tests pass.
- Manual end-to-end run: hotkey in a real Antigravity session produces all 7 sections, with markers on any item Jev scores below threshold.
- `pfmt review` shows probe, context-source, and confidence data from real logs.