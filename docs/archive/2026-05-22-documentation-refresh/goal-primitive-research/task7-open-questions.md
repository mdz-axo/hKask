# Task 7: Open Questions and Underspecified Areas

## 7.1 Prioritized Open Questions

### Priority 1: Architectural Decisions (Resolve Before Phase 1)

#### Q1: Should `goal` be a template type, a first-class entity, or both?

**Options:**
| Option | Pros | Cons |
|--------|------|------|
| **Template type only** | Simple, fits existing registry design | Cannot track goal state across sessions |
| **First-class entity only** | Full lifecycle tracking, audit trail | Duplicates registry functionality |
| **Both** (Recommended) | Best of both: routing + persistence | Increased complexity (~200 LOC) |

**Recommendation:** **Both** — `template_type: Goal` for routing, `goals` table for persistence.

**Rationale:**
- Registry handles template selection and rendering
- Database handles lifecycle, state transitions, audit
- Separation aligns with hexagonal architecture (templates = inbound, DB = outbound)

**Resolution:** Document in `hkask-goals/src/lib.rs` module-level docs.

---

#### Q2: What is the minimal viable `goal` primitive satisfying P1-P7?

**P1-P7 Constraints:**
- **P1:** Two consumers per trait → Need 2+ verifiers, 2+ executors
- **P2:** Two generic instantiations → `GoalCapability<Owner, Holder>`
- **P4:** Fallible builders → `GoalBuilder::build() -> Result<...>`
- **P6:** No stubs → Phase 0 types only, no incomplete implementations

**Minimal Viable Set:**
```rust
// Types (required)
- GoalId
- GoalState (enum)
- GoalSpec
- GoalCapability

// Ports (required)
- GoalRepository (2 consumers: SQLite + in-memory)
- GoalVerifier (2 consumers: CNS + LLM)

// NOT in MVP (defer to v0.22.0)
- Goal delegation (OCAP attenuation)
- Subgoals table
- Energy budget (turns only)
```

**Resolution:** MVP = types + 2 consumers per port. Delegation/subgoals/energy = Phase 2+.

---

### Priority 2: Security Design (Resolve Before Phase 3)

#### Q3: How do we prevent goal hijacking or goal injection attacks?

**Attack Vectors:**
1. **Unauthorized goal creation** — Attacker creates goal for victim
2. **Goal modification** — Attacker changes goal completion criteria
3. **Capability theft** — Attacker steals capability token

**Mitigation Strategies:**
| Vector | Mitigation | Implementation |
|--------|------------|----------------|
| Unauthorized creation | OCAP capability check | `CapabilityChecker::check()` before `create()` |
| Modification | HMAC-signed state | `GoalCapability::verify()` on load |
| Theft | Short expiration + attenuation | `expiration: i64`, `attenuation_level: u8` |

**Open Question:** Should goals be encrypted at rest beyond SQLCipher?

**Recommendation:** SQLCipher is sufficient (field-level encryption adds complexity without proportional security gain).

**Resolution:** Document threat model in `docs/architecture/security-architecture.md`.

---

#### Q4: Should goals be encrypted at rest (SQLCipher) and in transit?

**Current Design:**
- **At rest:** SQLCipher (AES-256-CBC, 256k KDF iterations)
- **In transit:** ACP messages (TLS for network, Unix socket for local)

**Additional Encryption Options:**
- **Field-level encryption:** Encrypt `goal_text`, `completion_criteria` separately
- **Envelope encryption:** Per-goal keys encrypted by master key

**Analysis:**
- SQLCipher already encrypts entire database
- Field-level encryption adds ~100 LOC, minimal security gain
- Envelope encryption adds ~200 LOC, enables per-goal access control

**Recommendation:** SQLCipher only for MVP. Envelope encryption for v0.22.0 if multi-tenant deployment required.

**Resolution:** Document in `docs/architecture/security-architecture.md`.

---

### Priority 3: CNS Integration (Resolve Before Phase 3)

#### Q5: How does `goal` interact with hKask's condensation/summarization pipeline?

**Condensation Pipeline:**
```
Episodic Memory → Condenser → Summary → Semantic Memory
```

**Interaction Points:**
1. **Goal completion** → Condense outcome to summary
2. **Goal failure** → Condense lessons learned
3. **Goal delegation** → Condense handoff record

**Open Question:** Should goal outcomes be condensed automatically?

**Options:**
| Option | Pros | Cons |
|--------|------|------|
| **Auto-condense on completion** | Captures lessons, reduces storage | May lose detail |
| **Manual condensation** | User controls what's summarized | Requires user action |
| **Hybrid** (Recommended) | Auto-condense metadata, user condenses text | Moderate complexity |

**Recommendation:** **Hybrid** — Auto-condense `GoalOutcome` metadata; user triggers text condensation via `kask goal condense <id>`.

**Resolution:** Implement in `hkask-mcp-condenser` (Phase 2+).

---

#### Q6: How does `goal` relate to the algedonic alert system (variety deficit detection)?

**Current Design:**
- Variety deficit >100 → `AlgedonicAlert::VarietyDeficit`
- Alert → Curator → Human escalation

**Goal-Specific Alerts:**
```rust
pub enum AlgedonicAlert {
    VarietyDeficit {
        goal_id: GoalId,
        deficit: usize,
        environmental_states: usize,
        internal_states: usize,
    },
    GoalBudgetExhausted {
        goal_id: GoalId,
        turns_used: u32,
        max_turns: u32,
    },
    GoalVerificationFailed {
        goal_id: GoalId,
        consecutive_failures: u32,
    },
}
```

**Open Question:** Should variety deficit trigger automatic goal pause?

**Recommendation:** **No** — Variety deficit triggers alert to Curator, but goal continues unless Curator pauses. Rationale: Some complex goals legitimately exceed variety threshold.

**Resolution:** Implement in `hkask-cns/src/goal_variety.rs`.

---

#### Q7: How does goal auditing integrate with CNS `cns.agent_pod.*` spans?

**Span Hierarchy:**
```
cns.goal.create
  └─ cns.agent_pod.activated (pod spawned for goal)
      └─ cns.tool.* (tool calls during execution)
          └─ cns.goal.verify (verification after turn)
              └─ cns.goal.complete (if done)
```

**Open Question:** Should goal spans link to agent pod spans?

**Recommendation:** **Yes** — `NuEvent` includes `parent_span_id` for tracing.

**Implementation:**
```rust
pub struct NuEvent {
    pub id: NuEventId,
    pub span: Span,
    pub parent_span_id: Option<NuEventId>,  // Link to parent
    pub goal_id: Option<GoalId>,            // Link to goal
    // ... other fields ...
}
```

**Resolution:** Add `parent_span_id` and `goal_id` fields to `NuEvent` (Phase 3).

---

### Priority 4: Goal Composition (Resolve Before Phase 4)

#### Q8: Should goals be composable (goal-of-goals)? What are the security implications?

**Composition Patterns:**
1. **Sequential:** Goal A → Goal B (B starts after A completes)
2. **Parallel:** Goal A ∥ Goal B (concurrent execution)
3. **Hierarchical:** Goal A contains subgoals B, C, D

**Security Implications:**
| Pattern | Risk | Mitigation |
|---------|------|------------|
| Sequential | Capability expiration mid-chain | Extend expiration on composition |
| Parallel | Capability conflicts (same resource) | Resource locking per capability |
| Hierarchical | Attenuation compounds | Track composition depth |

**Recommendation:** **Hierarchical only** for MVP (via `goal_subgoals` table). Sequential/parallel = v0.22.0.

**Resolution:** Document in `docs/architecture/goal-composition.md`.

---

#### Q9: Should there be a distinction between declarative, procedural, and constraint goals?

**Goal Types (from BDI literature):**
| Type | Description | Example |
|------|-------------|---------|
| **Declarative** | Desired state (belief-independent) | `¬believes(goal_achieved)` |
| **Procedural** | Task completion (plan-triggered) | `respond_to_event(e)` |
| **Constraint** | Never-cross boundaries | `¬exceed_budget(100)` |

**hKask Adaptation:**
```rust
#[serde(tag = "goal_type", rename_all = "snake_case")]
pub enum GoalSpec {
    Declarative {
        desired_state: String,
        verification: Vec<CompletionCriterion>,
    },
    Procedural {
        task: String,
        manifest_ref: String,
    },
    Constraint {
        invariant: String,
        violation_action: ViolationAction,
    },
}
```

**Recommendation:** **Single type** for MVP with `completion_criteria` covering all three patterns. Refactor to enum if divergence observed in usage.

**Resolution:** Start with unified `GoalSpec`, refactor if needed (Phase 4+).

---

### Priority 5: Termination and Error Handling (Resolve Before Phase 2)

#### Q10: What is the termination condition for a goal that cannot be satisfied?

**Termination Conditions:**
1. **Budget exhaustion** — `turns_used >= max_turns` → `Paused`
2. **Verification failure** — `consecutive_failures >= 3` → `Blocked`
3. **Variety deficit** — `deficit > 100` → Alert (no auto-pause)
4. **User cancellation** — `kask goal cancel <id>` → `Cleared`
5. **Capability expiration** — `expiration < now` → `Blocked`

**Open Question:** Should unrecoverable failures auto-block or require user confirmation?

**Recommendation:** **Auto-block** after 3 consecutive verification failures. User can resume with `kask goal resume <id>`.

**Resolution:** Implement in `GoalManager::evaluate_after_turn()` (Phase 2).

---

## 7.2 Research Spikes for Next Iteration

### Spike 1: Formal Verification of Goal Workflows

**Question:** Can goal workflows be formally verified (OCL, Prism)?

**Approach:**
1. Model goal lifecycle in OCL
2. Verify invariants (e.g., "goal cannot complete without verification")
3. Integrate with FALAA framework

**Timeline:** 1 week (post-Phase 5)

**Deliverable:** `docs/architecture/goal-formal-spec.md`

---

### Spike 2: Goal Learning from Experience

**Question:** Can agents learn goal templates from experience?

**Approach:**
1. Track successful goal patterns
2. Cluster by completion criteria
3. Generate new templates via LLM

**Timeline:** 2 weeks (post-Phase 5)

**Deliverable:** `hkask-mcp-condenser/src/goal_learning.rs`

---

### Spike 3: Cross-Agent Goal Protocols

**Question:** How do goals span multiple agent pods?

**Approach:**
1. Model goal delegation as ACP message protocol
2. Define handoff semantics (capability transfer)
3. Implement multi-agent goal tracking

**Timeline:** 2 weeks (post-Phase 5)

**Deliverable:** `docs/architecture/goal-protocol.md`

---

## 7.3 Summary Table

| Question | Priority | Resolution | Phase |
|----------|----------|------------|-------|
| Q1: Template vs. entity | P1 | Both | Phase 1 |
| Q2: Minimal viable primitive | P1 | Types + 2 consumers | Phase 1 |
| Q3: Goal hijacking prevention | P2 | OCAP + HMAC | Phase 3 |
| Q4: Encryption at rest | P2 | SQLCipher only | Phase 3 |
| Q5: Condensation pipeline | P3 | Hybrid auto/manual | Phase 3+ |
| Q6: Algedonic alerts | P3 | Alert, no auto-pause | Phase 3 |
| Q7: Span integration | P3 | Parent span links | Phase 3 |
| Q8: Goal composition | P4 | Hierarchical only | Phase 4+ |
| Q9: Goal type distinction | P4 | Unified for MVP | Phase 4+ |
| Q10: Termination conditions | P5 | Auto-block on failures | Phase 2 |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Task 7 Complete: 10 open questions prioritized with resolutions and 3 research spikes identified.*