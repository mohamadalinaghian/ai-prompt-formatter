use pfmt_core::AppContext;
use std::path::{Path, PathBuf};

/// Resolves application context (project_dir, session_id, sources) by inspecting
/// registered agent adapters and the filesystem from `cwd`.
pub fn resolve_probe(app: &str, cwd: &Path) -> AppContext {
    let mut sources = Vec::new();
    let mut project_dir: Option<PathBuf> = None;
    let mut session_id: Option<String> = None;

    let app_lower = app.to_lowercase().replace([' ', '-'], "");

    // 1. Inspect adapters for active sessions
    let adapters = pfmt_history::default_adapters();
    for adapter in adapters {
        let agent_name = adapter.agent_name().to_lowercase().replace([' ', '-'], "");
        if agent_name == app_lower
            || app_lower.contains(&agent_name)
            || agent_name.contains(&app_lower)
        {
            let sessions = adapter.scan_sessions();
            // Look for session matching cwd
            let mut matched_session = None;
            for sess in &sessions {
                if let Some(ref p) = sess.project_path {
                    let sess_path = Path::new(p);
                    if sess_path == cwd || cwd.starts_with(sess_path) || sess_path.starts_with(cwd)
                    {
                        matched_session = Some(sess.clone());
                        break;
                    }
                }
            }

            let target_session =
                matched_session.or_else(|| sessions.into_iter().find(|s| s.has_first_prompt));

            if let Some(sess) = target_session {
                session_id = Some(sess.session_id);
                sources.push(format!("adapter:{}", adapter.agent_name()));
                if let Some(p) = sess.project_path {
                    let pb = PathBuf::from(p);
                    if pb.exists() {
                        project_dir = Some(pb);
                        sources.push("project:adapter".to_string());
                    }
                }
                break;
            }
        }
    }

    // 2. If project_dir not determined from adapter, resolve via git or project workspace root
    if project_dir.is_none() {
        let base = if cwd.is_relative() {
            std::env::current_dir().unwrap_or_else(|_| cwd.to_path_buf())
        } else {
            cwd.to_path_buf()
        };

        let mut curr = base;
        loop {
            if curr.join(".git").exists()
                || curr.join("Cargo.toml").exists()
                || curr.join("package.json").exists()
                || curr.join("go.mod").exists()
                || curr.join("pyproject.toml").exists()
            {
                project_dir = Some(curr);
                sources.push("project:workspace".to_string());
                break;
            }
            if !curr.pop() {
                break;
            }
        }

        if project_dir.is_none() && cwd.exists() {
            project_dir = Some(cwd.to_path_buf());
            sources.push("project:cwd".to_string());
        }
    }

    AppContext {
        app_name: app.to_string(),
        project_dir,
        session_id,
        sources,
    }
}
