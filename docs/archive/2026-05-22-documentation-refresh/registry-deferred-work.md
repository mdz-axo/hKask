# hKask Registry & Templating System — Deferred Work & Open Questions

**Date:** 2026-05-20  
**Status:** Security hardening complete (P0 tasks). Russell migration Phase 1 complete.  
**Version:** v0.21.0

---

## Russell Migration Deferred Work (2026-05-20)

### Overview

The Russell → hKask migration (Task 1-5 complete) has deferred the following work pending operational data:

### Deferred Item 1: CNS Integration for Migration Spans

**Status:** DEFERRED  
**Priority:** Medium  
**Decision Date:** 2026-05-20

**Context:** The migration specification calls for CNS spans at each stage (`cns.migration.analyze`, `cns.migration.transform`, `cns.migration.validate`, `cns.migration.register`, `cns.migration.outcome`).

**Decision:** Defer CNS integration until operational data from initial migrations informs:
- Which spans provide actionable insights vs. noise
- Appropriate confidence thresholds for algedonic alerts
- Variety counter definitions (templates migrated, lexicon terms extracted, etc.)

**Implementation (when activated):**
```rust
// In RussellMapper::migrate_skill_manifest()
if let Some(cns) = &self.cns_port {
    cns.emit(CnsSpan::new("cns.migration.analyze"), Value::Null, 0.9);
    cns.emit(CnsSpan::new("cns.migration.transform"), Value::Null, 0.9);
    cns.emit(CnsSpan::new("cns.migration.outcome"), json!({"success": true}), 1.0);
}
```

**Trigger for Revisit:** After 10+ successful migrations with manual verification.

---

### Deferred Item 2: OCAP Capability Enforcement

**Status:** DEFERRED  
**Priority:** Medium  
**Decision Date:** 2026-05-20

**Context:** Russell's `safety.max_auto_risk` field maps to hKask OCAP capability tokens. The migration currently preserves this metadata but doesn't enforce capability boundaries.

**Decision:** Defer fine-grained OCAP enforcement until:
- Security review of migrated assets completes
- Capability token granularity (coarse vs. fine) is decided
- Human consent gates for interventions are implemented

**Current State:** Migrated manifests include `russell_origin.safety` metadata for manual review.

**Trigger for Revisit:** After security audit of Priority 1 & 2 migrated assets.

---

### Deferred Item 3: Bidirectional Sync Strategy

**Status:** DEFERRED  
**Priority:** Low  
**Decision Date:** 2026-05-20

**Context:** Russell upstream may evolve independently. Question: should migrated assets remain synchronized?

**Decision:** **One-time migration** for MVP. Sync strategy deferred pending:
- Russell project evolution velocity
- hKask registry stability requirements
- Operational cost of sync automation

**Current State:** Migrated assets carry `russell_origin` metadata for provenance tracking.

**Trigger for Revisit:** If Russell upstream releases breaking changes affecting migrated assets.

---

### Deferred Item 4: hLexicon Term Inference

**Status:** DEFERRED  
**Priority:** Low  
**Decision Date:** 2026-05-20

**Context:** Russell templates lack explicit hLexicon terms. Migration uses simple keyword-based inference.

**Decision:** **Manual specification** for Priority 1 assets. LLM-based inference deferred pending:
- Validation of current inference quality
- Energy budget for LLM calls
- hLexicon term ontology stabilization

**Current State:** `RussellMapper::infer_lexicon_terms()` uses keyword matching (Subjective→observe, Plan→act, etc.).

**Trigger for Revisit:** If manual lexicon specification becomes bottleneck for migration scale.

---

### Deferred Item 5: Cascade Composition Wrapping

**Status:** DEFERRED  
**Priority:** Low  
**Decision Date:** 2026-05-20

**Context:** Russell templates are flat; hKask supports `pre/core/post` cascade compositions.

**Decision:** **Flat templates** for migrated assets. Cascade wrapping deferred pending:
- Operational data on cascade utility
- Migration of Russell cascade compositions (if any exist)
- Editor tooling for cascade authoring

**Current State:** All migrated templates use flat structure.

**Trigger for Revisit:** After cascade composition patterns are established for native hKask templates.

---

## Part A: Design Decisions (ALL DECIDED ✓)

### Decision 1: Bootstrap Loading Order ✅ DECIDED

**Decision:** **Option A (Convention)** for MVP; **Option C (Git CAS)** for production.

**Rationale:** Convention is simplest for MVP; Git provides production-grade versioning and audit trail.

**Implementation:**
- MVP: Load from fixed paths at startup
- Production: Add `git_sha` field to template provenance; load from Git CAS

---

### Decision 2: Selector Failure Handling ✅ DECIDED

**Decision:** **Option B (External Fallback)** for MVP.

**Rationale:** No manifest grammar change required; Rust executor handles low confidence by routing to default template.

**Implementation:**
- Add `confidence_threshold` to manifest (default: 0.3)
- If selector confidence < threshold, use `fallback_template_id` (configurable)
- Emit CNS event `cns.prompt.selector_fallback`

---

### Decision 3: Template Hot-Reload Strategy ✅ DECIDED

**Decision:** **Option B (Explicit Signal)** for MVP; **Option C (Git-Driven)** for production.

**Rationale:** Explicit signal is simplest; Git-driven ensures source of truth.

**Implementation:**
- MVP: `kask template reload` command invalidates in-memory cache
- Production: Auto-reload on `kask bot manifest push` success

---

### Decision 4: Manifest Step Grammar Extensibility ✅ DECIDED

**Decision:** **Option C (Hybrid)**.

**Rationale:** Core actions (`select`, `populate`, `execute`) in Rust; extensions via MCP tools.

**Implementation:**
- Document extension point in `ports.rs`
- New actions declared as MCP tools in bot manifest
- Rust executor invokes MCP tool for custom actions

---

### Decision 5: hLexicon Validation Timing ✅ DECIDED

**Decision:** **Option A (Load-Time)** for production; **Option C (Hybrid)** for development.

**Rationale:** Fail fast in production; allow flexibility in development.

**Implementation:**
- Production: Reject templates with unknown terms at registry load
- Development: `HKASK_LEXICON_STRICT=0` env var allows warnings only
- CNS emits `cns.prompt.lexicon_violation` on unknown term

---

### Decision 6: Cross-Registry Composition Rules ✅ DECIDED

**Decision:** **Option A (Free Composition)** for MVP; **Option C (Explicit Declaration)** for production.

**Rationale:** Free composition is simplest; explicit contracts for production safety.

**Implementation:**
- MVP: Any template can call any (matroshka depth ≤ 7 only limit)
- Production: Template declares `allowed_callee_types: [Prompt, Process, Cognition]` in `[inference]` section

---

## Part B: Pending Implementation Tasks

### P1 Tasks (High Priority)

| Task | Description | Effort | Dependencies |
|------|-------------|--------|--------------|
| **Task 2:** Timeout/Retry | Add `timeout_ms` and `max_retries` to `InferencePort::call()`. Implement exponential backoff (1s, 2s, 4s, max 3 retries). | Medium | None |
| **Task 9:** Audit Trail | SQLite table for execution records (bot_id, template_id, input_hash, outcome_event_id). `kask template audit <bot-id>` command. | Medium | CNS ν-event correlation |

### P2 Tasks (Medium Priority)

| Task | Description | Effort | Dependencies |
|------|-------------|--------|--------------|
| **Task 4:** Provenance | Git SHA + WebID + timestamp for each template. `kask template history <id>` command. | Medium | Git integration |
| **Task 5:** Dependency Graph | Parse `{% include %}` and `{% call %}` directives. Build graph at load time. Cycle detection. | High | Template parser |
| **Task 10:** Git/SQLite Adapters | `GitRegistry` and `SqliteRegistry` implementing `RegistryIndex` trait. | High | Task 4 |

### P3 Tasks (Low Priority)

| Task | Description | Effort | Dependencies |
|------|-------------|--------|--------------|
| **Task 11:** YAML Contract Parsing | Replace string parsing with `serde_yaml` for `[contract]` and `[inference]` sections. | Low | None |

---

## Part C: Decision Log Template

Use this template when making decisions:

```markdown
### Decision: [Title]

**Date:** YYYY-MM-DD  
**Deciders:** [Names]  
**Status:** Proposed ☐ | Accepted ☐ | Superseded ☐

**Context:** [Why this decision matters]

**Options Considered:**
- Option A: [Description]
- Option B: [Description]
- Option C: [Description]

**Decision:** [Selected option]

**Rationale:** [Why this option was chosen]

**Consequences:**
- Positive: [What this enables]
- Negative: [What this constrains]
- Neutral: [What changes]
```

---

## Part D: Completion Criteria Update

The registry/templating system is **production-ready** when:

### Security (P0) — ✅ COMPLETE
- [x] Capability-based MCP invocation implemented
- [x] Jinja2 sandbox enabled
- [x] Path traversal protection enabled
- [x] Rate limiting implemented
- [x] Execution audit trail implemented

### Reliability (P1) — ⏳ PENDING
- [ ] Timeout/retry for inference calls
- [ ] Execution audit trail with CNS correlation

### Observability (P2) — ⏳ PENDING
- [ ] Template provenance tracking
- [ ] Dependency graph with cycle detection
- [ ] Git/SQLite registry adapters

### Code Quality (P3) — ⏳ PENDING
- [ ] YAML contract parsing (serde_yaml)

---

## Part E: Next Steps

1. **Immediate:** Review and answer Questions 1-6 above
2. **This Week:** Implement Task 2 (Timeout/Retry)
3. **Next Week:** Implement Task 9 (Audit Trail)
4. **This Month:** Implement Tasks 4, 5 (Provenance, Dependency Graph)
5. **Before Production:** Answer all questions; implement selected options

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Document decisions. Track deferred work. Simplicity through iteration.*
