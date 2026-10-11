# Rust Crates Pinning & Research

Access Date: 2026-10-11
Sources: crates.io index

## Pinned Dependencies
- `tokio`: 1.39+ (features = ["rt-multi-thread", "macros", "fs", "io-util", "process", "time"])
- `reqwest`: 0.12+ (features = ["json", "rustls-tls"], default-features = false)
- `rusqlite`: 0.31+ (features = ["bundled", "bundled-sqlcipher-vendored-openssl"])
- `schemars`: 0.8.21 (JSON schema generation for strict OpenAI / Groq structured outputs)
- `serde`: 1.0 (features = ["derive"])
- `serde_json`: 1.0
- `clap`: 4.5 (features = ["derive"])
- `notify`: 6.1 (cross-platform filesystem watcher / FSEvents)
- `sysinfo`: 0.31 (process checking and memory statistics)
- `ignore`: 0.4 (gitignore-aware directory traversal)
- `keyring`: 2.1 or 3.0 (macOS keychain integration)
- `tracing`: 0.1
- `tracing-subscriber`: 0.3 (features = ["env-filter", "json"])
- `thiserror`: 1.0 or 2.0 (typed domain error definitions)
- `anyhow`: 1.0 (application-level context errors)
- `toml`: 0.8
- `dirs`: 5.0
