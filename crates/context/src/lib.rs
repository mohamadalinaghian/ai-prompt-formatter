pub mod candidates;
pub mod git;
pub mod payload;
pub mod stack;

pub use candidates::extract_candidate_files;
pub use git::GitRepoInfo;
pub use payload::{
    build_writer_payload, estimate_tokens, PayloadInput, SessionMode, WriterPayload,
};
pub use stack::RepoStack;

use std::path::Path;

/// Collect full repository context summary (stack + git info).
pub fn collect_repo_summary(repo_path: &Path) -> String {
    let stack = RepoStack::detect(repo_path);
    let git = GitRepoInfo::collect(repo_path);

    let stack_summary = stack.summary();
    let git_summary = git.summary();

    if git_summary.is_empty() {
        stack_summary
    } else {
        format!("{}; {}", stack_summary, git_summary)
    }
}
