//! End-to-end cascade execution integration tests
//!
//! Tests the complete cascade execution model including:
//! - Stage execution with ManifestExecutor
//! - Named operations in CspExecutor
//! - Jinja2 rendering in Populate action
//! - Inference/MCP dispatch in Execute action
//! - Error classification and retry logic
//! - Condition evaluation for stage skipping
//! - Energy accounting and budget enforcement

use hkask_templates::cascade::{CascadeConfig, CascadeContext, CascadeEngine, CascadeLimits};
use hkask_templates::csp::{CspConfig, CspExecutor, StageConfig};
use hkask_templates::ports::{
    Action, InferenceConfig, InferencePort, ManifestExecutor, ManifestStep, McpPort,
    ProcessManifest, TemplateError, TemplateRenderer,
};
use serde_json::json;
use std::collections::HashMap;

// Mock implementations for testing

struct MockRenderer {
    templates: HashMap<String, String>,
}

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
    fn render(&self, template: &str, bindings: &serde_json::Value) -> Result<String, TemplateError> {
        let source = self
            .templates
            .get(template)
            .ok_or_else(|| TemplateError::NotFound(template.to_string()))?;

        // Simple Jinja2-like substitution: {{ key }}
        let mut result = source.clone();
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

struct MockInference {
    responses: HashMap<String, serde_json::Value>,
}

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

impl InferencePort for MockInference {
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

struct MockMcp {
    tools: HashMap<String, Box<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
}

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

impl McpPort for MockMcp {
    fn invoke(
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
}

struct MockManifestExecutor {
    manifests: HashMap<String, ProcessManifest>,
}

impl MockManifestExecutor {
    fn new() -> Self {
        Self {
            manifests: HashMap::new(),
        }
    }

    fn add_manifest(&mut self, id: &str, manifest: ProcessManifest) {
        self.manifests.insert(id.to_string(), manifest);
    }
}

impl ManifestExecutor for MockManifestExecutor {
    fn execute(
        &self,
        manifest: &ProcessManifest,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, TemplateError> {
        // Simple pass-through for testing
        Ok(json!({
            "manifest_id": manifest.id,
            "input": input,
            "executed": true
        }))
    }
}

// Test cases

#[tokio::test]
async fn test_cascade_context_energy_accounting() {
    let config = CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 3,
            energy_per_level: 100,
            timeout_ms: 5000,
        },
    };

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
async fn test_csp_executor_named_operations() {
    let config = CspConfig::default();
    let mut executor = CspExecutor::new(config);

    // Register named operations
    executor.register_operation("double", |input| {
        let value = input.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(json!({"value": value * 2}))
    });

    executor.register_operation("increment", |input| {
        let value = input.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(json!({"value": value + 1}))
    });

    // Create stages
    let stage1 = StageConfig {
        name: "double".to_string(),
        timeout_ms: 1000,
        retry_on_failure: false,
    };

    let stage2 = StageConfig {
        name: "increment".to_string(),
        timeout_ms: 1000,
        retry_on_failure: false,
    };

    // Execute stages
    let input = json!({"value": 5});
    let result1 = executor.execute_stage(&stage1, input).await;
    assert!(result1.output.is_ok());
    assert_eq!(result1.output.unwrap()["value"], 10);

    let input2 = json!({"value": 10});
    let result2 = executor.execute_stage(&stage2, input2).await;
    assert!(result2.output.is_ok());
    assert_eq!(result2.output.unwrap()["value"], 11);
}

#[tokio::test]
async fn test_csp_executor_error_classification() {
    let mut config = CspConfig::default();
    config.error_handling.classification.retryable = vec!["timeout".to_string()];
    config.error_handling.classification.non_retryable = vec!["invalid".to_string()];
    config.stage_execution.retry.max_retries = 2;
    config.stage_execution.retry.initial_delay_ms = 10;
    config.stage_execution.retry.max_delay_ms = 50;

    let mut executor = CspExecutor::new(config);

    // Register operation that always fails with retryable error
    executor.register_operation("fail_timeout", |_| {
        Err(TemplateError::Timeout("operation timed out".to_string()))
    });

    let stage = StageConfig {
        name: "fail_timeout".to_string(),
        timeout_ms: 1000,
        retry_on_failure: true,
    };

    let input = json!({});
    let result = executor.execute_stage(&stage, input).await;

    // Should fail after retries
    assert!(result.output.is_err());
    // Should have attempted multiple times (initial + retries)
    assert!(result.duration_ms > 0);
}

#[tokio::test]
async fn test_manifest_executor_populate() {
    let mut renderer = MockRenderer::new();
    renderer.add_template("greeting", "Hello, {{ name }}! You are {{ age }} years old.");

    let inference = MockInference::new();
    let mcp = MockMcp::new();

    let executor = ManifestExecutor::new(renderer, inference, mcp);

    let manifest = ProcessManifest {
        id: "test_populate".to_string(),
        steps: vec![ManifestStep {
            action: Action::Populate,
            template_id: "greeting".to_string(),
            bindings: json!({
                "name": "Alice",
                "age": 30
            }),
            condition: None,
        }],
    };

    let input = json!({});
    let result = executor.execute(&manifest, input);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["populated"], "Hello, Alice! You are 30 years old.");
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

    let executor = ManifestExecutor::new(renderer, inference, mcp);

    let manifest = ProcessManifest {
        id: "test_inference".to_string(),
        steps: vec![ManifestStep {
            action: Action::Execute,
            template_id: "".to_string(),
            bindings: json!({}),
            condition: None,
        }],
    };

    let input = json!({"prompt": "What is the capital of France?"});
    let result = executor.execute(&manifest, input);

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

    let executor = ManifestExecutor::new(renderer, inference, mcp);

    let manifest = ProcessManifest {
        id: "test_mcp".to_string(),
        steps: vec![ManifestStep {
            action: Action::Execute,
            template_id: "calculator".to_string(),
            bindings: json!({}),
            condition: None,
        }],
    };

    let input = json!({"a": 10, "b": 20});
    let result = executor.execute(&manifest, input);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["result"], 30);
}

#[tokio::test]
async fn test_cascade_engine_stage_conditions() {
    let config = CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 5,
            energy_per_level: 100,
            timeout_ms: 5000,
        },
    };

    let renderer = MockRenderer::new();
    let inference = MockInference::new();
    let mcp = MockMcp::new();
    let manifest_executor = MockManifestExecutor::new();

    let engine = CascadeEngine::new(config, renderer, inference, mcp, manifest_executor);

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
    let config = CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 3,
            energy_per_level: 100,
            timeout_ms: 5000,
        },
    };

    let mut renderer = MockRenderer::new();
    renderer.add_template("step1", "Step 1: {{ input }}");
    renderer.add_template("step2", "Step 2: {{ input }}");

    let mut inference = MockInference::new();
    inference.add_response("analyze", json!({"analysis": "complete"}));

    let mut mcp = MockMcp::new();
    mcp.add_tool("save", |input| {
        json!({"saved": true, "data": input})
    });

    let mut manifest_executor = MockManifestExecutor::new();
    manifest_executor.add_manifest(
        "manifest1",
        ProcessManifest {
            id: "manifest1".to_string(),
            steps: vec![],
        },
    );

    let engine = CascadeEngine::new(config, renderer, inference, mcp, manifest_executor);

    // Create cascade with multiple stages
    let cascade_yaml = r#"
stages:
  - name: preprocessing
    templates:
      - step1
    condition: null
  - name: analysis
    templates:
      - step2
    condition: "status=ready"
  - name: postprocessing
    templates: []
    condition: "skip=true"
"#;

    let cascade: serde_yaml::Value = serde_yaml::from_str(cascade_yaml).unwrap();
    let input = json!({
        "input": "test data",
        "status": "ready"
    });

    let result = engine.execute(cascade, input).await;

    assert!(result.is_ok());
    let output = result.unwrap();

    // Verify stages executed
    assert!(output.get("preprocessing").is_some());
    assert!(output.get("analysis").is_some());
    // postprocessing should be skipped due to condition
    assert!(output.get("postprocessing").is_none());
}

#[tokio::test]
async fn test_cascade_energy_budget_enforcement() {
    let config = CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 5,
            energy_per_level: 50, // Low energy per level
            timeout_ms: 5000,
        },
    };

    let renderer = MockRenderer::new();
    let inference = MockInference::new();
    let mcp = MockMcp::new();
    let manifest_executor = MockManifestExecutor::new();

    let engine = CascadeEngine::new(config, renderer, inference, mcp, manifest_executor);

    // Create cascade with many stages (will exceed energy budget)
    let mut stages = Vec::new();
    for i in 0..10 {
        stages.push(serde_json::json!({
            "name": format!("stage_{}", i),
            "templates": [],
            "condition": null
        }));
    }

    let cascade = json!({
        "stages": stages
    });

    let input = json!({});
    let result = engine.execute(cascade, input).await;

    // Should fail due to energy exhaustion
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("energy"));
}

#[tokio::test]
async fn test_cascade_depth_limit_enforcement() {
    let config = CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 2, // Very shallow
            energy_per_level: 1000,
            timeout_ms: 5000,
        },
    };

    let renderer = MockRenderer::new();
    let inference = MockInference::new();
    let mcp = MockMcp::new();
    let manifest_executor = MockManifestExecutor::new();

    let engine = CascadeEngine::new(config, renderer, inference, mcp, manifest_executor);

    // Create cascade with many stages (will exceed depth limit)
    let mut stages = Vec::new();
    for i in 0..5 {
        stages.push(serde_json::json!({
            "name": format!("stage_{}", i),
            "templates": [],
            "condition": null
        }));
    }

    let cascade = json!({
        "stages": stages
    });

    let input = json!({});
    let result = engine.execute(cascade, input).await;

    // Should fail due to depth limit
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("depth"));
}

#[tokio::test]
async fn test_end_to_end_cascade_workflow() {
    // This test simulates a realistic workflow:
    // 1. Preprocessing: Clean and validate input
    // 2. Analysis: Run inference to analyze data
    // 3. Action: Execute MCP tool based on analysis
    // 4. Postprocessing: Format and save results

    let config = CascadeConfig {
        cascade_limits: CascadeLimits {
            max_depth: 5,
            energy_per_level: 100,
            timeout_ms: 5000,
        },
    };

    let mut renderer = MockRenderer::new();
    renderer.add_template("validate", "Validating: {{ data }}");
    renderer.add_template("format", "Formatted: {{ result }}");

    let mut inference = MockInference::new();
    inference.add_response(
        "analyze data",
        json!({
            "analysis": "data is valid",
            "action": "save",
            "confidence": 0.98
        }),
    );

    let mut mcp = MockMcp::new();
    mcp.add_tool("save", |input| {
        json!({
            "saved": true,
            "id": "result_123",
            "timestamp": "2026-05-24T09:41:44-07:00",
            "data": input
        })
    });

    let mut manifest_executor = MockManifestExecutor::new();
    manifest_executor.add_manifest(
        "validation_manifest",
        ProcessManifest {
            id: "validation_manifest".to_string(),
            steps: vec![],
        },
    );

    let engine = CascadeEngine::new(config, renderer, inference, mcp, manifest_executor);

    let cascade_yaml = r#"
stages:
  - name: preprocessing
    templates:
      - validate
    condition: null
  - name: analysis
    templates: []
    condition: null
  - name: action
    templates: []
    condition: "action=save"
  - name: postprocessing
    templates:
      - format
    condition: null
"#;

    let cascade: serde_yaml::Value = serde_yaml::from_str(cascade_yaml).unwrap();
    let input = json!({
        "data": "user input data",
        "action": "save"
    });

    let result = engine.execute(cascade, input).await;

    assert!(result.is_ok());
    let output = result.unwrap();

    // Verify all stages executed
    assert!(output.get("preprocessing").is_some());
    assert!(output.get("analysis").is_some());
    assert!(output.get("action").is_some());
    assert!(output.get("postprocessing").is_some());

    // Verify final result
    let final_result = output.get("result").unwrap();
    assert_eq!(final_result["saved"], true);
    assert_eq!(final_result["id"], "result_123");
}
