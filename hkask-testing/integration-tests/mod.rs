//! Integration tests for hKask template loading and rendering

use hkask_templates::ports::{
    Action, CnsPort, InferenceConfig, InferencePort, ManifestExecutor, ManifestStep, McpPort,
    ProcessManifest, RegistryIndex, TemplateRenderer,
};
use hkask_templates::Registry;
use hkask_templates::manifest::ManifestExecutorImpl;
use serde_json::Value;
use std::path::Path;

/// Mock inference port for testing
struct MockInference;
impl InferencePort for MockInference {
    fn call(
        &self,
        _model_tier: &str,
        _prompt: &str,
        _config: &InferenceConfig,
    ) -> hkask_templates::ports::Result<Value> {
        Ok(serde_json::json!({
            "selected_template_id": "prompt/execute",
            "confidence": 0.85,
            "rationale": "Default template for testing"
        }))
    }
}

/// Mock renderer for testing
struct MockRenderer;
impl TemplateRenderer for MockRenderer {
    fn load(&self, _path: &Path) -> hkask_templates::ports::Result<hkask_templates::ports::CompositionTemplate> {
        Ok(hkask_templates::ports::CompositionTemplate {
            id: "test".to_string(),
            template_type: hkask_types::TemplateType::Prompt,
            lexicon_terms: vec![],
            source: "test".to_string(),
            contract: hkask_templates::ports::TemplateContract {
                input_fields: vec!["input".to_string()],
                output_fields: vec!["output".to_string()],
            },
        })
    }

    fn render(
        &self,
        _template: &hkask_templates::ports::CompositionTemplate,
        _bindings: Value,
    ) -> hkask_templates::ports::Result<String> {
        Ok("rendered output".to_string())
    }
}

/// Mock MCP port for testing
struct MockMcp;
impl McpPort for MockMcp {
    fn discover_tools(&self) -> Vec<String> {
        vec!["test_tool".to_string()]
    }

    fn invoke(&self, _tool_name: &str, input: Value) -> hkask_templates::ports::Result<Value> {
        Ok(input)
    }
}

/// Mock CNS port for testing
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

#[test]
fn test_load_all_bootstrap_templates_from_filesystem() {
    let registry = Registry::bootstrap();
    
    // Verify all 7 core templates are registered
    assert!(registry.exists("prompt/selector"));
    assert!(registry.exists("prompt/render"));
    assert!(registry.exists("prompt/execute"));
    assert!(registry.exists("cognition/detect"));
    assert!(registry.exists("cognition/calibrate"));
    assert!(registry.exists("process/memory/recall"));
    assert!(registry.exists("process/dispatch"));
}

#[test]
fn test_bootstrap_manifest_from_yaml() {
    let registry = Registry::bootstrap();
    let manifest = registry.bootstrap_manifest();
    
    assert!(manifest.is_some(), "Bootstrap manifest should load from YAML");
    
    let manifest = manifest.unwrap();
    assert_eq!(manifest.id, "registry/dispatch");
    assert_eq!(manifest.name, "Registry Dispatch");
    assert_eq!(manifest.steps.len(), 3);
    
    // Verify step structure
    assert_eq!(manifest.steps[0].action, Action::Select);
    assert_eq!(manifest.steps[0].template_ref, "prompt/selector");
    assert_eq!(manifest.steps[0].model_tier, Some("fast_local".to_string()));
    
    assert_eq!(manifest.steps[1].action, Action::Populate);
    
    assert_eq!(manifest.steps[2].action, Action::Execute);
    assert_eq!(manifest.steps[2].mcp, Some("from_template_contract".to_string()));
}

#[test]
fn test_manifest_executor_emits_cns_events() {
    let cns = MockCns::new();
    let executor = ManifestExecutorImpl::new(
        MockRenderer,
        MockInference,
        MockMcp,
        cns.clone(),
    );

    let manifest = ProcessManifest {
        id: "test".to_string(),
        name: "Test".to_string(),
        description: "Test manifest".to_string(),
        steps: vec![
            ManifestStep {
                ordinal: 1,
                action: Action::Select,
                description: "Select".to_string(),
                template_ref: "prompt/selector".to_string(),
                model_tier: Some("fast_local".to_string()),
                mcp: Some("hkask-mcp-inference".to_string()),
                renderer: Some("minijinja".to_string()),
            },
            ManifestStep {
                ordinal: 2,
                action: Action::Populate,
                description: "Populate".to_string(),
                template_ref: "prompt/execute".to_string(),
                model_tier: None,
                mcp: None,
                renderer: Some("minijinja".to_string()),
            },
            ManifestStep {
                ordinal: 3,
                action: Action::Execute,
                description: "Execute".to_string(),
                template_ref: "".to_string(),
                model_tier: None,
                mcp: Some("test_tool".to_string()),
                renderer: None,
            },
        ],
    };

    let result = executor.execute(&manifest, Value::String("test input".to_string()));
    assert!(result.is_ok());

    // Verify CNS events were emitted
    let events = cns.get_events();
    assert!(!events.is_empty(), "CNS events should be emitted");
    
    // Check for expected span types
    let has_select = events.iter().any(|(span, _, _)| span.contains("select"));
    let has_populate = events.iter().any(|(span, _, _)| span.contains("populate"));
    let has_execute = events.iter().any(|(span, _, _)| span.contains("execute"));
    let has_outcome = events.iter().any(|(span, _, _)| span.contains("outcome"));
    
    assert!(has_select, "Should emit cns.prompt.select event");
    assert!(has_populate, "Should emit cns.prompt.populate event");
    assert!(has_execute, "Should emit cns.prompt.execute event");
    assert!(has_outcome, "Should emit cns.prompt.outcome event");
}

#[test]
fn test_yaml_manifest_files_exist() {
    // Verify YAML manifest files exist (use workspace root)
    let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|p| std::path::PathBuf::from(p).parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    
    let dispatch_path = workspace_root.join("registry/manifests/dispatch.yaml");
    let recall_path = workspace_root.join("registry/manifests/memory_recall.yaml");
    let tool_path = workspace_root.join("registry/manifests/tool_dispatch.yaml");
    
    assert!(
        dispatch_path.exists(),
        "dispatch.yaml should exist at {:?}",
        dispatch_path
    );
    assert!(
        recall_path.exists(),
        "memory_recall.yaml should exist at {:?}",
        recall_path
    );
    assert!(
        tool_path.exists(),
        "tool_dispatch.yaml should exist at {:?}",
        tool_path
    );
}

#[test]
fn test_load_yaml_manifests() {
    let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|p| std::path::PathBuf::from(p).parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    
    let dispatch = ProcessManifest::load_from_yaml(&workspace_root.join("registry/manifests/dispatch.yaml"));
    assert!(dispatch.is_ok(), "Should load dispatch.yaml: {:?}", dispatch.err());
    
    let recall = ProcessManifest::load_from_yaml(&workspace_root.join("registry/manifests/memory_recall.yaml"));
    assert!(recall.is_ok(), "Should load memory_recall.yaml: {:?}", recall.err());
    
    let tool_dispatch = ProcessManifest::load_from_yaml(&workspace_root.join("registry/manifests/tool_dispatch.yaml"));
    assert!(tool_dispatch.is_ok(), "Should load tool_dispatch.yaml: {:?}", tool_dispatch.err());
}
