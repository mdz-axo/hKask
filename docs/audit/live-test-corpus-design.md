# Task 8 — Canonical Live Test Corpus Design

**Date:** 2026-06-16  
**Purpose:** Define end-to-end test prompts that exercise primary skills through the registry templates and `ManifestExecutor`, producing deterministic invariants for CI.

---

## 1. Scope

The corpus covers the **calibrated primary skills** that are currently active or near-active:

- `coding-guidelines`
- `constraint-forces`
- `deep-module`
- `diagnose`
- `essentialist`
- `grill-me`
- `handoff`
- `improv`
- `improve-codebase-architecture`
- `kata` family (`kata`, `kata-coaching`, `kata-improvement`, `kata-starter`)
- `magna-carta-verifier`
- `pragmatic-cybernetics`
- `pragmatic-laziness`
- `pragmatic-semantics`
- `refactor-service-layer`
- `rust-expertise`
- `skill-bundler`
- `skill-discovery`
- `skill-logic-audit`
- `skill-maintenance`
- `skill-manager`
- `skill-translator`
- `strangler-fig`
- `tdd`
- `zoom-out`

---

## 2. Fixture Loader

A deterministic fixture loader should populate an in-memory `Registry` and `SkillRegistryIndex` for repeatable tests.

### Rust fixture helper (to be added to `hkask-services` test suite)

```rust
use hkask_templates::{Registry, SkillLoader};
use hkask_types::ports::{RegistryIndex, SkillRegistryIndex};

/// REQ: TST-001
/// pre:  project_root contains registry/templates and .agents/skills
/// post: returns a Registry with skills loaded and template entries registered
pub fn load_skill_fixture(project_root: &Path) -> Registry {
    let mut registry = Registry::new();
    let loader = SkillLoader::new(project_root);
    loader.load_into(&mut registry);
    // Register all .j2 files found under registry/templates as RegistryEntry records.
    // (Exact implementation to reuse registry scanning logic from SkillAuditor.)
    registry
}
```

For unit tests, the fixture should create a temporary project tree with:

1. `registry/hlexicon/hlexicon-workspace.yaml`
2. `.agents/skills/<name>/SKILL.md`
3. `registry/templates/<name>/manifest.yaml`
4. `registry/templates/<name>/*.j2`
5. Optional: `registry/manifests/<name>/*.yaml` for FlowDef skills

---

## 3. Representative End-to-End Prompts

Each prompt is a `kask chat` input that should trigger a specific calibrated skill through the inference router and `ManifestExecutor`.

| # | Prompt | Expected active skill | Runtime artifact | Invariant |
|---|--------|----------------------|------------------|-----------|
| 1 | "Assess this coding task against the four principles" | `coding-guidelines` | `coding-guidelines/guidelines-assess.j2` | Output JSON has `assessment` field; no `todo!()` recommendation |
| 2 | "Classify this constraint: 'Episodic memory must not be exposed without consent'" | `constraint-forces` | `constraint-forces/constraint-forces-classify.j2` | `force_type == "Prohibition"`; `relaxable == false` |
| 3 | "Is this module deep? It has 20 public functions and 50 lines of logic." | `deep-module` | `deep-module/deep-module-assess.j2` | `classification` ∈ {Deep, Adequate, Shallow, Very Shallow}; `depth_score` present |
| 4 | "Run a TDD cycle for adding a new storage query method" | `tdd` | `tdd/tdd-plan.j2` | Output `functional_requirements` array non-empty; each item has a spec trace |
| 5 | "Classify this statement: 'The variety deficit is moderate'" | `pragmatic-semantics` | `pragmatic-semantics/semantics-classify-statement.j2` | `epistemic_mode == "probabilistic"`; `ontological_mode == "descriptive"` |
| 6 | "Audit the skill-logic-audit templates against their goals" | `skill-logic-audit` | `registry/manifests/skill-logic-audit/audit-flow.yaml` | Cascade completes all 5 steps without unknown template refs |
| 7 | "Find the lazy path for refactoring this service layer" | `pragmatic-laziness` | `pragmatic-laziness/pragmatic-laziness-flow.j2` | `phases_completed` includes at least phase 1; `stationary` boolean present |
| 8 | "Verify this code against the Magna Carta" | `magna-carta-verifier` | registry templates under `magna-carta-verifier/` | Output lists any P1–P4 violations; none for valid code |
| 9 | "List and validate all skills" | `skill-manager` / `skill-maintenance` | `skill-maintenance-audit.j2` | Report JSON has `entries` array and active count |
| 10 | "Translate this single-layer skill into dual-layer" | `skill-translator` | registry templates under `skill-translator/` | Output contains both Zed and registry artifacts |

---

## 4. Invariants for Every Test

1. The selected template id matches the expected runtime artifact.
2. The rendered template body contains no unknown Jinja2 variables (or handles missing vars gracefully).
3. The inference result parses as valid JSON matching the contract `output` schema.
4. No `FlowDef` is declared on a `.j2` file anywhere in the cascade.
5. No `todo!()`, `unimplemented!()`, or deprecated marker appears in any emitted code recommendation.
6. All hLexicon terms in the rendered templates exist in `hlexicon-workspace.yaml`.

---

## 5. Example Test Case: ManifestExecutor FlowDef Execution

A test that drives `ManifestExecutor` with the `skill-logic-audit/audit-flow.yaml` manifest and asserts the cascade completes.

```rust
#[tokio::test]
async fn skill_logic_audit_flow_executes_without_unknown_refs() {
    let project_root = Path::new(".");
    let mut registry = load_skill_fixture(project_root);
    let manifest = registry
        .get_bundle("skill-logic-audit/audit-flow")
        .expect("audit-flow manifest exists")
        .clone();

    // Mock inference and MCP ports for the test harness.
    let inference = Arc::new(TestInferencePort::new());
    let mcp = Arc::new(TestMcpPort::new());

    let executor = ManifestExecutor::new(
        inference,
        mcp,
        LLMParameters::default(),
        vec![0u8; 32],
    );

    let mut context = HashMap::new();
    context.insert(
        "target_path".to_string(),
        Value::String("registry/templates/deep-module/manifest.yaml".to_string()),
    );
    context.insert(
        "target_content".to_string(),
        Value::String(fs::read_to_string("registry/templates/deep-module/manifest.yaml").unwrap()),
    );
    context.insert("template_type".to_string(), Value::String("manifest".to_string()));
    context.insert("loop_depth".to_string(), Value::Number(0.into()));
    context.insert("user_counter_proposal".to_string(), Value::Null);

    let result = executor.execute_manifest(&manifest, context).await.unwrap();

    // Invariants
    assert!(result.contains_key("step_1_result"), "load-goal step produced output");
    assert!(result.contains_key("step_5_result"), "user-choice step produced output");
    let choice = result
        .get("step_5_result")
        .and_then(|v| v.get("user_choice"))
        .and_then(|v| v.as_str())
        .expect("user_choice present");
    assert!(matches!(choice, "accept" | "reject" | "counter-proposal"));
}
```

**Note:** This test requires `TestInferencePort` and `TestMcpPort` implementations in the test harness. They can be thin mocks that return deterministic JSON shaped like each step's `output_schema`.

---

## 6. Next Steps

1. Add the fixture loader helper to `hkask-services` test utilities.
2. Implement `TestInferencePort` and `TestMcpPort` in `hkask-test-harness`.
3. Add one integration test per prompt in the table above.
4. Wire the corpus into the CI gate so a regression in any primary skill fails the build.
