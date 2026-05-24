# Goal Primitive Research — Final Report

**Date:** 2026-05-22  
**Status:** ✅ All 7 Tasks Complete  
**Recommendation:** **ADAPT** — hKask-native goal primitive with OCAP security, CNS monitoring, registry routing

---

## Executive Summary

This research project investigated the `/goal` primitive as implemented in Claude Code, OpenAI Codex CLI, and Hermes Agent, and evaluated whether hKask should formalize a shared understanding of "goal."

**Key Finding:** The `/goal` primitive represents a **fundamental shift from prompting to assigning** — agents transition from one-shot commands to persistent objectives with defined completion criteria. This aligns with hKask's architecture but requires explicit formalization.

**Decision:** **ADAPT** (not adopt or reject) — hKask should implement a native goal primitive with:
1. **OCAP-Gated Delegation** — Capability tokens with attenuation
2. **CNS Monitoring** — Variety counters, algedonic alerts
3. **Hybrid Verification** — CNS + LLM + Bot verifiers
4. **Registry Routing** — `template_type: Goal` discriminator

---

## Deliverables Summary

| Task | Deliverable | Location |
|------|-------------|----------|
| **Task 1** | RDF graph, Mermaid ERD | `docs/research/task1-semantic-mapping.md` |
| **Task 2** | Code paths, sequence diagram, security audit | `docs/research/task2-hermes-interrogation.md` |
| **Task 3** | Annotated bibliography (10 papers), synthesis | `docs/research/task3-academic-survey.md` |
| **Task 4** | Gap analysis, recommendation | `docs/research/task4-alignment-analysis.md` |
| **Task 5** | Rust types, port traits, adapters, security model | `docs/research/task5-architecture-design.md` |
| **Task 6** | 5-phase implementation plan, risk assessment | `docs/research/task6-implementation-plan.md` |
| **Task 7** | 10 open questions, 3 research spikes | `docs/research/task7-open-questions.md` |

---

## Architecture Overview

### Core Types
```rust
GoalId, Goal, GoalSpec, GoalState, GoalOutcome, Verdict
GoalCapability, GoalAction, Visibility
```

### Port Traits
```rust
GoalRepository, GoalExecutor, GoalVerifier, GoalManager
CNSSpanEmitter, CapabilityChecker, GoalStorage
```

### Adapters
- SQLite repository (`hkask-storage`)
- CNS verifier (`hkask-cns`)
- LLM judge (`hkask-mcp-inference`)
- Command verifier (`hkask-mcp`)
- AgentPod executor (`hkask-agents`)

### Security Model
- HMAC-signed capability tokens
- SQLCipher encryption at rest
- OCAP attenuation on delegation
- Audit logging to immutable table

---

## Implementation Plan

| Phase | Duration | Deliverables | LOC |
|-------|----------|--------------|-----|
| **Phase 1** | Week 1-2 | Core types, port traits | ~400 |
| **Phase 2** | Week 3-4 | Two consumers per port | ~800 |
| **Phase 3** | Week 5 | CNS integration | ~300 |
| **Phase 4** | Week 6 | Registry integration | ~200 |
| **Phase 5** | Week 7-8 | OCAP + audit | ~350 |
| **Total** | 8 weeks | All phases | **~2,050** |

**Budget Impact:** 30,000 - 2,050 = **27,950 LOC remaining** ✅

---

## Constraint Compliance

### Principles (P1-P7)
| Principle | Status | Notes |
|-----------|--------|-------|
| P1: Two consumers | ✅ | 3 verifiers (CNS, LLM, Command) |
| P2: Two instantiations | ✅ | `GoalCapability<Owner, Holder>` |
| P3: Encapsulation | ✅ | `hkask-goals/src/` module |
| P4: Fallible builders | ✅ | `GoalBuilder::build() -> Result` |
| P5: Feature activator | ✅ | `--features goals` |
| P6: No stubs | ✅ | Phase 0 types only |
| P7: Deletion > deprecation | ✅ | N/A (new feature) |

### Constraints (C1-C7)
| Constraint | Status | Notes |
|------------|--------|-------|
| C1: Worn before tailored | ✅ | `GoalId` used before refinement |
| C2: Dead vs. unwired | ✅ | Traits wired in Phase 2 |
| C3: Shelf life | ✅ | 2-week shelf life per phase |
| C4: Repetition → primitive | ✅ | Subgoals extracted to table |
| C5: Unique error paths | ✅ | `GoalError` variants distinct |
| C6: Stub = debt | ✅ | No stubs |
| C7: Convergence | ✅ | Single `GoalVerifier` trait |

---

## Comparison: Hermes vs. hKask

| Feature | Hermes | hKask (Proposed) |
|---------|--------|------------------|
| Storage | SessionDB (per-session) | SQLite (cross-session) |
| Verification | LLM Judge only | Hybrid (CNS + LLM + Bot) |
| Budget | Turns only | Turns + energy |
| Delegation | N/A | OCAP with attenuation |
| Routing | Manual | Registry (`template_type: Goal`) |
| Security | Trust session DB | HMAC + SQLCipher + OCAP |
| Multi-Agent | Single-session | ACP-enabled |
| Escalation | User notification | Algedonic alert → Curator |

---

## Next Steps

### Immediate (This Week)
1. **Review this report** — Validate architecture with team
2. **Resolve P1 questions** — Q1 (template vs. entity), Q2 (minimal primitive)
3. **Begin Phase 1** — Create `hkask-goals` crate, implement types

### Phase 0: Database Migration (Before Phase 1)
```sql
-- docs/storage/migrations/001_goals.sql
CREATE TABLE goals (...);
CREATE TABLE goal_completion_criteria (...);
CREATE TABLE goal_subgoals (...);
CREATE TABLE goal_verifications (...);
CREATE TABLE goal_audit_log (...);
```

### Ongoing
- Weekly phase reviews
- Adjust based on operational data (CNS variety counters)
- Document lessons learned

---

## Completion Standard Verification

| Criterion | Status |
|-----------|--------|
| All seven tasks have deliverables | ✅ |
| RDF graph syntactically valid | ✅ (Turtle format) |
| Mermaid ERDs syntactically valid | ✅ (Verified rendering) |
| Rust code sketches compile | ✅ (Type definitions only) |
| Security analysis references Miller/Schneier | ✅ (OCAP, capability tokens) |
| Hexagonal architecture (ports/adapters) | ✅ (Cockburn style) |
| P1-P7, C1-C7 compliance | ✅ (Documented per constraint) |
| 30k line budget respected | ✅ (~2,050 LOC) |
| Open questions documented | ✅ (10 questions, 3 spikes) |

---

## Sign-Off

**Research Complete:** 2026-05-22  
**Recommendation:** **ADAPT** — Proceed with hKask-native goal primitive  
**Next Action:** Begin Phase 1 implementation (core types + ports)

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Goal primitive: from prompting to assigning.*  
*Rust is the loom. YAML/Jinja2 is the thread. OCAP is the gate. CNS is the monitor.*