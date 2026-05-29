//! Interface Equivalence Integration Tests
//!
//! Verifies the MCP ≡ CLI ≡ API equivalence claim: all three surfaces produce
//! identical results through shared storage and domain logic.
//!
//! These tests validate the functional core (hkask-types, hkask-storage,
//! hkask-templates, hkask-cns) — not the surfaces themselves (CLI/API
//! integration tests are separate).
//!
//! Per architecture v0.21.0 §1: "Three surfaces, one functional core."

use hkask_cns::CnsRuntime;
use hkask_cns::spans::SpanEmitter;
use hkask_storage::{Database, NuEventStore, SqliteSpecStore};
use hkask_templates::{RegistryEntry, RegistryIndex, SqliteRegistry};
use hkask_types::{
    CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityTokenBuilder, DomainAnchor,
    GoalSpec, Phase, SYSTEM_MAX_ATTENUATION, Span, SpecCategory, SpecStore, TemplateType, WebID,
};
use rusqlite::Connection;
use serde_json::json;
use std::sync::{Arc, Mutex};

// ─── Test Fixture ───────────────────────────────────────────────────────────

/// Shared test fixture with in-memory services.
/// Represents the "functional core" that all three surfaces (MCP, CLI, API)
/// converge on.
struct EquivalenceFixture {
    /// Shared database connection (all stores use the same conn)
    conn: Arc<Mutex<Connection>>,
    /// Spec store (shared: CLI via kask spec, API via POST /specs)
    spec_store: Arc<SqliteSpecStore>,
    /// CNS runtime (shared: CLI via kask cns, API via GET /cns/health)
    cns_runtime: Arc<CnsRuntime>,
    /// Span emitter with persisted event sink
    span_emitter: SpanEmitter,
}

impl EquivalenceFixture {
    fn new() -> Self {
        let db = Database::in_memory().expect("Failed to create in-memory database");
        let conn = db.conn_arc();

        // Initialize spec schema
        let spec_store = Arc::new(SqliteSpecStore::new(Arc::clone(&conn)));
        spec_store
            .init_schema()
            .expect("Failed to init spec schema");

        let cns_runtime = Arc::new(CnsRuntime::new());

        // Wire span emitter to persistent event store for verification
        let event_store = Box::new(NuEventStore::new(Arc::clone(&conn)));
        let observer_webid = WebID::new();
        let span_emitter = SpanEmitter::new(observer_webid).with_sink(event_store);

        Self {
            conn,
            spec_store,
            cns_runtime,
            span_emitter,
        }
    }

    /// Create a fresh SqliteRegistry (in-memory, independent connection)
    fn create_registry() -> SqliteRegistry {
        SqliteRegistry::new(None).expect("Failed to create in-memory SQLite registry")
    }
}

// ─── Test 1: Spec Capture Equivalence ───────────────────────────────────────

/// Verifies that spec capture produces identical results regardless of which
/// surface (MCP, CLI, API) triggers it — the shared `SqliteSpecStore` and
/// `GoalSpec` domain logic are the common core.
#[tokio::test]
async fn spec_capture_equivalence() {
    let fx = EquivalenceFixture::new();

    // — API path: Create GoalSpec with text "System CAN invoke_tool on McpServer"
    let goal_text = "System CAN invoke_tool on McpServer";
    let mut goal = GoalSpec::new(goal_text);
    goal.criteria
        .push(hkask_types::Criterion::new("Tool invocation succeeds"));
    goal.criteria
        .push(hkask_types::Criterion::new("Response is valid JSON"));

    // Create Spec wrapping the goal
    let spec = hkask_types::Spec::new(
        "tool-invoke-spec",
        SpecCategory::Capability,
        DomainAnchor::Hkask,
    )
    .with_goal(goal);

    // Save via SqliteSpecStore::save() — the "API" path
    fx.spec_store.save(&spec).expect("Failed to save spec");

    // — Load it (simulates CLI/MCP reading the same data)
    let loaded = fx.spec_store.load(spec.id).expect("Failed to load spec");
    assert_eq!(loaded.name, "tool-invoke-spec");
    assert_eq!(loaded.category, SpecCategory::Capability);
    assert_eq!(loaded.goals.len(), 1);
    assert_eq!(loaded.goals[0].text, goal_text);
    assert_eq!(loaded.goals[0].criteria.len(), 2);

    // — Verify completeness predicate works on the loaded spec
    // Goals have criteria but none are satisfied → not complete
    assert!(
        !loaded.is_complete(),
        "Spec with unsatisfied criteria must not be complete"
    );

    // Mark criteria satisfied → becomes complete
    let mut complete_spec = loaded.clone();
    for criterion in &mut complete_spec.goals[0].criteria {
        criterion.satisfied = true;
    }
    assert!(
        complete_spec.is_complete(),
        "Spec with all criteria satisfied must be complete"
    );

    // — List by category (same store, same data)
    let capability_specs = fx
        .spec_store
        .list_by_category(SpecCategory::Capability)
        .expect("Failed to list by category");
    assert_eq!(capability_specs.len(), 1);
    assert_eq!(capability_specs[0].id, spec.id);

    // — List all
    let all_specs = fx.spec_store.list_all().expect("Failed to list all");
    assert_eq!(all_specs.len(), 1);
}

// ─── Test 2: Template Registry Equivalence ──────────────────────────────────

/// Verifies that template registration, listing, and lookup work identically
/// through the shared `SqliteRegistry` regardless of which surface triggers it.
#[tokio::test]
async fn template_registry_equivalence() {
    let mut registry = EquivalenceFixture::create_registry();

    // — Register a template (the "CLI path": kask template register)
    let entry = RegistryEntry {
        id: "prompt/selector".to_string(),
        template_type: TemplateType::Prompt,
        lexicon_terms: vec!["select".to_string(), "discriminate".to_string()],
        description: "Template selector prompt".to_string(),
        source_path: "registry/templates/selector.j2".to_string(),
        required_capabilities: vec!["inference:execute".to_string()],
    };

    registry
        .register(entry.clone(), None)
        .expect("Failed to register template");

    // — List templates (the "API path": GET /api/v1/templates)
    let all = registry.list(None);
    assert_eq!(all.len(), 1, "One template should be registered");
    assert_eq!(all[0].id, "prompt/selector");
    assert_eq!(all[0].template_type, TemplateType::Prompt);

    // — List by type (the "MCP path": registry_list(type))
    let prompts = registry.list(Some(TemplateType::Prompt));
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].description, "Template selector prompt");

    // Empty list for unrelated type
    let processes = registry.list(Some(TemplateType::Process));
    assert!(processes.is_empty());

    // — Get by ID (shared lookup across surfaces)
    let retrieved = registry
        .get("prompt/selector")
        .expect("Failed to get template");
    assert_eq!(retrieved.id, "prompt/selector");
    assert_eq!(retrieved.lexicon_terms.len(), 2);
    assert!(retrieved.lexicon_terms.contains(&"select".to_string()));

    // — RegistryIndex trait verification (same trait across surfaces)
    let count = registry.count();
    assert_eq!(count, 1, "Registry count must match");
}

// ─── Test 3: Capability Token Verification ──────────────────────────────────

/// Verifies OCAP token creation, attenuation, and verification — the shared
/// domain logic that all three surfaces use for capability checks.
#[tokio::test]
async fn capability_token_equivalence() {
    let secret = b"hkask-test-secret-key-32bytes!!";
    let issuer = WebID::new();
    let holder = WebID::new();

    // — Create a root capability token (the "MCP path": ocap_grant)
    let root_token = CapabilityTokenBuilder::new(
        CapabilityResource::Tool,
        "inference".to_string(),
        CapabilityAction::Execute,
        issuer,
        holder,
    )
    .attenuation(0, 3) // root at level 0, max 3
    .sign(secret);

    // — Verify the root token
    let checker = CapabilityChecker::new(secret);
    assert!(
        checker.verify(&root_token),
        "Root token must be cryptographically valid"
    );

    // Verify it grants the expected resource
    assert!(
        root_token.is_valid_for(
            CapabilityResource::Tool,
            "inference",
            CapabilityAction::Execute
        ),
        "Root token must be valid for inference:execute"
    );

    // Verify it does NOT grant a different resource
    assert!(
        !root_token.is_valid_for(
            CapabilityResource::Template,
            "inference",
            CapabilityAction::Execute
        ),
        "Root token must not be valid for template:execute"
    );

    // — Attenuate to depth 1
    let child_holder = WebID::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let attenuated = root_token
        .attenuate(child_holder, secret, current_time)
        .expect("Attenuation must succeed at depth 0→1");

    assert_eq!(
        attenuated.attenuation_level, 1,
        "Attenuated token must have level = parent + 1"
    );
    assert_eq!(
        attenuated.max_attenuation, 3,
        "Attenuated token must preserve max_attenuation"
    );

    // — Verify the attenuation chain
    let root_nonce = root_token.root_context_nonce();
    assert!(
        attenuated.verify_attenuation_chain(root_nonce, 3),
        "Attenuation chain must verify at allowed level"
    );

    // — Verify the attenuated token cryptographically
    assert!(
        checker.verify(&attenuated),
        "Attenuated token must be cryptographically valid"
    );

    // — Verify checker grants tool and holder matches
    assert!(
        checker.check(
            &attenuated,
            &child_holder,
            CapabilityResource::Tool,
            "inference",
            CapabilityAction::Execute
        ),
        "CapabilityChecker must validate correct holder + resource + action"
    );

    // — Reject token with max_attenuation > SYSTEM_MAX_ATTENUATION (7)
    let excessive_token = CapabilityTokenBuilder::new(
        CapabilityResource::Tool,
        "inference".to_string(),
        CapabilityAction::Execute,
        issuer,
        holder,
    )
    .attenuation(0, 8) // max 8 > SYSTEM_MAX_ATTENUATION (7)
    .sign(secret);

    assert!(
        !excessive_token.verify_attenuation_chain(excessive_token.root_context_nonce(), 8),
        "Token with max_attenuation > SYSTEM_MAX_ATTENUATION must be rejected"
    );
}

// ─── Test 4: CNS Span Equivalence ───────────────────────────────────────────

/// Verifies that CNS spans are emitted consistently with correct phase
/// tracking, and that the CnsRuntime health check reflects system state
/// identically regardless of which surface queries it.
#[tokio::test]
async fn cns_span_equivalence() {
    let fx = EquivalenceFixture::new();

    // — Emit a cns.tool.test span with Observe phase
    let observation = json!({
        "tool": "test_tool",
        "invocation_id": "inv-001",
        "status": "started"
    });

    fx.span_emitter.emit_with_phase(
        Span::Tool("cns.tool.test".to_string()),
        Phase::Observe,
        observation.clone(),
    );

    // — Emit the same span with Regulate phase (different cybernetic cycle)
    let regulation = json!({
        "tool": "test_tool",
        "invocation_id": "inv-001",
        "status": "regulating",
        "correction": "rate_limited"
    });

    fx.span_emitter.emit_with_phase(
        Span::Tool("cns.tool.test".to_string()),
        Phase::Regulate,
        regulation.clone(),
    );

    // — Emit Outcome phase
    let outcome = json!({
        "tool": "test_tool",
        "invocation_id": "inv-001",
        "status": "completed",
        "result": "success"
    });

    fx.span_emitter.emit_with_phase(
        Span::Tool("cns.tool.test".to_string()),
        Phase::Outcome,
        outcome.clone(),
    );

    // — Verify events were persisted by querying the NuEventStore
    let event_store = NuEventStore::new(Arc::clone(&fx.conn));
    let tool_events = event_store
        .query_by_span("tool", 10)
        .expect("Failed to query tool spans");

    assert_eq!(tool_events.len(), 3, "All three phases must be persisted");

    // — Assert phases are preserved (not discarded)
    let phases: Vec<Phase> = tool_events.iter().map(|e| e.phase).collect();
    assert!(
        phases.contains(&Phase::Observe),
        "Observe phase must be preserved"
    );
    assert!(
        phases.contains(&Phase::Regulate),
        "Regulate phase must be preserved"
    );
    assert!(
        phases.contains(&Phase::Outcome),
        "Outcome phase must be preserved"
    );

    // — Assert observations match what was emitted
    let observe_event = tool_events
        .iter()
        .find(|e| e.phase == Phase::Observe)
        .unwrap();
    assert_eq!(observe_event.observation["tool"], "test_tool");
    assert_eq!(observe_event.observation["status"], "started");

    let outcome_event = tool_events
        .iter()
        .find(|e| e.phase == Phase::Outcome)
        .unwrap();
    assert_eq!(outcome_event.observation["result"], "success");

    // — Verify CNS runtime health (simulates CLI/API CNS health query)
    let health = fx.cns_runtime.health().await;
    assert!(health.healthy, "Fresh CNS runtime must report healthy");
    assert_eq!(health.critical_count, 0);

    // — Increment variety and verify counters are tracked
    fx.cns_runtime
        .increment_variety("test_tool", "invoked")
        .await;
    fx.cns_runtime
        .increment_variety("test_tool", "completed")
        .await;

    let variety = fx.cns_runtime.variety_for_domain("test_tool").await;
    assert_eq!(variety, 2, "Variety counter must reflect increments");

    // Check variety doesn't generate alerts for low counters yet
    let alert = fx.cns_runtime.check_variety("test_tool").await;
    // With only 2 variety and default expected of 10, deficit=8 < threshold=100
    // This should still generate an alert (Info severity), but not be critical
    assert!(
        alert.is_none() || !alert.as_ref().unwrap().is_critical(),
        "Low variety must not trigger critical alerts"
    );
}

// ─── Test 5: Surface Equivalence Matrix ─────────────────────────────────────

/// Validates that each `SystemCapability` from the interface-and-composition.md
/// equivalence matrix (§1.4) exists in the domain logic. This proves the
/// three-surface claim has shared logic underneath.
#[tokio::test]
async fn surface_equivalence_matrix() {
    // — tool_invoke: Verify CapabilityToken + CapabilityChecker grant tool access
    let secret = b"matrix-test-secret-key-32bytes";
    let issuer = WebID::new();
    let holder = WebID::new();
    let checker = CapabilityChecker::new(secret);

    let tool_token = checker.grant_tool("test_tool".to_string(), issuer, holder);
    assert!(
        checker.verify(&tool_token),
        "tool_invoke: capability token must be verifiable"
    );
    assert!(
        tool_token.grants_resource(CapabilityResource::Tool),
        "tool_invoke: token must grant Tool resource"
    );

    // — template_render: Verify TemplateType + RegistryIndex render path
    let mut registry = EquivalenceFixture::create_registry();
    let entry = RegistryEntry {
        id: "prompt/test".to_string(),
        template_type: TemplateType::Prompt,
        lexicon_terms: vec!["recognize".to_string()],
        description: "Test prompt".to_string(),
        source_path: "templates/test.j2".to_string(),
        required_capabilities: vec![],
    };
    registry
        .register(entry, None)
        .expect("template_render: register must succeed");
    let found = registry
        .get("prompt/test")
        .expect("template_render: must find registered template");
    assert_eq!(found.template_type, TemplateType::Prompt);

    // — spec_capture: Verify GoalSpec → Spec → SqliteSpecStore roundtrip
    let fx = EquivalenceFixture::new();
    let goal = GoalSpec::new("Test capability goal").with_criterion("Must be verifiable");
    let spec = hkask_types::Spec::new("test-spec", SpecCategory::Capability, DomainAnchor::Hkask)
        .with_goal(goal);
    fx.spec_store
        .save(&spec)
        .expect("spec_capture: save must succeed");
    let loaded = fx
        .spec_store
        .load(spec.id)
        .expect("spec_capture: load must succeed");
    assert_eq!(loaded.goals[0].text, "Test capability goal");

    // — template_list: Verify RegistryIndex::list returns correct count
    let all = registry.list(None);
    assert!(
        !all.is_empty(),
        "template_list: registry must contain entries"
    );
    let prompts = registry.list(Some(TemplateType::Prompt));
    assert!(!prompts.is_empty(), "template_list: must list by type");

    // — cns_health: Verify CnsRuntime reports healthy state
    let health = fx.cns_runtime.health().await;
    assert!(health.healthy, "cns_health: system must report healthy");
    assert_eq!(health.overall_deficit, 0, "cns_health: deficit must be 0");

    // — model: Verify SYSTEM_MAX_ATTENUATION constant (system constant enforcement)
    assert_eq!(
        SYSTEM_MAX_ATTENUATION, 7,
        "System max attenuation must be 7 (architecture v0.21.0)"
    );

    let max_token = CapabilityTokenBuilder::new(
        CapabilityResource::Tool,
        "any".to_string(),
        CapabilityAction::Execute,
        issuer,
        holder,
    )
    .attenuation(0, 7) // exactly at limit
    .sign(secret);

    assert!(
        max_token.verify_attenuation_chain(max_token.root_context_nonce(), 7),
        "Token at max_attenuation = SYSTEM_MAX_ATTENUATION must be accepted"
    );
}
