use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStack {
    pub language: String,
    pub frameworks: Vec<String>,
    pub test_cmd: Option<String>,
    pub lint_cmd: Option<String>,
}

impl RepoStack {
    /// Detect language, framework, test and lint commands from files in the repo root.
    pub fn detect(repo_path: &Path) -> Self {
        let mut languages = Vec::new();
        let mut frameworks = Vec::new();
        let mut test_cmd = None;
        let mut lint_cmd = None;

        // Rust
        if repo_path.join("Cargo.toml").exists() {
            languages.push("Rust");
            test_cmd = Some("cargo test".to_string());
            lint_cmd = Some("cargo clippy".to_string());
        }

        // Node / JavaScript / TypeScript
        let package_json_path = repo_path.join("package.json");
        if package_json_path.exists() {
            if repo_path.join("tsconfig.json").exists() {
                languages.push("TypeScript");
            } else {
                languages.push("JavaScript");
            }

            // Check package manager
            let pkg_mgr = if repo_path.join("pnpm-lock.yaml").exists() {
                "pnpm"
            } else if repo_path.join("yarn.lock").exists() {
                "yarn"
            } else if repo_path.join("bun.lockb").exists() || repo_path.join("bun.lock").exists() {
                "bun"
            } else {
                "npm"
            };

            // Inspect package.json for frameworks and scripts
            if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(deps) = json
                        .get("dependencies")
                        .or_else(|| json.get("devDependencies"))
                    {
                        if deps.get("react").is_some() {
                            frameworks.push("React".to_string());
                        }
                        if deps.get("next").is_some() {
                            frameworks.push("Next.js".to_string());
                        }
                        if deps.get("vue").is_some() {
                            frameworks.push("Vue".to_string());
                        }
                    }

                    if test_cmd.is_none() && json.pointer("/scripts/test").is_some() {
                        test_cmd = Some(format!("{} test", pkg_mgr));
                    }
                    if lint_cmd.is_none() && json.pointer("/scripts/lint").is_some() {
                        lint_cmd = Some(format!("{} run lint", pkg_mgr));
                    }
                }
            }

            if test_cmd.is_none() {
                test_cmd = Some(format!("{} test", pkg_mgr));
            }
        }

        // Python
        if repo_path.join("pyproject.toml").exists()
            || repo_path.join("requirements.txt").exists()
            || repo_path.join("Pipfile").exists()
            || repo_path.join("setup.py").exists()
        {
            languages.push("Python");
            if test_cmd.is_none() {
                test_cmd = Some("pytest".to_string());
            }
            if lint_cmd.is_none() {
                lint_cmd = Some("ruff check".to_string());
            }
        }

        // Go
        if repo_path.join("go.mod").exists() {
            languages.push("Go");
            if test_cmd.is_none() {
                test_cmd = Some("go test ./...".to_string());
            }
            if lint_cmd.is_none() {
                lint_cmd = Some("golangci-lint run".to_string());
            }
        }

        // Shell
        if languages.is_empty() {
            languages.push("Shell / Unknown");
        }

        Self {
            language: languages.join(", "),
            frameworks,
            test_cmd,
            lint_cmd,
        }
    }

    /// Formats stack information as a concise string for <repo>.
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("stack: {}", self.language)];
        if !self.frameworks.is_empty() {
            parts.push(format!("frameworks: {}", self.frameworks.join(", ")));
        }
        if let Some(ref t) = self.test_cmd {
            parts.push(format!("test: {}", t));
        }
        if let Some(ref l) = self.lint_cmd {
            parts.push(format!("lint: {}", l));
        }
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_rust_stack() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"foo\"").unwrap();

        let stack = RepoStack::detect(dir.path());
        assert_eq!(stack.language, "Rust");
        assert_eq!(stack.test_cmd.as_deref(), Some("cargo test"));
        assert_eq!(stack.lint_cmd.as_deref(), Some("cargo clippy"));
        assert!(stack.summary().contains("stack: Rust"));
    }

    #[test]
    fn test_detect_python_stack() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "pytest\n").unwrap();

        let stack = RepoStack::detect(dir.path());
        assert_eq!(stack.language, "Python");
        assert_eq!(stack.test_cmd.as_deref(), Some("pytest"));
    }
}
