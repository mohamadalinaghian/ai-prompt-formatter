use crate::DeciderError;
use async_trait::async_trait;
use pfmt_core::ItemConfidence;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

const DEFAULT_JEV_ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";

/// Trait for confidence checking via Jev (TypeSafe AI).
#[async_trait]
pub trait ConfidenceChecker: Send + Sync {
    /// Check confidence of LLM-written items against the given state.
    /// Returns a list of (section, index, score) tuples.
    async fn check_confidence(
        &self,
        state: &str,
        items: &[ConfidenceItem],
    ) -> Result<Vec<ItemConfidence>, DeciderError>;
}

/// An item to be confidence-checked.
#[derive(Debug, Clone)]
pub struct ConfidenceItem {
    /// Unique identifier for the question (e.g., "task", "ac_0", "ctx_1", "scope_0", "ver_0")
    pub id: String,
    /// The text content to check
    pub text: String,
    /// The section name for reporting
    pub section: String,
    /// Index within the section
    pub index: usize,
}

/// `JevConfidence` implements `ConfidenceChecker` by calling the same
/// Jev endpoint used by the decider, asking yes/no questions about
/// each LLM-written item.
pub struct JevConfidence {
    client: Client,
    api_key: String,
    endpoint: String,
    threshold: f64,
    timeout: Duration,
}

impl JevConfidence {
    pub fn new(api_key: String, timeout_ms: u64) -> Self {
        Self::with_endpoint(api_key, timeout_ms, DEFAULT_JEV_ENDPOINT.to_string())
    }

    pub fn with_endpoint(api_key: String, timeout_ms: u64, endpoint: String) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            endpoint,
            threshold: 0.6,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// Set the confidence threshold (default 0.6)
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }
}

#[async_trait]
impl ConfidenceChecker for JevConfidence {
    async fn check_confidence(
        &self,
        state: &str,
        items: &[ConfidenceItem],
    ) -> Result<Vec<ItemConfidence>, DeciderError> {
        if self.api_key.trim().is_empty() {
            return Err(DeciderError::AuthMissing);
        }

        if items.is_empty() {
            return Ok(Vec::new());
        }

        // Cap at 30 questions
        let items_to_check = &items[..items.len().min(30)];

        let mut questions = serde_json::Map::new();

        for item in items_to_check {
            let instruction = if item.id == "task" {
                format!(
                    "Does this task preserve the intent of the user prompt without adding requirements? Task: {}",
                    item.text
                )
            } else {
                format!("Is this statement directly supported by the state? Statement: {}", item.text)
            };

            questions.insert(
                item.id.clone(),
                json!({
                    "type": "noul",
                    "instructions": instruction
                }),
            );
        }

        let body = json!({
            "model": "jev-latest",
            "state": state,
            "questions": questions
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key.trim()),
            )
            .timeout(self.timeout)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DeciderError::Provider(format!(
                "Jev HTTP {}: {}",
                status, text
            )));
        }

        let json_resp: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DeciderError::Parse(e.to_string()))?;

        let answers = json_resp
            .get("answers")
            .and_then(|a| a.as_object())
            .ok_or_else(|| {
                DeciderError::Parse("Missing 'answers' object in Jev response".to_string())
            })?;

        let extract_prob = |key: &str| -> Option<f64> {
            answers
                .get(key)
                .and_then(|ans| ans.get("noul"))
                .and_then(|v| v.as_f64())
        };

        let mut results = Vec::new();
        for item in items_to_check {
            let prob = extract_prob(&item.id);
            results.push(ItemConfidence {
                section: item.section.clone(),
                index: item.index,
                score: prob,
            });
        }

        // Mark any remaining items (beyond 30) as unchecked
        if items.len() > 30 {
            for item in &items[30..] {
                results.push(ItemConfidence {
                    section: item.section.clone(),
                    index: item.index,
                    score: None,
                });
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_item_creation() {
        let item = ConfidenceItem {
            id: "task".to_string(),
            text: "Test task".to_string(),
            section: "task".to_string(),
            index: 0,
        };
        assert_eq!(item.id, "task");
        assert_eq!(item.section, "task");
    }
}