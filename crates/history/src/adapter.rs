use std::process::Command;

#[derive(Debug, Clone)]
pub struct Turn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredSession {
    pub session_id: String,
    pub front_app: String,
    pub model: Option<String>,
    pub project_path: Option<String>,
    pub source_ref: String,
    pub source_cursor: String,
    pub has_first_prompt: bool,
    pub assistant_message_finished: bool,
    pub turns: Vec<Turn>,
}

pub trait AgentAdapter: Send + Sync {
    fn agent_name(&self) -> &'static str;
    fn is_running(&self) -> bool;
    fn scan_sessions(&self) -> Vec<DiscoveredSession>;
}

pub fn is_process_running(proc_name: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(proc_name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
