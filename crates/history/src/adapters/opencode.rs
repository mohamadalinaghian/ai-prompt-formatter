use crate::adapter::{is_process_running, AgentAdapter, DiscoveredSession};
use std::path::PathBuf;

pub struct OpenCodeAdapter {
    root_dir: PathBuf,
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        let dir = dirs::data_dir()
            .map(|d| d.join("ai.opencode.desktop"))
            .unwrap_or_else(|| PathBuf::from("."));
        Self { root_dir: dir }
    }
}

impl OpenCodeAdapter {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }
}

impl AgentAdapter for OpenCodeAdapter {
    fn agent_name(&self) -> &'static str {
        "OpenCode"
    }

    fn is_running(&self) -> bool {
        is_process_running("OpenCode") || is_process_running("opencode")
    }

    fn scan_sessions(&self) -> Vec<DiscoveredSession> {
        let mut sessions = Vec::new();
        if !self.root_dir.exists() {
            return sessions;
        }

        // Scan for session data files or SQLite drafts in root_dir
        if let Ok(entries) = std::fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("dat")
                    || path.file_name().and_then(|n| n.to_str()) == Some("drafts.sqlite")
                {
                    let session_id = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("default")
                        .to_string();

                    // If file is non-empty, record discovered active session
                    if let Ok(meta) = path.metadata() {
                        if meta.len() > 0 {
                            sessions.push(DiscoveredSession {
                                session_id,
                                front_app: "OpenCode".to_string(),
                                model: Some("opencode-default".to_string()),
                                project_path: None,
                                source_ref: path.to_string_lossy().to_string(),
                                source_cursor: meta.len().to_string(),
                                has_first_prompt: true,
                                assistant_message_finished: true,
                                turns: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        sessions
    }
}
