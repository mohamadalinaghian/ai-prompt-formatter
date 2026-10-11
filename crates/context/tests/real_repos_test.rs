use pfmt_context::{
    build_writer_payload, collect_repo_summary, extract_candidate_files, SessionMode,
};
use std::path::Path;

#[test]
fn test_payload_under_budget_on_three_real_repos() {
    let real_repos = [
        (
            "/Users/mohamad_mac/tools/ai-prompt-formatter",
            "Refactor Cargo.toml and check crates/core/src/lib.rs for unused imports",
            "antigravity",
        ),
        (
            "/Users/mohamad_mac/tools/prompt-fmt",
            "Update ai_fmt_prompt.py and ai_run.sh to handle timeout errors properly",
            "opencode",
        ),
        (
            "/Users/mohamad_mac/chino_backend",
            "Review database migrations and main entry point",
            "hermes",
        ),
    ];

    let max_budget_tokens = 2500;

    for (repo_path_str, prompt, agent) in &real_repos {
        let repo_path = Path::new(repo_path_str);
        assert!(
            repo_path.exists(),
            "Real repo path must exist on this machine: {}",
            repo_path_str
        );

        // 1. Collect repo summary (stack + git)
        let repo_summary = collect_repo_summary(repo_path);
        assert!(
            !repo_summary.is_empty(),
            "Repo summary for {} must not be empty",
            repo_path_str
        );
        println!("Repo [{}]: {}", repo_path_str, repo_summary);

        // 2. Extract candidate files from prompt
        let candidates = extract_candidate_files(prompt, repo_path);
        println!("Candidates [{}]: {:?}", repo_path_str, candidates);

        // 3. Build payload in continue mode with realistic history
        let history_summary = "User initialized task and refactored core structures.";
        let history_turns = vec![
            "User: start implementation of the module".to_string(),
            "Assistant: implemented base types and initial unit tests".to_string(),
            "User: now check error cases".to_string(),
        ];

        let payload_input = pfmt_context::PayloadInput {
            agent,
            mode: SessionMode::Continue,
            repo_summary: Some(&repo_summary),
            candidates: &candidates,
            history_summary: Some(history_summary),
            history_turns: &history_turns,
            user_prompt: prompt,
            max_tokens: max_budget_tokens,
        };

        let payload = build_writer_payload(&payload_input);

        // 4. Assert token budget
        assert!(
            payload.estimated_tokens <= max_budget_tokens,
            "Payload for {} exceeded token budget! (estimated: {}, max: {})",
            repo_path_str,
            payload.estimated_tokens,
            max_budget_tokens
        );

        // 5. Assert structure
        assert!(payload.formatted.contains("<meta>"));
        assert!(payload.formatted.contains("<repo>"));
        assert!(payload.formatted.contains("<user_prompt>"));
        assert!(payload.formatted.contains(prompt));

        println!(
            "Payload for {} passed: {} estimated tokens (budget: {})",
            repo_path_str, payload.estimated_tokens, max_budget_tokens
        );
    }
}
