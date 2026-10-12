use pfmt_core::redact_secrets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    Fresh,
    Continue,
}

impl SessionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionMode::Fresh => "fresh",
            SessionMode::Continue => "continue",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriterPayload {
    pub formatted: String,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct PayloadInput<'a> {
    pub agent: &'a str,
    pub mode: SessionMode,
    pub repo_summary: Option<&'a str>,
    pub candidates: &'a [String],
    pub history_summary: Option<&'a str>,
    pub history_turns: &'a [String],
    pub user_prompt: &'a str,
    pub max_tokens: usize,
}

/// Estimate tokens using standard ~3.8 chars per token heuristic for code/English text.
pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    // Conservative estimation: ~3.5 chars per token
    (char_count * 10).div_ceil(35)
}

/// Builds the strict Writer Payload per Section 4.5, respecting token budget.
pub fn build_writer_payload(input: &PayloadInput) -> WriterPayload {
    let agent_clean = redact_secrets(input.agent.trim());
    let prompt_clean = redact_secrets(input.user_prompt.trim());

    // 1. <meta>
    let meta_tag = format!(
        "<meta>agent={}, mode={}</meta>",
        agent_clean,
        input.mode.as_str()
    );

    // 2. <repo>
    let repo_tag = match input.repo_summary {
        Some(s) if !s.trim().is_empty() => {
            let redacted_repo = redact_secrets(s.trim());
            format!("<repo>{}</repo>", redacted_repo)
        }
        _ => String::new(),
    };

    // 3. <candidates>
    let candidates_tag = if !input.candidates.is_empty() {
        let items: Vec<String> = input
            .candidates
            .iter()
            .map(|c| redact_secrets(c.trim()))
            .collect();
        format!("<candidates>{}</candidates>", items.join(", "))
    } else {
        String::new()
    };

    // 4. <history> (only if mode == continue and history is present)
    let mut history_str = String::new();
    if input.mode == SessionMode::Continue {
        let mut parts = Vec::new();
        if let Some(summary) = input.history_summary {
            let sum_trimmed = summary.trim();
            if !sum_trimmed.is_empty() {
                parts.push(format!("Summary: {}", sum_trimmed));
            }
        }
        for (i, turn) in input.history_turns.iter().rev().take(3).rev().enumerate() {
            let turn_redacted = redact_secrets(turn.trim());
            let capped_turn = if turn_redacted.chars().count() > 400 {
                let end = turn_redacted
                    .char_indices()
                    .map(|(idx, _)| idx)
                    .nth(400)
                    .unwrap_or(turn_redacted.len());
                format!("{}...", &turn_redacted[..end])
            } else {
                turn_redacted
            };
            parts.push(format!("Turn {}: {}", i + 1, capped_turn));
        }

        if !parts.is_empty() {
            history_str = format!("<history>\n{}\n</history>", parts.join("\n"));
        }
    }

    // 5. <user_prompt>
    let prompt_tag = format!("<user_prompt>\n{}\n</user_prompt>", prompt_clean);

    // Assemble payload
    let mut sections = vec![meta_tag];
    if !repo_tag.is_empty() {
        sections.push(repo_tag);
    }
    if !candidates_tag.is_empty() {
        sections.push(candidates_tag);
    }
    if !history_str.is_empty() {
        sections.push(history_str);
    }
    sections.push(prompt_tag);

    let mut full_payload = sections.join("\n\n");
    let mut tokens = estimate_tokens(&full_payload);

    // Budget enforcement: if over budget, trim history turns first, then repo
    if tokens > input.max_tokens && input.mode == SessionMode::Continue {
        if let Some(summary) = input.history_summary {
            let trimmed_hist = format!(
                "<history>\nSummary: {}\n</history>",
                redact_secrets(summary.trim())
            );
            sections[3] = trimmed_hist;
            full_payload = sections.join("\n\n");
            tokens = estimate_tokens(&full_payload);
        }
    }

    // If still over budget, truncate prompt safely
    if tokens > input.max_tokens {
        let budget_chars = input.max_tokens.saturating_mul(3);
        let safe_prompt = if prompt_clean.chars().count() > budget_chars {
            let end = prompt_clean
                .char_indices()
                .map(|(idx, _)| idx)
                .nth(budget_chars)
                .unwrap_or(prompt_clean.len());
            format!("{}... [TRUNCATED FOR BUDGET]", &prompt_clean[..end])
        } else {
            prompt_clean
        };
        let last_idx = sections.len() - 1;
        sections[last_idx] = format!("<user_prompt>\n{}\n</user_prompt>", safe_prompt);
        full_payload = sections.join("\n\n");
        tokens = estimate_tokens(&full_payload);
    }

    WriterPayload {
        formatted: full_payload,
        estimated_tokens: tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_builder_fresh_mode() {
        let input = PayloadInput {
            agent: "antigravity",
            mode: SessionMode::Fresh,
            repo_summary: Some("stack: Rust; test: cargo test"),
            candidates: &["src/main.rs".to_string()],
            history_summary: None,
            history_turns: &[],
            user_prompt: "Add feature X",
            max_tokens: 2500,
        };
        let payload = build_writer_payload(&input);

        assert!(payload
            .formatted
            .contains("<meta>agent=antigravity, mode=fresh</meta>"));
        assert!(payload
            .formatted
            .contains("<repo>stack: Rust; test: cargo test</repo>"));
        assert!(payload
            .formatted
            .contains("<candidates>src/main.rs</candidates>"));
        assert!(!payload.formatted.contains("<history>"));
        assert!(payload
            .formatted
            .contains("<user_prompt>\nAdd feature X\n</user_prompt>"));
        assert!(payload.estimated_tokens <= 2500);
    }

    #[test]
    fn test_payload_builder_continue_mode_with_history() {
        let input = PayloadInput {
            agent: "opencode",
            mode: SessionMode::Continue,
            repo_summary: Some("stack: TypeScript"),
            candidates: &[],
            history_summary: Some("User worked on authentication flow"),
            history_turns: &[
                "turn 1: let's add login".to_string(),
                "turn 2: login added".to_string(),
            ],
            user_prompt: "Now add logout button",
            max_tokens: 2500,
        };
        let payload = build_writer_payload(&input);

        assert!(payload
            .formatted
            .contains("<meta>agent=opencode, mode=continue</meta>"));
        assert!(payload.formatted.contains("<history>"));
        assert!(payload
            .formatted
            .contains("Summary: User worked on authentication flow"));
        assert!(payload
            .formatted
            .contains("Turn 1: turn 1: let's add login"));
    }

    #[test]
    fn test_payload_builder_redacts_secrets() {
        // Assembled at runtime so no real-key-shaped literal lives in the source tree
        // (GitHub push protection blocks `gsk_<32+ alnum>` even in test fixtures).
        let fake_key = format!("gsk_{}", "A1b2C3d4".repeat(5));
        let input = PayloadInput {
            agent: "antigravity",
            mode: SessionMode::Fresh,
            repo_summary: None,
            candidates: &[],
            history_summary: None,
            history_turns: &[],
            user_prompt: &format!("Please use {fake_key} for auth"),
            max_tokens: 2500,
        };
        let payload = build_writer_payload(&input);

        assert!(!payload.formatted.contains(&fake_key));
        assert!(payload.formatted.contains("[REDACTED_GROQ_KEY]"));
    }
}
