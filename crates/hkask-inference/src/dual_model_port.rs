//! Dual-model inference port — wraps an existing `InferencePort` and delegates
//! all calls to `generate_with_model` with a fixed model override.
//!
//! Used by runtimes to wire `ManifestExecutor::with_dual_inference()` without
//! constructing a second `InferenceRouter`. The wrapper reuses the existing
//! router's connection pools and provider backends — only the model name changes.
//!
//! # Principle grounding
//! - P3.1 Social Generativity: dual-model classification requires two peer
//!   models from different jurisdictions. This port provides the second peer.

use hkask_ports::{ChatToolDefinition, InferenceError, InferencePort, InferenceResult};
use hkask_types::template::LLMParameters;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Wraps an `InferencePort` and overrides the model for all calls.
///
/// Construct with `DualModelPort::new(existing_port, "DI/google/gemma-4-E4B-it")`.
/// All `generate()` calls are delegated to `generate_with_model()` with the
/// configured model as the override.
#[derive(Clone)]
pub struct DualModelPort {
    inner: Arc<dyn InferencePort>,
    model: String,
}

impl DualModelPort {
    /// Create a dual-model port that delegates to `inner` but uses `model`
    /// as the model override for all inference calls.
    #[must_use]
    pub fn new(inner: Arc<dyn InferencePort>, model: String) -> Self {
        Self { inner, model }
    }
}

impl InferencePort for DualModelPort {
    fn generate(
        &self,
        prompt: &str,
        params: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        let model = self.model.clone();
        self.inner
            .generate_with_model(prompt, params, Some(&model), tools)
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        params: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        // If an explicit override is given, use it; otherwise use our configured model.
        let effective = model_override.unwrap_or(&self.model);
        self.inner
            .generate_with_model(prompt, params, Some(effective), tools)
    }
}
