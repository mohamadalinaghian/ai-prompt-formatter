# rusqlite Research & Best Practices

Access Date: 2026-10-11
Source URL: https://docs.rs/rusqlite/latest/rusqlite/

## 1. Features
- `bundled`: Compiles modern SQLite from source, eliminating macOS system dylib version discrepancies.
- Version: `0.32` / `0.33` / `0.40`.

## 2. WAL Mode & Concurrency
- Enable WAL mode immediately after connection creation:
  `conn.pragma_update(None, "journal_mode", &"WAL")?;`
- WAL allows concurrent readers alongside a single active writer without reader blocking.
- Set busy timeout:
  `conn.busy_timeout(std::time::Duration::from_millis(5000))?;`
- Use immediate transactions for writes: `BEGIN IMMEDIATE` or serializing writes through a single channel writer in Rust.

## 3. Schema & Migrations
- Schema from Section 4.8:
  ```sql
  CREATE TABLE IF NOT EXISTS sessions (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      agent TEXT NOT NULL,
      session_id TEXT NOT NULL,
      front_app TEXT,
      model TEXT,
      project_path TEXT,
      source_ref TEXT,
      source_cursor TEXT,
      summary TEXT,
      summary_model TEXT,
      summary_version TEXT,
      status TEXT NOT NULL CHECK(status IN ('pending_first_prompt', 'active', 'ended')),
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      summarized_at TEXT,
      UNIQUE(agent, session_id)
  );

  CREATE TABLE IF NOT EXISTS jobs (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      session_ref INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
      kind TEXT NOT NULL CHECK(kind IN ('compact')),
      state TEXT NOT NULL CHECK(state IN ('queued', 'running', 'done', 'failed')),
      attempts INTEGER NOT NULL DEFAULT 0,
      last_error TEXT,
      enqueued_at TEXT NOT NULL,
      started_at TEXT,
      finished_at TEXT
  );
  ```
