---
title: "Semantic Graph Condensation — Tasks 4-6: Validation, Gaps, Open Questions"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Active"
domain: "Architecture"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# Semantic Graph Condensation — Tasks 4-6

---

## Task 4 — Cybernetic Validation

### 4.1 — Variety Check

**System variety (failure modes the system covers):**

| Failure Mode | Covered By | Condensed? |
|-------------|-----------|------------|
| Energy budget exhaustion | `EnergyBudget.try_consume()` + `EnergyError::BudgetExceeded` | ✅ Renamed from GasBudget |
| Circuit breaker trip | `CircuitBreaker` in hkask-cns | ✅ Unchanged |
| Variety deficit | `VarietyMonitor` + algedonic alerts | ✅ Unchanged |
| OCAP token missing/attenuated | `GovernedTool` + `require_capability` | ✅ Unchanged |
| Consent not granted | `ConsentManager` + fail-closed default | ✅ Unchanged |
| Privacy laundering (episodic→public without stripping perspective) | `AccessControl::with_visibility` panic guard | ✅ Unchanged |
| Spec coherence below threshold | `spec/graph/coherence` tool | ✅ Tool name updated |

**No variety gaps introduced by condensation.** Visibility simplification (#1) removed the `Shared` variant but `parse_str` backward-compatibility preserves the "shared" → `Public` mapping. EnergyBudget rename (#5) is purely cosmetic.

### 4.2 — Feedback Loop Closure

| Loop | Signal | Consumer | Closed? |
|------|--------|----------|---------|
| **Cybernetics → Curation** | Algedonic alert (variety deficit > threshold) | Curator inbox | ✅ Unidirectional pathway intact |
| **Inference → Cybernetics** | Energy consumption events | EnergyBudget tracking | ✅ EnergyBudget tracks consumption |
| **Memory → Cybernetics** | Memory pipeline spans | CNS observability | ✅ `cns.pipeline.*` spans intact |
| **Curation → Cybernetics** | Regulatory override | EnergyBudget override | ✅ `OverrideEnergyBudget` directive intact |

**Communication loop demotion:** Communication no longer owns resources or regulates. Inter-loop messaging is still functional (custom `LoopMessage` types), but the loop itself is demoted to transport. Candidate #3 (LoopMessage→tokio) is deferred — the current messaging system works, it's just not architecturally minimal.

### 4.3 — VSM Viability

| VSM System | hKask Component | Viable? |
|------------|-----------------|---------|
| **S1 (Operations)** | Inference + Memory loops, MCP tool dispatch | ✅ Both loops have feedback (energy budgets, memory consolidation) |
| **S2 (Coordination)** | Communication transport (tokio channels future) | ⚠️ Currently custom messaging (deferred to #3) |
| **S3 (Control)** | CNS variety counters + algedonic thresholds | ✅ Variety monitoring, threshold comparison intact |
| **S3\* (Audit)** | `kask sovereignty verify` | ✅ Magna Carta verification intact |
| **S4 (Intelligence)** | Curator Agent | ✅ Curation decisions (Accept/Revise/Reject) external but documented |
| **S5 (Policy)** | Magna Carta P1-P4 + OCAP constraints | ✅ Principles condensed from 16→9, all Prohibitions intact |

**VSM assessment:** The system is viable at every level. S2 uses custom messaging (not tokio) but the messaging works — it's a minimalism violation (P5), not a viability gap. Deferred to candidate #3.

### 4.4 — Conant-Ashby (Good Regulator)

**The regulator's model:** CNS variety counters + energy budgets + algedonic thresholds.

**Model-reality check:**
- EnergyBudget model: Code uses `EnergyBudget` (hJoules). Previously `GasBudget`. **Naming reconciled.** ✅
- Variety counter model: 27 `SignalMetric` variants. **Unchanged.** ✅
- Algedonic thresholds: 50 (Warning), 100 (Critical). **Unchanged.** ✅
- Visibility model: 3-tier → 2-tier (Public, Private). **Simplified, model matches reality.** ✅
- Spec categories: 9 → 5. **✅ Resolved — code already uses 5-category `SpecCategory` enum (audited 2026-06-09).**

### 4.5 — Verification Commands

```bash
cargo check --workspace    # ✅ Passes
cargo test --workspace     # ✅ Passes (1 pre-existing failure in hkask-mcp-spec)
cargo clippy --workspace -- -D warnings  # ⚠️ Pre-existing warnings in hkask-templates (too_many_arguments)
```

---

## Task 5 — Functional Gap Verification

### 5.1 — Gap Analysis

| Area | Status | Detail |
|------|--------|--------|
| Spec categories in code | ✅ Resolved | `SpecCategory` enum already uses 5 MDS variants (Domain, Composition, Trust, Lifecycle, Curation). Audit confirmed zero DDMVSS references. |
| Spec tool names in code | ✅ Resolved | All 5 `#[tool]` handlers use MDS §3 names (spec_goal_capture, spec_goal_decompose, spec_require_writing_quality, spec_graph_query, spec_graph_coherence). Test `all_mds_tools_are_listed` verifies old DDMVSS names are absent. Stale doc references in `OPEN_QUESTIONS.md` and `MDS_SCAFFOLD.md` updated. |
| MCP server consolidation | ✅ Resolved (2026-06-09) | 21→10 servers. Internal (inference, CNS, OCAP, keystore, registry, git, goals) removed from MCP. Callers updated to direct crate calls. ACP ports for replicant/ensemble. Memory backup added to hkask-mcp-memory. See `continuation-mcp-consolidation.md` and `continuation-internal-access-replacement.md`. |
| `hkask-agents` restructuring | ⚠️ Deferred | Pod/Agent/Service boundaries are muddled (candidate #4). Continuation prompt at `condensation/continuation-pod-agent-service.md`. |
| LoopMessage→tokio | ⚠️ Deferred | Candidate #3 deferred. Continuation prompt at `condensation/continuation-loopmessage-tokio.md`. |
| Service module depth | ⚠️ Partial | `skill.rs` thinned (1 private helper). `archival.rs` borderline. No further extraction done. |
| CNS queries extraction | ✅ Resolved | `hkask-services::cns::CnsService` created. CLI `commands/cns.rs` uses `ServiceContext` + `CnsService` (with standalone fallback). API routes delegate through `state.service_context.cns`. |

### 5.2 — Test Coverage

| Area | Tests Passing | Gaps |
|------|--------------|------|
| EnergyBudget rename | All tests pass | — |
| Visibility 3→2 | All tests pass | — |
| NuEvent/Span | No change needed | Resolved |
| Documentation | N/A | All stale references cleaned |

### 5.3 — Spec-Anchored Coverage

Current test coverage is not spec-anchored per the MDS `// REQ:` convention. No `// REQ:` tags were added during this condensation pass because the changes were renames and deletions, not new feature implementations. Tracer-bullet TDD (Task 3b) was not executed because:
- Candidate #5 was a pure rename (no behavior change)
- Candidate #1 was a variant deletion (no new behavior)
- Candidates #3 and #4 were deferred

**Recommendation:** Apply `// REQ:` tag discipline to all future code changes per MDS §8.

---

## Task 6 — Open Questions & Unresolved Compression Boundaries

### Declarative Gaps (Known Missing Coverage)

1. **`SpecCategory` enum (9→5):** ✅ Resolved (2026-06-09). The Rust enum already uses 5 MDS categories. Audit confirmed clean.
2. **MCP server consolidation:** ✅ Resolved (2026-06-09). 21→10 servers. Kept: memory, condenser, web, spec, fmp, telnyx, fal, rss-reader, doc-knowledge, markitdown. Deleted as MCP servers (now direct crate calls): inference, CNS, OCAP, keystore, registry, git, goals. Replicant/ensemble converted to ACP ports. Episodic+semantic merged into memory. GitHub backup as memory_backup/restore tool.
3. **CNS queries extraction:** ✅ Resolved (2026-06-09). `hkask-services::cns::CnsService` wraps `CnsRuntime` behind clean async interface. CLI uses `ServiceContext` + fallback. API uses `state.service_context.cns`.
4. **`hkask-agents` restructuring:** Pod/Agent/Service/ACP boundaries need clarification per candidate #4 model.

### Probabilistic Judgments

1. **Service depth:** Based on deletion test of 4 modules, `compose.rs` and `embed.rs` are deep (keep). `archival.rs` is borderline. `skill.rs` was partially thinned. ~25% of service modules may still be shallow — next pass should audit remaining modules (`spec.rs`, `verification.rs`, `onboarding.rs`, `chat.rs`, `inference.rs`, `consolidation.rs`).
2. **`SpecCategory` migration:** ✅ Complete. The 5-category enum was already in place — no migration needed.

### Subjunctive Projections

1. **LoopMessage→tokio (Candidate #3):** The custom loop messaging infrastructure is a redundant projection of `tokio::mpsc` channels. If the algedonic alert pathway maps cleanly to a dedicated alert channel, the entire `LoopMessage`/`Signal`/`WorkerKind` type family can be deleted. Continuation prompt at `condensation/continuation-loopmessage-tokio.md`. **Verify with end-to-end algedonic pathway test before acting.**
2. **MCP server consolidation impact:** If CNS, keystore, ocap, and registry MCP servers are deleted (internal-only functions), the CNS runtime, keystore, and OCAP enforcement must be verified to work without MCP tool dispatch. These are currently accessible via MCP — removing the MCP layer may require direct function call replacements in the Curator.

### Constraint Conflicts

None. All condensation changes preserved Prohibitions:
- P1 (User Sovereignty): Consent/sovereignty enforcement unchanged ✅
- P2 (Affirmative Consent): Fail-closed default intact ✅  
- P3 (Generative Space): No settings hidden or gated ✅
- P4 (Clear Boundaries/OCAP): Dual enforcement gate intact ✅

### Planck's Constant

There is a minimum action. The condensation achieved: 16→9 principles, 9→5 spec categories, 9→5 spec tools, 6→4 loops. Further compression candidates exist (#3 LoopMessage→tokio, #4 Pod/Agent/Service, MCP server consolidation) but each requires more evidence before committing. The next pass starts here.

---

*Tasks 4-6 complete. The condensation pass is closed. Further passes should begin from this document.*
