use pfmt_core::{ValidationError, WriterOutput};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use thiserror::Error;

const DEFAULT_GROQ_ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

pub fn is_openrouter_model(model: &str) -> bool {
    model.ends_with(":free") || model.starts_with("openrouter/") || model.contains(":free")
}

#[derive(Debug, Error)]
pub enum GroqChainError {
    #[error("All models in chain failed: {0:?}")]
    AllModelsFailed(Vec<(String, String)>),
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("Empty model chain configured")]
    EmptyChain,
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct GroqClient {
    client: Client,
    api_key: String,
    chain: Vec<String>,
    timeout: Duration,
    endpoint: String,
}

impl GroqClient {
    pub fn new(api_key: String, chain: Vec<String>, timeout_ms: u64) -> Self {
        Self::with_endpoint(
            api_key,
            chain,
            timeout_ms,
            DEFAULT_GROQ_ENDPOINT.to_string(),
        )
    }

    pub fn with_endpoint(
        api_key: String,
        chain: Vec<String>,
        timeout_ms: u64,
        endpoint: String,
    ) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            chain,
            timeout: Duration::from_millis(timeout_ms),
            endpoint,
        }
    }

    fn resolve_endpoint_and_key(&self, model: &str) -> (String, String) {
        if self.endpoint != DEFAULT_GROQ_ENDPOINT {
            (self.endpoint.clone(), self.api_key.clone())
        } else if is_openrouter_model(model) {
            let key = pfmt_core::resolve_api_key("keychain:pfmt-openrouter", "OPENROUTER_API_KEY")
                .unwrap_or_else(|_| self.api_key.clone());
            (OPENROUTER_ENDPOINT.to_string(), key)
        } else {
            (self.endpoint.clone(), self.api_key.clone())
        }
    }

    /// Executes the writer LLM call across the fallback model chain.
    /// Returns (WriterOutput, model_name_used).
    pub async fn execute_writer_chain(
        &self,
        system_prompt: &str,
        user_payload: &str,
    ) -> Result<(WriterOutput, String), GroqChainError> {
        if self.chain.is_empty() {
            return Err(GroqChainError::EmptyChain);
        }

        let mut errors = Vec::new();

        for model in &self.chain {
            let provider = if is_openrouter_model(model) {
                "OpenRouter"
            } else {
                "Groq"
            };
            tracing::info!("Attempting {} writer model: {}", provider, model);
            match self
                .call_single_model(model, system_prompt, user_payload)
                .await
            {
                Ok(output) => {
                    tracing::info!("{} writer succeeded with model: {}", provider, model);
                    return Ok((output, model.clone()));
                }
                Err(err) => {
                    tracing::warn!("{} model {} failed: {}", provider, model, err);
                    errors.push((model.clone(), err.to_string()));
                }
            }
        }

        Err(GroqChainError::AllModelsFailed(errors))
    }

    /// Executes arbitrary JSON completion across the model chain.
    /// Returns (raw_response_content, model_name_used).
    pub async fn execute_json_completion(
        &self,
        system_prompt: &str,
        user_payload: &str,
    ) -> Result<(String, String), GroqChainError> {
        if self.chain.is_empty() {
            return Err(GroqChainError::EmptyChain);
        }

        let mut errors = Vec::new();

        for model in &self.chain {
            let body = json!({
                "model": model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_payload}
                ],
                "temperature": 0.0,
                "response_format": {"type": "json_object"}
            });

            match self.send_with_retry(&body).await {
                Ok(content) => return Ok((content, model.clone())),
                Err(err) => {
                    tracing::warn!("Groq model {} failed for json completion: {}", model, err);
                    errors.push((model.clone(), err));
                }
            }
        }

        Err(GroqChainError::AllModelsFailed(errors))
    }

    async fn call_single_model(
        &self,
        model: &str,
        system_prompt: &str,
        user_payload: &str,
    ) -> Result<WriterOutput, String> {
        // Try strict json_schema first
        let schema_value = WriterOutput::json_schema();
        let schema_body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_payload}
            ],
            "temperature": 0.0,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "writer_output",
                    "strict": true,
                    "schema": schema_value
                }
            }
        });

        match self.send_with_retry(&schema_body).await {
            Ok(content) => parse_and_validate(&content),
            Err(e) => {
                // If json_schema mode returned 400 (unsupported), fallback to json_object mode
                if e.contains("400") || e.contains("response_format") {
                    tracing::info!("Fallback to json_object format for model: {}", model);
                    let fallback_system = format!(
                        "{}\n\nStrict output JSON schema:\n{}",
                        system_prompt,
                        serde_json::to_string(&schema_value).unwrap_or_default()
                    );
                    let obj_body = json!({
                        "model": model,
                        "messages": [
                            {"role": "system", "content": fallback_system},
                            {"role": "user", "content": user_payload}
                        ],
                        "temperature": 0.0,
                        "response_format": {"type": "json_object"}
                    });

                    let content = self.send_with_retry(&obj_body).await?;
                    parse_and_validate(&content)
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn send_with_retry(&self, body: &serde_json::Value) -> Result<String, String> {
        let max_retries = 2;
        let mut delay = Duration::from_millis(300);

        let model = body.get("model").and_then(|m| m.as_str()).unwrap_or("");
        let (endpoint, api_key) = self.resolve_endpoint_and_key(model);

        for attempt in 0..=max_retries {
            let req = self
                .client
                .post(&endpoint)
                .header(AUTHORIZATION, format!("Bearer {}", api_key.trim()))
                .timeout(self.timeout)
                .json(body);

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let res_json: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

                        let content = res_json
                            .pointer("/choices/0/message/content")
                            .and_then(|c| c.as_str())
                            .filter(|c| !c.is_empty())
                            .or_else(|| {
                                res_json
                                    .pointer("/choices/0/message/reasoning")
                                    .and_then(|c| c.as_str())
                            })
                            .ok_or_else(|| {
                                "Missing /choices/0/message/content in response".to_string()
                            })?;

                        return Ok(content.to_string());
                    } else if (status.as_u16() == 429 || status.is_server_error())
                        && attempt < max_retries
                    {
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                        continue;
                    } else {
                        let err_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error text".to_string());
                        return Err(format!("HTTP {}: {}", status, err_text));
                    }
                }
                Err(err) => {
                    if (err.is_timeout() || err.is_connect()) && attempt < max_retries {
                        tokio::time::sleep(delay).await;
                        delay *= 2;
                        continue;
                    }
                    return Err(format!("Request failed: {}", err));
                }
            }
        }

        Err("Exceeded maximum retries".to_string())
    }
}

/// Strips markdown fences if present.
pub fn clean_json_str(raw: &str) -> &str {
    let mut clean = raw.trim();
    if clean.starts_with("```json") {
        clean = clean.strip_prefix("```json").unwrap_or(clean).trim();
    } else if clean.starts_with("```") {
        clean = clean.strip_prefix("```").unwrap_or(clean).trim();
    }
    if clean.ends_with("```") {
        clean = clean.strip_suffix("```").unwrap_or(clean).trim();
    }
    clean
}

/// Parse and validate raw LLM JSON response.
pub fn parse_and_validate(raw: &str) -> Result<WriterOutput, String> {
    let clean = clean_json_str(raw);

    let output: WriterOutput = serde_json::from_str(clean)
        .map_err(|e| format!("JSON deserialization failed: {} on content: {}", e, clean))?;

    output
        .validate()
        .map_err(|e| format!("Schema validation failed: {}", e))?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_validate_clean_json() {
        let json_str = r#"{
            "task": "Build feature X",
            "context": ["Fact 1"],
            "relevant_files": ["src/main.rs"],
            "acceptance_criteria": ["Criteria 1"],
            "extra_rules": [],
            "extra_report": [],
            "assumptions": []
        }"#;

        let res = parse_and_validate(json_str);
        assert!(res.is_ok());
        let output = res.unwrap();
        assert_eq!(output.task, "Build feature X");
    }

    #[test]
    fn test_parse_and_validate_fenced_json() {
        let json_str = "```json\n{\"task\": \"Test\", \"acceptance_criteria\": [\"A\"]}\n```";
        let res = parse_and_validate(json_str);
        assert!(res.is_ok());
    }
}
