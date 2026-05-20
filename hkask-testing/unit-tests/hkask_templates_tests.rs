use hkask_templates::{
    parse_frontmatter, TemplateFrontmatter, TemplateType, ExecutionAudit, WebID,
    MockRegistryAdapter, CapabilityAwareValidator, OkapiCapabilities,
};
use hkask_templates::contract_validator::OkapiRequirements;
use hkask_templates::ports::{Action, ManifestStep};
use hkask_templates::manifest::ProcessManifest;
use hkask_templates::manifest_repository::FileSystemManifestRepository;
use hkask_templates::dependency::DependencyGraph;
use hkask_templates::error::CompositionError;
use tempfile::TempDir;

#[test]
fn test_parse_frontmatter_valid() {
    let source = r#"
contract:
  input:
    raw_prompt: string
  output:
    result: string
---
Template content
"#;

    let frontmatter = parse_frontmatter(source).unwrap();
    assert!(frontmatter.contract.is_some());
}

#[test]
fn test_execution_audit_new() {
    let bot_id = WebID::new();
    let audit = ExecutionAudit::new(
        bot_id,
        "prompt/selector".to_string(),
        "abc123".to_string(),
        1,
    );

    assert_eq!(audit.bot_id, bot_id);
    assert_eq!(audit.template_id, "prompt/selector");
    assert_eq!(audit.input_hash, "abc123");
    assert_eq!(audit.matroshka_depth, 1);
    assert!(audit.success);
}

#[test]
fn test_mock_registry_adapter_new() {
    let adapter = MockRegistryAdapter::new();
    let templates = adapter.templates.read().unwrap();
    assert!(templates.is_empty());
}

#[test]
fn test_capability_aware_validator_valid() {
    let capabilities = OkapiCapabilities {
        runner_type: "ollamarunner".to_string(),
        lora_hot_swap: true,
        token_probs: true,
        grammar_native: true,
        advanced_sampling: true,
    };
    let validator = CapabilityAwareValidator::new(
        capabilities,
        vec!["classify".to_string(), "recognize".to_string()],
    );

    let frontmatter = hkask_templates::RegistrationFrontmatter {
        template_type: TemplateType::Prompt,
        domain: "WordAct".to_string(),
        requires_okapi: Some(OkapiRequirements {
            n_probs: Some(5),
            grammar: None,
            adapter: None,
        }),
        confidence: None,
        lexicon_terms: vec!["classify".to_string()],
        contract: None,
    };

    let result = validator.validate(&frontmatter);
    assert!(result.is_ok());
}

#[test]
fn test_dependency_graph_new() {
    let graph = DependencyGraph::new();
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn test_transient_error() {
    let error = CompositionError::transient("network timeout");
    assert!(error.is_retryable());
    assert_eq!(error.retry_count(), Some(0));
    assert_eq!(error.category(), "transient");
}

#[test]
fn test_file_system_repository_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let repo = FileSystemManifestRepository::new(temp_dir.path().to_path_buf());

    let manifest = ProcessManifest {
        id: "test-manifest".to_string(),
        name: "Test Manifest".to_string(),
        description: "A test manifest".to_string(),
        steps: vec![ManifestStep {
            ordinal: 1,
            action: Action::Select,
            description: "Select template".to_string(),
            template_ref: "prompt/selector".to_string(),
            model_tier: Some("fast_local".to_string()),
            mcp: Some("hkask-mcp-inference".to_string()),
            renderer: Some("minijinja".to_string()),
        }],
    };

    repo.save(&manifest).unwrap();
    let loaded = repo.load("test-manifest").unwrap();
    assert_eq!(loaded.name, "Test Manifest");
}

#[test]
fn test_template_type_as_str() {
    assert_eq!(TemplateType::Prompt.as_str(), "Prompt");
    assert_eq!(TemplateType::Process.as_str(), "Process");
    assert_eq!(TemplateType::Cognition.as_str(), "Cognition");
}
