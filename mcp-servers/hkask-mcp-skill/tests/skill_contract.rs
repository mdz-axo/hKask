//! Contract tests for hkask-mcp-skill — skill loading and server construction.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: SkillServer construction, skill index loading (no inference dependency).

use hkask_mcp::server::CapabilityTier;
use hkask_mcp_skill::SkillServer;
use hkask_types::WebID;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A no-op InferencePort for testing SkillServer construction.
struct NoopInferencePort;

impl hkask_ports::InferencePort for NoopInferencePort {
    fn generate(
        &self,
        _prompt: &str,
        _parameters: &hkask_types::template::LLMParameters,
        _tools: Option<&[hkask_ports::ChatToolDefinition]>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<hkask_ports::InferenceResult, hkask_ports::InferenceError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async {
            Ok(hkask_ports::InferenceResult {
                text: String::new(),
                model: "noop".into(),
                usage: hkask_ports::InferenceUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                finish_reason: "stop".into(),
                token_probabilities: None,
                tool_calls: vec![],
            })
        })
    }
}

// ── Server construction tests ──────────────────────────────────────────────

#[test]
fn skill_server_constructs_with_noop_port() {
    let server = SkillServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        Arc::new(NoopInferencePort),
        HashMap::new(),
        CapabilityTier::detect(&HashMap::new()),
    );
    assert_eq!(server.replicant, "test-replicant");
    assert!(server.skills.is_empty(), "new server should have no skills");
}

#[test]
fn skill_server_loads_skills_from_registry() {
    let mut server = SkillServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        Arc::new(NoopInferencePort),
        HashMap::new(),
        CapabilityTier::detect(&HashMap::new()),
    );
    server.load_skills();
    // load_skills should succeed (even if empty) — no panic
}

// ── Skill index type tests ─────────────────────────────────────────────────────────

#[test]
fn skill_server_stores_registry_entries_directly() {
    // After C1, the server stores RegistryEntry values (no SkillToolDef wrapper).
    let mut server = SkillServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        Arc::new(NoopInferencePort),
        HashMap::new(),
        CapabilityTier::detect(&HashMap::new()),
    );
    assert!(
        server.skills.is_empty(),
        "new server should have no entries"
    );
    // Insert a synthetic entry to confirm the field type is HashMap<String, RegistryEntry>.
    server.skills.insert(
        "test.step".to_string(),
        hkask_ports::RegistryEntry {
            id: "test/step".into(),
            template_type: hkask_types::template_type::TemplateType::WordAct,
            name: "step".into(),
            lexicon_terms: vec![],
            description: "a test template".into(),
            source_path: "registry/templates/test/step.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: 7,
        },
    );
    assert_eq!(server.skills.len(), 1);
    assert_eq!(server.skills["test.step"].description, "a test template");
}

// ── Schema generation test ─────────────────────────────────────────────────

#[test]
fn skill_execute_request_has_schema() {
    let schema = schemars::schema_for!(hkask_mcp_skill::SkillExecuteRequest);
    let schema_json = serde_json::to_value(&schema).expect("schema should serialize");
    assert!(schema_json.is_object());
}
