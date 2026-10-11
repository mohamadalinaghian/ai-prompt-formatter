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
