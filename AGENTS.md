# AGENTS.md — ai-prompt-formatter (`pfmt`)

Rust workspace. macOS-only tool: `pfmt enhance` rewrites a stdin prompt to XML on stdout; status goes to stderr. Binary: `crates/cli` (`[[bin]] pfmt`).

## Workspace layout

- `crates/core` — `WriterOutput` schema/validation, XML `render`, `redact_secrets`, `Config`/`Profile` loading, exit codes, degradation banner.
- `crates/llm` — Groq writer chain, Jev/Groq deciders, prompt loader, output verifier.
- `crates/context` — repo/stack/git summary, candidate-file extraction with `repo_root` jailing, token-budgeted payload builder.
- `crates/history` — SQLite + adapters (Antigravity / OpenCode / Freebuff / Hermes) + compactor + `pfmt watch` daemon. `enhance` only *reads* the cached summary; never blocks on compaction.
- `crates/cli` — `main.rs` subcommands (`enhance --app <name> [--no-llm]`, `watch`, `review`, `replay <log>`, `doctor`) and `pipeline.rs` orchestrator (`input → idempotence → redact → context → history → gate → writer → verify → render`).
- `prompts/*.v1.md`, `profiles/default.toml`, root `config.toml`, `scripts/` (`ai_run.sh`, `install.sh`, `karabiner/`, `launchd/`), `tests/eval/golden_prompts.json`.

## Commands (use these exactly)

- `cargo build` / `cargo test` alone only cover `crates/cli` (`default-members` in root `Cargo.toml`). For the full workspace always add `--workspace`:
  - `cargo test --workspace -- --test-threads=1` (single-threaded; history SQLite tests assume it)
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo fmt --check` (write: `cargo fmt`)
- Focused runs: `cargo test -p pfmt-core|pfmt-context|pfmt-llm|pfmt-history|pfmt-cli`; single test: `cargo test -p <pkg> <test_name>`.
- Shell integration: build debug binary first, then `bash tests/test_ai_run.sh` (validates Karabiner JSON, `plutil -lint` on plist, lockfile logic, binary discovery).
- Manual check: `echo "x" | cargo run -p pfmt-cli -- enhance --app test --no-llm` is the deterministic offline path (valid XML, exit 0). Without `--no-llm` it may call live Groq/Jev if keys resolve.
- `cargo run -p pfmt-cli -- doctor` for env diagnostics; `./scripts/install.sh` does release build + `~/.local/bin` install + Karabiner/launchd setup + doctor.

## Gotchas

- Exit codes are contract: `0` = ok/unchanged, `10` = degraded (valid XML + banner on stdout, reason in `failed_stages`/stderr), `1` = abort (empty input, render failure; stdout empty). `scripts/ai_run.sh` pastes on 0/10 but must NOT paste on 1.
- Stage failures degrade, never crash, per `[stages.*] on_fail` (`input`/`render` = abort, rest default skip). Tests force each failure via `StageFailureHook` in `crates/cli/tests/pipeline_stages_test.rs` — follow that pattern when touching `pipeline.rs`.
- Config/profile resolution order matters: `~/.config/pfmt/config.toml` → `./config.toml` → embedded default (`DEFAULT_CONFIG_TOML` in `crates/core/src/config.rs`); profile likewise `~/.config/pfmt/profiles/default.toml` → `./profiles/default.toml` → embedded. The checked-in `config.toml` is sparse; full defaults live in code.
- Secrets: call `redact_secrets` before any network send. Keys resolve `keychain:pfmt-groq`/`keychain:pfmt-jev` with fallback to `GROQ_API_KEY`/`TYPESAFE_API_KEY`; missing key = writer/gate degradation, not abort. Never commit real keys; `live_smoke_test.rs` skips without keys — do not "fix" it to require network.
- Budgets are enforced: total pipeline `budget.total_ms` (9000) degrades to no-LLM on timeout; writer per-call `timeout_ms` (6000); payload capped at `payload_tokens_max` (2500) by dropping history then truncating. Writer chain order: `openai/gpt-oss-120b` → `qwen/qwen3.8-27b` → `openai/gpt-oss-20b` → `google/gemma-4-26b-a4b-it:free` → `nvidia/nemotron-3.5-lightning:free` → `google/gemma-4-31b-it:free`; `json_schema` falls back to `json_object` on 400.
- History DB is `~/.config/pfmt/history.db` (bundled SQLite, WAL). Logs write to `./daily_logs/` relative to CWD (gitignored) and are the input to `review`/`replay`. Idempotence: input already containing `<task>` + `<acceptance_criteria>` returns verbatim with exit 0.
- Candidate paths from the prompt must be verified to exist, pass `.gitignore`, and stay jailed under `repo_root`; verifier drops any `relevant_files` not in the filtered candidate list. Preserve this when editing `context`/`verifier`.
- macOS specifics: `keyring` uses `apple-native-keyring-store`; `doctor` shells to `pbcopy`/`pbpaste`/`osascript`; Karabiner binding is `Ctrl+Opt+P` with `/tmp/pfmt.lock` (30s stale expiry) and 0.4s modifier sleep in `ai_run.sh`. Don't port these to Linux assumptions.
