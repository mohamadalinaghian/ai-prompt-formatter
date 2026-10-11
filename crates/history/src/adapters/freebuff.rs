use crate::adapter::{is_process_running, AgentAdapter, DiscoveredSession};
use std::path::PathBuf;

pub struct FreebuffAdapter {
    root_dir: PathBuf,
}

impl Default for FreebuffAdapter {
    fn default() -> Self {
        let dir = dirs::data_dir()
            .map(|d| d.join("freebuff"))
            .unwrap_or_else(|| PathBuf::from("."));
        Self { root_dir: dir }
    }
}

impl FreebuffAdapter {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }
}

impl AgentAdapter for FreebuffAdapter {
    fn agent_name(&self) -> &'static str {
        "Freebuff"
    }

    fn is_running(&self) -> bool {
        is_process_running("Freebuff") || is_process_running("freebuff")
    }

    fn scan_sessions(&self) -> Vec<DiscoveredSession> {
        let mut sessions = Vec::new();
        if !self.root_dir.exists() {
            return sessions;
        }

        let partitions = self.root_dir.join("Partitions");
        if partitions.exists() {
            if let Ok(entries) = std::fs::read_dir(&partitions) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("default")
                            .to_string();
                        sessions.push(DiscoveredSession {
                            session_id: name,
                            front_app: "Freebuff".to_string(),
                            model: Some("freebuff-default".to_string()),
                            project_path: None,
                            source_ref: path.to_string_lossy().to_string(),
                            source_cursor: "0".to_string(),
                            has_first_prompt: true,
                            assistant_message_finished: true,
                            turns: Vec::new(),
                        });
                    }
                }
            }
        }

        sessions
    }
}
