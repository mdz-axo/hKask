# ℏKask Open Questions — Capability Composition Graph

**Document Version:** 1.0  
**Date:** 2026-05-20  
**Status:** OPEN — Requires architectural decisions

---

## Overview

This document captures open questions and underspecified aspects of the hKask capability-energy-authority model. These questions arise from the adversarial review remediation work (Tasks 4.1-4.6) and require architectural decisions before Phase 3 implementation.

---

## Open Questions

### Q1: Capability Chain Attenuation in Multi-Manifest Execution

**Question:** When manifest A calls manifest B (nested execution), should capabilities be re-attenuated at each level?

**Current State:** Capabilities attenuated once at template render time.

**Options:**
1. **Single attenuation** — Attenuate at outermost manifest only (current behavior)
2. **Cascading attenuation** — Re-attenuate at each manifest boundary (more restrictive)
3. **Delegated attenuation** — Outer manifest delegates attenuated capability to inner manifest

**Implications:**
- Option 1: Simpler, but inner manifest has same authority as outer
- Option 2: Most secure (least authority), but complex capability tracking
- Option 3: Middle ground, requires delegation mechanism

**Recommendation:** Option 2 (cascading attenuation) — follows Miller's least authority principle

**Decision Deadline:** Phase 3 kickoff

---

### Q2: Energy Budget Inheritance Across Manifest Hierarchy

**Question:** Does parent manifest energy budget include child manifest costs, or are they separate?

**Current State:** Each manifest has independent `energy_budget` in OCAP config.

**Options:**
1. **Independent budgets** — Each manifest has separate budget (current)
2. **Hierarchical budgets** — Parent budget includes all child costs
3. **Quota system** — Parent allocates quota to children

**Implications:**
- Option 1: Simple, but no global energy control
- Option 2: Global control, but complex tracking
- Option 3: Flexible, requires quota management

**Recommendation:** Option 3 (quota system) — balances control with flexibility

**Decision Deadline:** Phase 3 kickoff

---

### Q3: Capability Delegation Between WebIDs

**Question:** Can a WebID delegate capability to another WebID for manifest execution?

**Current State:** Capabilities tied to single WebID (delegated_to field).

**Options:**
1. **No delegation** — Capabilities non-transferable (current)
2. **Controlled delegation** — Delegation requires explicit authorization
3. **Open delegation** — Any holder can delegate (with attenuation)

**Implications:**
- Option 1: Most secure, least flexible
- Option 2: Balanced, requires delegation tracking
- Option 3: Most flexible, risk of authority dilution

**Recommendation:** Option 2 (controlled delegation) — enables collaboration without losing control

**Decision Deadline:** Phase 3 design review

---

### Q4: Cross-Machine Capability Verification

**Question:** If hKask runs distributed (multiple machines), how are capabilities verified across machines?

**Current State:** Single-machine assumption — capabilities verified in-memory.

**Options:**
1. **Centralized verification** — Single authority verifies all capabilities
2. **Distributed verification** — Each machine verifies independently (shared secret)
3. **Cryptographic verification** — Capabilities are self-verifying (HMAC signatures)

**Implications:**
- Option 1: Simple, creates bottleneck
- Option 2: Fast, requires secret distribution
- Option 3: Most scalable, requires signature infrastructure

**Recommendation:** Option 3 (cryptographic verification) — already in place via CapabilityToken signatures

**Decision Deadline:** Phase 3 distributed design

---

### Q5: Capability Expiry at Scale

**Question:** With 100+ manifests executing concurrently, how is capability expiry managed efficiently?

**Current State:** Capabilities have `expires_at` timestamp, checked at use time.

**Options:**
1. **Lazy expiry** — Check at use time only (current)
2. **Eager cleanup** — Background job removes expired capabilities
3. **Hybrid** — Lazy check + periodic cleanup

**Implications:**
- Option 1: Simple, expired capabilities accumulate
- Option 2: Clean, requires scheduler
- Option 3: Best of both, more complex

**Recommendation:** Option 3 (hybrid) — lazy check for correctness, cleanup for efficiency

**Decision Deadline:** Phase 3 performance planning

---

### Q6: Energy Budget Overflow Behavior

**Question:** What happens when energy cap is exceeded during manifest execution — abort, escalate, or continue?

**Current State:** `energy_budget` declared but not enforced at runtime.

**Options:**
1. **Hard abort** — Execution fails immediately
2. **Escalate** — Request additional budget from curator/human
3. **Continue with warning** — Log overflow, continue execution
4. **Graceful degradation** — Reduce quality/scope to fit budget

**Implications:**
- Option 1: Most secure, may break legitimate workflows
- Option 2: Flexible, requires escalation mechanism
- Option 3: Most permissive, defeats purpose of budgets
- Option 4: Sophisticated, requires quality metrics

**Recommendation:** Option 1 (hard abort) for security-critical, Option 2 (escalate) for user-facing

**Decision Deadline:** Phase 3 error handling design

---

### Q7: Capability Revocation Mid-Execution

**Question:** If capability is revoked during manifest execution, does execution abort or continue?

**Current State:** No revocation mechanism — capabilities valid until expiry.

**Options:**
1. **Continue** — Execution continues (capability was valid at start)
2. **Abort** — Execution aborts immediately
3. **Checkpoint** — Save state, allow resumption with new capability

**Implications:**
- Option 1: Simple, may execute with revoked authority
- Option 2: Most secure, may lose work
- Option 3: Most flexible, requires checkpointing

**Recommendation:** Option 2 (abort) for security, Option 3 (checkpoint) for production workflows

**Decision Deadline:** Phase 3 security review

---

### Q8: Sandbox Violation Escalation Path

**Question:** After sandbox violation detected, is template permanently blocked or can it be corrected?

**Current State:** `SandboxMonitor` detects violations, emits CNS span — no enforcement action.

**Options:**
1. **Permanent block** — Template blacklisted after violation
2. **Temporary block** — Template blocked until human review
3. **Warning only** — Log violation, allow execution
4. **Corrective action** — Attempt to sanitize template automatically

**Implications:**
- Option 1: Most secure, may block false positives permanently
- Option 2: Balanced, requires review queue
- Option 3: Most permissive, defeats security purpose
- Option 4: Sophisticated, may not be possible for all violations

**Recommendation:** Option 2 (temporary block + human review) — balances security with flexibility

**Decision Deadline:** Phase 3 security policy

---

## Underspecified Aspects

### US1: Capability Graph Traversal for Multi-Manifest Workflows

**Issue:** No specification for how capabilities compose when multiple manifests execute in sequence or parallel.

**Resolution Required:** Define capability composition semantics (intersection, union, or hierarchical).

---

### US2: Energy Budget Aggregation Across Manifest Hierarchy

**Issue:** No mechanism to aggregate energy costs across parent-child manifest relationships.

**Resolution Required:** Define energy accounting semantics for nested execution.

---

### US3: CNS Span Correlation for Multi-Manifest Transactions

**Issue:** CNS spans emitted per-manifest, but no correlation mechanism for multi-manifest transactions.

**Resolution Required:** Define transaction ID propagation across manifest boundaries.

---

### US4: Capability Revocation Signaling Mechanism

**Issue:** No mechanism to signal revocation to executing manifests.

**Resolution Required:** Define revocation channel (event bus, polling, or interrupt).

---

### US5: Sandbox Violation Escalation Path

**Issue:** No specification for what happens after sandbox violation is detected and logged.

**Resolution Required:** Define escalation workflow (Curator notification, human review, automatic block).

---

## Decision Tracking

| Question | Status | Decision | Date Decided | Rationale |
|----------|--------|----------|--------------|-----------|
| Q1 | OPEN | — | — | — |
| Q2 | OPEN | — | — | — |
| Q3 | OPEN | — | — | — |
| Q4 | OPEN | — | — | — |
| Q5 | OPEN | — | — | — |
| Q6 | OPEN | — | — | — |
| Q7 | OPEN | — | — | — |
| Q8 | OPEN | — | — | — |

---

## Next Steps

1. **Phase 3 Kickoff:** Review all open questions with architecture team
2. **Decision Deadlines:** Set dates for each question resolution
3. **Design Documents:** Create detailed design for each decision
4. **Implementation Plan:** Map decisions to implementation tasks
5. **Security Review:** Validate decisions against Schneier/Miller principles

---

*ℏKask v0.21.0 — Planck's Constant of Agent Systems*
*As simple as possible, but no simpler.*
*Questions documented. Decisions pending. Implementation deferred.*
