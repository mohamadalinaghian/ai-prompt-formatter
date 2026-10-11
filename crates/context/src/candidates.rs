use ignore::gitignore::GitignoreBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Extracts candidate file paths from a prompt, verifies they exist in `repo_root`,
/// confirms they are jailed within the repo root, and filters out gitignored files.
pub fn extract_candidate_files(prompt: &str, repo_root: &Path) -> Vec<String> {
    let canonical_root = match repo_root.canonicalize() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    // Build gitignore matcher for the repo
    let mut gi_builder = GitignoreBuilder::new(repo_root);
    let gitignore_path = repo_root.join(".gitignore");
    if gitignore_path.exists() {
        let _ = gi_builder.add(&gitignore_path);
    }
    let gitignore = gi_builder
        .build()
        .unwrap_or_else(|_| GitignoreBuilder::new(repo_root).build().unwrap());

    let mut found_candidates = Vec::new();
    let mut seen = HashSet::new();

    // Split prompt into candidate tokens
    for raw_token in prompt.split_whitespace() {
        let clean = clean_path_token(raw_token);
        if clean.is_empty() || clean.len() < 3 {
            continue;
        }

        // Only consider tokens that look like paths or files (have an extension or directory separator)
        if !looks_like_path(&clean) {
            continue;
        }

        let rel_path = PathBuf::from(&clean);
        let full_path = repo_root.join(&rel_path);

        // Check if file exists
        if !full_path.exists() {
            continue;
        }

        // Canonicalize and enforce jail
        let canonical_file = match full_path.canonicalize() {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !canonical_file.starts_with(&canonical_root) {
            // Path traversal attempt outside repo root - drop
            continue;
        }

        // Calculate clean relative path from repo root
        let rel_clean = match canonical_file.strip_prefix(&canonical_root) {
            Ok(rel) => rel.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        // Check gitignore
        if gitignore
            .matched_path_or_any_parents(Path::new(&rel_clean), false)
            .is_ignore()
        {
            continue;
        }

        if seen.insert(rel_clean.clone()) {
            found_candidates.push(rel_clean);
            if found_candidates.len() >= 15 {
                break;
            }
        }
    }

    found_candidates
}

/// Strip wrapping punctuation, quotes, @, etc. from candidate path token.
fn clean_path_token(token: &str) -> String {
    let mut s = token.trim();
    // Strip leading @, `, ", '
    s = s.trim_start_matches(|c| {
        c == '@' || c == '`' || c == '"' || c == '\'' || c == '(' || c == '['
    });
    // Strip trailing commas, periods, quotes, etc.
    s = s.trim_end_matches(|c| {
        c == ','
            || c == '.'
            || c == ';'
            || c == ':'
            || c == ')'
            || c == ']'
            || c == '`'
            || c == '"'
            || c == '\''
    });
    s.to_string()
}

/// Heuristic: does token contain path separators or common file extensions?
fn looks_like_path(token: &str) -> bool {
    if token.contains('/') || token.contains('\\') {
        return true;
    }

    if let Some((_, ext)) = token.rsplit_once('.') {
        matches!(
            ext.to_lowercase().as_str(),
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "json"
                | "toml"
                | "yaml"
                | "yml"
                | "md"
                | "html"
                | "css"
                | "sh"
                | "sql"
                | "txt"
                | "lock"
                | "c"
                | "cpp"
                | "h"
        )
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_candidates_jailed_and_existing() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();

        // Create files inside repo
        let src_dir = repo_root.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(repo_root.join("Cargo.toml"), "").unwrap();

        let prompt = "Please check @src/main.rs and also update `Cargo.toml`. Also see ../outside.txt or fake.rs";
        let candidates = extract_candidate_files(prompt, repo_root);

        assert!(candidates.contains(&"src/main.rs".to_string()));
        assert!(candidates.contains(&"Cargo.toml".to_string()));
        assert!(!candidates.contains(&"fake.rs".to_string()));
    }

    #[test]
    fn test_extract_candidates_ignores_gitignored() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();

        // Create gitignore
        std::fs::write(repo_root.join(".gitignore"), "target/\n*.log\n").unwrap();

        let target_dir = repo_root.join("target");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("debug.rs"), "").unwrap();
        std::fs::write(repo_root.join("build.log"), "").unwrap();
        std::fs::write(repo_root.join("valid.rs"), "").unwrap();

        let prompt = "Check target/debug.rs and build.log and valid.rs";
        let candidates = extract_candidate_files(prompt, repo_root);

        assert!(candidates.contains(&"valid.rs".to_string()));
        assert!(!candidates.contains(&"target/debug.rs".to_string()));
        assert!(!candidates.contains(&"build.log".to_string()));
    }
}
