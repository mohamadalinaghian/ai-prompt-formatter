# Agent Stores & Adapters Research

Access Date: 2026-10-11
Inspected on host: macOS (Darwin)

## 1. Antigravity (`com.google.antigravity`)
- Application: `/Applications/Antigravity.app` (v2.22.0)
- Process detection: `pgrep -x Antigravity` or `pgrep -f "Antigravity.app"`
- Data Root: `~/.gemini/antigravity/` and `~/Library/Application Support/Antigravity/`
- Active Sessions & Transcripts:
  - SQLite databases: `~/.gemini/antigravity/conversations/<conversation_id>.db`
  - Summaries: `~/.gemini/antigravity/conversation_summaries.db`
  - JSONL transcripts: `~/.gemini/antigravity/brain/<conversation_id>/.system_generated/logs/transcript.jsonl`
- Session identification: `<conversation_id>` UUID
- Assistant message end: In transcript JSONL, step type `MODEL` or `PLANNER_RESPONSE` with `status: "DONE"`.
- First prompt: First line in `transcript.jsonl` with `type: "USER_INPUT"`.

## 2. OpenCode (`ai.opencode.desktop`)
- Application: `/Applications/OpenCode.app` (v1.18.35)
- Process detection: `pgrep -x OpenCode` or `pgrep -f opencode`
- Data Root:
  - App Support: `~/Library/Application Support/ai.opencode.desktop/`
  - CLI / Config: `~/.config/opencode/` and `~/.opencode/`
  - Workspace sessions: `~/Library/Application Support/ai.opencode.desktop/opencode.workspace.*.dat` and `drafts.sqlite`
- First prompt / assistant end: Tracked via SQLite or session JSON data files.

## 3. Freebuff (`com.freebuff.desktop`)
- Application: `/Applications/Freebuff.app` (v0.0.169)
- Process detection: `pgrep -x Freebuff`
- Data Root: `~/Library/Application Support/Freebuff/`
- Browser / Chat partitions: `~/Library/Application Support/Freebuff/Partitions/freebuff-browser-*-default/`
- Storage: Chromium LevelDB / IndexedDB storage under `IndexedDB/` and `Local Storage/leveldb`.

## 4. Hermes (`com.nousresearch.hermes.setup`)
- Application: `/Applications/Hermes.app` (v0.0.1) and CLI `~/.local/bin/hermes`
- Process detection: `pgrep -x hermes` or `pgrep -x Hermes`
- Data Root: `~/.hermes/` and `~/Library/Application Support/Hermes/`
- State: SQLite databases `~/.hermes/kanban.db`, `~/.hermes/shared-state.db`, JSON files `~/.hermes/gateway_state.json`.
