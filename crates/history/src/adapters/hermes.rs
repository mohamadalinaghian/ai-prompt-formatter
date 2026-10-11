use crate::adapter::{is_process_running, AgentAdapter, DiscoveredSession};
use std::path::PathBuf;

pub struct HermesAdapter {
    root_dir: PathBuf,
}

impl Default for HermesAdapter {
    fn default() -> Self {
        let dir = dirs::home_dir()
            .map(|h| h.join(".hermes"))
            .unwrap_or_else(|| PathBuf::from("."));
        Self { root_dir: dir }
    }
}

impl HermesAdapter {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }
}

impl AgentAdapter for HermesAdapter {
    fn agent_name(&self) -> &'static str {
        "Hermes"
    }

    fn is_running(&self) -> bool {
        is_process_running("hermes") || is_process_running("Hermes")
    }

    fn scan_sessions(&self) -> Vec<DiscoveredSession> {
        let mut sessions = Vec::new();
        if !self.root_dir.exists() {
            return sessions;
        }

        let kanban_db = self.root_dir.join("kanban.db");
        if kanban_db.exists() {
            if let Ok(meta) = kanban_db.metadata() {
                sessions.push(DiscoveredSession {
                    session_id: "hermes-default".to_string(),
                    front_app: "Hermes".to_string(),
                    model: Some("hermes-model".to_string()),
                    project_path: None,
                    source_ref: kanban_db.to_string_lossy().to_string(),
                    source_cursor: meta.len().to_string(),
                    has_first_prompt: true,
                    assistant_message_finished: true,
                    turns: Vec::new(),
                });
            }
        }

        sessions
    }
}
