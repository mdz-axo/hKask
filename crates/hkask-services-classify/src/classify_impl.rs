//! Section type classifier — config-driven, multi-provider.
//!
//! Classifier configs live in registry/classify/{name}.yaml.
//! corpus.yaml references which one to use via the `classifier` field.
//!
//! Supports DeepInfra (OpenAI-compatible) with concurrent batch requests.
//! Graceful degradation: no API key → all passages default to fallback category.

use hkask_services_core::ServiceError;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

/// Classification result for a single passage.
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    /// The classified section type: "Statement", "Evidence", "Diagram", or "Implications".
    pub category: String,
}

/// Semantic triple extraction result for a single passage.
/// Produced by the triple-extractor classifier (Gemma 4).
#[derive(Debug, Clone, Default)]
pub struct TripleExtraction {
    /// One-sentence summary of what the passage is about.
    pub topic: String,
    /// Key concepts mentioned in the passage.
    pub concepts: Vec<String>,
    /// Named entities, tools, frameworks, or services mentioned.
    pub entities: Vec<String>,
    /// Relationships between concepts or entities.
    pub relationships: Vec<String>,
    /// Which Gentle Lovelace dimension the passage primarily exemplifies.
    pub primary_dimension: String,
    /// Quality assessment flags for the passage.
    pub quality_flags: Vec<String>,
    /// Extra fields from classifier output that don't map to the standard fields.
    /// Each key-value pair is stored as a triple: entity_ref → key → value.
    /// Literary classifiers use this for themes, characters, setting, tone, imagery, etc.
    pub extra: std::collections::HashMap<String, serde_json::Value>,
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

/// Classifier configuration loaded from registry/classify/{name}.yaml.
#[derive(Debug, Deserialize)]
pub struct ClassifierYaml {
    pub classifier: ClassifierDef,
}

#[derive(Debug, Deserialize)]
pub struct ClassifierDef {
    pub name: String,
    pub model: String,
    #[serde(default)]
    pub provider: String,
    pub concurrency: usize,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    pub system_prompt: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default)]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_fallback")]
    pub fallback_category: String,
}

impl Default for ClassifierDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: String::new(),
            provider: String::new(),
            concurrency: 1,
            timeout_secs: 30,
            system_prompt: String::new(),
            base_url: String::new(),
            api_key_env: String::new(),
            temperature: 0.0,
            max_tokens: 15,
            fallback_category: "Statement".to_string(),
        }
    }
}

fn default_timeout() -> u64 {
    30
}
fn default_max_tokens() -> u32 {
    15
}
fn default_fallback() -> String {
    "Statement".to_string()
}

/// Load a classifier config from registry/classify/{name}.yaml.
pub fn load_classifier_config(
    name: &str,
    registry_dir: &Path,
) -> Result<ClassifierDef, ServiceError> {
    let config_path = registry_dir.join("classify").join(format!("{name}.yaml"));
    let yaml_str = std::fs::read_to_string(&config_path).map_err(|e| {
        let msg = format!(
            "Failed to read classifier config {}: {e}",
            config_path.display()
        );
        ServiceError::Embed {
            source: None,
            message: msg,
        }
    })?;
    let config: ClassifierYaml = serde_yaml_neo::from_str(&yaml_str).map_err(|e| {
        let msg = format!(
            "Failed to parse classifier config {}: {e}",
            config_path.display()
        );
        ServiceError::Embed {
            source: None,
            message: msg,
        }
    })?;
    Ok(config.classifier)
}

/// Runtime classifier configuration (derived from YAML + env).
#[derive(Clone)]
pub struct ClassifierConfig {
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    pub system_prompt: String,
    pub concurrency: usize,
    pub timeout: Duration,
    pub temperature: f64,
    pub max_tokens: u32,
    pub fallback_category: String,
}

impl ClassifierConfig {
    /// Build from a ClassifierDef, resolving API key from environment.
    pub fn from_def(def: &ClassifierDef) -> Self {
        let api_key = if def.api_key_env.is_empty() {
            String::new()
        } else {
            std::env::var(&def.api_key_env).unwrap_or_default()
        };
        Self {
            model: def.model.clone(),
            api_key,
            base_url: if def.base_url.is_empty() {
                "https://api.deepinfra.com/v1/openai/chat/completions".to_string()
            } else {
                def.base_url.clone()
            },
            system_prompt: def.system_prompt.clone(),
            concurrency: def.concurrency,
            timeout: Duration::from_secs(def.timeout_secs),
            temperature: def.temperature,
            max_tokens: def.max_tokens,
            fallback_category: def.fallback_category.clone(),
        }
    }
}

/// Classify a single passage.
async fn classify_one(
    client: &Client,
    config: &ClassifierConfig,
    text: &str,
) -> Result<ClassifyResult, ServiceError> {
    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": config.system_prompt},
            {"role": "user", "content": text}
        ],
        "temperature": config.temperature,
        "max_tokens": config.max_tokens
    });

    let resp = client
        .post(&config.base_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .timeout(config.timeout)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("Classifier HTTP error: {e}");
            ServiceError::Embed {
            source: None,
                message: msg,
            }
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Embed {
            source: None,
            message: format!(
                "Classifier error {status}: {}",
                body.chars().take(200).collect::<String>()
            ),
        });
    }

    let chat: ChatResponse = resp.json().await.map_err(|e| {
        let msg = format!("Classifier JSON parse error: {e}");
        ServiceError::Embed {
            source: None,
            message: msg,
        }
    })?;

    let content = chat
        .choices
        .first()
        .map(|c| c.message.content.as_str())
        .unwrap_or("");

    // Parse the JSON category from the response
    let category = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
        parsed["category"]
            .as_str()
            .unwrap_or(&config.fallback_category)
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
            config.fallback_category.clone()
        }
    };

    Ok(ClassifyResult { category })
}

/// Classify a batch of passages concurrently.
///
/// Returns results in the same order as the input texts.
/// Failed classifications default to "Statement".
///
/// REQ: P8-svc-classify-277
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  texts must be non-empty; config must have valid timeout and concurrency
/// post: returns Vec<ClassifyResult> in input order; failed classifications fall back to config.fallback_category; all fallback if no API key
pub async fn classify_batch(
    texts: &[String],
    config: ClassifierConfig,
) -> Result<Vec<ClassifyResult>, ServiceError> {
    if config.api_key.is_empty() {
        // No API key — return all fallback category (skip classification)
        let fallback = &config.fallback_category;
        return Ok(texts
            .iter()
            .map(|_| ClassifyResult {
                category: fallback.clone(),
            })
            .collect());
    }

    let client = Client::builder()
        .timeout(config.timeout)
        .build()
        .map_err(|e| {
            let msg = format!("Classifier client build error: {e}");
            ServiceError::Embed {
            source: None,
                message: msg,
            }
        })?;

    let config = std::sync::Arc::new(config.clone()); // share across spawned tasks
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let mut handles = Vec::with_capacity(texts.len());

    for (i, text) in texts.iter().enumerate() {
        let client = client.clone();
        let cfg = config.clone();
        let text = text.clone();
        let permit = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit.acquire().await;
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
                tracing::warn!(index = i, error = %e, "Classifier failed for passage, using fallback");
                results[i] = Some(ClassifyResult {
                    category: config.fallback_category.clone(),
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
                category: config.fallback_category.clone(),
            })
        })
        .collect())
}

// ── Triple Extraction ──────────────────────────────────────────────────

/// Extract semantic triples from a batch of passages using the Gemma 4 classifier.
///
/// Returns results in the same order as the input texts.
/// Failed extractions default to empty TripleExtraction.
/// Graceful degradation: no API key → all empty extractions.
///
/// REQ: P8-svc-classify-278
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  texts must be non-empty; config must have valid timeout and concurrency
/// post: returns Vec<TripleExtraction> in input order; failed extractions fall back to empty; all empty if no API key
pub async fn extract_triples_batch(
    texts: &[String],
    config: &ClassifierConfig,
) -> Result<Vec<TripleExtraction>, ServiceError> {
    if config.api_key.is_empty() {
        tracing::info!("No API key for triple extraction — returning empty extractions");
        return Ok(texts.iter().map(|_| TripleExtraction::default()).collect());
    }

    let client = Client::builder()
        .timeout(config.timeout)
        .build()
        .map_err(|e| {
            let msg = format!("Triple extractor client build error: {e}");
            ServiceError::Embed {
            source: None,
                message: msg,
            }
        })?;

    let config = std::sync::Arc::new(config.clone());
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let mut handles = Vec::with_capacity(texts.len());

    for (i, text) in texts.iter().enumerate() {
        let client = client.clone();
        let cfg = config.clone();
        let text = text.clone();
        let permit = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit.acquire().await;
            let result = extract_triples_one(&client, &cfg, &text).await;
            (i, result)
        }));
    }

    let mut results: Vec<Option<TripleExtraction>> = vec![None; texts.len()];
    for handle in handles {
        match handle.await {
            Ok((i, Ok(result))) => {
                results[i] = Some(result);
            }
            Ok((i, Err(e))) => {
                tracing::warn!(index = i, error = %e, "Triple extraction failed, using empty");
                results[i] = Some(TripleExtraction::default());
            }
            Err(e) => {
                tracing::warn!(error = %e, "Triple extraction task panicked");
            }
        }
    }

    Ok(results.into_iter().map(|r| r.unwrap_or_default()).collect())
}

/// Extract triples from a single passage.
async fn extract_triples_one(
    client: &Client,
    config: &ClassifierConfig,
    text: &str,
) -> Result<TripleExtraction, ServiceError> {
    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": config.system_prompt},
            {"role": "user", "content": text}
        ],
        "temperature": config.temperature,
        "max_tokens": config.max_tokens
    });

    let resp = client
        .post(&config.base_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .timeout(config.timeout)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("Triple extractor HTTP error: {e}");
            ServiceError::Embed {
            source: None,
                message: msg,
            }
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Embed {
            source: None,
            message: format!(
                "Triple extractor error {status}: {}",
                body.chars().take(200).collect::<String>()
            ),
        });
    }

    let chat: ChatResponse = resp.json().await.map_err(|e| {
        let msg = format!("Triple extractor JSON parse error: {e}");
        ServiceError::Embed {
            source: None,
            message: msg,
        }
    })?;

    let content = chat
        .choices
        .first()
        .map(|c| c.message.content.as_str())
        .unwrap_or("");

    // Parse the structured JSON from the response
    parse_triple_extraction(content)
}

/// Parse a TripleExtraction from classifier JSON response.
fn parse_triple_extraction(content: &str) -> Result<TripleExtraction, ServiceError> {
    // Try to extract JSON from the response (may be wrapped in markdown code blocks)
    let json_str = if let Some(start) = content.find("{") {
        if let Some(end) = content.rfind("}") {
            &content[start..=end]
        } else {
            content
        }
    } else {
        content
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        let msg = format!(
            "Triple extraction JSON parse error: {e}. Content: {}",
            &json_str[..json_str.len().min(200)]
        );
        ServiceError::Embed {
            source: None,
            message: msg,
        }
    })?;

    Ok(TripleExtraction {
        topic: parsed["topic"].as_str().unwrap_or("").to_string(),
        concepts: parsed["concepts"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        entities: parsed["entities"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        relationships: parsed["relationships"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        primary_dimension: parsed["primary_dimension"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        quality_flags: parsed["quality_flags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        extra: {
            // Capture any fields not in the standard schema
            let standard = [
                "topic",
                "concepts",
                "entities",
                "relationships",
                "primary_dimension",
                "quality_flags",
            ];
            let mut extra = std::collections::HashMap::new();
            if let Some(obj) = parsed.as_object() {
                for (key, val) in obj {
                    if !standard.contains(&key.as_str()) && !val.is_null() {
                        extra.insert(key.clone(), val.clone());
                    }
                }
            }
            extra
        },
    })
}
