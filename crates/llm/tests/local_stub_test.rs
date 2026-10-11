use pfmt_llm::{Decider, GroqClient, JevDecider};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Simple local HTTP stub server to test client retry, fallback, and parsing.
async fn run_mock_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&request_count);

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };

            let count = Arc::clone(&count_clone);
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let req_text = String::from_utf8_lossy(&buf[..n]);
                let _req_num = count.fetch_add(1, Ordering::SeqCst);

                // If request contains "model_fail", return 500 error
                if req_text.contains("model_fail") {
                    let resp = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 21\r\n\r\n{\"error\": \"failed\"}";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    return;
                }

                // If Jev endpoint
                if req_text.contains("systemone") {
                    let body = json!({
                        "answers": {
                            "needs_history": {"noul": 0.9},
                            "already_structured": {"noul": 0.1},
                            "injection_risk": {"noul": 0.05},
                            "extra_rules": {"noul": 0.8},
                            "extra_report": {"noul": 0.2},
                            "cand_0": {"noul": 0.95}
                        }
                    })
                    .to_string();

                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    return;
                }

                // Groq writer output endpoint
                let writer_json = json!({
                    "task": "Refactored auth module",
                    "context": ["Fact 1"],
                    "relevant_files": ["src/auth.rs"],
                    "acceptance_criteria": ["Criteria 1"],
                    "extra_rules": ["Rule 1"],
                    "extra_report": [],
                    "assumptions": []
                })
                .to_string();

                let body = json!({
                    "choices": [
                        {
                            "message": {
                                "content": writer_json
                            }
                        }
                    ]
                })
                .to_string();

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    (format!("http://{}", addr), request_count)
}

#[tokio::test]
async fn test_groq_model_chain_fallback_on_stub() {
    let (url, _) = run_mock_server().await;

    // Chain: first model fails with 500, second model succeeds
    let client = GroqClient::with_endpoint(
        "mock_key".to_string(),
        vec!["model_fail".to_string(), "model_success".to_string()],
        2000,
        url,
    );

    let (output, used_model) = client
        .execute_writer_chain("System prompt", "User prompt")
        .await
        .unwrap();

    assert_eq!(used_model, "model_success");
    assert_eq!(output.task, "Refactored auth module");
    assert_eq!(output.relevant_files, vec!["src/auth.rs"]);
}

#[tokio::test]
async fn test_jev_decider_on_stub() {
    let (url, _) = run_mock_server().await;

    let decider = JevDecider::with_endpoint(
        "mock_jev_key".to_string(),
        2000,
        format!("{}/systemone", url),
    );

    let decisions = decider
        .decide(
            "Please fix auth and update src/auth.rs",
            &["src/auth.rs".to_string()],
            true,
        )
        .await
        .unwrap();

    assert!(decisions.needs_history);
    assert!(!decisions.already_structured);
    assert!(!decisions.injection_risk);
    assert!(decisions.extra_rules_requested);
    assert_eq!(decisions.filtered_candidates, vec!["src/auth.rs"]);
}
