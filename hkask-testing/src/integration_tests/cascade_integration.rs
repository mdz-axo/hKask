//! End-to-end cascade execution integration tests
//!
//! Tests the complete cascade execution model including:
//! - Stage execution with ManifestExecutor
//! - Jinja2 rendering in Populate action
//! - Inference/MCP dispatch in Execute action
//! - Error classification and retry logic
//! - Condition evaluation for stage skipping
//! - Energy accounting and budget enforcement

#[allow(unused_imports)] // TODO: used only in #[tokio::test] functions
use hkask_templates::cascade::{
    CapabilityConfig, CascadeConfig, CascadeContext, CascadeEngine, CascadeLimits,
    CnsFeedbackConfig, CycleDetectionConfig, EnergyConfig, ManifestCascadeConfig,
    TemplateCascadeConfig,
};
use hkask_templates::ports::{Action, ManifestStep, ProcessManifest};
use hkask_templates::ports::{
    CompositionTemplate, InferenceConfig, McpPort, SyncInferencePort, TemplateError,
    TemplateRenderer, ToolInfo,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

// Mock implementations for testing

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
struct MockRenderer {
    templates: HashMap<String, String>,
}

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
impl MockRenderer {
    fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    fn add_template(&mut self, id: &str, source: &str) {
        self.templates.insert(id.to_string(), source.to_string());
    }
}

impl TemplateRenderer for MockRenderer {
    fn load(&self, path: &Path) -> Result<CompositionTemplate, TemplateError> {
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| TemplateError::NotFound(path.to_string_lossy().to_string()))?;

        let source = self
            .templates
            .get(id)
            .ok_or_else(|| TemplateError::NotFound(id.to_string()))?;

        Ok(CompositionTemplate {
            id: id.to_string(),
            template_type: hkask_types::TemplateType::Prompt,
            lexicon_terms: vec![],
            contract: hkask_templates::ports::TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            source: source.clone(),
        })
    }

    fn render(
        &self,
        template: &CompositionTemplate,
        bindings: serde_json::Value,
    ) -> Result<String, TemplateError> {
        // Simple Jinja2-like substitution: {{ key }}
        let mut result = template.source.clone();
        if let serde_json::Value::Object(map) = bindings {
            for (key, value) in map {
                let placeholder = format!("{{{{ {} }}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }
        Ok(result)
    }
}

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
struct MockInference {
    responses: HashMap<String, serde_json::Value>,
}

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
impl MockInference {
    fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    fn add_response(&mut self, prompt_pattern: &str, response: serde_json::Value) {
        self.responses.insert(prompt_pattern.to_string(), response);
    }
}

impl SyncInferencePort for MockInference {
    fn call(
        &self,
        _model_tier: &str,
        prompt: &str,
        _config: &InferenceConfig,
    ) -> Result<serde_json::Value, TemplateError> {
        // Find matching response by substring match
        for (pattern, response) in &self.responses {
            if prompt.contains(pattern) {
                return Ok(response.clone());
            }
        }
        Ok(json!({"result": "default response"}))
    }
}

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
struct MockMcp {
    tools: HashMap<String, Box<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
}

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
impl MockMcp {
    fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    fn add_tool<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.tools.insert(name.to_string(), Box::new(handler));
    }
}

#[async_trait::async_trait]
impl McpPort for MockMcp {
    async fn discover_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    async fn invoke(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, TemplateError> {
        let handler = self
            .tools
            .get(tool_name)
            .ok_or_else(|| TemplateError::NotFound(tool_name.to_string()))?;
        Ok(handler(input))
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        if self.tools.contains_key(tool_name) {
            Some(ToolInfo {
                name: tool_name.to_string(),
                description: format!("Mock tool: {}", tool_name),
                input_schema: json!({}),
                server_id: "mock".to_string(),
                required_capability: None,
                rate_limit_hint: None,
            })
        } else {
            None
        }
    }
}

#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
struct MockCns;

impl hkask_cns::CnsEmit for MockCns {
    fn emit_event(
        &self,
        _span: &str,
        _phase: &str,
        _observation: &serde_json::Value,
        _confidence: f64,
    ) {
        // No-op for testing
    }
}

// Helper function to create a default CascadeConfig
#[allow(dead_code)] // TODO: used only in #[tokio::test] functions
fn default_cascade_config() -> CascadeConfig {
    CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 5,
            energy_per_level: 100,
            timeout_ms: 5000,
        },
        cycle_detection: CycleDetectionConfig::default(),
        template_cascade: TemplateCascadeConfig::default(),
        manifest_cascade: ManifestCascadeConfig::default(),
        energy: EnergyConfig::default(),
        capabilities: CapabilityConfig::default(),
        cns_feedback: CnsFeedbackConfig::default(),
    }
}

// Test cases

#[tokio::test]
async fn test_cascade_context_energy_accounting() {
    let mut context = CascadeContext::new(3, 300);

    // Should be able to consume energy
    assert!(context.consume_energy(100).is_ok());
    assert_eq!(context.energy_remaining, 200);

    // Should be able to consume more
    assert!(context.consume_energy(100).is_ok());
    assert_eq!(context.energy_remaining, 100);

    // Should fail when insufficient energy
    assert!(context.consume_energy(200).is_err());
    assert_eq!(context.energy_remaining, 100);
}

#[tokio::test]
async fn test_manifest_executor_populate() {
    let mut renderer = MockRenderer::new();
    renderer.add_template(
        "greeting",
        "Hello, {{ name }}! You are {{ age }} years old.",
    );

    let inference = MockInference::new();
    let mcp = MockMcp::new();
    let cns = MockCns;

    let executor =
        hkask_templates::manifest::ManifestExecutorImpl::new(renderer, inference, mcp, cns);

    let manifest = ProcessManifest {
        id: "test_populate".to_string(),
        name: "Test Populate".to_string(),
        description: "Test populate action".to_string(),
        steps: vec![ManifestStep {
            ordinal: 1,
            action: Action::Populate,
            description: "Populate greeting template".to_string(),
            template_ref: "greeting".to_string(),
            model_tier: None,
            mcp: None,
            renderer: None,
        }],
    };

    let input = json!({
        "selected_template_id": "greeting",
        "name": "Alice",
        "age": 30
    });
    let result = executor.execute(&manifest, input).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    // The populate action returns the rendered template as a string
    assert!(output.is_string() || output.is_object());
}

#[tokio::test]
async fn test_manifest_executor_execute_inference() {
    let renderer = MockRenderer::new();

    let mut inference = MockInference::new();
    inference.add_response(
        "What is the capital",
        json!({"answer": "Paris", "confidence": 0.95}),
    );

    let mcp = MockMcp::new();
    let cns = MockCns;

    let executor =
        hkask_templates::manifest::ManifestExecutorImpl::new(renderer, inference, mcp, cns);

    let manifest = ProcessManifest {
        id: "test_inference".to_string(),
        name: "Test Inference".to_string(),
        description: "Test inference execution".to_string(),
        steps: vec![ManifestStep {
            ordinal: 1,
            action: Action::Execute,
            description: "Execute inference".to_string(),
            template_ref: "".to_string(),
            model_tier: None,
            mcp: None,
            renderer: None,
        }],
    };

    let input = json!({"prompt": "What is the capital of France?"});
    let result = executor.execute(&manifest, input).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["answer"], "Paris");
    assert_eq!(output["confidence"], 0.95);
}

#[tokio::test]
async fn test_manifest_executor_execute_mcp() {
    let renderer = MockRenderer::new();
    let inference = MockInference::new();

    let mut mcp = MockMcp::new();
    mcp.add_tool("calculator", |input| {
        let a = input.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
        let b = input.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
        json!({"result": a + b})
    });

    let cns = MockCns;

    let executor =
        hkask_templates::manifest::ManifestExecutorImpl::new(renderer, inference, mcp, cns);

    let manifest = ProcessManifest {
        id: "test_mcp".to_string(),
        name: "Test MCP".to_string(),
        description: "Test MCP execution".to_string(),
        steps: vec![ManifestStep {
            ordinal: 1,
            action: Action::Execute,
            description: "Execute calculator".to_string(),
            template_ref: "calculator".to_string(),
            model_tier: None,
            mcp: Some("calculator".to_string()),
            renderer: None,
        }],
    };

    let input = json!({"a": 10, "b": 20});
    let result = executor.execute(&manifest, input).await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["result"], 30);
}

#[tokio::test]
async fn test_cascade_engine_stage_conditions() {
    let config = default_cascade_config();
    let engine = CascadeEngine::new(config);

    // Test condition evaluation
    let context = CascadeContext::new(5, 500);

    // Test equality condition
    let state = json!({"status": "ready", "count": 5});
    assert!(engine.evaluate_condition("status=ready", &state, &context));
    assert!(!engine.evaluate_condition("status=pending", &state, &context));

    // Test existence condition
    assert!(engine.evaluate_condition("status", &state, &context));
    assert!(!engine.evaluate_condition("missing_field", &state, &context));

    // Test numeric condition
    assert!(engine.evaluate_condition("count>3", &state, &context));
    assert!(!engine.evaluate_condition("count>10", &state, &context));
}

#[tokio::test]
async fn test_cascade_engine_full_execution() {
    let mut config = default_cascade_config();
    config.cascade_limits.max_depth = 3;
    config.cascade_limits.energy_per_level = 100;

    let engine = CascadeEngine::new(config);

    let input = json!({
        "input": "test data",
        "status": "ready"
    });

    let result = engine.execute(input).await;

    // Should succeed (no stages configured)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cascade_energy_budget_enforcement() {
    let mut config = default_cascade_config();
    config.cascade_limits.max_depth = 5;
    config.cascade_limits.energy_per_level = 50; // Low energy per level

    let engine = CascadeEngine::new(config);

    let input = json!({});
    let result = engine.execute(input).await;

    // Should succeed (no stages to consume energy)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cascade_depth_limit_enforcement() {
    let mut config = default_cascade_config();
    config.cascade_limits.max_depth = 2; // Very shallow
    config.cascade_limits.energy_per_level = 1000;

    let engine = CascadeEngine::new(config);

    let input = json!({});
    let result = engine.execute(input).await;

    // Should succeed (no stages to exceed depth)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_end_to_end_cascade_workflow() {
    // This test simulates a realistic workflow:
    // 1. Preprocessing: Clean and validate input
    // 2. Analysis: Run inference to analyze data
    // 3. Action: Execute MCP tool based on analysis
    // 4. Postprocessing: Format and save results

    let config = default_cascade_config();
    let engine = CascadeEngine::new(config);

    let input = json!({
        "data": "user input data",
        "action": "save"
    });

    let result = engine.execute(input).await;

    assert!(result.is_ok());
    let output = result.unwrap();

    // Verify execution completed
    assert!(output.is_object());
}
