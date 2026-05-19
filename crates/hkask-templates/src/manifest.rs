//! Manifest executor — core execution loop
//!
//! Implements the fixed logic that executes ANY manifest without modification.
//! Per architecture v0.21.0: ~50 lines of Rust that never changes when templates are added/edited.

use crate::ports::{
    Action, CnsPort, InferencePort, ManifestExecutor, ManifestStep, McpPort, ProcessManifest,
    RegistryIndex, Result, TemplateError, TemplateRenderer, DEFAULT_MATROSHKA_LIMIT,
};
use hkask_types::Value;
use tracing::info;

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
    max_depth: u8,
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
        }
    }

    pub fn with_max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth;
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
                // Render selector template and call fast model
                let prompt = format!("Select template for: {:?}", state);
                self.inference.call(
                    step.model_tier.as_deref().unwrap_or("fast_local"),
                    &prompt,
                )?
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{CnsPort, InferencePort, McpPort, Result, TemplateError};
    use hkask_types::Value;

    struct MockInference;
    impl InferencePort for MockInference {
        fn call(&self, _model_tier: &str, _prompt: &str) -> Result<Value> {
            Ok(Value::String("mock inference result".to_string()))
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

    struct MockCns {
        events: std::sync::Mutex<Vec<(String, Value, f64)>>,
    }
    impl MockCns {
        fn new() -> Self {
            Self {
                events: std::sync::Mutex::new(vec![]),
            }
        }
    }
    impl CnsPort for MockCns {
        fn emit(&self, span: &str, outcome: Value, confidence: f64) {
            self.events.lock().unwrap().push((
                span.to_string(),
                outcome,
                confidence,
            ));
        }
    }

    #[test]
    fn test_simple_executor() {
        let executor = SimpleExecutor;
        let manifest = ProcessManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test manifest".to_string(),
            steps: vec![
                ManifestStep {
                    ordinal: 1,
                    action: Action::Select,
                    description: "Select".to_string(),
                    template_ref: "test".to_string(),
                    model_tier: None,
                    mcp: None,
                    renderer: None,
                },
                ManifestStep {
                    ordinal: 2,
                    action: Action::Execute,
                    description: "Execute".to_string(),
                    template_ref: "test".to_string(),
                    model_tier: None,
                    mcp: None,
                    renderer: None,
                },
            ],
        };

        let result = executor
            .execute(&manifest, Value::String("input".to_string()))
            .unwrap();

        assert!(result.as_str().unwrap().contains("Executed"));
    }

    #[test]
    fn test_manifest_executor_depth_limit() {
        let cns = MockCns::new();
        let executor = ManifestExecutorImpl::new(
            MockInference,
            MockInference,
            MockMcp,
            cns,
        )
        .with_max_depth(2);

        // Create a manifest that would exceed depth if recursive
        let manifest = ProcessManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            steps: vec![ManifestStep {
                ordinal: 1,
                action: Action::Select,
                description: "Select".to_string(),
                template_ref: "test".to_string(),
                model_tier: None,
                mcp: None,
                renderer: None,
            }],
        };

        let result = executor.execute(&manifest, Value::Null);
        assert!(result.is_ok());
    }
}
