# Session Reading & Reuse Survey

Access Date: 2026-10-11

## Survey of Existing Tools:
1. `cass`:
   - Evaluated for session extraction. Findings: `cass` crates on crates.io relate to Cassandra CQL drivers, not conversational session reading.
   - Decision: Skip.
2. `txcript`:
   - Transactional logging library. Lightweight, but standard Rust filesystem writes + SQLite WAL + `tracing` provide direct, zero-overhead control.
   - Decision: Skip external dependency; implement straightforward typed SQLite + daily JSON logging.
3. `ccusage` and similar adapters:
   - Tooling exists for code-completion token tracking, but lacks adapter support for the 4 target agents (Antigravity, Freebuff, OpenCode, Hermes).
   - Decision: Implement clean, decoupled `Adapter` traits in `crates/history/src/adapters/`.
