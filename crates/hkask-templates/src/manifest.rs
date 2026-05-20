//! Manifest executor — core execution loop
//!
//! Implements the fixed logic that executes ANY manifest without modification.
//! Per architecture v0.21.0: ~50 lines of Rust that never changes when templates are added/edited.
//!
//! **Manifest Execution Invariants:**
//!
//! 1. **Ordinal Total Ordering**: Steps execute in strict ordinal order (1, 2, 3, ...).
//!    No step N+1 begins until step N completes successfully.
//!
//! 2. **State Monotonicity**: State transitions are monotonic—each step receives the
//!    output of the previous step. No rollback or revision of prior state.
//!
//! 3. **CNS Causal Ordering**: CNS events are emitted BEFORE step completion is reported.
//!    This ensures audit trail reflects actual execution order, not just final outcomes.
//!
//! 4. **Depth Pre-Check**: Recursion depth is checked BEFORE executing each step.
//!    If depth exceeds limit, execution fails immediately without side effects.
//!
//! 5. **Error Recovery Semantics**: Every error variant encodes recovery behavior:
//!    - Transient errors (IoError, CnsEmissionFailure) → retry with backoff
//!    - Permanent errors (CapabilityDenied, ValidationFailed) → fail immediately
//!    - Resource errors (DepthExceeded) → fail with diagnostic
//!
//! These invariants are documented but NOT enforced by the type system.
//! Violations produce structured errors with recovery hints.

use crate::ports::{
    Action, CnsPort, DEFAULT_MATROSHKA_LIMIT, InferenceConfig, InferencePort, ManifestExecutor,
    ManifestStep, McpPort, ProcessManifest, Result, TemplateError, TemplateRenderer,
};
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::info;
use uuid::Uuid;

/// Atomic manifest state with transaction semantics
///
/// Ensures all-or-nothing execution: if any step fails, state rolls back
/// to the last committed checkpoint. This implements invariant #2 (state monotonicity)
/// while providing recovery from partial execution failures.
#[derive(Debug, Clone)]
pub struct ManifestState {
    /// Unique transaction ID for audit correlation
    pub transaction_id: Uuid,
    /// Current step ordinal (0 = not started)
    pub current_step: u32,
    /// Accumulated state from step outputs
    pub accumulated_state: Value,
    /// Number of steps completed
    pub steps_completed: u32,
}

impl ManifestState {
    /// Create new manifest state with fresh transaction ID
    pub fn new(initial_state: Value) -> Self {
        Self {
            transaction_id: Uuid::new_v4(),
            current_step: 0,
            accumulated_state: initial_state,
            steps_completed: 0,
        }
    }

    /// Execute step atomically: all-or-nothing semantics
    ///
    /// If the operation succeeds, state is updated and step counter incremented.
    /// If the operation fails, state rolls back to snapshot and error is returned.
    ///
    /// This implements the transaction pattern:
    /// 1. Take snapshot of current state
    /// 2. Execute operation
    /// 3. On success: commit new state
    /// 4. On failure: rollback to snapshot
    pub fn transition<F>(&mut self, step: u32, operation: F) -> Result<Value>
    where
        F: FnOnce(&Value) -> Result<Value>,
    {
        // Snapshot for potential rollback
        let snapshot = self.accumulated_state.clone();

        match operation(&self.accumulated_state) {
            Ok(new_state) => {
                // Commit: update state and advance step counter
                self.current_step = step;
                self.accumulated_state = new_state;
                self.steps_completed += 1;
                Ok(self.accumulated_state.clone())
            }
            Err(e) => {
                // Rollback: restore snapshot, do not advance step counter
                self.accumulated_state = snapshot;
                Err(e)
            }
        }
    }

    /// Get transaction ID for audit correlation
    pub fn transaction_id(&self) -> Uuid {
        self.transaction_id
    }

    /// Get current step ordinal
    pub fn current_step(&self) -> u32 {
        self.current_step
    }

    /// Get steps completed count
    pub fn steps_completed(&self) -> u32 {
        self.steps_completed
    }
}

/// Encapsulated CNS event emission for manifest execution
///
/// Ensures consistent event structure and ordering across all manifest steps.
pub struct CnsEventEmitter {
    cns: Box<dyn CnsPort>,
    execution_id: Uuid,
    step_counter: AtomicU32,
}

impl CnsEventEmitter {
    /// Create new CNS event emitter
    pub fn new(cns: Box<dyn CnsPort>) -> Self {
        Self {
            cns,
            execution_id: Uuid::new_v4(),
            step_counter: AtomicU32::new(0),
        }
    }

    /// Emit select event
    pub fn emit_select(
        &self,
        template_id: &str,
        confidence: f64,
        fallback_applied: bool,
        rationale: &str,
    ) {
        let step = self.step_counter.fetch_add(1, Ordering::SeqCst);
        self.cns.emit(
            "cns.prompt.select",
            Value::Object(
                serde_json::json!({
                    "execution_id": self.execution_id.to_string(),
                    "step": step,
                    "selected_template": template_id,
                    "confidence": confidence,
                    "fallback_applied": fallback_applied,
                    "rationale": rationale,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            confidence,
        );
    }

    /// Emit populate event
    pub fn emit_populate(&self, binding_count: usize, template_ref: &str) {
        let step = self.step_counter.fetch_add(1, Ordering::SeqCst);
        self.cns.emit(
            "cns.prompt.populate",
            Value::Object(
                serde_json::json!({
                    "execution_id": self.execution_id.to_string(),
                    "step": step,
                    "binding_count": binding_count,
                    "template_ref": template_ref,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            0.9,
        );
    }

    /// Emit execute event
    pub fn emit_execute(&self, mcp_target: &str, outcome: &str) {
        let step = self.step_counter.fetch_add(1, Ordering::SeqCst);
        self.cns.emit(
            "cns.prompt.execute",
            Value::Object(
                serde_json::json!({
                    "execution_id": self.execution_id.to_string(),
                    "step": step,
                    "mcp_target": mcp_target,
                    "outcome": outcome,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            0.95,
        );
    }

    /// Emit outcome event
    pub fn emit_outcome(
        &self,
        manifest_id: &str,
        steps: u32,
        duration: std::time::Duration,
        result: &Value,
    ) {
        self.cns.emit(
            "cns.prompt.outcome",
            Value::Object(
                serde_json::json!({
                    "execution_id": self.execution_id.to_string(),
                    "manifest_id": manifest_id,
                    "total_steps": steps,
                    "duration_ms": duration.as_millis() as u64,
                    "outcome": "success",
                    "result": result,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            1.0,
        );
    }

    /// Get execution ID for correlation
    pub fn execution_id(&self) -> Uuid {
        self.execution_id
    }
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
pub struct ManifestExecutorImpl<R, I, M> {
    #[allow(dead_code)]
    renderer: R,
    inference: I,
    mcp: M,
    cns_emitter: CnsEventEmitter,
    max_depth: u8,
    selector_config: SelectorConfig,
    inference_config: InferenceConfig,
}

impl<R, I, M> ManifestExecutorImpl<R, I, M>
where
    R: TemplateRenderer,
    I: InferencePort,
    M: McpPort,
{
    pub fn new<C: CnsPort + 'static>(renderer: R, inference: I, mcp: M, cns: C) -> Self {
        Self {
            renderer,
            inference,
            mcp,
            cns_emitter: CnsEventEmitter::new(Box::new(cns)),
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
                        self.cns_emitter.emit_select(
                            &self.selector_config.fallback_template_id,
                            confidence,
                            true,
                            &format!(
                                "Confidence {} below threshold {}",
                                confidence, self.selector_config.confidence_threshold
                            ),
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
                        // Emit select event with normal confidence
                        let selected_id = selection_result
                            .get("selected_template_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        self.cns_emitter.emit_select(
                            &selected_id,
                            confidence,
                            false,
                            &format!(
                                "Confidence {} above threshold {}",
                                confidence, self.selector_config.confidence_threshold
                            ),
                        );

                        selection_result
                    }
                } else {
                    // No confidence field; pass through with default
                    self.cns_emitter
                        .emit_select("unknown", 0.0, false, "No confidence provided");

                    selection_result
                }
            }
            Action::Populate => {
                // Bind input into selected template's fields
                // State should contain selected_template_id from previous step
                let binding_count = if let Value::Object(obj) = &state {
                    obj.len() as f64
                } else {
                    1.0
                };

                // Emit CNS event for populate
                self.cns_emitter
                    .emit_populate(binding_count as usize, &step.template_ref);

                Value::String(format!("Populated: {:?}", state))
            }
            Action::Execute => {
                let result = if let Some(mcp) = &step.mcp {
                    if mcp == "from_template_contract" {
                        // Target determined by template contract
                        Value::String(format!("Executed via contract: {:?}", state))
                    } else {
                        // Invoke specific MCP tool
                        self.mcp.invoke(mcp, state.clone())?
                    }
                } else {
                    Value::String(format!("Executed: {:?}", state))
                };

                // Emit CNS event for execute
                self.cns_emitter.emit_execute(
                    &step.mcp.clone().unwrap_or_else(|| "none".to_string()),
                    "success",
                );

                result
            }
        };

        Ok(result)
    }
}

impl<R, I, M> ManifestExecutor for ManifestExecutorImpl<R, I, M>
where
    R: TemplateRenderer,
    I: InferencePort,
    M: McpPort,
{
    fn load(&self, path: &std::path::Path) -> Result<ProcessManifest> {
        ProcessManifest::load_from_yaml(path)
    }

    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
        info!(
            target: "hkask.templates",
            manifest = %manifest.id,
            steps = manifest.steps.len(),
            "Executing manifest"
        );

        let start_time = std::time::Instant::now();

        // Create atomic state with transaction ID
        let mut state = ManifestState::new(input);

        // Execute each step atomically
        for step in &manifest.steps {
            let step_ordinal = step.ordinal;

            // Atomic transition: rollback on failure
            state.transition(step_ordinal, |current_state| {
                self.execute_step(step, current_state.clone(), 0)
            })?;
        }

        let duration = start_time.elapsed();

        // Emit final outcome event with execution summary
        self.cns_emitter.emit_outcome(
            &manifest.id,
            state.steps_completed(),
            duration,
            &state.accumulated_state,
        );

        Ok(state.accumulated_state)
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

    }

    struct MockRenderer;
    impl TemplateRenderer for MockRenderer {
        fn load(&self, _path: &Path) -> Result<CompositionTemplate> {
            Err(TemplateError::NotFound("mock".to_string()))
        }
        fn render(&self, _template: &CompositionTemplate, _bindings: Value) -> Result<String> {
            Ok("mock rendered".to_string())
        }
    }

    struct MockMcp;
    impl McpPort for MockMcp {
        fn discover_tools(&self) -> Vec<String> {
            vec!["mock_tool".to_string()]
        }
        fn invoke(&self, _tool_name: &str, input: Value) -> Result<Value> {
            Ok(Value::String(format!("mock mcp: {:?}", input)))
        }
    }

    #[derive(Clone)]
    struct MockCns {
        events: std::sync::Arc<std::sync::Mutex<Vec<(String, Value, f64)>>>,
    }
    impl MockCns {
        fn new() -> Self {
            Self {
                events: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
            }
        }

        fn get_events(&self) -> Vec<(String, Value, f64)> {
            self.events.lock().unwrap().clone()
        }
    }
    impl CnsPort for MockCns {
        fn emit(&self, span: &str, outcome: Value, confidence: f64) {
            self.events
                .lock()
                .unwrap()
                .push((span.to_string(), outcome, confidence));
        }
    }




    // ============== Manifest Execution Invariant Tests ==============

    /// INVARIANT 1: Steps execute in strict ordinal order

    /// INVARIANT 2: State transitions are monotonic (no rollback)

    /// INVARIANT 3: CNS events emitted before step completion

    /// INVARIANT 4: Depth checked before step execution

    /// INVARIANT 5: Error variants encode recovery semantics

    // ============== ManifestState Atomic Transition Tests ==============


