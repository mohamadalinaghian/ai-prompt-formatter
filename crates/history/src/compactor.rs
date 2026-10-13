use crate::adapter::Turn;
use crate::db::HistoryDb;
use pfmt_core::{redact_secrets, resolve_api_key, Config};
use pfmt_llm::{clean_json_str, GroqClient, PromptLoader};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompactorError {
    #[error("API key resolution error: {0}")]
    Key(String),
    #[error("Groq client error: {0}")]
    Groq(String),
    #[error("Failed to parse compactor response: {0}")]
    Parse(String),
    #[error("Database error: {0}")]
    Db(#[from] crate::db::DbError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompactorOutput {
    pub summary: String,
}

pub struct CompactorWorker {
    db: HistoryDb,
    model: String,
    api_key_source: String,
}

impl CompactorWorker {
    pub fn new(db: HistoryDb, config: &Config) -> Self {
        Self {
            db,
            model: config.llm.compactor.model.clone(),
            api_key_source: config.llm.writer.api_key_source.clone(),
        }
    }

    /// Compacts a sequence of turns into a ≤600 char telegraphic summary.
    pub async fn compact_session(
        &self,
        session_id_pk: i64,
        turns: &[Turn],
    ) -> Result<String, CompactorError> {
        if turns.is_empty() {
            return Ok(String::new());
        }

        // 1. Redact turns and build session transcript
        let mut formatted_turns = Vec::new();
        for turn in turns {
            let role_label = if turn.role == "user" {
                "user"
            } else {
                "assistant"
            };
            let clean_content = redact_secrets(&turn.content);
            formatted_turns.push(format!("[{}]: {}", role_label, clean_content));
        }

        // Trim oldest turns if over ~12000 chars (conservative limit for compacting)
        while formatted_turns.len() > 1 && formatted_turns.join("\n").len() > 12000 {
            formatted_turns.remove(0);
        }

        let transcript = formatted_turns.join("\n");
        let system_prompt = PromptLoader::load_compactor_prompt();

        let api_key = resolve_api_key(&self.api_key_source, "GROQ_API_KEY")
            .map_err(|e| CompactorError::Key(e.to_string()))?;

        let client = GroqClient::new(api_key, vec![self.model.clone()], 8000);

        let (raw_json, model_used) = client
            .execute_json_completion(&system_prompt, &transcript)
            .await
            .map_err(|e| CompactorError::Groq(e.to_string()))?;

        let clean_json = clean_json_str(&raw_json);
        let parsed: CompactorOutput = serde_json::from_str(clean_json)
            .map_err(|e| CompactorError::Parse(format!("{}: {}", e, clean_json)))?;

        let mut summary = parsed.summary.trim().to_string();
        if summary.len() > 600 {
            summary = summary.chars().take(600).collect();
        }

        // Save to SQLite
        self.db
            .update_summary(session_id_pk, &summary, &model_used, "v1")?;

        if let Ok(Some(session_id)) = self.db.get_session_id(session_id_pk) {
            let _ = write_session_summary_file(&default_summaries_dir(), &session_id, &summary);
        }

        Ok(summary)
    }

    /// Processes one claimed compact job from the queue.
    pub async fn process_job(
        &self,
        job: &crate::db::CompactJob,
        turns: &[Turn],
    ) -> Result<(), CompactorError> {
        match self.compact_session(job.session_ref, turns).await {
            Ok(summary) => {
                let _ =
                    write_session_summary_file(&default_summaries_dir(), &job.session_id, &summary);
                self.db.finish_job(job.id)?;
                Ok(())
            }
            Err(e) => {
                self.db.fail_job(job.id, &e.to_string())?;
                Err(e)
            }
        }
    }
}

/// Resolves the default summaries directory: `~/.config/pfmt/summaries`.
pub fn default_summaries_dir() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("pfmt").join("summaries"))
        .unwrap_or_else(|| PathBuf::from(".pfmt").join("summaries"))
}

/// Writes a session summary markdown file: `<session_id>.summary.md`.
pub fn write_session_summary_file(
    summaries_dir: &Path,
    session_id: &str,
    summary: &str,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(summaries_dir)?;
    let safe_id: String = session_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let file_path = summaries_dir.join(format!("{}.summary.md", safe_id));
    std::fs::write(&file_path, summary)?;
    Ok(file_path)
}
