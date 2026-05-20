//! Unit tests for hkask-templates crate
//! Migrated from inline tests in production code (git commit f9ed608)

use hkask_templates::{
    audit::{AuditTrail, ExecutionAudit},
    cascade::{Cascade, CascadeBuilder, CascadeContext, CascadeExecutor, MAX_CASCADE_DEPTH},
    contracts::{parse_frontmatter, validate_lexicon_terms},
    dependency::{DependencyGraph, parse_dependencies},
    ports::{
        Action, CnsPort, CompositionTemplate, InferenceConfig, InferencePort, ManifestExecutorImpl,
        ManifestStep, McpPort, ProcessManifest, RegistryEntry, RegistryIndex, Result,
        SimpleExecutor, TemplateRenderer,
    },
};
use hkask_types::{TemplateType, WebID};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

mod audit_tests {
    use super::*;

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
        assert!(audit.error_message.is_none());
    }

    #[test]
    fn test_execution_audit_with_outcome_event() {
        let bot_id = WebID::new();
        let event_id = uuid::Uuid::new_v4();
        let audit = ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc123".to_string(),
            1,
        )
        .with_outcome_event(event_id);

        assert_eq!(audit.outcome_event_id, Some(event_id));
    }

    #[test]
    fn test_execution_audit_with_error() {
        let bot_id = WebID::new();
        let audit = ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc123".to_string(),
            1,
        )
        .with_error("Template not found".to_string());

        assert!(!audit.success);
        assert_eq!(audit.error_message, Some("Template not found".to_string()));
    }

    #[test]
    fn test_execution_audit_hash_input() {
        let input = "test input";
        let hash = ExecutionAudit::hash_input(input);

        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars

        // Same input produces same hash
        assert_eq!(hash, ExecutionAudit::hash_input(input));

        // Different input produces different hash
        assert_ne!(hash, ExecutionAudit::hash_input("different input"));
    }

    #[test]
    fn test_audit_trail_record() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        let audit = ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc123".to_string(),
            1,
        );

        trail.record(audit);

        assert_eq!(trail.count(), 1);
    }

    #[test]
    fn test_audit_trail_get_by_bot() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();
        let other_bot = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc".to_string(),
            1,
        ));
        trail.record(ExecutionAudit::new(
            other_bot,
            "prompt/selector".to_string(),
            "def".to_string(),
            1,
        ));

        let by_bot = trail.get_by_bot(&bot_id);
        assert_eq!(by_bot.len(), 1);
        assert_eq!(by_bot[0].bot_id, bot_id);
    }

    #[test]
    fn test_audit_trail_get_by_template() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/selector".to_string(),
            "abc".to_string(),
            1,
        ));
        trail.record(ExecutionAudit::new(
            bot_id,
            "process/dispatch".to_string(),
            "def".to_string(),
            1,
        ));

        let by_template = trail.get_by_template("prompt/selector");
        assert_eq!(by_template.len(), 1);
        assert_eq!(by_template[0].template_id, "prompt/selector");
    }

    #[test]
    fn test_audit_trail_trim_old_records() {
        let mut trail = AuditTrail::new(5);
        let bot_id = WebID::new();

        for i in 0..10 {
            trail.record(ExecutionAudit::new(
                bot_id,
                format!("template/{}", i),
                format!("hash{}", i),
                1,
            ));
        }

        assert_eq!(trail.count(), 5);
        // Oldest records should be trimmed
        assert!(trail.get_by_template("template/0").is_empty());
        assert!(!trail.get_by_template("template/9").is_empty());
    }

    #[test]
    fn test_audit_trail_get_failures() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(ExecutionAudit::new(
            bot_id,
            "prompt/success".to_string(),
            "abc".to_string(),
            1,
        ));
        trail.record(
            ExecutionAudit::new(bot_id, "prompt/fail".to_string(), "def".to_string(), 1)
                .with_error("Failed".to_string()),
        );

        let failures = trail.get_failures();
        assert_eq!(failures.len(), 1);
        assert!(!failures[0].success);
    }

    #[test]
    fn test_audit_trail_get_stats() {
        let mut trail = AuditTrail::new(100);
        let bot_id = WebID::new();

        trail.record(
            ExecutionAudit::new(bot_id, "prompt/success".to_string(), "abc".to_string(), 1)
                .with_duration_ms(100),
        );

        trail.record(
            ExecutionAudit::new(bot_id, "prompt/success2".to_string(), "def".to_string(), 1)
                .with_duration_ms(200),
        );

        trail.record(
            ExecutionAudit::new(bot_id, "prompt/fail".to_string(), "ghi".to_string(), 1)
                .with_error("Failed".to_string())
                .with_duration_ms(50),
        );

        let stats = trail.get_stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.avg_duration, 116); // (100 + 200 + 50) / 3 = 116
    }
}

mod cascade_tests {
    use super::*;

    struct MockRegistry {
        entries: HashMap<String, RegistryEntry>,
    }

    impl MockRegistry {
        fn new() -> Self {
            let mut entries = HashMap::new();
            entries.insert(
                "prompt/test".to_string(),
                RegistryEntry {
                    id: "prompt/test".to_string(),
                    template_type: TemplateType::Prompt,
                    lexicon_terms: vec!["test".to_string()],
                    description: "Test prompt".to_string(),
                    source_path: "test.j2".to_string(),
                },
            );
            entries.insert(
                "process/test".to_string(),
                RegistryEntry {
                    id: "process/test".to_string(),
                    template_type: TemplateType::Process,
                    lexicon_terms: vec!["test".to_string()],
                    description: "Test process".to_string(),
                    source_path: "test.yaml".to_string(),
                },
            );
            Self { entries }
        }
    }

    impl RegistryIndex for MockRegistry {
        fn list(&self, _domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
            self.entries.values().cloned().collect()
        }

        fn get(&self, id: &str) -> hkask_templates::Result<RegistryEntry> {
            self.entries.get(id).cloned().ok_or_else(|| {
                hkask_templates::TemplateError::NotFound(format!("Template '{}' not found", id))
            })
        }

        fn bootstrap_manifest(&self) -> Option<ProcessManifest> {
            Some(ProcessManifest {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test manifest".to_string(),
                steps: vec![ManifestStep {
                    ordinal: 1,
                    action: Action::Execute,
                    description: "Test step".to_string(),
                    template_ref: "test".to_string(),
                    model_tier: None,
                    mcp: None,
                    renderer: None,
                }],
            })
        }
    }

    #[test]
    fn test_cascade_new() {
        let cascade = Cascade::new("test");
        assert_eq!(cascade.id, "test");
        assert!(cascade.pre.is_empty());
        assert_eq!(cascade.max_depth, MAX_CASCADE_DEPTH);
    }

    #[test]
    fn test_cascade_builder() {
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["prompt/test"])
            .core("compose", vec!["process/test"])
            .post("format", vec!["prompt/test"])
            .max_depth(5)
            .build();

        assert_eq!(cascade.pre.len(), 1);
        assert_eq!(cascade.core.len(), 1);
        assert_eq!(cascade.post.len(), 1);
        assert_eq!(cascade.max_depth, 5);
    }

    #[test]
    fn test_cascade_context_depth_check() {
        let context = CascadeContext::new().with_depth(8);
        let result = context.check_depth(7);
        assert!(result.is_err());

        let context = CascadeContext::new().with_depth(6);
        let result = context.check_depth(7);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cascade_context_cycle_detection() {
        let mut context = CascadeContext::new();
        context.visit_template("template-1");

        let result = context.check_template_cycle("template-1");
        assert!(result.is_err());

        let result = context.check_template_cycle("template-2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cascade_context_energy() {
        let mut context = CascadeContext::new().with_energy(1000);
        assert!(context.check_energy(500).is_ok());
        assert!(context.check_energy(1500).is_err());

        context.consume_energy(500);
        assert_eq!(context.energy_remaining, 500);
    }

    #[test]
    fn test_cascade_context_child() {
        let context = CascadeContext::new().with_depth(3);
        let child = context.child_context();

        assert_eq!(child.current_depth, 4);
        assert_eq!(child.energy_remaining, context.energy_remaining);
    }

    #[test]
    fn test_cascade_executor_new() {
        let executor = CascadeExecutor::new();
        // Fields are private - just verify construction works
        drop(executor);
    }

    #[test]
    fn test_cascade_executor_execute() {
        let registry = MockRegistry::new();
        let executor = CascadeExecutor::new();
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["prompt/test"])
            .core("compose", vec!["process/test"])
            .build();

        let result = executor.execute(&cascade, Value::Null, &registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_cascade_depth_constant() {
        assert_eq!(MAX_CASCADE_DEPTH, 7);
    }

    #[test]
    fn test_cascade_max_depth_limit() {
        let cascade = Cascade::new("test").with_max_depth(10);
        assert_eq!(cascade.max_depth, 7); // Capped at MAX_CASCADE_DEPTH
    }
}

mod contracts_tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_valid() {
        let source = r#"
contract:
  input:
    raw_prompt: string
    context: object
  output:
    result: string
    confidence: float

inference:
  template_type: Prompt
  lexicon:
    - recognize
    - classify
  model_tier: fast_local
  timeout_ms: 5000

---
Template content here
"#;

        let frontmatter = parse_frontmatter(source).unwrap();

        assert!(frontmatter.contract.is_some());
        let contract = frontmatter.contract.unwrap();
        assert!(contract.input_fields.contains(&"raw_prompt".to_string()));
        assert!(contract.input_fields.contains(&"context".to_string()));
        assert!(contract.output_fields.contains(&"result".to_string()));
        assert!(contract.output_fields.contains(&"confidence".to_string()));

        assert!(frontmatter.inference.is_some());
        let inference = frontmatter.inference.unwrap();
        assert_eq!(inference.template_type, Some(TemplateType::Prompt));
        assert!(inference.lexicon_terms.contains(&"recognize".to_string()));
        assert_eq!(inference.model_tier, Some("fast_local".to_string()));
        assert_eq!(inference.timeout_ms, Some(5000));
    }

    #[test]
    fn test_parse_frontmatter_missing_delimiter() {
        let source = r#"
contract:
  input: {}
Template content without delimiter
"#;

        let result = parse_frontmatter(source);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("missing"));
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let source = r#"
contract:
  input: {invalid yaml
---
Content
"#;

        let result = parse_frontmatter(source);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("Invalid YAML"));
    }

    #[test]
    fn test_parse_frontmatter_minimal() {
        let source = r#"
inference:
  template_type: Process

---
Minimal template
"#;

        let frontmatter = parse_frontmatter(source).unwrap();

        assert!(frontmatter.contract.is_none());
        assert!(frontmatter.inference.is_some());
        assert_eq!(
            frontmatter.inference.unwrap().template_type,
            Some(TemplateType::Process)
        );
    }

    #[test]
    fn test_validate_lexicon_terms_valid() {
        let terms = vec!["recognize".to_string(), "classify".to_string()];
        let valid = ["recognize", "classify", "match"];

        let result = validate_lexicon_terms(&terms, &valid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_lexicon_terms_invalid() {
        let terms = vec!["invalid_term".to_string()];
        let valid = ["recognize", "classify", "match"];

        let result = validate_lexicon_terms(&terms, &valid);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("Unknown hLexicon term"));
    }

    #[test]
    fn test_validate_lexicon_terms_empty() {
        let terms: Vec<String> = vec![];
        let valid = ["recognize", "classify"];

        let result = validate_lexicon_terms(&terms, &valid);
        assert!(result.is_ok());
    }
}

mod dependency_tests {
    use super::*;

    #[test]
    fn test_dependency_graph_new() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_dependency_graph_add_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("caller".to_string(), "callee".to_string(), 1);

        assert_eq!(graph.edge_count(), 1);
        let deps = graph.get_dependencies("caller");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].callee, "callee");
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        graph.add_edge("b".to_string(), "c".to_string(), 1);

        // Adding d->a would NOT create a cycle (d is not in graph)
        assert!(!graph.would_create_cycle("d", "a"));

        // Adding c->d would NOT create a cycle (d is not reachable from a or b)
        assert!(!graph.would_create_cycle("c", "d"));
    }

    #[test]
    fn test_dependency_graph_detect_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        graph.add_edge("b".to_string(), "c".to_string(), 1);
        graph.add_edge("c".to_string(), "a".to_string(), 1);

        assert!(graph.would_create_cycle("c", "a"));

        let cycles = graph.find_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_dependency_graph_max_depth() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("root".to_string(), "level1".to_string(), 1);
        graph.add_edge("level1".to_string(), "level2".to_string(), 2);
        graph.add_edge("level2".to_string(), "level3".to_string(), 3);

        assert_eq!(graph.get_max_depth("root"), 3);
    }

    #[test]
    fn test_parse_dependencies_include() {
        let source = r#"
        Some text
        {% include "prompt/selector" %}
        More text
        {% include 'process/dispatch' %}
        "#;

        let deps = parse_dependencies("test", source);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"prompt/selector".to_string()));
        assert!(deps.contains(&"process/dispatch".to_string()));
    }

    #[test]
    fn test_parse_dependencies_call() {
        let source = r#"
        {% call "cognition/detect" %}
        "#;

        let deps = parse_dependencies("test", source);
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&"cognition/detect".to_string()));
    }

    #[test]
    fn test_parse_dependencies_empty() {
        let source = r#"
        No dependencies here
        Just regular content
        "#;

        let deps = parse_dependencies("test", source);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_dependency_graph_clear() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);

        graph.clear();

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_dependency_graph_dependents() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        graph.add_edge("c".to_string(), "b".to_string(), 1);

        let dependents = graph.get_dependents("b");
        assert_eq!(dependents.len(), 2);
    }
}

mod manifest_tests {
    use super::*;

    struct MockInference;
    impl InferencePort for MockInference {
        fn call(
            &self,
            _model_tier: &str,
            _prompt: &str,
            _config: &InferenceConfig,
        ) -> Result<Value> {
            Ok(Value::String("mock inference result".to_string()))
        }
    }

    struct MockRenderer;
    impl TemplateRenderer for MockRenderer {
        fn load(&self, _path: &Path) -> Result<CompositionTemplate> {
            Err(hkask_templates::TemplateError::NotFound("mock".to_string()))
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
            self.events
                .lock()
                .unwrap()
                .push((span.to_string(), outcome, confidence));
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
}
