use crate::adapter::Turn;
use crate::db::HistoryDb;
use pfmt_core::{redact_secrets, resolve_api_key, Config};
use pfmt_llm::{clean_json_str, GroqClient, PromptLoader};
use serde::{Deserialize, Serialize};
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

        Ok(summary)
    }

    /// Processes one claimed compact job from the queue.
    pub async fn process_job(
        &self,
        job: &crate::db::CompactJob,
        turns: &[Turn],
    ) -> Result<(), CompactorError> {
        match self.compact_session(job.session_ref, turns).await {
            Ok(_) => {
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
