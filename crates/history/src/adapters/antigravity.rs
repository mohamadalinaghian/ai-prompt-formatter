use crate::adapter::{is_process_running, AgentAdapter, DiscoveredSession, Turn};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub struct AntigravityAdapter {
    root_dir: PathBuf,
}

impl Default for AntigravityAdapter {
    fn default() -> Self {
        let dir = dirs::home_dir()
            .map(|h| h.join(".gemini/antigravity"))
            .unwrap_or_else(|| PathBuf::from("."));
        Self { root_dir: dir }
    }
}

impl AntigravityAdapter {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn parse_transcript(
        &self,
        conv_id: &str,
        transcript_path: &Path,
    ) -> Option<DiscoveredSession> {
        let file = File::open(transcript_path).ok()?;
        let reader = BufReader::new(file);

        let mut turns = Vec::new();
        let mut has_first_prompt = false;
        let mut assistant_message_finished = false;
        let mut line_count = 0;

        for line_res in reader.lines() {
            line_count += 1;
            let line = line_res.ok()?;
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let step_type = val["type"].as_str().unwrap_or("");
                let status = val["status"].as_str().unwrap_or("");
                let content = val["content"].as_str().unwrap_or("");

                if step_type == "USER_INPUT" {
                    has_first_prompt = true;
                    assistant_message_finished = false; // Waiting for model response
                    if !content.trim().is_empty() {
                        turns.push(Turn {
                            role: "user".to_string(),
                            content: content.trim().to_string(),
                        });
                    }
                } else if step_type == "PLANNER_RESPONSE" || step_type == "MODEL" {
                    if status == "DONE" {
                        assistant_message_finished = true;
                    }
                    if !content.trim().is_empty() {
                        turns.push(Turn {
                            role: "assistant".to_string(),
                            content: content.trim().to_string(),
                        });
                    }
                }
            }
        }

        if !has_first_prompt {
            return None;
        }

        Some(DiscoveredSession {
            session_id: conv_id.to_string(),
            front_app: "Antigravity".to_string(),
            model: Some("gemini".to_string()),
            project_path: None,
            source_ref: transcript_path.to_string_lossy().to_string(),
            source_cursor: line_count.to_string(),
            has_first_prompt,
            assistant_message_finished,
            turns,
        })
    }
}

impl AgentAdapter for AntigravityAdapter {
    fn agent_name(&self) -> &'static str {
        "Antigravity"
    }

    fn is_running(&self) -> bool {
        is_process_running("Antigravity")
    }

    fn scan_sessions(&self) -> Vec<DiscoveredSession> {
        let mut sessions = Vec::new();
        let brain_dir = self.root_dir.join("brain");
        if !brain_dir.exists() {
            return sessions;
        }

        if let Ok(entries) = std::fs::read_dir(&brain_dir) {
            for entry in entries.flatten() {
                let conv_path = entry.path();
                if conv_path.is_dir() {
                    let conv_id = conv_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    let transcript = conv_path.join(".system_generated/logs/transcript.jsonl");
                    if transcript.exists() {
                        if let Some(session) = self.parse_transcript(&conv_id, &transcript) {
                            sessions.push(session);
                        }
                    }
                }
            }
        }

        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_antigravity_transcript_parsing() {
        let dir = tempdir().unwrap();
        let brain = dir.path().join("brain/conv-1/.system_generated/logs");
        std::fs::create_dir_all(&brain).unwrap();

        let transcript = brain.join("transcript.jsonl");
        let mut file = File::create(&transcript).unwrap();

        // 1. Initial user input
        writeln!(
            file,
            r#"{{"type":"USER_INPUT","status":"DONE","content":"Add login button"}}"#
        )
        .unwrap();

        // 2. Assistant response done
        writeln!(
            file,
            r#"{{"type":"PLANNER_RESPONSE","status":"DONE","content":"Added LoginButton component in src/login.rs"}}"#
        )
        .unwrap();

        let adapter = AntigravityAdapter::new(dir.path().to_path_buf());
        let sessions = adapter.scan_sessions();
        assert_eq!(sessions.len(), 1);

        let s = &sessions[0];
        assert_eq!(s.session_id, "conv-1");
        assert!(s.has_first_prompt);
        assert!(s.assistant_message_finished);
        assert_eq!(s.turns.len(), 2);
        assert_eq!(s.turns[0].content, "Add login button");
        assert_eq!(
            s.turns[1].content,
            "Added LoginButton component in src/login.rs"
        );
    }
}
