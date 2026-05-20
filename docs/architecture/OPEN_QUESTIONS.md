# Open Questions — hKask Remediation

**Date:** 2026-05-20  
**Status:** Active  
**Related:** Adversarial Review Remediation Plan, Phase 2/5 Completion

---

## Summary

This document tracks open questions and deferred decisions from the remediation plan. Phase 2 (Security Hardening) and Phase 5 (Integration Tests) completed on 2026-05-20.

---

## Completed Tasks

### ✅ Phase 1: Consolidation (2026-05-20)
1. **Task 1.1: Audit composition.rs** — Identified unique functionality
2. **Task 1.2: Eliminate redundancy** — Deleted `composition.rs`, migrated to `ports.rs` and `dependency.rs`
3. **Task 1.3: Unify executor traits** — `DependencyProvider` trait in `ports.rs`
4. **Task 1.4: Remove dead code** — Removed `default_timeout_ms`, `channel_rx`, `MAX_RECURSION_DEPTH`

### ✅ Phase 2: Security Hardening (2026-05-20)
5. **Task 2.1: Capability Attenuation** — `CascadeContext::child_context()` attenuates on recursion
6. **Task 2.2: Security Adapter** — `CascadeExecutor` integrates `SecurityAdapter`
7. **Task 2.3: CNS Integration** — Documented, deferred to API/CLI layer

### ✅ Phase 3: CSP Channel Integration (2026-05-20)
8. **Task 3.1: Complete CSP** — `IsolatedStageRunner` used by all stage executors

### ✅ Phase 4: Dependency Injection (2026-05-20)
9. **Task 4.1: DependencyProvider** — Trait functional, `InMemoryDependencyProvider` implemented

### ✅ Phase 5: Integration Tests (2026-05-20)
10. **Task 5.1: End-to-end tests** — 6 new tests for security properties

### ✅ Phase 6: Public API Audit (2026-05-20)
11. **Task 6.1: API cleanup** — Exports cleaned in `lib.rs`

---

## Test Results

**Total:** 237 tests passing across workspace

| Crate | Tests | Status |
|-------|-------|--------|
| hkask-types | 50 | ✅ |
| hkask-templates | 138 | ✅ |
| hkask-cns | 49 | ✅ |
| hkask-mcp | (varies) | ✅ |
| hkask-storage | (varies) | ✅ |

---

## Resolved Issues (2026-05-20)

### Phase 2: Security Hardening

#### Capability Attenuation Implementation
- **Issue:** How to implement OCAP attenuation on recursive calls?
- **Resolution:** `CascadeContext::child_context(new_holder: WebID)` creates attenuated token
- **Evidence:** `test_cascade_context_child_with_attenuation` verifies attenuation level increases
- **Status:** ✅ Complete

#### Security Adapter Integration
- **Issue:** How to integrate security checks into cascade execution?
- **Resolution:** `CascadeExecutor` holds `SecurityAdapter`, validates paths and capabilities
- **Evidence:** `test_cascade_security_path_traversal_blocked` verifies `../etc/passwd` blocked
- **Status:** ✅ Complete

#### CNS Span Emission
- **Issue:** Where should CNS spans be emitted for composition events?
- **Resolution:** Defer to API/CLI layer — template library应保持 hexagonal boundary
- **Evidence:** Module docstrings document CNS integration points
- **Status:** ✅ Documented, deferred appropriately

### Dead Code Warnings Resolved (2026-05-20)
- **`default_timeout_ms`** in `csp.rs` — Removed unused field from `CspPipelineExecutor`
- **`MAX_RECURSION_DEPTH`** in `security.rs` — Removed unused constant  
- **`channel_rx`** in `skill_translation/mod.rs` — Removed unused field from `SkillTranslationPipeline`

**Verification:** `cargo check -p hkask-templates` ✅ (0 warnings)

---

## Open Questions

### 1. Capability Revocation Lists

**Context:** Capabilities attenuate on delegation but cannot be revoked.

**Question:** Should compromised capability tokens be revocable?

**Current Behavior:** Tokens expire via `expires_at` timestamp or reach `max_attenuation`.

**Considerations:**
- Revocation list adds storage and lookup overhead
- Could use bloom filter for efficient membership testing
- Alternative: Short expiration times + rotation

**Action Items:**
- [ ] Design revocation list schema
- [ ] Implement `CapabilityChecker::revoke(token_id)`
- [ ] Add revocation check to `CapabilityToken::verify()`

---

### 2. Security Adapter Configuration

**Context:** `SecurityAdapter` has hardcoded path patterns and Jinja2 dangerous patterns.

**Question:** Should security policies be configurable per deployment?

**Current Behavior:** Constants defined in `security.rs`.

**Considerations:**
- Different deployments may have different security requirements
- Could load policies from configuration file
- Alternative: Environment variable overrides

**Action Items:**
- [ ] Design security policy configuration schema
- [ ] Implement `SecurityAdapter::with_config(config)`
- [ ] Document security policy best practices

---

### 3. Jinja2 Injection Prevention

**Context:** `SecurityAdapter::sanitize_jinja2_input()` blocks dangerous patterns.

**Question:** Is pattern blocking sufficient, or need sandboxed execution?

**Current Behavior:** Regex-based pattern matching.

**Considerations:**
- Pattern matching may miss novel attack vectors
- `minijinja` has sandboxing features
- Could use both: pattern blocking + sandbox

**Action Items:**
- [ ] Evaluate `minijinja` sandboxing capabilities
- [ ] Test pattern bypass attempts
- [ ] Consider defense-in-depth approach

---

## Deferred Work

### Not Implemented (Per Architecture v0.21.0)

The following were explicitly excluded from implementation per architecture spec:

1. **Bot reputation systems** — Not part of MVP
2. **Bot swarms / consensus mechanisms** — NO swarms per spec
3. **Cross-machine sync** — Out of scope
4. **Bot marketplace** — Out of scope
5. **Curator customization** — Single system persona
6. **SemVer versioning** — Git-only versioning
7. **Separate feedback crate** — CNS handles all feedback
8. **Promotion pipeline** — Episodic/semantic categorical
9. **Escalation primitive** — Algedonic alerts handle escalation
10. **Visibility type system** — OCAP-enforced
11. **OCT-H currency** — Not implemented
12. **Fine-tuning (axolotl)** — Out of scope
13. **OpenCode/OpenHands-style condenser** — Out of scope
14. **UCAN for h-bar** — OCAP-only
15. **Three separate registries** — Unified registry with template_type discriminator
16. **Rust-based template selection** — Selection intelligence in Jinja2/LLM

---

## Next Steps

1. **Address P4 open questions** based on production feedback
2. **Monitor CNS channel performance** under load
3. **Review error handling** patterns across all crates
4. **Document operational procedures** for salt backup/recovery
5. **Consider persistence strategy** for variety state

---

*This document should be reviewed and updated as open questions are resolved.*
