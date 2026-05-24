//! Manifest executor — core execution loop
//!
//! Implements the fixed logic that executes ANY manifest without modification.
//! Per architecture v0.21.0: ~50 lines of Rust that never changes when templates are added/edited.

use crate::context_assembly::{ContextAssembler, ContextFragment, FragmentSource};
use crate::ports::{
    Action, CnsPort, DEFAULT_MATROSHKA_LIMIT, InferenceConfig, SyncInferencePort, ManifestExecutor,
    ManifestStep, McpPort, MemoryPort, ProcessManifest, Result, TemplateError, TemplateRenderer,
};
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

/// Core manifest execution loop — fixed logic, applies to ANY manifest
///
/// This is the "loom" that weaves the "thread" (YAML/Jinja2 templates).
/// It doesn't change when templates are added, edited, or removed.
/// Only changes if the grammar of steps themselves changes.
pub struct ManifestExecutorImpl<R, I, M, C> {
    renderer: R,
    inference: I,
    mcp: M,
    cns: C,
    memory: Option<Box<dyn MemoryPort>>,
    max_depth: u8,
    selector_config: SelectorConfig,
    inference_config: InferenceConfig,
    context_budget: usize,
}

impl<R, I, M, C> ManifestExecutorImpl<R, I, M, C>
where
    R: TemplateRenderer,
    I: SyncInferencePort,
    M: McpPort,
    C: CnsPort,
{
    pub fn new(renderer: R, inference: I, mcp: M, cns: C) -> Self {
        Self {
            renderer,
            inference,
            mcp,
            cns,
            memory: None,
            max_depth: DEFAULT_MATROSHKA_LIMIT,
            selector_config: SelectorConfig::default(),
            inference_config: InferenceConfig::default(),
            context_budget: 4096,
        }
    }

    pub fn with_memory(mut self, memory: Box<dyn MemoryPort>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_context_budget(mut self, budget: usize) -> Self {
        self.context_budget = budget;
        self
    }

    /// Assemble context from all sources with deduplication
    ///
    /// Priority order:
    /// 1. System instructions (from manifest metadata)
    /// 2. User message (from input state)
    /// 3. Memory context (semantic + episodic triples)
    /// 4. Session history (most recent first)
    fn assemble_context(
        &self,
        manifest: &ProcessManifest,
        input: &Value,
    ) -> (String, crate::context_assembly::AssemblyStats) {
        let mut assembler = ContextAssembler::new(self.context_budget);

        // Priority 1: System instructions from manifest
        let system_prompt = format!(
            "You are executing the {} manifest. {}",
            manifest.name, manifest.description
        );
        assembler.add(ContextFragment {
            content: system_prompt,
            source: FragmentSource::System,
            embedding: None,
            priority: 0,
        });

        // Priority 2: User message from input
        if let Some(user_msg) = input
            .get(context_keys::USER_MESSAGE)
            .and_then(|v| v.as_str())
        {
            assembler.add(ContextFragment {
                content: user_msg.to_string(),
                source: FragmentSource::User,
                embedding: None,
                priority: 1,
            });
        } else if let Some(prompt) = input.get(context_keys::PROMPT).and_then(|v| v.as_str()) {
            assembler.add(ContextFragment {
                content: prompt.to_string(),
                source: FragmentSource::User,
                embedding: None,
                priority: 1,
            });
        }

        // Priority 3: Memory context (if memory port available)
        if let Some(memory) = &self.memory {
            // Extract entity from input for memory queries
            let entity = input
                .get(context_keys::ENTITY)
                .and_then(|v| v.as_str())
                .unwrap_or("default");

            // Semantic memory
            let semantic_fragments = memory.query_semantic(entity).unwrap_or_default();
            for fragment in semantic_fragments {
                assembler.add(ContextFragment {
                    content: fragment.content,
                    source: FragmentSource::SemanticMemory,
                    embedding: None,
                    priority: 2,
                });
            }

            // Episodic memory (if perspective available)
            if let Some(perspective) = input
                .get(context_keys::PERSPECTIVE)
                .and_then(|v| v.as_str())
            {
                let episodic_fragments = memory.query_episodic(entity, perspective).unwrap_or_default();
                for fragment in episodic_fragments {
                    assembler.add(ContextFragment {
                        content: fragment.content,
                        source: FragmentSource::EpisodicMemory,
                        embedding: None,
                        priority: 2,
                    });
                }
            }

            // Session history (if session_id available)
            if let Some(session_id) = input.get(context_keys::SESSION_ID).and_then(|v| v.as_str()) {
                let history = memory.get_session_history(session_id, 20).unwrap_or_default();
                for message in history {
                    assembler.add(ContextFragment {
                        content: message,
                        source: FragmentSource::SessionHistory,
                        embedding: None,
                        priority: 3,
                    });
                }
            }
        }

        // Priority 3: Memory context (if memory port available)
        if let Some(memory) = &self.memory {
            // Extract entity from input for memory queries
            let entity = input
                .get(context_keys::ENTITY)
                .and_then(|v| v.as_str())
                .unwrap_or("default");

            // Semantic memory
            let semantic_fragments = memory.query_semantic(entity).unwrap_or_default();
            for fragment in semantic_fragments {
                assembler.add(ContextFragment {
                    content: fragment.content,
                    source: FragmentSource::SemanticMemory,
                    embedding: None,
                    priority: 2,
                });
            }

            // Episodic memory (if perspective available)
            if let Some(perspective) = input
                .get(context_keys::PERSPECTIVE)
                .and_then(|v| v.as_str())
            {
                let episodic_fragments = memory.query_episodic(entity, perspective).unwrap_or_default();
                for fragment in episodic_fragments {
                    assembler.add(ContextFragment {
                        content: fragment.content,
                        source: FragmentSource::EpisodicMemory,
                        embedding: None,
                        priority: 2,
                    });
                }
            }

            // Session history (if session_id available)
            if let Some(session_id) = input.get(context_keys::SESSION_ID).and_then(|v| v.as_str()) {
                let history = memory.get_session_history(session_id, 20).unwrap_or_default();
                for message in history {
                    assembler.add(ContextFragment {
                        content: message,
                        source: FragmentSource::SessionHistory,
                        embedding: None,
                        priority: 3,
                    });
                }
            }
        }

        let stats = assembler.stats().clone();
        let prompt = assembler.render();
        (prompt, stats)
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

    fn execute_step(
        &self,
        manifest: &ProcessManifest,
        step: &ManifestStep,
        state: Value,
        depth: u8,
    ) -> Result<Value> {
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
                // Bind input into selected template's fields using Jinja2 rendering
                // State should contain selected_template_id from previous Select step
                let template_id = state
                    .get("selected_template_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        TemplateError::Manifest(
                            "Populate action requires selected_template_id in state".to_string(),
                        )
                    })?;

                // Load template from registry
                let template_path = std::path::Path::new(template_id);
                let template = self.renderer.load(template_path)?;

                // Render template with state as bindings
                let rendered = self.renderer.render(&template, state.clone())?;

                // Emit CNS event for populate
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
                // Execute via MCP tool or inference
                if let Some(mcp) = &step.mcp {
                    if mcp == "from_template_contract" {
                        // Target determined by template contract
                        // Get selected_template_id from state
                        let template_id = state
                            .get("selected_template_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                TemplateError::Manifest(
                                    "Execute with from_template_contract requires selected_template_id in state".to_string(),
                                )
                            })?;

                        // Load template to get its contract
                        let template_path = std::path::Path::new(template_id);
                        let template = self.renderer.load(template_path)?;

                        // Extract target tool from contract (first output field or use template_id as tool name)
                        let target_tool = template
                            .contract
                            .output_fields
                            .first()
                            .cloned()
                            .unwrap_or_else(|| template_id.to_string());

                        // Invoke the target tool
                        let result = self.mcp.invoke(&target_tool, state.clone())?;

                        // Emit CNS event for contract-based execution
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
                        // Invoke specific MCP tool
                        self.mcp.invoke(mcp, state.clone())?
                    }
                } else {
                    // Assemble context with deduplication before inference
                    let (assembled_prompt, assembly_stats) =
                        self.assemble_context(manifest, &state);

                    // Emit CNS event for context assembly
                    self.cns.emit(
                        "cns.prompt.context_assembly",
                        serde_json::json!({
                            "fragments_offered": assembly_stats.fragments_offered,
                            "fragments_accepted": assembly_stats.fragments_accepted,
                            "duplicates_exact": assembly_stats.duplicates_exact,
                            "duplicates_similar": assembly_stats.duplicates_similar,
                            "budget_rejected": assembly_stats.budget_rejected,
                            "tokens_used": assembly_stats.tokens_used,
                            "tokens_budget": assembly_stats.tokens_budget,
                        }),
                        1.0,
                    );

                    // Call inference with assembled prompt
                    self.inference.call(
                        step.model_tier.as_deref().unwrap_or("balanced"),
                        &assembled_prompt,
                        &self.inference_config,
                    )?
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
    I: SyncInferencePort,
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
            state = self.execute_step(manifest, step, state, 0)?;
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
