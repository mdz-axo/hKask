//! Section type classifier — DeepInfra + Gemma 4 26B MoE.
//!
//! Classifies passages into: Statement, Evidence, Diagram, Implications.
//! Uses concurrent requests to DeepInfra's OpenAI-compatible API for
//! high-throughput batch classification (~95 req/s at 150 concurrent).

use crate::error::ServiceError;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// Classification result for a single passage.
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    /// The classified section type: "Statement", "Evidence", "Diagram", or "Implications".
    pub category: String,
}

/// OpenAI-compatible chat completion response (minimal fields).
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

/// Classifier configuration.
pub struct ClassifierConfig {
    /// DeepInfra API key.
    pub api_key: String,
    /// Model name on DeepInfra.
    pub model: String,
    /// Maximum concurrent requests.
    pub concurrency: usize,
    /// Request timeout.
    pub timeout: Duration,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("DEEPINFRA_API_KEY")
                .or_else(|_| std::env::var("DI_API_KEY"))
                .unwrap_or_default(),
            model: "google/gemma-4-26B-A4B-it".to_string(),
            concurrency: 150,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Classify a single passage using DeepInfra.
async fn classify_one(
    client: &Client,
    config: &ClassifierConfig,
    text: &str,
) -> Result<ClassifyResult, ServiceError> {
    let system_prompt = "Classify. Return ONLY: {\"category\":\"X\"}. \
        Statement=principle/rule/assertion. \
        Evidence=example/data/citation (look for: \"for instance\", \"for example\"). \
        Diagram=structure/layout/mechanical description. \
        Implications=consequence (\"therefore\", \"thus\", \"hence\").";

    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": text}
        ],
        "temperature": 0.0,
        "max_tokens": 15
    });

    let resp = client
        .post("https://api.deepinfra.com/v1/openai/chat/completions")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .timeout(config.timeout)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("Classifier HTTP error: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Embed(format!(
            "Classifier error {status}: {}",
            body.chars().take(200).collect::<String>()
        )));
    }

    let chat: ChatResponse = resp
        .json()
        .await
        .map_err(|e| ServiceError::Embed(format!("Classifier JSON parse error: {e}")))?;

    let content = chat
        .choices
        .first()
        .map(|c| c.message.content.as_str())
        .unwrap_or("");

    // Parse the JSON category from the response
    let category = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
        parsed["category"]
            .as_str()
            .unwrap_or("Statement")
            .to_string()
    } else {
        // Fallback: try to extract from raw text
        if content.contains("Evidence") {
            "Evidence".to_string()
        } else if content.contains("Diagram") {
            "Diagram".to_string()
        } else if content.contains("Implications") {
            "Implications".to_string()
        } else {
            "Statement".to_string()
        }
    };

    Ok(ClassifyResult { category })
}

/// Classify a batch of passages concurrently.
///
/// Returns results in the same order as the input texts.
/// Failed classifications default to "Statement".
pub async fn classify_batch(
    texts: &[String],
    config: &ClassifierConfig,
) -> Result<Vec<ClassifyResult>, ServiceError> {
    if config.api_key.is_empty() {
        // No API key — return all Statement (skip classification)
        return Ok(texts
            .iter()
            .map(|_| ClassifyResult {
                category: "Statement".to_string(),
            })
            .collect());
    }

    let client = Client::builder()
        .timeout(config.timeout)
        .build()
        .map_err(|e| ServiceError::Embed(format!("Classifier client build error: {e}")))?;

    let api_key = config.api_key.clone();
    let model = config.model.clone();
    let timeout = config.timeout;
    let concurrency = config.concurrency;

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(texts.len());

    for (i, text) in texts.iter().enumerate() {
        let client = client.clone();
        let api_key = api_key.clone();
        let model = model.clone();
        let text = text.clone();
        let permit = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit.acquire().await;
            let cfg = ClassifierConfig {
                api_key,
                model,
                concurrency: 1,
                timeout,
            };
            let result = classify_one(&client, &cfg, &text).await;
            (i, result)
        }));
    }

    let mut results: Vec<Option<ClassifyResult>> = vec![None; texts.len()];
    for handle in handles {
        match handle.await {
            Ok((i, Ok(result))) => {
                results[i] = Some(result);
            }
            Ok((i, Err(e))) => {
                tracing::warn!(index = i, error = %e, "Classifier failed for passage, defaulting to Statement");
                results[i] = Some(ClassifyResult {
                    category: "Statement".to_string(),
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "Classifier task panicked");
            }
        }
    }

    Ok(results
        .into_iter()
        .map(|r| {
            r.unwrap_or(ClassifyResult {
                category: "Statement".to_string(),
            })
        })
        .collect())
}
