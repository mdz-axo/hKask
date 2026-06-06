//! Integration test for the ManifestExecutor — runs a real manifest through the executor
//! with mock ports, verifying end-to-end cascade behavior.

use hkask_templates::{ManifestExecutor, ports::McpPort};
use hkask_types::{
    BundleManifest, BundleManifestStep, DelegationToken, InferenceError, InferenceResult,
    LLMParameters, ports::InferencePort, ports::InferenceUsage, ports::ToolInfo,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// --- Mock inference port ---

/// Mock inference port that returns predefined responses for select steps.
struct MockInference {
    responses: Mutex<Vec<String>>,
}

impl MockInference {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }
}

impl InferencePort for MockInference {
    fn generate(
        &self,
        _prompt: &str,
        _params: &LLMParameters,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let response = self.responses.lock().unwrap().remove(0);
        let result = InferenceResult {
            text: response,
            model: "test-model".to_string(),
            usage: InferenceUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            finish_reason: "stop".to_string(),
            token_probabilities: None,
            tool_calls: vec![],
        };
        Box::pin(async move { Ok(result) })
    }
}

// --- Mock MCP port ---

/// Mock MCP port that returns a fixed JSON result for any tool invocation.
struct MockMcp {
    tool_response: Value,
}

impl MockMcp {
    fn new(tool_response: Value) -> Self {
        Self { tool_response }
    }
}

impl McpPort for MockMcp {
    fn discover_tools(&self) -> impl std::future::Future<Output = Vec<String>> + Send {
        std::future::ready(vec!["test-tool".to_string()])
    }

    fn invoke(
        &self,
        _tool_name: &str,
        _input: Value,
        _token: &DelegationToken,
    ) -> impl std::future::Future<Output = Result<Value, hkask_templates::ports::TemplateError>> + Send
    {
        let response = self.tool_response.clone();
        std::future::ready(Ok(response))
    }

    fn get_tool_info(
        &self,
        _tool_name: &str,
    ) -> impl std::future::Future<Output = Option<ToolInfo>> + Send {
        std::future::ready(None)
    }
}

// --- Helper: build a manifest with given steps ---

fn make_manifest(id: &str, steps: Vec<BundleManifestStep>) -> BundleManifest {
    BundleManifest {
        id: id.to_string(),
        name: format!("Test {}", id),
        description: "Integration test manifest".to_string(),
        version: "1.0.0".to_string(),
        editor: "test".to_string(),
        visibility: hkask_types::Visibility::Shared,
        skills: vec![],
        conflicts: vec![],
        complementarities: vec![],
        steps,
        convergence: hkask_types::bundle::ConvergenceConfig::default(),
        gas: hkask_types::bundle::GasConfig::default(),
        error_handling: hkask_types::bundle::ErrorHandlingConfig::default(),
        ocap: hkask_types::bundle::OcapConfig::default(),
        cns: hkask_types::bundle::CnsConfig::default(),
        audit: hkask_types::bundle::AuditConfig::default(),
        functional_role: None,
        inputs: None,
        principles: None,
    }
}

fn make_step(ordinal: u32, action: &str, description: &str) -> BundleManifestStep {
    BundleManifestStep {
        ordinal,
        action: action.to_string(),
        description: description.to_string(),
        renderer: None,
        template_ref: None,
        model_tier: None,
        mcp: None,
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: None,
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Core,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    }
}

fn test_acp_secret() -> Vec<u8> {
    b"test-acp-secret-for-integration-test".to_vec()
}

// --- Integration tests ---

#[tokio::test]
async fn manifest_executor_populate_then_execute() {
    // Build a manifest with two steps:
    // 1. Populate: render a template with user_input context
    // 2. Execute: invoke an MCP tool with the populated result
    let populate_step = BundleManifestStep {
        ordinal: 1,
        action: "populate".to_string(),
        description: "Render user context".to_string(),
        renderer: Some("Processing: {{user_input}}".to_string()),
        template_ref: None,
        model_tier: None,
        mcp: None,
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: None,
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Pre,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let execute_step = BundleManifestStep {
        ordinal: 2,
        action: "execute".to_string(),
        description: "Invoke test tool".to_string(),
        renderer: None,
        template_ref: None,
        model_tier: None,
        mcp: Some("test-server/test-tool".to_string()),
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: Some(json!({
            "query": "{{step_1_populated}}"
        })),
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Core,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let manifest = make_manifest("populate-execute", vec![populate_step, execute_step]);

    let inference = Arc::new(MockInference::new(vec![]));
    let mcp = Arc::new(MockMcp::new(json!({"result": "tool response data"})));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let mut initial_ctx = HashMap::new();
    initial_ctx.insert("user_input".to_string(), json!("hello world"));
    initial_ctx.insert("agent".to_string(), json!("TestAgent"));

    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(
        result.is_ok(),
        "Manifest execution should succeed: {:?}",
        result.err()
    );
    let ctx = result.unwrap();

    // Step 1 populate should have produced step_1_populated
    assert!(
        ctx.contains_key("step_1_populated"),
        "Should have step_1_populated key"
    );
    let populated = ctx.get("step_1_populated").unwrap();
    assert!(
        populated
            .as_str()
            .unwrap()
            .contains("Processing: hello world"),
        "Populated template should contain the user input, got: {:?}",
        populated
    );

    // Step 2 execute should have produced step_2_result
    assert!(
        ctx.contains_key("step_2_result"),
        "Should have step_2_result key"
    );
    let tool_result = ctx.get("step_2_result").unwrap();
    assert_eq!(tool_result["result"], "tool response data");
}

#[tokio::test]
async fn manifest_executor_select_step() {
    // Build a manifest with a select step that calls inference and parses JSON
    let select_step = BundleManifestStep {
        ordinal: 1,
        action: "select".to_string(),
        description: "Classify user intent".to_string(),
        renderer: Some("Classify the following: {{user_input}}".to_string()),
        template_ref: None,
        model_tier: Some("fast_local".to_string()),
        mcp: None,
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: None,
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Pre,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let manifest = make_manifest("select-only", vec![select_step]);

    let inference = Arc::new(MockInference::new(vec![
        r#"{"intent": "question", "confidence": 0.95}"#.to_string(),
    ]));
    let mcp = Arc::new(MockMcp::new(json!({})));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let mut initial_ctx = HashMap::new();
    initial_ctx.insert("user_input".to_string(), json!("What is hKask?"));

    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(
        result.is_ok(),
        "Manifest execution should succeed: {:?}",
        result.err()
    );
    let ctx = result.unwrap();

    // Step 1 select should have produced step_1_result
    assert!(
        ctx.contains_key("step_1_result"),
        "Should have step_1_result key"
    );
    let select_result = ctx.get("step_1_result").unwrap();
    assert_eq!(select_result["intent"], "question");
    assert_eq!(select_result["confidence"], 0.95);
}

#[tokio::test]
async fn manifest_executor_multi_step_cascade() {
    // Build a 3-step cascade: select → populate → execute
    let select_step = BundleManifestStep {
        ordinal: 1,
        action: "select".to_string(),
        description: "Select operation type".to_string(),
        renderer: Some("Analyze: {{user_input}}".to_string()),
        template_ref: None,
        model_tier: None,
        mcp: None,
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: None,
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Pre,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let populate_step = BundleManifestStep {
        ordinal: 2,
        action: "populate".to_string(),
        description: "Format selected result".to_string(),
        renderer: Some("Selected: {{step_1_result}}".to_string()),
        template_ref: None,
        model_tier: None,
        mcp: None,
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: None,
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Core,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let execute_step = BundleManifestStep {
        ordinal: 3,
        action: "execute".to_string(),
        description: "Execute tool with formatted data".to_string(),
        renderer: None,
        template_ref: None,
        model_tier: None,
        mcp: Some("test-server/tool".to_string()),
        gas_cap: 5000,
        timeout_seconds: 30,
        input_mapping: Some(json!({
            "formatted": "{{step_2_populated}}"
        })),
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Core,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let manifest = make_manifest(
        "full-cascade",
        vec![select_step, populate_step, execute_step],
    );

    let inference = Arc::new(MockInference::new(vec![
        r#"{"operation": "analyze", "priority": "high"}"#.to_string(),
    ]));
    let mcp = Arc::new(MockMcp::new(
        json!({"status": "completed", "data": [1, 2, 3]}),
    ));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let mut initial_ctx = HashMap::new();
    initial_ctx.insert("user_input".to_string(), json!("analyze this"));

    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(
        result.is_ok(),
        "Full cascade should succeed: {:?}",
        result.err()
    );
    let ctx = result.unwrap();

    // All three steps should have produced results
    assert!(
        ctx.contains_key("step_1_result"),
        "Step 1 select should produce result"
    );
    assert!(
        ctx.contains_key("step_2_populated"),
        "Step 2 populate should produce result"
    );
    assert!(
        ctx.contains_key("step_3_result"),
        "Step 3 execute should produce result"
    );

    // Verify the select result
    let select_result = ctx.get("step_1_result").unwrap();
    assert_eq!(select_result["operation"], "analyze");

    // Verify the tool invocation produced a result
    let tool_result = ctx.get("step_3_result").unwrap();
    assert_eq!(tool_result["status"], "completed");
}

#[tokio::test]
async fn manifest_executor_empty_steps() {
    // A manifest with no steps should just return the initial context
    let manifest = make_manifest("empty", vec![]);

    let inference = Arc::new(MockInference::new(vec![]));
    let mcp = Arc::new(MockMcp::new(json!({})));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let mut initial_ctx = HashMap::new();
    initial_ctx.insert("user_input".to_string(), json!("test"));

    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(result.is_ok());
    let ctx = result.unwrap();
    assert_eq!(ctx.get("user_input").unwrap(), &json!("test"));
}

#[tokio::test]
async fn manifest_executor_unknown_action_fails() {
    // A step with an unknown action type should fail with a Manifest error
    let mut bad_step = make_step(1, "unknown_action", "Bad step");
    bad_step.renderer = Some("test".to_string());

    let manifest = make_manifest("bad-action", vec![bad_step]);

    let inference = Arc::new(MockInference::new(vec![]));
    let mcp = Arc::new(MockMcp::new(json!({})));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let initial_ctx = HashMap::new();
    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(result.is_err(), "Unknown action should fail");
    let err = result.unwrap_err();
    match err {
        hkask_templates::ports::TemplateError::Manifest(msg) => {
            assert!(
                msg.contains("Unknown manifest step action"),
                "Error should mention unknown action: {}",
                msg
            );
        }
        other => panic!("Expected Manifest error, got: {:?}", other),
    }
}

#[tokio::test]
async fn manifest_executor_feedback_action_invokes_tool() {
    // A feedback step should dispatch to the MCP tool (same as execute)
    let feedback_step = BundleManifestStep {
        ordinal: 1,
        action: "feedback".to_string(),
        description: "Emit CNS feedback".to_string(),
        renderer: None,
        template_ref: None,
        model_tier: None,
        mcp: Some("hkask-mcp-cns/emit_feedback".to_string()),
        gas_cap: 500,
        timeout_seconds: 5,
        input_mapping: Some(json!({
            "variety_delta": 10,
            "principle_compliance": { "think_before_coding": 0.9 }
        })),
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Post,
        feedback: Some(json!({
            "variety_delta": "integer",
            "principle_compliance": "object"
        })),
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let manifest = make_manifest("feedback-test", vec![feedback_step]);

    let inference = Arc::new(MockInference::new(vec![]));
    let mcp = Arc::new(MockMcp::new(
        json!({"status": "emitted", "cns_event_id": "evt-123"}),
    ));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let initial_ctx = HashMap::new();
    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(
        result.is_ok(),
        "Feedback action should succeed: {:?}",
        result.err()
    );
    let ctx = result.unwrap();
    assert!(
        ctx.contains_key("step_1_result"),
        "Feedback step should produce step_1_result"
    );
    assert_eq!(ctx.get("step_1_result").unwrap()["status"], "emitted");
}

#[tokio::test]
async fn manifest_executor_validate_action_invokes_tool() {
    // A validate step should dispatch to the MCP tool (same as execute)
    let validate_step = BundleManifestStep {
        ordinal: 1,
        action: "validate".to_string(),
        description: "Validate output quality".to_string(),
        renderer: None,
        template_ref: None,
        model_tier: None,
        mcp: Some("hkask-mcp-storage/validate".to_string()),
        gas_cap: 1000,
        timeout_seconds: 15,
        input_mapping: Some(json!({
            "rules": ["all_categories_tested"]
        })),
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Post,
        feedback: None,
        validation_rules: Some(json!(["all_categories_tested"])),
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let manifest = make_manifest("validate-test", vec![validate_step]);

    let inference = Arc::new(MockInference::new(vec![]));
    let mcp = Arc::new(MockMcp::new(json!({"valid": true, "score": 0.92})));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let initial_ctx = HashMap::new();
    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(
        result.is_ok(),
        "Validate action should succeed: {:?}",
        result.err()
    );
    let ctx = result.unwrap();
    assert!(
        ctx.contains_key("step_1_result"),
        "Validate step should produce step_1_result"
    );
    assert_eq!(ctx.get("step_1_result").unwrap()["valid"], true);
}

#[tokio::test]
async fn manifest_executor_retrieve_action_invokes_tool() {
    // A retrieve step should dispatch to the MCP tool (same as execute)
    let retrieve_step = BundleManifestStep {
        ordinal: 1,
        action: "retrieve".to_string(),
        description: "Retrieve semantic data".to_string(),
        renderer: None,
        template_ref: None,
        model_tier: None,
        mcp: Some("hkask-mcp-semantic/search".to_string()),
        gas_cap: 500,
        timeout_seconds: 10,
        input_mapping: Some(json!({
            "query": "hemingway style"
        })),
        output_schema: None,
        phase: hkask_types::bundle::CascadePhase::Pre,
        feedback: None,
        validation_rules: None,
        tool: None,
        target: None,
        arguments: None,
        bindings: None,
        loop_over: None,
        condition: None,
        token_cap: None,
        temperature: None,
        extra: serde_json::Map::new(),
    };

    let manifest = make_manifest("retrieve-test", vec![retrieve_step]);

    let inference = Arc::new(MockInference::new(vec![]));
    let mcp = Arc::new(MockMcp::new(json!({"results": ["passage1", "passage2"]})));

    let executor =
        ManifestExecutor::new(inference, mcp, LLMParameters::default(), test_acp_secret());

    let initial_ctx = HashMap::new();
    let result = executor.execute_manifest(&manifest, initial_ctx).await;

    assert!(
        result.is_ok(),
        "Retrieve action should succeed: {:?}",
        result.err()
    );
    let ctx = result.unwrap();
    assert!(
        ctx.contains_key("step_1_result"),
        "Retrieve step should produce step_1_result"
    );
}
