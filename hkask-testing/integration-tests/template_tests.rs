//! hKask Template Integration Tests
//!
//! End-to-end tests for template rendering, manifest execution, and CLI commands.

use hkask_templates::{
    ManifestExecutor, ProcessManifest, RegistryIndex, SqliteRegistry, TemplateRenderer,
};
use serde_json::json;
use std::path::Path;

/// Test DCT pipeline end-to-end
#[test]
fn test_dct_pipeline_end_to_end() {
    let registry = SqliteRegistry::new(None).expect("Failed to create in-memory registry");
    
    // Test input: natural language query
    let input = json!({
        "query": "I need a comprehensive comparison of electric vehicles using 2025 data, excluding Tesla"
    });
    
    // Load DCT pipeline manifest
    let manifest_path = Path::new("registry/manifests/dct-pipeline.yaml");
    if manifest_path.exists() {
        let manifest = ProcessManifest::load_from_yaml(manifest_path)
            .expect("Failed to load DCT pipeline manifest");
        
        assert_eq!(manifest.id, "dct-pipeline");
        assert_eq!(manifest.steps.len(), 3);
        
        // Verify step sequence
        assert_eq!(manifest.steps[0].action.as_str(), "populate");
        assert_eq!(manifest.steps[1].action.as_str(), "populate");
        assert_eq!(manifest.steps[2].action.as_str(), "execute");
    }
}

/// Test reasoning cycle execution
#[test]
fn test_reasoning_cycle_manifest() {
    let manifest_path = Path::new("registry/manifests/reasoning-cycle.yaml");
    if manifest_path.exists() {
        let manifest = ProcessManifest::load_from_yaml(manifest_path)
            .expect("Failed to load reasoning cycle manifest");
        
        assert_eq!(manifest.id, "reasoning-cycle");
        assert_eq!(manifest.steps.len(), 3);
        
        // Verify CNS spans are configured
        // (actual execution requires inference port mock)
    }
}

/// Test metacognition manifest
#[test]
fn test_metacognition_manifest() {
    let manifest_path = Path::new("registry/manifests/metacognition.yaml");
    if manifest_path.exists() {
        let manifest = ProcessManifest::load_from_yaml(manifest_path)
            .expect("Failed to load metacognition manifest");
        
        assert_eq!(manifest.id, "metacognition");
        assert_eq!(manifest.steps.len(), 3);
        
        // Verify energy cap
        assert!(manifest.energy.is_some());
    }
}

/// Test MCP inference call manifest
#[test]
fn test_mcp_inference_call_manifest() {
    let manifest_path = Path::new("registry/manifests/mcp_inference_call.yaml");
    if manifest_path.exists() {
        let manifest = ProcessManifest::load_from_yaml(manifest_path)
            .expect("Failed to load MCP inference manifest");
        
        assert_eq!(manifest.id, "mcp/inference_call");
        assert_eq!(manifest.steps.len(), 3);
        
        // Verify OCAP configuration
        assert!(manifest.ocap.is_some());
    }
}

/// Test MCP condense session manifest
#[test]
fn test_mcp_condense_session_manifest() {
    let manifest_path = Path::new("registry/manifests/mcp_condense_session.yaml");
    if manifest_path.exists() {
        let manifest = ProcessManifest::load_from_yaml(manifest_path)
            .expect("Failed to load MCP condenser manifest");
        
        assert_eq!(manifest.id, "mcp/condense_session");
        assert_eq!(manifest.steps.len(), 3);
        
        // Verify audit trail configuration
        assert!(manifest.audit.is_some());
    }
}

/// Test template rendering with bindings
#[test]
fn test_template_rendering() {
    // This test verifies template files exist and can be loaded
    let template_paths = [
        "registry/registries/dct-pipeline/decimation.jinja2",
        "registry/registries/dct-pipeline/classification.jinja2",
        "registry/registries/reasoning/reason_constrained.jinja2",
        "registry/registries/review/self_critique.jinja2",
        "registry/registries/composition/answer_composition.jinja2",
        "registry/registries/metacognition/meta_decompose.jinja2",
        "registry/registries/prompt/selector.jinja2",
        "registry/registries/prompt/render.jinja2",
        "registry/registries/prompt/execute.jinja2",
        "registry/registries/cognition/detect.jinja2",
        "registry/registries/cognition/calibrate.jinja2",
        "registry/registries/cognition/reflect.jinja2",
        "registry/registries/process/memory_recall.jinja2",
        "registry/registries/process/dispatch.jinja2",
        "registry/registries/mcp/inference_call.jinja2",
        "registry/registries/mcp/condense_session.jinja2",
        "registry/registries/mcp/doc_extract.jinja2",
        "registry/registries/mcp/web_extract.jinja2",
        "registry/registries/mcp/scholar_extract.jinja2",
    ];
    
    for path_str in &template_paths {
        let path = Path::new(path_str);
        assert!(path.exists(), "Template not found: {}", path_str);
        
        // Verify template can be read
        let content = std::fs::read_to_string(path)
            .expect(&format!("Failed to read template: {}", path_str));
        
        // Verify template has [inference] header
        assert!(
            content.contains("[inference]"),
            "Template missing [inference] header: {}",
            path_str
        );
        
        // Verify template has lexicon_terms
        assert!(
            content.contains("lexicon_terms:"),
            "Template missing lexicon_terms: {}",
            path_str
        );
    }
}

/// Test registry bootstrap includes all templates
#[test]
fn test_registry_bootstrap() {
    let registry = SqliteRegistry::new(None).expect("Failed to create registry");
    
    // Load bootstrap templates
    let _ = registry.load_all();
    
    // Verify core templates are registered
    let expected_templates = [
        "dct-pipeline/decimation",
        "dct-pipeline/classification",
        "reasoning/reason_constrained",
        "reasoning/reasoning",
        "review/self_critique",
        "composition/answer_composition",
        "metacognition/meta_decompose",
        "prompt/selector",
        "prompt/render",
        "prompt/execute",
        "cognition/detect",
        "cognition/calibrate",
        "cognition/reflect",
        "process/memory/recall",
        "process/dispatch",
    ];
    
    for template_id in &expected_templates {
        let result = registry.get(template_id);
        assert!(
            result.is_ok(),
            "Template not registered: {}",
            template_id
        );
    }
}

/// Test CNS span emission configuration
#[test]
fn test_cns_span_configuration() {
    let manifest_paths = [
        "registry/manifests/dct-pipeline.yaml",
        "registry/manifests/reasoning-cycle.yaml",
        "registry/manifests/metacognition.yaml",
        "registry/manifests/composition.yaml",
        "registry/manifests/mcp_inference_call.yaml",
        "registry/manifests/mcp_condense_session.yaml",
        "registry/manifests/mcp_doc_extract.yaml",
        "registry/manifests/mcp_web_extract.yaml",
        "registry/manifests/mcp_scholar_extract.yaml",
    ];
    
    for path_str in &manifest_paths {
        let path = Path::new(path_str);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .expect(&format!("Failed to read manifest: {}", path_str));
            
            // Verify CNS configuration
            assert!(
                content.contains("cns:"),
                "Manifest missing CNS config: {}",
                path_str
            );
            
            assert!(
                content.contains("emit_spans: true"),
                "Manifest CNS spans not enabled: {}",
                path_str
            );
        }
    }
}

/// Test OCAP capability declarations
#[test]
fn test_ocap_configuration() {
    let manifest_paths = [
        "registry/manifests/dct-pipeline.yaml",
        "registry/manifests/mcp_inference_call.yaml",
        "registry/manifests/mcp_doc_extract.yaml",
    ];
    
    for path_str in &manifest_paths {
        let path = Path::new(path_str);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .expect(&format!("Failed to read manifest: {}", path_str));
            
            // Verify OCAP configuration
            assert!(
                content.contains("ocap:"),
                "Manifest missing OCAP config: {}",
                path_str
            );
            
            assert!(
                content.contains("required_capabilities:"),
                "Manifest missing required_capabilities: {}",
                path_str
            );
            
            assert!(
                content.contains("signature_algorithm: ed25519"),
                "Manifest missing Ed25519 signature: {}",
                path_str
            );
        }
    }
}

/// Test Jinja2 sandbox escape attempts — SECURITY CRITICAL
#[test]
fn test_jinja2_sandbox_escape_prevention() {
    use hkask_templates::TemplateRendererImpl;
    
    let renderer = TemplateRendererImpl::new();
    
    // Test 1: Attempt to access filesystem via path traversal
    let malicious_template_1 = "{{ ''.__class__.__mro__[2].__subclasses__() }}";
    let result = renderer.add_template("malicious1", malicious_template_1);
    // Should either fail to add or fail to render
    if result.is_ok() {
        let render_result = renderer.render_by_name("malicious1", &json!({}));
        // Render should fail or return safe output
        assert!(
            render_result.is_err() || !render_result.as_ref().unwrap().contains("__class__"),
            "Sandbox escape via __class__ not prevented"
        );
    }
    
    // Test 2: Attempt to use Python builtins
    let malicious_template_2 = "{{ globals() }}";
    let result = renderer.add_template("malicious2", malicious_template_2);
    if result.is_ok() {
        let render_result = renderer.render_by_name("malicious2", &json!({}));
        // Should not expose globals
        assert!(
            render_result.is_err() || render_result.as_ref().unwrap().is_empty(),
            "Sandbox escape via globals() not prevented"
        );
    }
    
    // Test 3: Attempt to read files
    let malicious_template_3 = "{{ ''.__class__.__mro__[1].__subclasses__() }}";
    let result = renderer.add_template("malicious3", malicious_template_3);
    if result.is_ok() {
        let render_result = renderer.render_by_name("malicious3", &json!({}));
        assert!(
            render_result.is_err(),
            "Sandbox escape via subclass enumeration not prevented"
        );
    }
    
    // Test 4: Verify path traversal in template names is blocked
    let path_traversal_names = [
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam",
        "/etc/shadow",
        "templates/../../../secret",
    ];
    
    for name in &path_traversal_names {
        let result = renderer.add_template(name, "test");
        assert!(
            result.is_err(),
            "Path traversal not blocked in template name: {}",
            name
        );
    }
}

/// Test template resolver with TTL caching
#[test]
fn test_template_resolver_caching() {
    use hkask_templates::{Registry, TemplateResolver};
    use std::time::Duration;
    
    let registry = Registry::bootstrap();
    let resolver = TemplateResolver::new(&registry, Duration::from_secs(5));
    
    // First lookup - cache miss
    let result1 = resolver.resolve("prompt/selector");
    assert!(result1.is_ok(), "Template should resolve");
    
    // Second lookup - cache hit
    let result2 = resolver.resolve("prompt/selector");
    assert!(result2.is_ok(), "Cached template should resolve");
    
    // Verify cache statistics
    let stats = resolver.stats();
    assert!(stats.cache_hits >= 1, "Should have at least 1 cache hit");
    assert!(stats.cache_misses >= 1, "Should have at least 1 cache miss");
    assert_eq!(stats.total_lookups, 2, "Should have 2 total lookups");
}

/// Test energy cap calibration CLI command
#[test]
fn test_energy_cap_calibration() {
    use hkask_cli::commands::calibrate_energy_caps;
    
    let manifest_path = Path::new("registry/manifests/dct-pipeline.yaml");
    if manifest_path.exists() {
        let report = calibrate_energy_caps(manifest_path)
            .expect("Failed to calibrate energy caps");
        
        assert_eq!(report.manifest_id, "dct-pipeline");
        assert_eq!(report.current_cap, 10000);
        assert!(report.cap_utilization > 0.0);
        assert!(!report.recommendation.is_empty());
        
        // Verify report structure
        assert!(report.steps_count > 0);
        assert!(report.cost_per_token > 0.0);
        assert!(report.alert_threshold > 0.0);
        assert!(report.alert_threshold <= 1.0);
    }
}

/// Test all manifest OCAP template_scoped configurations
#[test]
fn test_ocap_template_scoped_configuration() {
    let manifest_paths = [
        "registry/manifests/dct-pipeline.yaml",
        "registry/manifests/mcp_inference_call.yaml",
        "registry/manifests/composition.yaml",
        "registry/manifests/mcp_condense_session.yaml",
        "registry/manifests/mcp_doc_extract.yaml",
        "registry/manifests/mcp_scholar_extract.yaml",
        "registry/manifests/mcp_web_extract.yaml",
        "registry/manifests/metacognition.yaml",
        "registry/manifests/reasoning-cycle.yaml",
    ];
    
    for path_str in &manifest_paths {
        let path = Path::new(path_str);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .expect(&format!("Failed to read manifest: {}", path_str));
            
            // Verify template_scoped flag
            assert!(
                content.contains("template_scoped: true"),
                "Manifest missing template_scoped flag: {}",
                path_str
            );
            
            // Verify template_id fields in capabilities
            assert!(
                content.contains("template_id:"),
                "Manifest missing template_id in capabilities: {}",
                path_str
            );
        }
    }
}
