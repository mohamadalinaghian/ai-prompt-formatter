use pfmt_history::adapters::AntigravityAdapter;
use pfmt_history::db::HistoryDb;
use pfmt_history::AgentAdapter;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_history_db_session_lifecycle_and_jobs() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_history.db");
    let db = HistoryDb::open_path(&db_path).expect("Failed to open test history db");

    // 1. Initial upsert upon first prompt detection
    let session_pk = db
        .upsert_session(
            "Antigravity",
            "conv-123",
            Some("Antigravity"),
            Some("gemini"),
            Some("/Users/test/project"),
            Some("/path/to/transcript.jsonl"),
            Some("10"),
        )
        .expect("Failed to upsert session");
    assert!(session_pk > 0);

    // Initial state: no summary
    let summary = db
        .get_session_summary("Antigravity", "conv-123")
        .expect("query error");
    assert_eq!(summary, None);

    // 2. Enqueue compact job (debounced)
    let job_id_first = db
        .enqueue_compact_job(session_pk)
        .expect("enqueue job failed");
    assert!(job_id_first > 0);

    // Duplicate enqueue should be debounced and return the same existing job ID
    let job_id_second = db
        .enqueue_compact_job(session_pk)
        .expect("second enqueue failed");
    assert_eq!(
        job_id_first, job_id_second,
        "Duplicate enqueue should return existing job ID"
    );

    // 3. Claim job
    let claimed = db.claim_next_compact_job().expect("claim job failed");
    assert!(claimed.is_some());
    let job = claimed.unwrap();
    assert_eq!(job.session_ref, session_pk);
    assert_eq!(job.agent, "Antigravity");
    assert_eq!(job.session_id, "conv-123");

    // 4. Update summary and finish job
    let test_summary =
        "Refactored login module to support OAuth2. Open issue: token refresh on 401.";
    db.update_summary(session_pk, test_summary, "qwen3.8-27b", "v1")
        .expect("update summary failed");
    db.finish_job(job.id).expect("finish job failed");

    // 5. Verify summary retrieval
    let fetched = db
        .get_session_summary("Antigravity", "conv-123")
        .expect("fetch failed");
    assert_eq!(fetched, Some(test_summary.to_string()));

    // Verify get_latest_summary with project path
    let latest = db
        .get_latest_summary("Antigravity", Some("/Users/test/project"))
        .expect("latest failed");
    assert_eq!(latest, Some(test_summary.to_string()));
}

#[test]
fn test_antigravity_adapter_parsing() {
    let dir = tempdir().unwrap();
    let brain_dir = dir.path().join("brain/session-abc/.system_generated/logs");
    std::fs::create_dir_all(&brain_dir).unwrap();

    let transcript_file = brain_dir.join("transcript.jsonl");
    let mut f = File::create(&transcript_file).unwrap();

    // Turn 1: user prompt
    writeln!(
        f,
        r#"{{"type":"USER_INPUT","status":"DONE","content":"Please implement SQLite storage"}}"#
    )
    .unwrap();

    // Turn 2: assistant response running
    writeln!(
        f,
        r#"{{"type":"PLANNER_RESPONSE","status":"RUNNING","content":"Setting up rusqlite..."}}"#
    )
    .unwrap();

    let adapter = AntigravityAdapter::new(dir.path().to_path_buf());
    let parsed = adapter
        .parse_transcript("session-abc", &transcript_file)
        .expect("Should parse session");

    assert_eq!(parsed.session_id, "session-abc");
    assert!(parsed.has_first_prompt);
    assert!(
        !parsed.assistant_message_finished,
        "Status was RUNNING, not DONE"
    );

    // Add assistant completion
    writeln!(
        f,
        r#"{{"type":"PLANNER_RESPONSE","status":"DONE","content":"SQLite setup completed."}}"#
    )
    .unwrap();

    let parsed_done = adapter
        .parse_transcript("session-abc", &transcript_file)
        .expect("Should parse session again");
    assert!(parsed_done.assistant_message_finished, "Now status is DONE");
    assert_eq!(parsed_done.turns.len(), 3);
}

#[test]
fn test_antigravity_live_brain_scan() {
    let default_adapter = AntigravityAdapter::default();
    let discovered = default_adapter.scan_sessions();
    // In our live environment, brain has sessions
    println!("Discovered {} live Antigravity sessions", discovered.len());
    for s in &discovered {
        assert!(s.has_first_prompt);
        assert!(!s.turns.is_empty());
    }
}

#[test]
fn test_write_session_summary_file_and_get_session_id() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_summary.db");
    let db = HistoryDb::open_path(&db_path).expect("open db");

    let session_pk = db
        .upsert_session(
            "Antigravity",
            "session-xyz-456",
            Some("Antigravity"),
            Some("gemini"),
            Some("/path/to/project"),
            None,
            None,
        )
        .expect("upsert");

    let session_id = db.get_session_id(session_pk).expect("get_session_id");
    assert_eq!(session_id, Some("session-xyz-456".to_string()));

    let summaries_dir = dir.path().join("summaries");
    let summary_text = "Goal: Fix compiler error in pipeline. Decided to use tokio::join.";
    let file_path =
        pfmt_history::write_session_summary_file(&summaries_dir, "session-xyz-456", summary_text)
            .expect("write summary file");

    assert!(file_path.exists());
    let contents = std::fs::read_to_string(&file_path).expect("read summary file");
    assert_eq!(contents, summary_text);
    assert_eq!(
        file_path.file_name().unwrap().to_str().unwrap(),
        "session-xyz-456.summary.md"
    );
}
