use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitRepoInfo {
    pub branch: Option<String>,
    pub changed_files: Vec<String>,
}

impl GitRepoInfo {
    /// Collect current branch and changed file names from the git repository.
    /// Captures file names only (no contents, no tree).
    pub fn collect(repo_path: &Path) -> Self {
        let branch = Self::get_current_branch(repo_path);
        let changed_files = Self::get_changed_files(repo_path);

        Self {
            branch,
            changed_files,
        }
    }

    /// Read the branch from .git/HEAD or git command.
    fn get_current_branch(repo_path: &Path) -> Option<String> {
        let head_path = repo_path.join(".git/HEAD");
        if head_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&head_path) {
                let trimmed = content.trim();
                if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                    return Some(branch.trim().to_string());
                }
                // Detached HEAD
                if trimmed.len() >= 7 {
                    return Some(trimmed[..7].to_string());
                }
            }
        }

        // Fallback to git CLI
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo_path)
            .output()
            .ok()?;

        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }

        None
    }

    /// Run git status --porcelain to collect changed file names.
    fn get_changed_files(repo_path: &Path) -> Vec<String> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(repo_path)
            .output();

        let mut files = Vec::new();
        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.len() > 3 {
                        // Porcelain status is "XY filename" or "XY orig -> renamed"
                        let file_part = &trimmed[2..].trim();
                        let target_file = if let Some((_, dest)) = file_part.split_once("->") {
                            dest.trim()
                        } else {
                            file_part
                        };
                        if !target_file.is_empty() {
                            files.push(target_file.to_string());
                        }
                    }
                    // Cap at 50 to prevent token budget explosion
                    if files.len() >= 50 {
                        break;
                    }
                }
            }
        }
        files
    }

    /// Formats git information as a concise summary for <repo>.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref b) = self.branch {
            parts.push(format!("branch: {}", b));
        }
        if !self.changed_files.is_empty() {
            parts.push(format!("changed files: {}", self.changed_files.join(", ")));
        }
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_info_on_current_repo() {
        // Run on the current repo
        let info = GitRepoInfo::collect(Path::new("."));
        // Even if not a git repo or on detached HEAD, should not panic
        let summary = info.summary();
        println!("Git summary: {}", summary);
    }
}
