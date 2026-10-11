use pfmt_core::{
    is_idempotent, no_llm_wrap, render, Config, Degradation, OnFail, Profile, ValidationError,
    WriterOutput,
};

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
fn test_validation_success() {
    let out = WriterOutput {
        task: "Implement foo".to_string(),
        context: vec!["ctx1".to_string()],
        relevant_files: vec!["src/main.rs".to_string()],
        acceptance_criteria: vec!["criteria 1".to_string()],
        extra_rules: vec!["rule 1".to_string()],
        extra_report: vec!["report 1".to_string()],
        assumptions: vec!["assume X".to_string()],
    };
    assert!(out.validate().is_ok());
}

#[test]
fn test_validation_failure_task_too_long() {
    let long_task = "a".repeat(1201);
    let out = WriterOutput {
        task: long_task,
        context: vec![],
        relevant_files: vec![],
        acceptance_criteria: vec![],
        extra_rules: vec![],
        extra_report: vec![],
        assumptions: vec![],
    };
    let err = out.validate().unwrap_err();
    assert_eq!(err, ValidationError::TaskTooLong(1201));
}

#[test]
fn test_render_and_xml_escaping() {
    let out = WriterOutput {
        task: "Fix <Button> & <Modal>".to_string(),
        context: vec!["File A & File B".to_string()],
        relevant_files: vec!["src/button.tsx".to_string()],
        acceptance_criteria: vec!["Render <Button> with count > 0".to_string()],
        extra_rules: vec!["Never use <any> type".to_string()],
        extra_report: vec!["Include metrics & timing".to_string()],
        assumptions: vec![],
    };
    let profile = sample_profile();
    let xml = render(&out, &profile, None);

    // Verify XML escaping
    assert!(xml.contains("Fix &lt;Button&gt; &amp; &lt;Modal&gt;"));
    assert!(xml.contains("- File A &amp; File B"));
    assert!(xml.contains("- Render &lt;Button&gt; with count &gt; 0"));
    assert!(xml.contains("- Never use &lt;any&gt; type"));
    assert!(xml.contains("Include metrics &amp; timing"));

    // Check tags
    assert!(xml.contains("<task>"));
    assert!(xml.contains("<context>"));
    assert!(xml.contains("<acceptance_criteria>"));
    assert!(xml.contains("<rules>"));
    assert!(xml.contains("<report>"));
}

#[test]
fn test_degradation_banner() {
    let deg = Degradation::new(
        vec![
            "writer (all 3 Groq models timed out)".to_string(),
            "history (opencode adapter: db locked)".to_string(),
        ],
        "your raw prompt was wrapped in the default skeleton and NOT enhanced.",
    );
    let banner = deg.render_banner();

    assert!(banner.contains("⚠️⚠️⚠️ PFMT DEGRADED OUTPUT: READ BEFORE SENDING ⚠️⚠️⚠️"));
    assert!(banner.contains("Failed stages: writer (all 3 Groq models timed out), history (opencode adapter: db locked)"));
    assert!(banner
        .contains("Result: your raw prompt was wrapped in the default skeleton and NOT enhanced."));

    let out = WriterOutput {
        task: "Plain prompt".to_string(),
        context: vec![],
        relevant_files: vec![],
        acceptance_criteria: vec![],
        extra_rules: vec![],
        extra_report: vec![],
        assumptions: vec![],
    };
    let xml = render(&out, &sample_profile(), Some(&deg));
    assert!(xml.starts_with(&banner));
}

#[test]
fn test_idempotence_detection() {
    let input = "<task>\ndo it\n</task>\n<acceptance_criteria>\nok\n</acceptance_criteria>";
    assert!(is_idempotent(input));
    let not_idem = "just a plain prompt";
    assert!(!is_idempotent(not_idem));
}

#[test]
fn test_no_llm_wrap() {
    let profile = sample_profile();
    let raw = "Make a new file & run tests";
    let xml = no_llm_wrap(raw, &profile);

    assert!(xml.contains("<task>\nMake a new file &amp; run tests\n</task>"));
    assert!(xml.contains("<rules>\n"));
    assert!(xml.contains("<report>\n"));
    assert!(!xml.contains("<context>"));
}

#[test]
fn test_config_parsing_and_stage_overrides() {
    let toml = r#"
[budget]
total_ms = 8500
payload_tokens_max = 2000

[stages.gate]
enabled = true
on_fail = "skip"

[stages.history]
enabled = true
on_fail = "skip"

[apps.opencode.stages.history]
enabled = false
on_fail = "abort"
"#;
    let cfg = Config::from_toml_str(toml).expect("parse valid config");
    assert_eq!(cfg.budget.total_ms, 8500);
    assert_eq!(cfg.budget.payload_tokens_max, 2000);

    // Default app
    assert!(cfg.is_stage_enabled("antigravity", "history"));
    assert_eq!(cfg.stage_on_fail("antigravity", "history"), OnFail::Skip);

    // Opencode override
    assert!(!cfg.is_stage_enabled("opencode", "history"));
    assert_eq!(cfg.stage_on_fail("opencode", "history"), OnFail::Abort);
}

#[test]
fn test_schemars_json_schema_generation() {
    let schema = WriterOutput::json_schema();
    assert!(schema.is_object());
    let obj = schema.as_object().unwrap();
    assert!(obj.contains_key("properties"));
    let props = obj.get("properties").unwrap().as_object().unwrap();
    assert!(props.contains_key("task"));
    assert!(props.contains_key("acceptance_criteria"));
    assert!(props.contains_key("relevant_files"));
}
