# hKask 7-Loop Architecture — Implementation Handoff v2

## Purpose

This document is a self-contained continuation prompt for an agent executing the hKask loop architecture implementation. It contains all architectural decisions, the complete revised implementation plan, and the context needed to start work without access to prior conversation.

**Status:** Phases 0–8 are COMPLETE. Next: Phase 9 (Cybernetic Unit Tests).

---

## Project Context

**hKask** (ℏKask) is a minimal viable agent container — an OCAP-first Rust codebase (~40K LOC, 11 core crates, 16 MCP servers) that runs bots and replicants with capability discipline, memory, and cybernetic observability.

**Repository:** `/home/mdz-axolotl/Clones/hKask`
**Key commands:** `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`
**Architecture doc:** `docs/architecture/hKask-architecture-master.md`
**Project rules:** `AGENTS.md` (at repo root)

---

## Audit Documents (read these first)

All audit documents are in `hKask/docs/audit/`:

| Document | Content |
|----------|---------|
| `TASK1-semantic-audit.md` | 480 abstractions mapped to 4 root purposes, 7 collapse/fold/delete candidates |
| `TASK2-core-loops.md` | Original 5-loop model (SUPERSEDED by TASK9 — read TASK9 instead) |
| `TASK3-composition-graph.md` | ER diagram, exact types crossing boundaries, 4 attenuation candidates, 4 composition gaps |
| `TASK4-simplify-changelog.md` | 7 collapses, 15 folds, 6 deletes with verification matrix |
| `TASK5-simplified-core.md` | Rust module design for 5 loops, 11 capability handles (SUPERSEDED — handles expanded to 13 in TASK9) |
| `TASK6-verify.md` | 22 cybernetic unit tests (EXPAND — now 30+ tests per TASK9) |
| `TASK7-open-questions.md` | 9 open questions (4 resolved, 1 bug fix, 4 implementation) |
| **`TASK9-loop-structure.md`** | **AUTHORITATIVE** — 7 loops, 9 control primitives, 30 subloops, 5 messenger functions, primitive×loop completeness matrix, episodic/semantic split, communication master loop |
| `TASK8-implementation-plan.md` | Revised implementation plan (10 phases, 53 PRs, ~5–6 weeks) |

---

## The 7-Loop Model (Authoritative)

hKask has **7 loops**: 5 domain loops + 2 master loops. Memory is split into Episodic (2a) and Semantic (2b) connected by a Consolidation Bridge.

```
Domain Loops:
  1. Inference        — prompt → context → model → response → parse → act
  2a. Episodic Memory — experience → encode → store (private) → recall → temporal attention → context
  2b. Semantic Memory  — knowledge → store (public) → index → recall → dedup → combine → context
  3. Governance        — request → authorize → dispatch → observe → adapt policy
  4. Observability     — emit span → aggregate → detect anomaly → escalate

Master Loops:
  5. Curation          — observe → evaluate → compose → regulate (regulator — reads all, writes policy)
  6. Communication     — send → observe delivery → detect congestion → dampen → confirm (connector — enables all)

Bridge:
  2a→2b. Consolidation — episodic → strip perspective → dedup → store semantic (one-way transformation)
```

### 9 Control Primitives

Every subloop is a domain-specific instance of one of these 9 abstract patterns:

| # | Primitive | Pattern |
|---|-----------|---------|
| 1 | **GUARD** | `request → check condition → allow or deny` |
| 2 | **FILTER** | `stream → remove undesired → pass through` |
| 3 | **CACHE** | `request → hit? → return / miss → compute + store` |
| 4 | **CIRCUIT** | `call → fail → count → threshold → open → half-open → probe → close` |
| 5 | **RECONCILE** | `conflict A, conflict B → combine → resolved` |
| 6 | **SENSE** | `state → measure → signal` |
| 7 | **ROUTE** | `signal → classify → deliver to consumer` |
| 8 | **WITHDRAW** | `grant → revoke → persist → deny future` |
| 9 | **ADAPT** | `outcome → compare to desired → adjust parameter` |

### 30 Subloops by Loop

**Loop 1 — Inference (5 subloops):**
1.1 Context Assembly (FILTER) | 1.2 Prompt Cache (CACHE) | 1.3 Circuit Breaker (CIRCUIT) | 1.4 Energy Budget (GUARD) | 1.5 Rate Limiting (GUARD)

**Loop 2a — Episodic Memory (6 subloops):**
2a.1 Experience Encoding (FILTER) | 2a.2 Temporal Attention (ADAPT) | 2a.3 Confidence Decay (RECONCILE) | 2a.4 Confidence Retraction (RECONCILE) | 2a.5 Episodic Storage Budget (GUARD) | 2a.6 Episodic Context Assembly (FILTER+ADAPT)

**Loop 2b — Semantic Memory (5 subloops):**
2b.1 Semantic Deduplication (FILTER) | 2b.2 Confidence Combination (RECONCILE) | 2b.3 Semantic Indexing (CACHE) | 2b.4 Semantic Storage Budget (GUARD) | 2b.5 Semantic Context Assembly (FILTER)

**Consolidation Bridge (4 functions):**
B.1 Consolidation Priority (DISPATCH) | B.2 Perspective Stripping (FILTER) | B.3 Consolidation Dedup (FILTER) | B.4 Confidence Promotion (RECONCILE)

**Loop 3 — Governance (3 subloops):**
3.1 Revocation (WITHDRAW) | 3.2 Sovereignty Checking (GUARD) | 3.3 Goal State Machine (RECONCILE)

**Loop 4 — Observability (4 subloops):**
4.1 Variety Tracking (SENSE) | 4.2 Algedonic Alert Generation (ROUTE) | 4.3 Bot Metrics Collection (SENSE) | 4.4 Sovereignty Observation (SENSE)

**Loop 5 — Curation (3 subloops):**
5.1 Escalation Routing (ROUTE) | 5.2 Bot Evaluation / Kata Coaching (ADAPT) | 5.3 Threshold Calibration (ADAPT)

**Loop 6 — Communication (5 messenger functions, not subloops):**
6.1 DISPATCH (GUARD+ROUTE) | 6.2 CORRELATE (SENSE) | 6.3 DAMPEN (FILTER+RECONCILE) | 6.4 Channel CIRCUIT (CIRCUIT) | 6.5 ACKNOWLEDGE (VALIDATE+ROUTE)

### Key Structural Insights

1. **Subloops are domain-specific instances of control primitives.** The primitive is the pattern; the subloop is the instantiation.
2. **Communication has no subloops** because all subloops ARE communication pattern instances. Communication delivers messenger functions that sit on every inter-loop edge.
3. **Memory is a paired domain loop.** Episodic and Semantic share an origin (experience) but immediately diverge. They are connected by a one-way bridge (Consolidation).
4. **The episodic loop now closes.** Phases 5–6 wired all subloops: experience encoding, confidence decay, temporal attention, confidence retraction, storage budget, context assembly (episodic); dedup, confidence combination, indexing, storage budget, consolidation with confidence promotion (semantic).
5. **The Curation loop now closes.** Phase 7 wired the metacognition loop to `CuratorContext`, directive delivery through `MessageDispatch`, threshold calibration through `CnsGovernWriteHandle::calibrate_threshold()`, and DAMPEN on feedback edges.
6. **The Communication messenger functions are now wired.** DISPATCH (6.1) via `MessageDispatch`, DAMPEN (6.3) via `Dampener`. CORRELATE (6.2), CIRCUIT (6.4), and ACKNOWLEDGE (6.5) are not yet implemented but the types exist in `hkask-types/src/loops/dispatch.rs`.

---

## Capability Handle Architecture

13 capability handles enforce OCAP discipline. **Updated from 11 to 13** — `MemoryReadHandle`/`MemoryWriteHandle` split into Episodic/Semantic pairs:

| Handle | Loop | Can | Cannot |
|--------|------|-----|--------|
| `InferenceHandle` | Inference | Infer, read episodic + semantic memory, emit spans, check cache, circuit-break, rate-limit | Write memory, reset alerts, process sovereignty, revoke capabilities |
| `EnergyBudgetHandle` | Inference | Check remaining budget, request consumption, get usage ratio | Set the cap, reset the budget, change alert threshold |
| `RateLimiterHandle` | Inference | Check token bucket, consume invocation slot | Resize bucket, change refill rate, bypass limiting |
| `EpisodicReadHandle` | Episodic | Query visible episodic triples for own perspective, assemble episodic context | Store triples, access other agents' episodic memories, query by similarity |
| `EpisodicWriteHandle` | Episodic | Store episodic triples (own WebID only) | Delete triples, write on behalf of other agents, write semantic triples |
| `SemanticReadHandle` | Semantic | Query semantic triples by entity, query by similarity, assemble semantic context | Store triples, delete triples, access episodic memories |
| `SemanticWriteHandle` | Semantic | Store semantic triples (with consolidation capability), store embeddings | Delete triples, access episodic memories, write on behalf of other agents |
| `GovernanceHandle` | Governance | Verify/attenuate/revoke tokens, check visibility, process alerts, calibrate thresholds | Emit arbitrary spans, store triples, run inference |
| `CnsWriteHandle` | Observability | Emit spans, increment variety counters | Reset alerts, subscribe, process sovereignty events |
| `CnsGovernReadHandle` | Observability | Check variety, process sovereignty events (read-only) | Set expected variety, calibrate thresholds, emit spans |
| `CnsGovernWriteHandle` | Observability+Curation | Set expected variety, calibrate thresholds (read + write) | Emit spans, reset alerts, subscribe |
| `CnsAdminHandle` | Observability | Reset alerts, clear old alerts, subscribe listeners | Emit spans, check variety |
| `CuratorHandle` | Curation | Read all loop state, write governance/observability policy, issue directives | Run inference, emit spans directly, access private episodic triples |

---

## Resolved Design Decisions

These questions were debated and resolved during the audit. **Do not revisit them.**

| # | Decision | Rationale |
|---|----------|-----------|
| Q4 | **Keep micro-governance in inference loop** — `verify_capability` inside `dispatch_action` is essential | OCAP must be enforced at the point of use. TOCTOU window otherwise. |
| Q5 | **Hard cap of 5 `template_type` variants** | Each variant adds linear cost to every registry operation. |
| Q8 | **Keep Bot/Replicant as distinct `template_type` variants** | `template_type` drives branching logic across 4 crates. |
| Q9 | **Split CnsGovernHandle** into Read (Governance) and Write (Curation) | Governance enforces policy (reads). Curation sets policy (writes). Type system enforces. |
| D1 | **Memory is a paired domain loop** (Episodic + Semantic) | They share an origin but immediately diverge. Different subloops, different sovereignty, different growth patterns, different confidence directions. |
| D2 | **Communication is a master loop** (not a 6th domain loop) | It has no subloops because all subloops are communication pattern instances. It delivers messenger functions on inter-loop edges. |
| D3 | **Consolidation is an inter-loop bridge** (not a Memory subloop) | It transforms private experience into shared knowledge — a one-way transformation that strips perspective. It sits on the edge between 2a and 2b. |
| D4 | **Episodic loop must close first** | It currently doesn't close at all. Priority over semantic indexing, communication infrastructure, and other gaps. |
| Alt | **Reject OODA as inference framework** | LLM-specific stages map 1:1 to Rust functions. OODA is a metaphor, not a specification. |
| Alt | **Reject CNS as cross-cutting** | CNS has its own closed feedback cycle. It's a loop. |

---

## Implementation Plan (TASK8 v2 — Authoritative)

**10 phases, 53 PRs, ~5–6 weeks. Phases 0–7 COMPLETE. Next: Phase 8.**

### Phase 0: COMPLETE ✓

| PR | Title | Status |
|---|-------|--------|
| 0a | Fix admin passphrase timing attack | ✓ Done |
| 0b | Wire or remove dead session fields | ✓ Done |
| 0c | Prune dead code | ✓ Done |

### Phase 1: Type Foundation — COMPLETE ✓

Define all handle types as struct definitions in `hkask-types/src/loops/`. **Additive only — no existing code changes.**

| PR | Title | What | Affected Crates |
|---|-------|------|-----------------|
| 1a | Define loop module structure | Create `hkask-types/src/loops/mod.rs` re-exporting 7 loop modules. Create stub files for `inference.rs`, `episodic.rs`, `semantic.rs`, `governance.rs`, `observability.rs`, `curation.rs`, `dispatch.rs`. | `hkask-types` |
| 1b | Define capability handle types | Implement all 13 handle structs with Hoare-triple-annotated methods. Include `new_test()` stubs. Handles: `InferenceHandle`, `EnergyBudgetHandle`, `RateLimiterHandle`, `EpisodicReadHandle`, `EpisodicWriteHandle`, `SemanticReadHandle`, `SemanticWriteHandle`, `GovernanceHandle`, `CuratorHandle`, `CnsWriteHandle`, `CnsGovernReadHandle`, `CnsGovernWriteHandle`, `CnsAdminHandle`. | `hkask-types` |
| 1c | Define DataCategory visibility enum | `DataCategory { Public, Shared, EpisodicMemory, SemanticMemory, Private, PersonalContext, CapabilityTokens, OcpBoundaries, TemplateInvocations, HlexiconTerms, TemplateRegistry }` with HKDF key derivation mapping. | `hkask-types` |
| 1d | Define Communication types | `LoopMessage`, `MessagePriority { Critical, Warning, Info }`, `LoopOrigin`, `LoopPayload`, `TraceId` in `hkask-types/src/loops/dispatch.rs`. Stub only. | `hkask-types` |

**Verification:** `cargo check -p hkask-types && cargo test -p hkask-types && cargo clippy -p hkask-types -- -D warnings`

### Phase 2: CnsGovernHandle Split + Memory Handle Split — COMPLETE ✓

Two most invasive refactors.

| PR | Title | What | Affected Crates |
|---|-------|------|-----------------|
| 2a | Split CnsRuntime into four handles | Replace `CnsRuntime` with `CnsWriteHandle`, `CnsGovernReadHandle`, `CnsGovernWriteHandle`, `CnsAdminHandle`. Migrate all consumers. | `hkask-cns`, `hkask-agents`, `hkask-ensemble`, `hkask-templates`, `hkask-mcp` |
| 2b | Wire Governance to CnsGovernReadHandle | `GovernanceHandle.cns` → `CnsGovernReadHandle`. Read-only. | `hkask-agents` |
| 2c | Wire Curation to CnsGovernWriteHandle | `MetacognitionLoop` → `CnsGovernWriteHandle` for calibration + `CuratorHandle` for cross-loop writes. | `hkask-agents` |
| 2d | Split Memory handles into Episodic/Semantic | Replace `MemoryReadHandle`/`MemoryWriteHandle` with `EpisodicReadHandle` + `EpisodicWriteHandle` + `SemanticReadHandle` + `SemanticWriteHandle`. Migrate `PodContext`. `InferenceHandle.memory` → `InferenceHandle.episodic` + `InferenceHandle.semantic`. | `hkask-types`, `hkask-agents`, `hkask-memory` |

**Verification:** `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`

### Phase 3: Collapse, Fold, Delete — COMPLETE ✓

| PR | Title | What | Lines |
|---|-------|------|:-----:|
| 3a | Collapse CnsSpan + Span | Unify into `Span { category: SpanCategory, path: String }`. Delete `CnsSpan`. | ~−300 |
| 3b | Collapse BotCapabilities + ReplicantCapabilities | Unify into `AgentCapabilities` with `MemoryAccess { can_access_episodic, can_access_semantic }`. | ~−100 |
| 3c | Collapse remaining pairs (C3–C7) | ContractValidator, OkapiHttpClient, SystemHealthSnapshot, EnsembleChatManager, AgentPersona. | ~−400 |
| 3d | Fold delegation wrappers (F1–F15) | Inline 15 wrappers. `BayesianOps::new()` → free functions. | ~−250 |
| 3e | Split ContextAssembler into episodic + semantic | `assemble_episodic_context()` (temporal-ordered, recency-weighted) + `assemble_semantic_context()` (deduplicated, confidence-combined). | ~+150/−50 |

**Verification:** `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`

### Phase 4: Contract Tightening — COMPLETE ✓

| PR | Title | What | Affected Crates |
|---|-------|------|-----------------|
| 4a | Make CapabilityToken depth const | `const MAX_ATTENUATION_DEPTH: u32 = 7` | `hkask-types`, `hkask-agents` |
| 4b | SecurityGateway.authorize() returns token | `Result<()>` → `Result<CapabilityToken>` | `hkask-mcp`, `hkask-agents` |
| 4c | AgentPod state machine guards | `can_transition_to(current, target) → bool` | `hkask-agents` |
| 4d | Make required_capabilities non-optional | `Vec::new()` = public | `hkask-templates` |
| 4e | Collapse ContextAssembler to 4 priorities | System, User, Memory(Episodic), Memory(Semantic), Tool | `hkask-templates` |
| 4f | Align schema naming | `entity/attribute/value` in code; uni-temporal `valid_from` | `hkask-storage`, `hkask-types`, docs |
| 4g | Wire BayesianOps as free functions | Remove `BayesianOps` struct. Make `combine`, `retract`, `decay`, `join`, `weighted_average` free functions. Wire `decay` and `retract` into episodic recall. | `hkask-memory` |

**Verification:** `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`

### Phase 5: Close the Episodic Loop — COMPLETE ✓

The episodic loop now closes. Experience goes in, gets classified, stored with confidence, recalled with decay and temporal attention, assembled into context with recency weighting, and budgeted.

| PR | Title | What | Affected Crates | Status |
|---|-------|------|-----------------|--------|
| 5a | Wire confidence decay into episodic recall | `bayesian::decay()` called in `query_for_deduped()`, `query_deduped()`, `query_deduped_with_stats()`. Uses `valid_from` timestamp and configurable `decay_rate`. `#[allow(dead_code)]` removed from `decay`. | `hkask-memory` | ✓ Done |
| 5b | Wire confidence retraction into episodic memory | `retract_triple(entity, attribute, retraction_confidence, perspective)` reduces confidence via `bayesian::retract()`. Creates versioned update (closes old, inserts new). `#[allow(dead_code)]` removed from `retract`. | `hkask-memory` | ✓ Done |
| 5c | Implement temporal attention in episodic recall | All `query_for*()` methods sort by `valid_from` DESC (most recent first). New `query_for_weighted()` returns `Vec<RecalledTriple>` with `decayed_confidence`, `recency_weight`, `time_since_storage_secs`. | `hkask-memory` | ✓ Done |
| 5d | Implement episodic storage budget | `check_budget(perspective, count)`, `storage_usage(perspective)`, `consolidation_candidates(perspective, limit)`. `cns.memory.budget` tracing span on overflow. `EpisodicStoragePort::episodic_storage_usage()` added. `PodContext::episodic_storage_usage()`. | `hkask-memory`, `hkask-agents` | ✓ Done |
| 5e | Enhance experience encoding | `ExperienceClassification` enum (Success=0.9, Failure=0.3, Observation=0.7, Inference=0.5, Instruction=0.8). `PodContext::store_episodic_experience()` with classification + optional confidence override. `EpisodicStoragePort::store_episodic_classified()`. `cns.memory.encode` span emitted. | `hkask-types`, `hkask-agents` | ✓ Done |
| 5f | Implement episodic context assembly | New `assemble_episodic_context_from_recalled()` takes `Vec<RecalledTriple>` with confidence threshold filtering and recency-weighted priority. Original `assemble_episodic_context()` preserved for backward compat. | `hkask-templates`, `hkask-memory` | ✓ Done |

**Verification:** `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`

### Phase 6: Close the Semantic Gaps + Consolidation Bridge — COMPLETE ✓

The semantic loop now closes. Semantic recall has confidence combination, semantic indexing is wired, consolidation promotes confidence, and per-entity storage budgets are enforced.

| PR | Title | What | Affected Crates | Status |
|---|-------|------|-----------------|--------|
| 6a | Wire semantic indexing | `SemanticMemory::query_similar(entity, embedding, k)` using `EmbeddingStore`. Merge embedding results with entity results. `TripleStore::get_by_id()` added for lookup. `SemanticMemory::recall_with_similarity()` combines both paths with dedup and confidence combination. | `hkask-memory`, `hkask-storage` | ✓ Done |
| 6b | Wire confidence combination in semantic recall | `SemanticMemory::recall_combined(entity)` groups triples by `(entity, attribute)`, combines confidences via `bayesian::join()`. `recall_combined_with_stats()` returns `CombineResult` with merge statistics. `combine_triples_by_attribute()` helper. | `hkask-memory` | ✓ Done |
| 6c | Implement consolidation priority and trigger | `EpisodicMemory::consolidation_candidates()` (from 5d) identifies lowest-confidence/oldest triples. `SemanticMemory::consolidate()` uses Bayesian seeding (6d). Trigger is via `PodContext` methods. `cns.memory.budget` span emission. | `hkask-agents`, `hkask-memory` | ✓ Done |
| 6d | Implement confidence promotion in consolidation | `bayesian::combine(episodic_conf, 0.5)` in `SemanticMemory::consolidate()`. `CONSOLIDATION_PRIOR = 0.5` constant. Semantic confidence is seeded from episodic rather than copied directly. | `hkask-memory` | ✓ Done |
| 6e | Implement semantic storage budget | `SemanticMemory` now has `storage_budget`, `check_budget()`, `storage_usage()`, `retraction_candidates()`. `SemanticMemoryError::BudgetExceeded`. `SemanticStoragePort::semantic_storage_usage()`. `PodContext::semantic_storage_usage()`. | `hkask-memory`, `hkask-agents` | ✓ Done |

**Verification:** `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`

### Phase 7: Curation Loop Wiring + Communication Foundation — COMPLETE ✓

| PR | Title | What | Affected Crates | Status |
|---|-------|------|-----------------|--------|
| 7d | Implement LoopMessage dispatch | `MessageDispatch` with priority queuing (Critical/Warning/Info), `send()`, `receive()`, `send_curator_directive()`, `send_escalation()`, `TraceId` propagation. 10 unit tests. | `hkask-agents` | ✓ Done |
| 7a | Wire MetacognitionLoop to CuratorContext | Replaced `Arc<CnsGovernWriteAdapter>` + `tokio::sync::Mutex<Arc<EscalationQueue>>` with `CuratorContext` (aggregates `CuratorHandle`, `CnsGovernWriteAdapter`, `MessageDispatch`, `EscalationQueue`, `Dampener`). Removed deprecated `from_legacy_adapter()`. | `hkask-agents` | ✓ Done |
| 7b | Wire Curation → Governance directive delivery | `MetacognitionLoop::issue_directive()` sends `CuratorDirective` through `CuratorContext::issue_directive()` → `MessageDispatch::send_curator_directive()`. `CalibrateThreshold` wired in `check_escalation_triggers()`. `AdjustEnergyBudget` and `UpdateCapabilities` supported via `CuratorDirective` enum. | `hkask-agents`, `hkask-cli` | ✓ Done |
| 7c | Wire Curation → Observability threshold calibration | `CnsGovernWriteHandle::calibrate_threshold()` + `CnsGovernWriteAdapter::calibrate_threshold()` + `CnsRuntime::calibrate_threshold()`. `MetacognitionLoop` calls `context.cns().calibrate_threshold()` in `check_escalation_triggers()`. 2 unit tests. | `hkask-agents`, `hkask-cns` | ✓ Done |
| 7e | Implement DAMPEN on feedback edges | `Dampener` struct with fingerprint-based suppression (type + target). Configurable time window (default 60s). Lazy garbage collection. `CuratorContext::issue_directive()` integrates DAMPEN — returns `None` if dampened. 6 unit tests. | `hkask-agents` | ✓ Done |

**Verification:** `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`

### Phase 7 Discoveries

1. **`CnsRuntime::calibrate_threshold()` was a missing method.** The `AlgedonicManager::set_expected_variety()` existed but was not exposed through `CnsGovernWriteHandle`. Phase 7c wired it: `CnsRuntime::calibrate_threshold()` → `CnsGovernWriteHandle::calibrate_threshold()` → `CnsGovernWriteAdapter::calibrate_threshold()`. The Curation loop now has a direct write path to adjust observability thresholds.

2. **`EscalationQueue` doesn't need `tokio::sync::Mutex` wrapping.** The original `MetacognitionLoop` wrapped `Arc<EscalationQueue>` in `tokio::sync::Mutex`, but `EscalationQueue` already uses `Arc<Mutex<Connection>>` internally. Phase 7a removed the redundant outer mutex — calls go through `CuratorContext::escalation_queue()` directly.

3. **`CuratorContext` aggregates OCAP boundaries.** The `CuratorHandle` type in `hkask-types` defines what the Curator *can* do (read all, write policy). `CuratorContext` in `hkask-agents` holds the *runtime adapters* that implement those capabilities. This two-layer pattern (types handle + runtime context) mirrors the `CnsWriteHandle`/`CnsWriteAdapter` pattern established in Phase 2.

4. **DAMPEN uses `DirectiveFingerprint` (type + target), not content.** Two `CalibrateThreshold` directives for different domains are NOT dampened against each other — only identical (type, target) fingerprints are suppressed. This is intentional: the Curator must be able to calibrate multiple domains in the same cycle.

5. **`from_legacy_adapter()` was fully removed.** The deprecated bridge method that panics with `unimplemented!()` has been deleted. All consumers now pass `CuratorContext` to `MetacognitionLoop::new()`.

### Phase 8 Discoveries

1. **`LockPriority` maps to `MessagePriority`.** The storage lock priority enum has a natural mapping from the Communication loop's `MessagePriority`: Critical→Critical, Warning→High, Info→Normal. Low priority is storage-specific (consolidation, maintenance).

2. **`Database::acquire()` returns `PriorityLockGuard` with Deref/DerefMut to `Connection`.** The `Database.conn` field needed to be `pub(crate)` to allow the `lock_priority` module to access it. The guard type implements `Deref<Target=Connection>` and `DerefMut` so callers can use it like a regular `Connection` reference.

3. **`DataCategory::derivation_context()` returns `String`, not `&'static str`.** Custom categories require dynamic context strings, which can't be `&'static str`. The HKDF `derive_sub_key()` function takes `&str`, so the `String` is passed by reference — no allocation in the hot path for well-known categories.

4. **`UnifiedVarietyTracker` shares the `AlgedonicManager` via `Arc<RwLock>`.** The previous `SovereigntyObserver` had its own `Arc<RwLock<AlgedonicManager>>`. The unified tracker receives a clone of the same `Arc`, so all variety tracking, sovereignty alerting, and bot metrics use the same algedonic manager for consistent escalation.

5. **`CnsState` holds one `UnifiedVarietyTracker` instead of `VarietyMonitor + SovereigntyObserver`.** The `SovereigntyObserver` struct still exists as a standalone type in `observers/sovereignty.rs` for backward compatibility, but `CnsRuntime` no longer uses it directly. All sovereignty event processing goes through `UnifiedVarietyTracker::process_sovereignty_event()`.

### Phase 8: Implementation-Phase Open Questions — COMPLETE ✓

| PR | Title | What | Affected Crates | Status |
|---|-------|------|-----------------|--------|
| 8a | Priority-tagged lock in storage | `LockPriority` enum (Critical/High/Normal/Low) with `PriorityLockGuard`, `Database::acquire(priority)`, CNS span tracing, `LockPriority::from_message_priority()` | `hkask-storage` | ✓ Done |
| 8b | DataCategory visibility enforcement | `DataCategory::derivation_context()` method, data category derivation contexts in `hkask-types`, `derive_data_category_key()` in `hkask-keystore`, HKDF-SHA256 key derivation per category ensures storage-layer OCAP enforcement | `hkask-types`, `hkask-keystore` | ✓ Done |
| 8c | Minimum CNS: unify VarietyTracker | `UnifiedVarietyTracker` replaces `VarietyMonitor` + `SovereigntyObserver` in `CnsState`. Domain variety, sovereignty events, bot metrics, and goal variety all feed into one tracker. `CnsState` holds one `tracker: UnifiedVarietyTracker` instead of `variety + sovereignty_observer`. | `hkask-cns` | ✓ Done |

### Phase 9: Cybernetic Unit Tests

| PR | Title | Tests | Affected Crates |
|---|-------|------|-----------------|
| 9a | Inference loop tests | 1–5b: loop closing, capability boundary, energy budget, circuit breaker, context assembly, rate limiter | `hkask-types` (test) |
| 9b | Episodic memory tests | Loop closing, episodic write/read, episodic visibility, temporal attention, confidence decay, confidence retraction, episodic storage budget | `hkask-memory` (test) |
| 9c | Semantic memory tests | Loop closing, semantic read/write, semantic visibility, deduplication, consolidation, confidence combination, semantic indexing | `hkask-memory` (test) |
| 9d | Consolidation bridge tests | Perspective stripping, dedup prevention, priority selection, confidence promotion | `hkask-memory` (test) |
| 9e | Governance loop tests | 11–14: loop closing, attenuation, revocation, algedonic escalation | `hkask-agents` (test) |
| 9f | Observability loop tests | 15–17: loop closing, write/cannot-govern boundary, span emission | `hkask-cns` (test) |
| 9g | Curation loop tests | 18–22: loop closing, escalation routing, bot evaluation, kata coaching, threshold calibration | `hkask-agents` (test) |
| 9h | Communication tests | LoopMessage dispatch with priority, TraceId correlation, DAMPEN suppression within window | `hkask-types` (test) |

**Verification:** `cargo test cyber_ --workspace && cargo clippy --workspace -- -D warnings`

### Phase 10: Documentation & Verification

| PR | Title | What | Affected Crates |
|---|-------|------|-----------------|
| 10a | Update architecture docs | Cross-reference TASK9 diagrams against `pub` APIs. Update master doc with 7-loop structure, episodic/semantic split, communication messenger functions, handle matrix. | docs |
| 10b | CNS span audit | Add `cns.memory.encode`, `cns.memory.decay`, `cns.memory.retract`, `cns.memory.budget` spans for new episodic subloops. Verify existing spans map to core loops. | `hkask-cns` |
| 10c | BotMetricsCollector investigation | Verify consumed by Curation. Keep or remove. | `hkask-cns` |

---

## Dependency Graph

```
Phase 0 ✓ → Phase 1 ✓ → Phase 2 ✓ → Phase 3 ✓ → Phase 4 ✓ → Phase 5 ✓ → Phase 6 ✓ → Phase 7 ✓ → Phase 8 → Phase 9 → Phase 10

Phase 2 → Phase 7 (handles must exist before Curation can use them) ✓
Phase 1 → Phase 4 (types must exist before contracts can tighten them) ✓
Phase 1 → Phase 5 (episodic handles must exist before episodic subloops can be wired) ✓
Phase 4 → Phase 7 (contracts must be tight before Curation wiring and communication) ✓
Phase 5 → Phase 6 partially overlapped (5a–5c before 6a–6c; 5f before 6a) ✓
```

Within each phase, most PRs can run in parallel. **Exception:** Phase 3, PR 3a must land before 3b.

---

## Verification Gate (run after every PR)

```bash
cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check
```

After Phase 9, additionally:
```bash
cargo test cyber_ --workspace
```

---

## Design Constraints (from AGENTS.md — NON-NEGOTIABLE)

1. **No visual UI.** hKask is headless — CLI/MCP/API only. No Grafana, dashboards, web frontends.
2. **No excess complexity.** No unused traits, stubs, deprecations, feature flags that aren't wired. Delete stubs, don't publish them.
3. **CNS observability is programmatic.** Spans, variety counters, algedonic alerts — no external monitoring stack.
4. **Capability discipline is enforced by the type system.** `EpisodicReadHandle` cannot call `store_episodic()` because the method doesn't exist on that type. `CnsGovernReadHandle` cannot call `set_expected_variety()` because the method doesn't exist. This is the strongest enforcement possible.

---

## Key Files to Reference During Implementation

| File | Purpose |
|------|----------|
| `crates/hkask-memory/src/episodic.rs` | `EpisodicMemory` with all subloops wired: decay, retraction, temporal attention, budget, consolidation candidates, weighted recall. `RecalledTriple` struct. |
| `crates/hkask-memory/src/semantic.rs` | `SemanticMemory` with confidence combination (`recall_combined`), semantic indexing (`query_similar`, `recall_with_similarity`), confidence promotion in `consolidate()`, storage budget, retraction candidates |
| `crates/hkask-memory/src/bayesian.rs` | Free functions: `combine`, `retract`, `decay`, `join`, `weighted_average` — wired into episodic (5a–5b) and semantic (6b, 6d) subloops |
| `crates/hkask-memory/src/recall_dedup.rs` | `dedup_triples` — BLAKE3-based deduplication (works for both episodic and semantic) |
| `crates/hkask-types/src/loops/episodic.rs` | `EpisodicReadHandle`, `EpisodicWriteHandle`, `ExperienceClassification`, `EpisodicBudgetExceeded` |
| `crates/hkask-agents/src/pod/context.rs` | `PodContext` — `store_episodic()`, `recall_episodic()`, `store_episodic_experience()`, `episodic_storage_usage()` |
| `crates/hkask-agents/src/ports/memory_storage.rs` | `EpisodicStoragePort` (with `store_episodic_classified()`, `episodic_storage_usage()`), `SemanticStoragePort`, legacy `MemoryStoragePort` |
| `crates/hkask-agents/src/adapters/memory_storage.rs` | `MemoryStorageAdapter` — concrete impl of all storage ports |
| `crates/hkask-storage/src/triples.rs` | `TripleStore` with `is_episodic()`/`is_semantic()`, `query_by_perspective()`, `update()` for versioned retraction |
| `crates/hkask-templates/src/context_assembly.rs` | `assemble_episodic_context()`, `assemble_episodic_context_from_recalled()`, `assemble_semantic_context()` |
| `crates/hkask-agents/src/curator/metacognition.rs` | `MetacognitionLoop` — uses `CuratorContext` for capability-disciplined access. `issue_directive()` with DAMPEN filtering. |
| `crates/hkask-agents/src/curator/context.rs` | `CuratorContext` — aggregates `CuratorHandle`, `CnsGovernWriteAdapter`, `MessageDispatch`, `EscalationQueue`, `Dampener`. `issue_directive()` with DAMPEN. |
| `crates/hkask-agents/src/curator/dispatch.rs` | `MessageDispatch` — priority-ordered (Critical/Warning/Info) inter-loop message queue. `send()`, `receive()`, `send_curator_directive()`, `send_escalation()`. |
| `crates/hkask-agents/src/curator/dampener.rs` | `Dampener` — DAMPEN messenger function (6.3). Fingerprint-based directive suppression with configurable time window. |
| `crates/hkask-agents/src/curator/escalation.rs` | `EscalationQueue` — persistent queue for escalated outputs that require human review. |
| `crates/hkask-cns/src/runtime.rs` | `CnsRuntime` with 4 handle types. `CnsGovernWriteHandle::calibrate_threshold()` for threshold calibration (5.3 ADAPT). `CnsState` now uses `UnifiedVarietyTracker` instead of separate `VarietyMonitor` + `SovereigntyObserver`. |
| `crates/hkask-cns/src/unified_tracker.rs` | `UnifiedVarietyTracker` — single SENSE point for all CNS observation domains (4.1 domain variety, 4.3 bot metrics, 4.4 sovereignty events, goal variety). |
| `crates/hkask-storage/src/lock_priority.rs` | `LockPriority` enum (Critical/High/Normal/Low) with `PriorityLockGuard` and `Database::acquire(priority)`. |
| `crates/hkask-keystore/src/master_key.rs` | `derive_data_category_key()` — HKDF-SHA256 key derivation per `DataCategory` for storage-layer OCAP enforcement. |

---

## What NOT To Do

- **Do NOT** collapse Bot and Replicant into `AgentPod + interaction_mode` (Q8 resolved)
- **Do NOT** simplify away the micro-governance check in `dispatch_action` (Q4 resolved)
- **Do NOT** treat CNS as merely cross-cutting (resolved: it's Loop 4 with its own closed cycle)
- **Do NOT** add Grafana, Prometheus, or any visual monitoring (AGENTS.md constraint)
- **Do NOT** create parallel infrastructures — capability handles wrap existing types, not new ones (TASK5 principle)
- **Do NOT** combine episodic and semantic memory back into one loop — they are structurally different with different subloops, different sovereignty models, and different confidence directions (TASK9 resolved)
- **Do NOT** treat the Consolidation Bridge as a subloop of either memory loop — it's an inter-loop bridge that sits on the communication edge between 2a and 2b (TASK9 resolved)
- **Do NOT** skip Phase 5 — it's DONE now
- **Do NOT** skip Phase 6 — it's DONE now
- **Do NOT** skip Phase 7 — it's DONE now
- **Do NOT** assume `CnsRuntimeAdapter` is the way to access CNS — use the 4 handle types (`CnsWriteHandle`, `CnsGovernReadHandle`, `CnsGovernWriteHandle`, `CnsAdminHandle`) or their adapters. The monolithic adapter is deprecated.

---

*ℏKask — Implementation Handoff v3 — Phases 0–7 complete, next: Phase 8*