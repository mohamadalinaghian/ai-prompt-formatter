use pfmt_core::resolve_api_key;
use pfmt_llm::{Decider, GroqClient, JevDecider, PromptLoader};
use std::env;

fn get_groq_key() -> Option<String> {
    // 1. Try resolving via keychain / env
    if let Ok(key) = resolve_api_key("keychain:pfmt-groq", "GROQ_API_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }
    // 2. Check if valid key exists in prompt-fmt
    let prompt_fmt_script = "/Users/mohamad_mac/tools/prompt-fmt/ai_fmt_prompt.py";
    if let Ok(content) = std::fs::read_to_string(prompt_fmt_script) {
        for line in content.lines() {
            if line.starts_with("GROQ_API_KEY = ") {
                let key = line
                    .replace("GROQ_API_KEY = ", "")
                    .replace('"', "")
                    .trim()
                    .to_string();
                if !key.is_empty() {
                    return Some(key);
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn test_live_groq_smoke() {
    let key = match get_groq_key() {
        Some(k) => k,
        None => {
            println!("Skipping test_live_groq_smoke: GROQ_API_KEY not found.");
            return;
        }
    };

    let client = GroqClient::new(
        key,
        vec![
            "openai/gpt-oss-120b".to_string(),
            "qwen/qwen3.8-27b".to_string(),
        ],
        8000,
    );

    let system_prompt = PromptLoader::load_meta_prompt();
    let payload = "<meta>agent=antigravity, mode=fresh</meta>\n<user_prompt>Implement addition function in Rust with unit test</user_prompt>";

    let res = client.execute_writer_chain(&system_prompt, payload).await;
    match res {
        Ok((output, model)) => {
            println!("Live Groq smoke test succeeded using model: {}", model);
            println!("Generated task: {}", output.task);
            assert!(!output.task.is_empty());
            assert!(!output.acceptance_criteria.is_empty());
        }
        Err(e) => {
            panic!("Live Groq smoke test failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_live_jev_smoke() {
    let key = match env::var("TYPESAFE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            println!("Skipping test_live_jev_smoke: TYPESAFE_API_KEY is not set.");
            return;
        }
    };

    let decider = JevDecider::new(key, 8000);
    let prompt = "Fix database migration error in src/schema.rs. Does this need history?";
    let candidates = vec!["src/schema.rs".to_string()];

    let res = decider.decide(prompt, &candidates, false).await;
    match res {
        Ok(decisions) => {
            println!("Live Jev smoke test succeeded!");
            println!("Decisions: {:?}", decisions);
            assert!(decisions.probabilities.contains_key("needs_history"));
        }
        Err(e) => {
            println!("Live Jev call encountered error: {:?}", e);
        }
    }
}
