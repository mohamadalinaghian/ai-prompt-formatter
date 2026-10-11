# Decisions Log

## Phase 0: Research Decisions (2026-10-11)

### 1. Groq Models & Structured Outputs
- **Decision:** GO.
- **Rationale:** Groq natively supports `openai/gpt-oss-120b`, `qwen/qwen3.8-27b`, and `openai/gpt-oss-20b`. We will configure `response_format` with `json_schema` generated directly from `WriterOutput` via `schemars`.
- **Fallback:** If `json_schema` mode returns 400 on any model, fallback to `json_object` mode with the schema embedded in the system prompt.

### 2. TypeSafe Jev & Decider Trait
- **Decision:** GO.
- **Rationale:** Jev API (`https://api.typesafe.ai/v1/systemone`) provides sub-300ms probabilistic evaluation (`noul` for binary gate decisions, `choice` for classification).
- **Architecture:** We define a `Decider` trait with two implementations:
  1. `JevDecider`: Calls Jev API using `TYPESAFE_API_KEY`.
  2. `GroqDecider`: Fallback implementation using fast Groq completion with JSON output.

### 3. Agent Stores & Adapters
- **Decision:** GO.
- **Antigravity:** Native transcripts in `~/.gemini/antigravity/brain/<id>/.system_generated/logs/transcript.jsonl` and `conversations/<id>.db`. Clean JSONL parser.
- **OpenCode:** Native SQLite `drafts.sqlite` and `.dat` files in `~/Library/Application Support/ai.opencode.desktop`.
- **Freebuff:** LevelDB/IndexedDB in application support partitions.
- **Hermes:** SQLite `kanban.db` and `shared-state.db` in `~/.hermes/`.
- **Architecture:** All adapters implement a common `AgentStoreAdapter` trait and are individually toggleable via config.

### 4. macOS Integration
- **Decision:** GO.
- Thin bash script (`ai_run.sh`) handles Karabiner hotkey, `sleep 0.4` modifier clearance, terminal vs GUI selection (`Ctrl+A`/`Ctrl+C` vs `Cmd+A`/`Cmd+C`), and invokes `pfmt enhance --app <app>`.
- Keychain key retrieval via `keyring` crate with transparent fallback to environment variables (`GROQ_API_KEY`, `TYPESAFE_API_KEY`).

### 5. Repository Layout
- Workspace root is `/Users/mohamad_mac/tools/ai-prompt-formatter`.
- Workspace crates:
  - `crates/core`: Shared schemas, validation, rendering, config, errors, redaction.
  - `crates/llm`: Groq client, Jev decider, prompt loader.
  - `crates/context`: Repo collectors, git inspection, token budget.
  - `crates/history`: Local SQLite db, watcher, adapters.
  - `crates/cli`: Main `pfmt` binary.

## Phase 1: Skeleton and Contracts (2026-10-11)

### 1. Workspace and Core Types
- **Decision:** Implemented `pfmt-core` and `pfmt-cli` in Cargo workspace.
- **Contract:** Defined `WriterOutput` with strict validation rules matching Section 4.3 (character length limits, array caps, required fields, `schemars` JSON schema generation).
- **XML Renderer:** Strict XML-escaping (`&`, `<`, `>`), appending `extra_rules` and `extra_report` to immutable `<rules>` and `<report>` from profile.
- **Profiles:** Loaded from TOML with embedded fallback so binary is self-contained.
- **Degradation Banner & Exit Codes:** 0 for success/unchanged, 10 for degraded, 1 for abort. Banner prepended to XML output when degraded.
- **Idempotence & No-LLM:** Detected via presence of `<task>` and `<acceptance_criteria>`; returns input untouched with exit code 0. No-LLM fallback wraps raw prompt in default profile skeleton.

### 2. Exit Check Results
- `echo "x" | pfmt enhance --app test` prints valid XML offline and exits with code 0.
- `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` pass cleanly with zero warnings.
- 12 unit and golden integration tests pass across the workspace (`lib_test.rs`, `golden_test.rs`).

## Phase 2: Safety and Context (2026-10-11)

### 1. Secret Redaction
- **Decision:** Implemented regex pattern matching in `pfmt-core::redact` covering Groq, OpenAI, Anthropic, GitHub, AWS, Bearer tokens, PEM private keys, database URLs with passwords, and generic `.env` credential assignments.
- **Safety guarantee:** Redaction is performed before any network call, before logging, and in CLI stdin processing.

### 2. Keychain and Environment Key Resolution
- **Decision:** Integrated `keyring` (v4.2 with `apple-native-keyring-store` feature) in `pfmt-core::keys`.
- **Resolution Strategy:** Supports `keychain:<service>` and `env:<VAR>`. If Keychain retrieval returns `NoEntry` or fails, transparently falls back to environment variable (`GROQ_API_KEY`, `TYPESAFE_API_KEY`). Failures are explicit (`KeyError`).

### 3. Context Collectors and Jailing
- **Decision:** Created `pfmt-context` crate containing:
  - `stack`: Discovers languages and test/lint commands across Rust, TypeScript, JavaScript, Python, Go, and Shell.
  - `git`: Reads current branch and porcelain changed file names (capped at 50, names only, no file tree, no contents).
  - `candidates`: Extracts file paths from prompt, verifies existence, checks `ignore` `.gitignore` rules, and strictly verifies canonical paths are jailed within `repo_root` to prevent directory traversal.

### 4. Payload Builder and Token Budget
- **Decision:** Implemented `build_writer_payload` assembling `<meta>`, `<repo>`, `<candidates>`, `<history>`, and `<user_prompt>`.
- **Budgeting:** Enforces `payload_tokens_max` (default 2500 tokens). On overflow, degrades by dropping older history turns, then summary, then safe truncation.

### 5. Exit Check Results
- All unit and integration tests pass across the entire workspace (29 tests).
- Verified payload builder against 3 real repositories on this machine (`/Users/mohamad_mac/tools/ai-prompt-formatter` - 145 tokens, `/Users/mohamad_mac/tools/prompt-fmt` - 161 tokens, `/Users/mohamad_mac/chino_backend` - 122 tokens). All strictly within the 2500 token budget.
- Redaction verified via unit test suite and end-to-end CLI check.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass with 0 warnings.

## Phase 3: LLM Layer (2026-10-11)

### 1. Prompts
- **Decision:** Versioned prompt templates created:
  - `prompts/meta_prompt.v1.md`: Meta-prompt instructing LLM to rewrite developer prompts into strict JSON briefs according to `WriterOutput` schema. Emphasizes imperative task, verbatim identifiers/paths, minimal context, and treating user input as untrusted.
  - `prompts/compactor.v1.md`: Session compactor prompt producing concise summaries (≤600 chars) focused on goals, decisions, constraints, and open issues.
- **Loading:** `PromptLoader` in `pfmt-llm` embeds defaults at compile-time while allowing runtime file overrides.

### 2. Groq Client and Model Fallback Chain
- **Decision:** Implemented `GroqClient` with a multi-model fallback chain:
  - Default chain: `openai/gpt-oss-120b` -> `qwen/qwen3.8-27b` -> `openai/gpt-oss-20b`.
  - Structured output: Requests `response_format` type `json_schema` with the schemars schema; falls back to `json_object` with embedded schema if 400 is returned.
  - Resiliency: Exponential backoff retries on HTTP 429 and 5xx errors.
  - Validation: Parses raw JSON (stripping markdown fences if present) and runs strict schema and boundary validation via `pfmt_core::WriterOutput::validate()`.

### 3. Decider Trait and Gate Implementations
- **Decision:** Created `Decider` trait for typed probabilistic gating decisions:
  - `needs_history`
  - `already_structured`
  - `injection_risk`
  - `extra_rules_requested`
  - `extra_report_requested`
  - `filter_candidate_relevance(prompt, candidates)`
- **Implementations:**
  - `JevDecider`: Calls TypeSafe Jev API (`https://api.typesafe.ai/v1/systemone`) using `noul` decisions.
  - `GroqJsonDecider`: Robust fallback implementation invoking fast Groq model with structured JSON decision schema.

### 4. Output Verifier and Candidates Jailing
- **Decision:** Implemented `OutputVerifier` ensuring the writer LLM adheres to contract:
  - Dropping any paths in `relevant_files` that were not present in the filtered `<candidates>` list.
  - Rejecting empty tasks or unparsable structures.

### 5. Exit Check Results
- All unit, stub mock, and live integration tests pass (33 tests across workspace).
- `test_live_jev_smoke`: Live smoke test verified real TypeSafe Jev API connectivity and probability evaluation.
- `test_live_groq_smoke`: Live smoke test verified Groq API key resolution from macOS Keychain (`pfmt-groq`) and successfully received structured JSON output from `openai/gpt-oss-120b`.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass cleanly.

## Phase 4: Orchestrator and Doctor (2026-10-11)

### 1. Orchestrator Pipeline
- **Decision:** Implemented `pipeline::execute_enhance_pipeline` in `crates/cli`:
  - Enforces total time budget (`budget.total_ms`, default 9000ms); on timeout, automatically degrades to no-LLM output.
  - Sequentially manages stages: `input` -> `idempotence` -> `redact` -> `context` -> `history` -> `gate` -> `writer` -> `verify` -> `render`.
  - Implements the strict degradation ladder per Section 4.1:
    - `input` failure: aborts with exit code 1, stderr status `empty input: nothing to do`.
    - `idempotence`: exits code 0 with input verbatim and stderr status `unchanged`.
    - `redact` failure: skips network stages and falls back to no-LLM skeleton with degradation banner.
    - `context` failure: continues without `<repo>`, prepends degradation banner, exit code 10.
    - `history` failure: continues without `<history>`, prepends degradation banner, exit code 10.
    - `gate` failure: falls back to heuristics (candidates capped at 10, regex injection check), prepends degradation banner, exit code 10.
    - `writer` failure: model fallback chain exhausted or timed out; degrades to raw prompt in default profile skeleton with fixed rules/report and degradation banner, exit code 10.
    - `verify` failure: marks output "unverified" in banner, exit code 10.
    - `render` failure: aborts with exit code 1.
  - Stderr notification: emits one-line human-readable status for macOS notification (e.g. `OK (enhanced via openai/gpt-oss-120b, app=Antigravity)` or `DEGRADED: failed stages [...] (app=...)`).

### 2. Diagnostics: `pfmt doctor`
- **Decision:** Implemented `doctor::run_doctor`:
  - **Configuration:** Verifies config file resolution (local `config.toml` -> `~/.config/pfmt/config.toml` -> embedded default) and default profile validity.
  - **API Keys:** Audits Groq key (`keychain:pfmt-groq` or `GROQ_API_KEY`) and TypeSafe Jev key (`keychain:pfmt-jev` or `TYPESAFE_API_KEY`) with masked output.
  - **macOS Integration:** Tests clipboard availability (`pbcopy`, `pbpaste`) and checks Accessibility / System Events permissions via `osascript` (detecting current front app).
  - **Agent Stores:** Scans local filesystem for all 4 supported agent environments (`~/.gemini/antigravity`, `ai.opencode.desktop`, `freebuff`, `~/.hermes`).
  - **Model Reachability:** Performs live ping checks against Groq (`https://api.groq.com/openai/v1/models`) and TypeSafe Jev (`https://api.typesafe.ai/v1/systemone`) measuring roundtrip latency.
  - Emits recommendations for any warning or failure and exits 0 on system health or 1 on critical missing dependencies.

### 3. Exit Check Results
- Verified that **every stage can be forced to fail** and output is still valid XML with the correct degradation banner and exit code:
  - Unit/integration test suite in `crates/cli/tests/pipeline_stages_test.rs` covers all 12 failure and override permutations.
  - Full workspace test suite passes: **51 tests passed, 0 failed**.
  - `pfmt doctor` runs cleanly and reports all systems operational (`[PASS]` across config, keys, permissions, stores, and model endpoints).
  - Live CLI enhancement verified end-to-end:
    - `echo "" | pfmt enhance --app Antigravity` -> exits code 1 (abort).
    - `echo "<task>...</task><acceptance_criteria>...</acceptance_criteria>" | pfmt enhance --app Antigravity` -> exits code 0 (`unchanged`).
    - `echo "..." | pfmt enhance --app Antigravity --no-llm` -> exits code 0 (`OK (enhanced via no-LLM)`).
    - `echo "..." | pfmt enhance --app Antigravity` -> exits code 0, enhanced via `openai/gpt-oss-120b` with contextual `<task>`, `<context>`, `<acceptance_criteria>`, `<rules>`, and `<report>`.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass cleanly.

## Phase 5: Logs, Review, Replay (2026-10-11)

### 1. Incremental Log Writer
- **Decision:** Implemented `IncrementalLogger` in `crates/cli::logging`:
  - Path format: `./daily_logs/YYYY-MM-DD/<app>_<session?>_<topic>_<HHMMSS>.json` where `topic` is a slugified topic (≤40 chars) generated by `slugify_topic`.
  - Incremental persistence: The log file is initialized at the very start of the pipeline with redacted input, timestamp, and exit code 1 (pending/aborted).
  - As each stage executes, `record_stage` logs execution status, elapsed milliseconds, tokens in/out, and errors.
  - On run completion, `finalize` records final XML output, degradation banner, and final exit code.
  - Automatic retention cleanup: Removes log subdirectories older than `logs.retention_days` (default 30 days).

### 2. Metric Review Tool: `pfmt review`
- **Decision:** Implemented `review::run_review` and `review::print_review_report`:
  - Parses JSON logs across `./daily_logs/` bounded by optional `--from` and `--to` date flags.
  - Aggregates overall success, degradation, and abort rates.
  - Stage performance table: total calls, failure rates, average latency, and p95 latency per stage.
  - Verifier metrics: rejections and hallucinated files cleaned.
  - Identifies top 3 slowest runs as worst cases.

### 3. Execution Replay Tool: `pfmt replay`
- **Decision:** Implemented `replay::run_replay`:
  - Reads a specified log JSON file, extracts the original prompt input, and re-executes it through the enhancement pipeline.
  - Outputs a comparison summary (exit codes, degraded status, latency, model used) and displays the newly enhanced XML output.

### 4. Golden Prompt Evaluation Suite
- **Decision:** Created regression test dataset in `tests/eval/golden_prompts.json` covering:
  - Bug fixes
  - Feature additions
  - Refactors mentioning files
  - Prompt injection attempts
  - Pre-structured idempotent inputs
- Validated via `review_replay_test.rs` ensuring consistent schema adherence and idempotent detection.

### 5. Exit Check Results
- Verified that **every run, including aborted and degraded ones, leaves a valid log file on disk**.
- `pfmt review` aggregates metrics and displays formatted stage summaries on real logs.
- `pfmt replay` re-executed a real logged prompt and produced a detailed comparison against live LLM output.
- All workspace tests pass: **56 tests passed, 0 failed**.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass cleanly with 0 warnings.

## Phase 6: History and Watcher Daemon (2026-10-11)

### 1. SQLite Database Architecture (`crates/history::db`)
- **Decision:** Implemented `HistoryDb` with bundled SQLite in WAL mode (`journal_mode=WAL`, `synchronous=NORMAL`) at `~/.config/pfmt/history.db`.
- **Schema:**
  - `sessions`: `(id PK, agent, session_id, front_app, model, project_path, source_ref, source_cursor, summary, summary_model, summary_version, status, created_at, updated_at, summarized_at, UNIQUE(agent, session_id))`
  - `jobs`: `(id PK, session_ref FK, kind, state, attempts, last_error, enqueued_at, started_at, finished_at)`
- **Debounced Enqueueing:** `enqueue_compact_job` ensures at most 1 queued/running job exists per session, returning the existing job ID if already pending.

### 2. Native Agent Adapters (`crates/history::adapters`)
- **Decision:** Built pluggable adapters implementing `AgentAdapter`:
  - `AntigravityAdapter`: Scans `~/.gemini/antigravity/brain/*/logs/transcript.jsonl`. Parses `USER_INPUT` (detecting session activation on 1st prompt) and `PLANNER_RESPONSE` (`status: DONE` signals assistant turn completion).
  - `OpenCodeAdapter`: Detects SQLite stores in `~/Library/Application Support/ai.opencode.desktop`.
  - `FreebuffAdapter`: Monitors application support path `~/Library/Application Support/freebuff`.
  - `HermesAdapter`: Monitors `~/.hermes/` session databases.

### 3. Compactor Worker (`crates/history::compactor`)
- **Decision:** Configurable model (`[llm.compactor]`, default `qwen/qwen3.8-27b`).
- **Prompt:** `prompts/compactor.v1.md` generates strict JSON: `{"summary": "string ≤600 chars"}`.
- **Redaction & Jailing:** All conversation turns are redacted via `redact_secrets` before being sent to LLM.
- **Summary Caching:** Compactor updates SQLite `sessions.summary`. `pfmt enhance` only *reads* the cached summary and never blocks on background compactor jobs.

### 4. Watcher Daemon (`crates/history::watcher`)
- **Decision:** `pfmt watch` runs `WatcherDaemon` in the background with graceful shutdown on `Ctrl+C`.
- **Polling Loop:** Scans active agent directories every `watcher.poll_interval_ms` (default 3000ms), upserts active sessions to SQLite, and drains compactor job queue.

### 5. Exit Check Results
- Integrated history query into CLI pipeline: `pfmt enhance` reads latest cached summary for the current app and project path, seamlessly injecting `<history>` into the payload.
- All workspace tests pass: **56 tests passed, 0 failed** across all crates (`pfmt-core`, `pfmt-context`, `pfmt-llm`, `pfmt-history`, `pfmt-cli`).
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass with 0 warnings.
- `pfmt doctor` reports all checks passing including agent store paths and endpoint reachability.

## Phase 7: Karabiner and Shell Integration (2026-10-11)

### 1. macOS Shell Runner (`scripts/ai_run.sh`)
- **Decision:** Built single executable bash wrapper invoked by Karabiner-Elements shortcut.
- **Lockfile & Concurrency:** Uses atomic directory locking (`/tmp/pfmt.lock`) with PID monitoring and 30-second stale lock expiration to prevent multiple overlapping executions.
- **Modifier Clearance:** Inserts a 0.4s debounce sleep to let physical key modifiers release before keystroke synthesis.
- **Frontmost App Detection:** Uses `osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true'` to dynamically supply the `--app` argument to `pfmt enhance`.
- **Clipboard Preservation:** Backs up user clipboard before running; on abort or empty selection, original clipboard contents are safely restored.
- **Degradation & Notification:**
  - Exit 0 (Success): Pastes enhanced brief, sends macOS notification with sound `Glass`.
  - Exit 10 (Degraded): Pastes degraded brief with banner, sends alert notification with sound `Basso`.
  - Exit 1 (Abort): Does **not** paste; leaves original prompt untouched in agent text box and notifies user of failure reason.

### 2. Karabiner Complex Modifications (`scripts/karabiner/pfmt.json`)
- **Decision:** Configured standard manipulator binding `Control + Option + P` to trigger `scripts/ai_run.sh`.
- **Validation:** JSON syntax validated against Karabiner-Elements schema.

### 3. LaunchAgent Background Service (`scripts/launchd/pfmt-watch.plist`)
- **Decision:** Created user LaunchAgent plist for `com.pfmt.watch`:
  - `KeepAlive = true` to maintain watcher daemon execution.
  - Redirects stdout/stderr to `~/.config/pfmt/watch.stdout.log` and `watch.stderr.log`.
  - Validated via macOS `plutil -lint`.

### 4. Installer (`scripts/install.sh`)
- **Decision:** Automates building release binary (`cargo build --release`), copying to `~/.local/bin/pfmt` and `~/.local/bin/ai_run.sh`, installing Karabiner rules into `~/.config/karabiner/assets/complex_modifications`, registering LaunchAgent, and running `pfmt doctor`.

### 5. Exit Check Results
- Validated `scripts/karabiner/pfmt.json` and `scripts/launchd/pfmt-watch.plist` syntax.
- Verified lockfile acquisition, stale lock recovery, and binary discovery via `tests/test_ai_run.sh`.
- All workspace tests pass: **56 tests passed, 0 failed**.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass cleanly.

## Phase 8: Per-App Validation and Release (2026-10-11)

### 1. Release Binary Optimization
- **Decision:** Built production release binary via `cargo build --release`.
- **Metrics:**
  - Standalone binary size: `9.1 MB` (`target/release/pfmt`).
  - Zero external runtime shared-library dependencies beyond macOS libSystem/Security framework.
  - Bundled SQLite, regex, JSON schema generation, and Groq/Jev networking.

### 2. Multi-Agent Validation Matrix
- Tested 4 target coding environments across 3 realistic developer prompt archetypes (Bug fix with errors, New feature request, Refactoring with candidate files):
  1. **Antigravity (`--app Antigravity`):**
     - Prompt: *"Fix the SQLite database lock panic in crates/history/src/db.rs: sqlite3_step returned busy timeout after 5000ms"*
     - Result: Exit code 0, enhanced via `openai/gpt-oss-120b`, strictly formatted XML with verbatim paths and testable criteria.
  2. **OpenCode (`--app OpenCode`):**
     - Prompt: *"Add support for export to markdown in daily logs with --format md flag"*
     - Result: Exit code 0, enhanced via `openai/gpt-oss-120b`, contextual command extraction (`cargo test`, `cargo clippy`).
  3. **Freebuff (`--app Freebuff`):**
     - Prompt: *"Refactor parse_and_validate in crates/llm/src/groq.rs to use clean_json_str and reduce code duplication"*
     - Result: Exit code 0, preserved exact candidate file paths and function identifiers.
  4. **Hermes (`--app Hermes`):**
     - Prompt: *"Optimize prompt formatting performance to under 200ms. Make sure we never use regex for simple string prefix checks."*
     - Result: Exit code 0, derived acceptance criteria and rules.

### 3. Log Review & Replay Verification
- `pfmt review` successfully aggregated the 6 real enhancement runs with 100% success rate, 0% degradation, and precise latency distribution.
- `pfmt replay` verified reproducibility on a live log, replaying the SQLite bugfix prompt and verifying matching output schema with latency improvement (2781ms vs 5666ms).

### 4. Exit Check Results
- Release binary compiled and verified: `target/release/pfmt` (9.1M).
- `target/release/pfmt doctor` reports all checks passing `[PASS]`.
- All 56 workspace tests pass: `cargo test --workspace -- --test-threads=1`.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass with 0 errors and 0 warnings.
- End-to-end prompt enhancement verified across Antigravity, OpenCode, Freebuff, and Hermes.




