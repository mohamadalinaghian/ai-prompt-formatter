use pfmt_core::{render, Degradation, Profile, WriterOutput};

fn sample_profile() -> Profile {
    Profile::new(
        "- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.\n\
         - Follow existing patterns in the repo; check neighboring files first.\n\
         - No mock data or swallowed errors; handle failures explicitly.\n\
         - Ask before: destructive actions, schema or public API changes.\n\
         - For other ambiguity, pick the most reasonable option and list it in the report.",
        "KEEP YOUR RESPONSE EXTREMELY SHORT.\n\
         1. 🎯 **Key Changes**: (bullet points, max 1 line each)\n\
         2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)\n\
         3. 📋 **Remaining Tasks**: (what is left to do, actionable items)",
    )
}

#[test]
fn test_golden_standard_output() {
    let output = WriterOutput {
        task: "Refactor auth handler to use Bearer tokens".to_string(),
        context: vec![
            "Repo uses axum 0.7".to_string(),
            "Tokens are validated via jwt crate".to_string(),
        ],
        relevant_files: vec!["src/auth.rs".to_string(), "src/main.rs".to_string()],
        acceptance_criteria: vec![
            "Missing token returns 401 Unauthorized".to_string(),
            "Invalid token returns 401 Unauthorized".to_string(),
            "Valid token populates RequestContext".to_string(),
        ],
        extra_rules: vec!["Add unit tests in auth_test.rs".to_string()],
        extra_report: vec!["4. 🔐 Security Impact: none".to_string()],
        assumptions: vec![],
        out_of_scope: Vec::new(),
        verify: Vec::new(),
        confidence: Vec::new(),
    };

    let profile = sample_profile();
    let rendered = render(&output, &profile, None, None, 0.6);

    let expected = r#"<task>
Refactor auth handler to use Bearer tokens
</task>

<acceptance_criteria>
- Missing token returns 401 Unauthorized
- Invalid token returns 401 Unauthorized
- Valid token populates RequestContext
</acceptance_criteria>

<context>
- Repo uses axum 0.7
- Tokens are validated via jwt crate
</context>

<scope>
<relevant_files>src/auth.rs, src/main.rs</relevant_files>
</scope>

<rules>
- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- For other ambiguity, pick the most reasonable option and list it in the report.
- Add unit tests in auth_test.rs
</rules>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🎯 **Key Changes**: (bullet points, max 1 line each)
2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)
3. 📋 **Remaining Tasks**: (what is left to do, actionable items)
4. 🔐 Security Impact: none
</report>"#;

    assert_eq!(rendered.trim(), expected.trim());
}

#[test]
fn test_golden_degraded_output() {
    let output = WriterOutput {
        task: "Fix navbar z-index on mobile".to_string(),
        context: vec![],
        relevant_files: vec![],
        acceptance_criteria: vec![],
        extra_rules: vec![],
        extra_report: vec![],
        assumptions: vec![],
        out_of_scope: Vec::new(),
        verify: Vec::new(),
        confidence: Vec::new(),
    };

    let degradation = Degradation::new(
        vec!["writer (Groq timeout)".to_string()],
        "your raw prompt was wrapped in the default skeleton and NOT enhanced.",
    );

    let profile = sample_profile();
    let rendered = render(&output, &profile, Some(&degradation), None, 0.6);

    let expected = r#"⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️
Failed stages: writer (Groq timeout)
Result: your raw prompt was wrapped in the default skeleton and NOT enhanced.
Delete this banner before sending it to the agent.
⚠️⚠️⚠️ ─────────────────────────────────────────── ⚠️⚠️⚠️

<task>
Fix navbar z-index on mobile
</task>

<rules>
- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- For other ambiguity, pick the most reasonable option and list it in the report.
</rules>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🎯 **Key Changes**: (bullet points, max 1 line each)
2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)
3. 📋 **Remaining Tasks**: (what is left to do, actionable items)
</report>"#;

    assert_eq!(rendered.trim(), expected.trim());
}
