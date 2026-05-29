//! Manifest executor — core execution loop
//!
//! Implements the fixed logic that executes ANY manifest without modification.
//! Per architecture v0.21.0: ~50 lines of Rust that never changes when templates are added/edited.

use crate::adapters::AppMemoryAdapter;
use crate::ports::{
    Action, CnsPort, DEFAULT_MATROSHKA_LIMIT, InferenceConfig, ManifestStep, McpPort,
    ProcessManifest, Result, TemplateError,
};
use crate::renderer::TemplateRendererImpl;
use hkask_cns::EnergyBudget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

/// Canonical input keys for context assembly
///
/// These keys are used to extract context from the input JSON value.
/// Manifests that use context assembly should declare these in their
/// `TemplateContract.input_fields`.
pub mod context_keys {
    /// User's message or prompt
    pub const USER_MESSAGE: &str = "user_message";
    /// Alternative key for user message
    pub const PROMPT: &str = "prompt";
    /// Entity to query from memory
    pub const ENTITY: &str = "entity";
    /// Perspective for episodic memory (agent WebID)
    pub const PERSPECTIVE: &str = "perspective";
    /// Session ID for history retrieval
    pub const SESSION_ID: &str = "session_id";
}

/// Model category for selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCategory {
    Fast,
    Balanced,
    Reasoning,
    Embedding,
}

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

/// Energy budget tracker for manifest execution
///
/// Mirrors `hkask_cns::EnergyBudget` for local step-based accounting.
/// Integrates token-based cost estimation via `hkask_types::estimate_tokens()`.
#[derive(Debug, Clone)]
pub struct EnergyAccount {
    pub budget: u64,
    pub consumed: u64,
    /// Optional CNS budget for algedonic integration
    pub cns_budget: Option<hkask_cns::EnergyBudget>,
}

impl EnergyAccount {
    pub fn new(budget: u64) -> Self {
        Self {
            budget,
            consumed: 0,
            cns_budget: None,
        }
    }

    /// Create with a CNS energy budget for alerting
    pub fn with_cns_budget(budget: u64, cns_budget: hkask_cns::EnergyBudget) -> Self {
        Self {
            budget,
            consumed: 0,
            cns_budget: Some(cns_budget),
        }
    }

    pub fn remaining(&self) -> u64 {
        self.budget.saturating_sub(self.consumed)
    }

    pub fn debit(&mut self, cost: u64) -> bool {
        if self.consumed + cost > self.budget {
            return false;
        }
        self.consumed += cost;

        // Mirror debit to CNS budget for token-based algedonic alerts
        if let Some(ref mut cns_budget) = self.cns_budget {
            // Map step cost to approximate tokens (1 energy ≈ 4 tokens)
            let approx_tokens = cost * 4;
            let _ = cns_budget.allocate(approx_tokens);
        }
        true
    }

    /// Check if CNS budget alert threshold has been exceeded
    pub fn should_alert(&self) -> bool {
        self.cns_budget
            .as_ref()
            .map(|b| b.should_alert())
            .unwrap_or(false)
    }
}

/// No-op CSP enforcer for when CSP is not configured
pub struct NoopCsp;

impl NoopCsp {
    pub fn enforce(&self, _step: &ManifestStep, _state: &Value) -> Result<()> {
        Ok(())
    }
}

/// Core manifest execution loop — fixed logic, applies to ANY manifest
///
/// This is the "loom" that weaves the "thread" (YAML/Jinja2 templates).
/// It doesn't change when templates are added, edited, or removed.
/// Only changes if the grammar of steps themselves changes.
pub struct ManifestExecutorImpl<M, C> {
    renderer: TemplateRendererImpl,
    mcp: M,
    cns: C,
    memory: Option<AppMemoryAdapter>,
    csp: Option<Box<NoopCsp>>,
    max_depth: u8,
    selector_config: SelectorConfig,
    inference_config: InferenceConfig,
    context_budget: usize,
    energy_budget: u64,
}

impl<M, C> ManifestExecutorImpl<M, C>
where
    M: McpPort,
    C: CnsPort,
{
    pub fn new(renderer: TemplateRendererImpl, mcp: M, cns: C) -> Self {
        Self {
            renderer,
            mcp,
            cns,
            memory: None,
            csp: None,
            max_depth: DEFAULT_MATROSHKA_LIMIT,
            selector_config: SelectorConfig::default(),
            inference_config: InferenceConfig::default(),
            context_budget: 4096,
            energy_budget: 10_000,
        }
    }

    pub fn with_memory(mut self, memory: AppMemoryAdapter) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_csp(mut self, csp: Box<NoopCsp>) -> Self {
        self.csp = Some(csp);
        self
    }

    pub fn with_context_budget(mut self, budget: usize) -> Self {
        self.context_budget = budget;
        self
    }

    pub fn with_energy_budget(mut self, budget: u64) -> Self {
        self.energy_budget = budget;
        self
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

    async fn execute_step(
        &self,
        _manifest: &ProcessManifest,
        step: &ManifestStep,
        state: Value,
        depth: u8,
        energy: &mut EnergyAccount,
    ) -> Result<Value> {
        if depth > self.max_depth {
            return Err(TemplateError::RecursionLimit {
                max: self.max_depth,
            });
        }

        let step_cost = match step.action {
            Action::Select => 100,
            Action::Populate => 50,
            Action::Execute => 500,
        };
        if !energy.debit(step_cost) {
            self.cns.emit(
                "cns.energy.algedonic",
                serde_json::json!({
                    "consumed": energy.consumed,
                    "budget": energy.budget,
                    "step": step.action.as_str(),
                }),
                0.0,
            );
            return Err(TemplateError::Manifest(format!(
                "Energy exhausted: {}/{}",
                energy.consumed, energy.budget
            )));
        }

        if let Some(csp) = &self.csp {
            csp.enforce(step, &state)?;
        }

        let result = match step.action {
            Action::Select => {
                return Err(TemplateError::Manifest(
                    "Select action is not available — use Populate or Execute".to_string(),
                ));
            }
            Action::Populate => {
                let template_id = state
                    .get("selected_template_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        TemplateError::Manifest(
                            "Populate action requires selected_template_id in state".to_string(),
                        )
                    })?;

                let template_path = std::path::Path::new(template_id);
                let template = self.renderer.load(template_path)?;
                let rendered = self.renderer.render(&template, state.clone())?;

                self.cns.emit(
                    "cns.prompt.populate",
                    serde_json::json!({
                        "template_id": template_id,
                        "rendered_length": rendered.len(),
                    }),
                    1.0,
                );

                Value::String(rendered)
            }
            Action::Execute => {
                if let Some(mcp) = &step.mcp {
                    if mcp == "from_template_contract" {
                        let template_id = state
                            .get("selected_template_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                TemplateError::Manifest(
                                    "Execute with from_template_contract requires selected_template_id in state".to_string(),
                                )
                            })?;

                        let template_path = std::path::Path::new(template_id);
                        let template = self.renderer.load(template_path)?;
                        let target_tool = template
                            .contract
                            .output_fields
                            .first()
                            .cloned()
                            .unwrap_or_else(|| template_id.to_string());

                        let result = self.mcp.invoke(&target_tool, state.clone()).await?;
                        self.cns.emit(
                            "cns.prompt.execute_contract",
                            serde_json::json!({
                                "template_id": template_id,
                                "target_tool": target_tool,
                            }),
                            1.0,
                        );
                        result
                    } else {
                        self.mcp.invoke(mcp, state.clone()).await?
                    }
                } else {
                    return Err(TemplateError::Manifest(
                        "Execute action requires an MCP step".to_string(),
                    ));
                }
            }
        };

        self.cns.emit(
            &format!("cns.prompt.{}", step.action.as_str()),
            result.clone(),
            1.0,
        );

        Ok(result)
    }
}

impl<M, C> ManifestExecutorImpl<M, C>
where
    M: McpPort,
    C: CnsPort + Send + Sync,
{
    pub fn load(&self, _path: &std::path::Path) -> Result<ProcessManifest> {
        Err(TemplateError::Manifest(
            "Use RegistryIndex::bootstrap_manifest() instead".to_string(),
        ))
    }

    pub async fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
        info!(
            target: "hkask.templates",
            manifest = %manifest.id,
            steps = manifest.steps.len(),
            "Executing manifest"
        );

        let mut energy = EnergyAccount::with_cns_budget(
            self.energy_budget,
            EnergyBudget::new(self.energy_budget).with_alert_threshold(0.8),
        );
        let mut state = input;
        for step in &manifest.steps {
            let step_result = self
                .execute_step(manifest, step, state.clone(), 0, &mut energy)
                .await?;
            state = merge_state(state, step_result);
        }

        self.cns.emit(
            "cns.energy.final",
            serde_json::json!({
                "consumed": energy.consumed,
                "budget": energy.budget,
            }),
            1.0,
        );
        self.cns.emit("cns.prompt.outcome", state.clone(), 1.0);

        Ok(state)
    }
}

fn merge_state(mut base: Value, step_output: Value) -> Value {
    match (&mut base, step_output) {
        (Value::Object(base_map), Value::Object(step_map)) => {
            for (k, v) in step_map {
                base_map.insert(k, v);
            }
            base
        }
        (_, step_output) => step_output,
    }
}

/// Simple manifest executor for testing
pub struct SimpleExecutor;

impl SimpleExecutor {
    pub fn load(&self, _path: &std::path::Path) -> Result<ProcessManifest> {
        Err(TemplateError::Manifest(
            "SimpleExecutor does not support loading".to_string(),
        ))
    }

    pub async fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
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
