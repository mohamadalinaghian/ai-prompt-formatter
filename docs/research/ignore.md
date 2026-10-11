# Ignore Crate Research

Access Date: 2026-10-11
Sources:
- docs.rs: https://docs.rs/ignore/latest/ignore/
- crates.io: https://crates.io/crates/ignore

## Usage & API
- Primary builder: `ignore::WalkBuilder::new(repo_path)`.
- Respects: `.gitignore`, global git ignore, and nested `.gitignore` files.
- Configuration:
  - `standard_filters(true)` respects `.gitignore` rules.
  - `hidden(true)` ignores hidden files like `.git`.
  - `parents(true)` respects parent directory ignore rules.
- Candidate file filtering:
  - When candidate file paths are extracted from user prompts, they must be canonicalized and checked against `repo_root` to prevent path traversal (`../` directory escapes).
  - An ignore matcher or walker ensures ignored files (e.g. `node_modules/`, `.git/`, build artifacts) are pruned from `<candidates>`.
