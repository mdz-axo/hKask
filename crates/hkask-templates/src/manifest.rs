//! Manifest executor — core execution loop
//!
//! Implements the fixed logic that executes ANY manifest without modification.
//! Per architecture v0.21.0: ~50 lines of Rust that never changes when templates are added/edited.

use crate::ports::{
    Action, CnsPort, DEFAULT_MATROSHKA_LIMIT, InferenceConfig, InferencePort, ManifestExecutor,
    ManifestStep, McpPort, ProcessManifest, Result, TemplateError, TemplateRenderer,
};
use hkask_storage::ModelCategory;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

/// Model requirements for template execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequirements {
    /// Required model ID (e.g., "ollama/llama-3.1-8b-instruct")
    pub required: String,
    /// Model category for fallback selection
    pub category: ModelCategory,
    /// Fallback category if required model unavailable
    pub fallback_category: Option<ModelCategory>,
    /// Minimum context length required
    pub min_context: u32,
    /// Whether reasoning capability is required
    #[serde(default)]
    pub reasoning_required: bool,
    /// Required capabilities (e.g., "code", "math", "analysis")
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Embedding dimension (for embedding models only)
    pub dimension: Option<u32>,
    /// Pooling strategy (for embedding models only)
    pub pooling: Option<String>,
}

/// Configuration for selector fallback
#[derive(Debug, Clone)]
pub struct SelectorConfig {
    /// Confidence threshold below which fallback is triggered
    pub confidence_threshold: f64,
    /// Fallback template ID to use when confidence is low
    pub fallback_template_id: String,
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.3,
            fallback_template_id: "prompt/execute".to_string(),
        }
    }
}

/// Core manifest execution loop — fixed logic, applies to ANY manifest
///
/// This is the "loom" that weaves the "thread" (YAML/Jinja2 templates).
/// It doesn't change when templates are added, edited, or removed.
/// Only changes if the grammar of steps themselves changes.
pub struct ManifestExecutorImpl<R, I, M, C> {
    #[allow(dead_code)]
    renderer: R,
    inference: I,
    mcp: M,
    cns: C,
    max_depth: u8,
    selector_config: SelectorConfig,
    inference_config: InferenceConfig,
}

impl<R, I, M, C> ManifestExecutorImpl<R, I, M, C>
where
    R: TemplateRenderer,
    I: InferencePort,
    M: McpPort,
    C: CnsPort,
{
    pub fn new(renderer: R, inference: I, mcp: M, cns: C) -> Self {
        Self {
            renderer,
            inference,
            mcp,
            cns,
            max_depth: DEFAULT_MATROSHKA_LIMIT,
            selector_config: SelectorConfig::default(),
            inference_config: InferenceConfig::default(),
        }
    }

    pub fn with_max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_selector_config(mut self, config: SelectorConfig) -> Self {
        self.selector_config = config;
        self
    }

    pub fn with_inference_config(mut self, config: InferenceConfig) -> Self {
        self.inference_config = config;
        self
    }

    fn execute_step(&self, step: &ManifestStep, state: Value, depth: u8) -> Result<Value> {
        if depth > self.max_depth {
            return Err(TemplateError::RecursionLimit {
                max: self.max_depth,
            });
        }

        let result = match step.action {
            Action::Select => {
                // Render selector template and call fast model with timeout/retry
                let prompt = format!("Select template for: {:?}", state);
                let selection_result = self.inference.call(
                    step.model_tier.as_deref().unwrap_or("fast_local"),
                    &prompt,
                    &self.inference_config,
                )?;

                // Check confidence and apply fallback if needed
                if let Some(confidence) =
                    selection_result.get("confidence").and_then(|v| v.as_f64())
                {
                    if confidence < self.selector_config.confidence_threshold {
                        // Emit CNS event for fallback
                        self.cns.emit(
                            "cns.prompt.selector_fallback",
                            Value::String(format!(
                                "Confidence {} below threshold {}",
                                confidence, self.selector_config.confidence_threshold
                            )),
                            confidence,
                        );

                        // Use fallback template
                        let mut fallback_result = selection_result;
                        if let Some(obj) = fallback_result.as_object_mut() {
                            obj.insert(
                                "selected_template_id".to_string(),
                                Value::String(self.selector_config.fallback_template_id.clone()),
                            );
                            obj.insert("fallback_applied".to_string(), Value::Bool(true));
                        }
                        fallback_result
                    } else {
                        selection_result
                    }
                } else {
                    // No confidence field; pass through
                    selection_result
                }
            }
            Action::Populate => {
                // Bind input into selected template's fields
                // State should contain selected_template_id from previous step
                Value::String(format!("Populated: {:?}", state))
            }
            Action::Execute => {
                // Execute via MCP tool or inference
                if let Some(mcp) = &step.mcp {
                    if mcp == "from_template_contract" {
                        // Target determined by template contract
                        Value::String(format!("Executed via contract: {:?}", state))
                    } else {
                        // Invoke specific MCP tool
                        self.mcp.invoke(mcp, state.clone())?
                    }
                } else {
                    Value::String(format!("Executed: {:?}", state))
                }
            }
        };

        // Emit CNS event for this step
        self.cns.emit(
            &format!("cns.prompt.{}", step.action.as_str()),
            result.clone(),
            1.0,
        );

        Ok(result)
    }
}

impl<R, I, M, C> ManifestExecutor for ManifestExecutorImpl<R, I, M, C>
where
    R: TemplateRenderer,
    I: InferencePort,
    M: McpPort,
    C: CnsPort,
{
    fn load(&self, _path: &std::path::Path) -> Result<ProcessManifest> {
        // In production, this would load from YAML file
        // For now, return bootstrap manifest from registry
        Err(TemplateError::Manifest(
            "Use RegistryIndex::bootstrap_manifest() instead".to_string(),
        ))
    }

    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
        info!(
            target: "hkask.templates",
            manifest = %manifest.id,
            steps = manifest.steps.len(),
            "Executing manifest"
        );

        let mut state = input;
        for step in &manifest.steps {
            state = self.execute_step(step, state, 0)?;
        }

        // Emit final outcome event
        self.cns.emit("cns.prompt.outcome", state.clone(), 1.0);

        Ok(state)
    }
}

/// Simple manifest executor for testing
pub struct SimpleExecutor;

impl ManifestExecutor for SimpleExecutor {
    fn load(&self, _path: &std::path::Path) -> Result<ProcessManifest> {
        Err(TemplateError::Manifest(
            "SimpleExecutor does not support loading".to_string(),
        ))
    }

    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
        let mut state = input;
        for step in &manifest.steps {
            state = match step.action {
                Action::Select => Value::String(format!("Selected: {:?}", state)),
                Action::Populate => Value::String(format!("Populated: {:?}", state)),
                Action::Execute => Value::String(format!("Executed: {:?}", state)),
            };
        }
        Ok(state)
    }
}
