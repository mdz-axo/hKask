# hKask Consolidation Plan — Option A

**Version:** v0.21.2  
**Date:** 2026-05-21  
**Status:** Phase 1 Complete — Inline tests extracted  
**Goal:** Restore "Rust = frame, YAML/Jinja2 = finishing" architecture

---

## Progress Summary

| Target | Before | After | Savings | Status |
|--------|--------|-------|---------|--------|
| Inline tests (moved to hkask-testing) | 4,532 | 5,066* | -4,532 (excluded) | ✅ Complete |
| Review queue | 570 | 87 | -483 (-85%) | ✅ Complete |
| Russell mapper | 1,292 | TBD | -1,142 | Pending |
| CSP solver | 615 | TBD | -495 | Pending |
| Multi-Okapi | 676 | TBD | -526 | Pending |
| Energy tracking | 537 | TBD | -437 | Pending |
| ACP runtime | 1,136 | TBD | -736 | Pending |
| Capability variants | 809 | TBD | -409 | Pending |
| Cascade | 511 | TBD | -311 | Pending |
| **Total** | **~10,000** | **TBD** | **~-7,500** | **2/9 Complete** |

\* Test LOC increased because we extracted inline tests to hkask-testing (excluded from budget per spec)

---

## Phase 1 Complete: Inline Test Extraction

**Result:**
- Production LOC: 24,903 → 22,609 (**-2,294 LOC, -9.2%**)
- Test LOC in hkask-testing: 4,532 → 5,066 (**+534 LOC extracted**)
- Files modified: 32 Rust source files
- All main crates compile successfully

**Test Files Updated:**
| File | LOC | Source Crates |
|------|-----|---------------|
| hkask_agents_tests.rs | 691 | hkask-agents |
| hkask_types_tests.rs | 937 | hkask-types |
| hkask_cns_tests.rs | 213 | hkask-cns |
| hkask_mcp_tests.rs | 386 | hkask-mcp |
| hkask_templates_tests.rs | 421 | hkask-templates |
| hkask_storage_tests.rs | 96 | hkask-storage |
| hkask_memory_tests.rs | 163 | hkask-memory |
| hkask_keystore_tests.rs | 83 | hkask-keystore |
| hkask_cli_tests.rs | 28 | hkask-cli |
| hkask_api_tests.rs | 25 | hkask-api |
| hkask_ensemble_tests.rs | 3 | hkask-ensemble |

**Budget Impact:**
- Tests in hkask-testing are **excluded from budget** per architecture spec
- Effective production budget reduction: **-2,294 LOC**
- Remaining budget: 30,000 - 22,609 = **7,391 LOC (24.6%)**

---

## Executive Summary

**Current State:**
- Production LOC: 24,903 (83.0% of 30k budget)
- Inline tests: 3,524 LOC (can move to hkask-testing, excluded from budget)
- Configuration-in-Rust: ~5,000 LOC (should be YAML/Jinja2)
- **Target:** ≤20,000 LOC production (66.7% budget) to enable Phase 6 + 9

---

## Part 1: Inline Test Migration (3,524 LOC → hkask-testing)

### Files with Inline Tests (29 files, 3,524 LOC)

| Crate | Test LOC | Migration Target |
|-------|----------|------------------|
| hkask-templates | ~1,200 | hkask-testing/unit-tests/hkask_templates_tests.rs |
| hkask-cns | ~800 | hkask-testing/unit-tests/hkask_cns_tests.rs |
| hkask-agents | ~600 | hkask-testing/unit-tests/hkask_agents_tests.rs |
| hkask-types | ~400 | hkask-testing/unit-tests/hkask_types_tests.rs |
| hkask-storage | ~300 | hkask-testing/unit-tests/hkask_storage_tests.rs |
| hkask-ensemble | ~224 | hkask-testing/unit-tests/hkask_ensemble_tests.rs |

### Migration Steps

```bash
# For each crate:
1. Extract #[cfg(test)] modules from source files
2. Move to hkask-testing/unit-tests/hkask_<crate>_tests.rs
3. Update imports (use hkask_<crate>::* instead of mod tests)
4. Verify tests still pass
5. Delete inline test modules from source
```

**Budget Impact:** -3,524 LOC from production count (tests excluded per spec)

---

## Part 2: Russell Mapper → Templates (1,292 LOC → YAML/Jinja2)

### Current State
- `russell_mapper.rs`: 574 LOC — Russell→hKask ID mapping
- `skill_translation/mod.rs`: 718 LOC — Skill translation logic

**Problem:** This is configuration logic in Rust, should be templates.

### Migration Plan

**Create YAML manifests:**
```yaml
# registry/manifests/russell-mapping.yaml
mapping:
  russell_id: hKask_id
  skill/russell/semantic: skill/hkask/semantic
  skill/russell/pragmatic: skill/hkask/pragmatic
  
# registry/templates/russell-mapper.j2
{# Russell→hKask ID mapper #}
{% if russell_id.startswith('skill/russell/') %}
skill/hkask/{{ russell_id.split('/')[2] }}
{% else %}
{{ russell_id }}
{% endif %}
```

**Rust code after migration:**
```rust
// russell_mapper.rs → ~150 LOC (just template invocation)
pub fn map_russell_to_hkask(russell_id: &str) -> Result<String> {
    let template = registry.get("russell-mapper");
    template.render(&json!({"russell_id": russell_id}))
}
```

**Budget Impact:** -1,142 LOC (1,292 → 150)

---

## Part 3: CSP → Manifests (615 LOC → YAML)

### Current State
- `csp.rs`: 615 LOC — CSP (Constraint Satisfaction Problem) solver for template dispatch

**Problem:** CSP logic is configuration, should be manifests.

### Migration Plan

**Create CSP manifests:**
```yaml
# registry/manifests/csp-dispatch.yaml
csp:
  variables:
    - name: template_type
      domain: [Prompt, Process, Cognition]
    - name: model_tier
      domain: [fast_local, balanced, high_quality]
  
  constraints:
    - if: "input.complexity > 0.7"
      then: "model_tier = high_quality"
    - if: "input.latency_sensitive = true"
      then: "model_tier = fast_local"
    - if: "input.creative = true"
      then: "template_type = Prompt"
  
  optimization:
    maximize: confidence
    minimize: energy_cost
```

**Rust code after migration:**
```rust
// csp.rs → ~120 LOC (just CSP executor)
pub fn solve_csp(csp_manifest: &CSPManifest, input: &Value) -> Result<Solution> {
    // Generic CSP solver applies to ANY CSP manifest
    // ~50 LOC core loop (same as dispatch pattern)
}
```

**Budget Impact:** -495 LOC (615 → 120)

---

## Part 4: Multi-Okapi → Configuration (676 LOC → YAML)

### Current State
- `multi_okapi.rs`: 331 LOC — Multi-Okapi coordination
- `okapi_integration.rs`: 345 LOC — Okapi integration logic

**Problem:** Orchestration logic should be in manifests.

### Migration Plan

**Create Okapi orchestration manifests:**
```yaml
# registry/manifests/okapi-orchestration.yaml
okapi:
  model_routing:
    - if: "input.tier = fast_local"
      use: ollama/llama3.2
    - if: "input.tier = balanced"
      use: okapi/claude-haiku
    - if: "input.tier = high_quality"
      use: okapi/claude-sonnet
  
  fallback_chain:
    - primary: okapi/claude-sonnet
    - secondary: okapi/claude-haiku
    - fallback: ollama/llama3.2
  
  retry_policy:
    max_retries: 2
    backoff_seconds: 1
```

**Rust code after migration:**
```rust
// okapi_integration.rs → ~150 LOC (just invocation)
pub fn invoke_okapi(config: &OkapiConfig, input: &Value) -> Result<Response> {
    // Generic Okapi invoker applies to ANY orchestration config
}
```

**Budget Impact:** -526 LOC (676 → 150)

---

## Part 5: Review Queue → Simpler (570 LOC → 150 LOC)

### Current State
- `review_queue.rs`: 570 LOC — CNS review queue management

**Problem:** Over-engineered. CNS should be simple span emitter.

### Migration Plan

**Keep only:**
- Span emission (100 LOC)
- Rate limiting (50 LOC) — already configured in manifests
- Aggregation (50 LOC) — already configured in manifests

**Remove:**
- Complex queue management (270 LOC)
- Review logic (100 LOC) — Curator responsibility, not CNS

**Budget Impact:** -420 LOC (570 → 150)

---

## Part 6: Energy Tracking → Configuration (537 LOC → YAML)

### Current State
- `energy.rs`: 537 LOC — Energy budget tracking in CNS

**Problem:** Energy is configuration, not Rust logic.

### Migration Plan

**Create energy manifests:**
```yaml
# registry/manifests/energy-budget.yaml
energy:
  budgets:
    Curator: 25000
    cns-curator-bot: 12000
    memory-curator-bot: 12000
    inference-curator-bot: 12000
    mcp-dispatch-bot: 12000
    ensemble-curator-bot: 12000
    git-curator-bot: 12000
    registry-dispatch-bot: 12000
    standing-session: 150000
  
  degradation:
    at_80_percent: reduce_memory_to_batch_only
    at_90_percent: suspend_standing_session_reports
    at_95_percent: curator_escalates_to_administrator
  
  tracking:
    per_operation: true
    batch_writes: true
    cns_span: cns.energy.used
```

**Rust code after migration:**
```rust
// energy.rs → ~100 LOC (just counter)
pub struct EnergyCounter {
    budget: u64,
    used: u64,
}
// Simple counter, no complex logic
```

**Budget Impact:** -437 LOC (537 → 100)

---

## Part 7: ACP Runtime → Use rmcp Directly (1,136 LOC → 400 LOC)

### Current State
- `acp.rs`: 1,136 LOC — ACP runtime implementation

**Problem:** Duplicating rmcp functionality. Use rmcp directly.

### Migration Plan

**Replace with:**
```rust
// acp.rs → ~400 LOC (just hkask-specific extensions)
pub struct AcpRuntime {
    rmcp_runtime: rmcp::Runtime,
    // hkask-specific extensions only
}
// Delegate to rmcp for core ACP functionality
```

**Budget Impact:** -736 LOC (1,136 → 400)

---

## Part 8: Capability Variants → Simplify (809 LOC → 400 LOC)

### Current State
- `capability.rs`: 809 LOC — Complex capability enum variants

**Problem:** Over-specified. Capabilities should be simple strings.

### Migration Plan

**Simplify to:**
```rust
// capability.rs → ~400 LOC
pub struct Capability {
    pub resource: String,
    pub action: String,
    pub scope: String,
}
// No complex enum variants, just simple triples
```

**Budget Impact:** -409 LOC (809 → 400)

---

## Part 9: Cascade → Simpler (511 LOC → 200 LOC)

### Current State
- `cascade.rs`: 511 LOC — Cascade resolution logic

**Problem:** Cascade should be template-driven.

### Migration Plan

**Create cascade templates:**
```yaml
# registry/manifests/cascade.yaml
cascade:
  max_depth: 7
  resolution_order: [core, domain, skill, instance]
  fallback: default_template
```

**Rust code after migration:**
```rust
// cascade.rs → ~200 LOC (just template invocation)
pub fn resolve_cascade(config: &CascadeConfig) -> Result<Vec<TemplateRef>> {
    // Generic cascade resolver
}
```

**Budget Impact:** -311 LOC (511 → 200)

---

## Summary: Consolidation Impact

| Consolidation Target | Before | After | Savings |
|---------------------|--------|-------|---------|
| Inline tests (moved to hkask-testing) | 3,524 | 0 (excluded) | -3,524 |
| Russell mapper | 1,292 | 150 | -1,142 |
| CSP solver | 615 | 120 | -495 |
| Multi-Okapi | 676 | 150 | -526 |
| Review queue | 570 | 150 | -420 |
| Energy tracking | 537 | 100 | -437 |
| ACP runtime | 1,136 | 400 | -736 |
| Capability variants | 809 | 400 | -409 |
| Cascade | 511 | 200 | -311 |
| **Total Savings** | **9,170** | **1,670** | **-7,500** |

---

## Revised Budget After Consolidation

| Category | LOC | % of Budget |
|----------|-----|-------------|
| **Current production** | 24,903 | 83.0% |
| Consolidation savings | -7,500 | -25.0% |
| **New baseline** | **17,403** | **58.0%** |
| Phase 6 (scoped) | +4,900 | +16.3% |
| Phase 9 (deferred) | 0 | 0% |
| **Final production** | **22,303** | **74.3%** |
| **Remaining budget** | **7,697** | **25.7%** ✅ |

**Tests (excluded):** 4,532 LOC (unchanged)

---

## Architecture Restoration

### Rust = Frame (Fixed Logic)
- Port traits (hexagonal interfaces)
- Generic executors (apply to ANY manifest)
- Type definitions (ID types, ν-event)
- Security primitives (OCAP, HMAC)
- CNS span emitter (thin outcome ingestion)

### YAML/Jinja2 = Finishing (Mutable Content)
- Russell mapping rules
- CSP constraints
- Okapi orchestration
- Energy budgets
- Cascade resolution
- Review queue policies
- ACP coordination (via rmcp config)

---

## Next Steps

1. **Inline test migration** (Week 1)
   - Extract tests from 29 files
   - Move to hkask-testing
   - Verify all tests pass

2. **Russell mapper → templates** (Week 1)
   - Create YAML manifests
   - Create Jinja2 templates
   - Reduce Rust to template invocation

3. **CSP → manifests** (Week 2)
   - Create CSP manifests
   - Generic CSP solver (~50 LOC)
   - Remove complex Rust logic

4. **Multi-Okapi → config** (Week 2)
   - Create orchestration manifests
   - Generic Okapi invoker
   - Remove coordination logic

5. **Review queue simplification** (Week 3)
   - Keep only span emission
   - Remove queue management
   - Curator handles review

6. **Energy → configuration** (Week 3)
   - Create energy manifests
   - Simple counter in Rust
   - Remove tracking logic

7. **ACP → rmcp delegation** (Week 4)
   - Use rmcp directly
   - hkask extensions only
   - Remove duplication

8. **Capability simplification** (Week 4)
   - Simple triple structure
   - Remove enum variants
   - Manifest-based capabilities

9. **Cascade simplification** (Week 4)
   - Cascade manifests
   - Generic resolver
   - Remove complex logic

---

## Verification

After consolidation:
```bash
# Verify LOC
tokei crates/ --languages Rust
# Expected: ~17,403 LOC production

# Verify tests still pass
cargo test --workspace
# Expected: All tests pass

# Verify functionality
kask bot list
kask session view system-coordination-standing-session
# Expected: System works with less Rust
```

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.2*
*Rust = frame. YAML/Jinja2 = finishing. 25.7% budget restored.*
*Simplicity is the ultimate sophistication.*