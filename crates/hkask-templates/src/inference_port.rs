//! Okapi LLM inference port for high-temperature templates
//!
//! This module provides the InferencePort trait for LLM invocations
//! with temperature-controlled parameters for anti-normative generation.

use hkask_types::{BotID, LLMParameters, TemplateId, TemplateInvocation, TemplateOutcome};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Okapi connection error: {0}")]
    Connection(String),
    #[error("Model error: {0}")]
    Model(String),
    #[error("Generation error: {0}")]
    Generation(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Inference result from Okapi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub text: String,
    pub model: String,
    pub usage: Usage,
    pub finish_reason: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Okapi inference port
///
/// Trait for LLM backends. Okapi is the default implementation.
pub trait InferencePort: Send + Sync {
    /// Generate text with parameters
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError>;

    /// Generate multiple outputs for template selection
    fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Result<Vec<InferenceResult>, InferenceError> {
        let mut results = Vec::with_capacity(n);
        for _ in 0..n {
            results.push(self.generate(prompt, parameters)?);
        }
        Ok(results)
    }
}

/// Okapi-backed inference implementation
pub struct OkapiInference {
    model: String,
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl OkapiInference {
    pub fn new(model: &str, base_url: &str) -> Self {
        Self {
            model: model.to_string(),
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Default local Okapi endpoint
    pub fn local(model: &str) -> Self {
        Self::new(model, "http://localhost:8080")
    }

    /// Fast local model preset
    pub fn fast_local() -> Self {
        Self::local("fast-local-model")
    }
}

impl InferencePort for OkapiInference {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        // Okapi API request structure (stub for now)
        let _request = OkapiRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: parameters.temperature,
            top_p: parameters.top_p,
            top_k: parameters.top_k as i32,
            frequency_penalty: parameters.frequency_penalty,
            presence_penalty: parameters.presence_penalty,
            max_tokens: parameters.max_tokens as i32,
            seed: parameters.seed,
        };

        // In production, this would call Okapi API
        // For now, return a stub result
        Ok(InferenceResult {
            text: format!("[Okapi stub] Generated for: {}", prompt),
            model: self.model.clone(),
            usage: Usage {
                prompt_tokens: prompt.len() as u32 / 4,
                completion_tokens: 100,
                total_tokens: (prompt.len() as u32 / 4) + 100,
            },
            finish_reason: "stop".to_string(),
        })
    }
}

/// Okapi API request structure
#[derive(Debug, Serialize)]
struct OkapiRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    top_p: f32,
    top_k: i32,
    frequency_penalty: f32,
    presence_penalty: f32,
    max_tokens: i32,
    seed: Option<u64>,
}

/// Okapi API message structure
#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Template invocation with Okapi inference
pub async fn invoke_template_with_okapi(
    inference: &dyn InferencePort,
    template_id: TemplateId,
    bot_id: BotID,
    parameters: LLMParameters,
    rendered_prompt: &str,
    input: Value,
) -> Result<TemplateInvocation, InferenceError> {
    let result = inference.generate(rendered_prompt, &parameters)?;

    let mut invocation = TemplateInvocation::new(template_id, bot_id, parameters, input);
    invocation.outputs.push(Value::String(result.text));
    invocation.outcome = TemplateOutcome::Success;

    Ok(invocation)
}

/// Invoke template with N outputs for selection (anti-normative pattern)
pub async fn invoke_template_with_selection(
    inference: &dyn InferencePort,
    template_id: TemplateId,
    bot_id: BotID,
    parameters: LLMParameters,
    rendered_prompt: &str,
    input: Value,
    n: usize,
) -> Result<TemplateInvocation, InferenceError> {
    let results = inference.generate_n(rendered_prompt, &parameters, n)?;

    let mut invocation = TemplateInvocation::new(template_id, bot_id, parameters.clone(), input);

    for result in results {
        invocation.outputs.push(Value::String(result.text));
    }

    // Select best output (simple heuristic: first non-empty)
    // In production, Curator would evaluate and select
    invocation.selected_index = Some(0);
    invocation.outcome = TemplateOutcome::Merged;

    Ok(invocation)
}
