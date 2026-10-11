use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Session not found: id={0}")]
    SessionNotFound(i64),
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: i64,
    pub agent: String,
    pub session_id: String,
    pub front_app: Option<String>,
    pub model: Option<String>,
    pub project_path: Option<String>,
    pub source_ref: Option<String>,
    pub source_cursor: Option<String>,
    pub summary: Option<String>,
    pub summary_model: Option<String>,
    pub summary_version: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub summarized_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompactJob {
    pub id: i64,
    pub session_ref: i64,
    pub agent: String,
    pub session_id: String,
    pub source_ref: Option<String>,
    pub source_cursor: Option<String>,
    pub attempts: i64,
}

#[derive(Clone)]
pub struct HistoryDb {
    conn: Arc<Mutex<Connection>>,
}

impl HistoryDb {
    /// Opens the default history database at `~/.config/pfmt/history.db`.
    pub fn open_default() -> Result<Self, DbError> {
        let dir = dirs::config_dir()
            .map(|d| d.join("pfmt"))
            .unwrap_or_else(|| PathBuf::from(".pfmt"));
        std::fs::create_dir_all(&dir)?;
        Self::open_path(&dir.join("history.db"))
    }

    /// Opens or creates SQLite database at a specific path with WAL mode.
    pub fn open_path(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::init_connection(conn)
    }

    /// Opens an in-memory database (useful for unit testing).
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        Self::init_connection(conn)
    }

    fn init_connection(conn: Connection) -> Result<Self, DbError> {
        // 1. Enable WAL mode and set busy timeout
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;

        // 2. Run migrations
        conn.execute_batch(
            r#"
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

            CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);

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

            CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state, enqueued_at);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Creates or updates a session upon detecting prompts/activity.
    /// Only sets status to 'active' once first prompt occurs.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_session(
        &self,
        agent: &str,
        session_id: &str,
        front_app: Option<&str>,
        model: Option<&str>,
        project_path: Option<&str>,
        source_ref: Option<&str>,
        source_cursor: Option<&str>,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn.prepare(
            r#"
            INSERT INTO sessions (
                agent, session_id, front_app, model, project_path, source_ref,
                source_cursor, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?8)
            ON CONFLICT(agent, session_id) DO UPDATE SET
                front_app = COALESCE(?3, sessions.front_app),
                model = COALESCE(?4, sessions.model),
                project_path = COALESCE(?5, sessions.project_path),
                source_ref = COALESCE(?6, sessions.source_ref),
                source_cursor = COALESCE(?7, sessions.source_cursor),
                status = 'active',
                updated_at = ?8
            RETURNING id;
            "#,
        )?;

        let id: i64 = stmt.query_row(
            params![
                agent,
                session_id,
                front_app,
                model,
                project_path,
                source_ref,
                source_cursor,
                now
            ],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    /// Fetches the cached summary for a specific session.
    pub fn get_session_summary(
        &self,
        agent: &str,
        session_id: &str,
    ) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT summary FROM sessions WHERE agent = ?1 AND session_id = ?2 AND summary IS NOT NULL",
        )?;
        let summary: Option<String> = stmt
            .query_row(params![agent, session_id], |row| row.get(0))
            .optional()?;
        Ok(summary)
    }

    /// Fetches the latest summary for an agent, optionally matching a specific project path.
    pub fn get_latest_summary(
        &self,
        agent: &str,
        project_path: Option<&str>,
    ) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().unwrap();
        if let Some(path) = project_path {
            let mut stmt = conn.prepare(
                r#"
                SELECT summary FROM sessions
                WHERE agent = ?1 AND project_path = ?2 AND summary IS NOT NULL
                ORDER BY updated_at DESC LIMIT 1
                "#,
            )?;
            let summary: Option<String> = stmt
                .query_row(params![agent, path], |row| row.get(0))
                .optional()?;
            if summary.is_some() {
                return Ok(summary);
            }
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT summary FROM sessions
            WHERE agent = ?1 AND summary IS NOT NULL
            ORDER BY updated_at DESC LIMIT 1
            "#,
        )?;
        let summary: Option<String> = stmt
            .query_row(params![agent], |row| row.get(0))
            .optional()?;
        Ok(summary)
    }

    /// Updates the summary of a session.
    pub fn update_summary(
        &self,
        session_id_pk: i64,
        summary: &str,
        summary_model: &str,
        summary_version: &str,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            UPDATE sessions
            SET summary = ?1, summary_model = ?2, summary_version = ?3, summarized_at = ?4, updated_at = ?4
            WHERE id = ?5
            "#,
            params![summary, summary_model, summary_version, now, session_id_pk],
        )?;
        Ok(())
    }

    /// Enqueues a compact job with built-in debouncing (only 1 queued job allowed per session).
    pub fn enqueue_compact_job(&self, session_id_pk: i64) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        // Check if an existing queued or running job exists for this session
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM jobs WHERE session_ref = ?1 AND state IN ('queued', 'running')",
                params![session_id_pk],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(job_id) = existing {
            return Ok(job_id);
        }

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO jobs (session_ref, kind, state, attempts, enqueued_at) VALUES (?1, 'compact', 'queued', 0, ?2)",
            params![session_id_pk, now],
        )?;
        let job_id = conn.last_insert_rowid();
        Ok(job_id)
    }

    /// Claims the oldest queued compact job, marking it as running.
    pub fn claim_next_compact_job(&self) -> Result<Option<CompactJob>, DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let job_info: Option<(i64, i64, i64)> = conn
            .query_row(
                "SELECT id, session_ref, attempts FROM jobs WHERE state = 'queued' ORDER BY enqueued_at ASC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        if let Some((job_id, session_ref, attempts)) = job_info {
            conn.execute(
                "UPDATE jobs SET state = 'running', attempts = attempts + 1, started_at = ?1 WHERE id = ?2",
                params![now, job_id],
            )?;

            // Query session details
            let session_data = conn.query_row(
                "SELECT agent, session_id, source_ref, source_cursor FROM sessions WHERE id = ?1",
                params![session_ref],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )?;

            Ok(Some(CompactJob {
                id: job_id,
                session_ref,
                agent: session_data.0,
                session_id: session_data.1,
                source_ref: session_data.2,
                source_cursor: session_data.3,
                attempts: attempts + 1,
            }))
        } else {
            Ok(None)
        }
    }

    /// Marks a job as completed.
    pub fn finish_job(&self, job_id: i64) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE jobs SET state = 'done', finished_at = ?1 WHERE id = ?2",
            params![now, job_id],
        )?;
        Ok(())
    }

    /// Marks a job as failed with retry or terminal failure.
    pub fn fail_job(&self, job_id: i64, error: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE jobs SET state = 'failed', last_error = ?1, finished_at = ?2 WHERE id = ?3",
            params![error, now, job_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_lifecycle_and_jobs() {
        let db = HistoryDb::open_in_memory().expect("In-memory DB creation");

        // 1. Upsert session
        let sid = db
            .upsert_session(
                "Antigravity",
                "test-uuid-123",
                Some("Antigravity"),
                Some("gpt-oss-120b"),
                Some("/path/to/repo"),
                Some("transcript.jsonl"),
                Some("cursor_0"),
            )
            .expect("Upsert session");
        assert!(sid > 0);

        // 2. Summary initially None
        let summary = db
            .get_session_summary("Antigravity", "test-uuid-123")
            .expect("Get summary");
        assert!(summary.is_none());

        // 3. Enqueue compact job
        let jid1 = db.enqueue_compact_job(sid).expect("Enqueue job");
        let jid2 = db
            .enqueue_compact_job(sid)
            .expect("Enqueue job duplicate (debounce)");
        assert_eq!(jid1, jid2);

        // 4. Claim job
        let claimed = db.claim_next_compact_job().expect("Claim job");
        assert!(claimed.is_some());
        let job = claimed.unwrap();
        assert_eq!(job.id, jid1);
        assert_eq!(job.agent, "Antigravity");
        assert_eq!(job.session_id, "test-uuid-123");

        // 5. Update summary and finish job
        db.update_summary(
            sid,
            "Goal: fix auth bug. Decisions: use bearer tokens.",
            "qwen/qwen3.8-27b",
            "v1",
        )
        .expect("Update summary");
        db.finish_job(job.id).expect("Finish job");

        // 6. Verify summary is retrieved
        let updated = db
            .get_session_summary("Antigravity", "test-uuid-123")
            .expect("Get updated summary");
        assert_eq!(
            updated.as_deref(),
            Some("Goal: fix auth bug. Decisions: use bearer tokens.")
        );

        let latest = db
            .get_latest_summary("Antigravity", Some("/path/to/repo"))
            .expect("Get latest summary");
        assert_eq!(
            latest.as_deref(),
            Some("Goal: fix auth bug. Decisions: use bearer tokens.")
        );
    }
}
