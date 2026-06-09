---
title: "hKask Open Questions and Underspecified Aspects"
audience: [architects, developers, decision-makers]
last_updated: 2026-06-08
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Open Questions and Underspecified Aspects

**Purpose:** Unresolved aspects requiring decision-making before they can be addressed. Each question is tagged with its MDS category and includes the decision options under consideration.

**Related:** [`MDS.md`](architecture/MDS.md), [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md)

---

## Resolved Questions (2026-05-29 Sprint)

### OQ-1: hKask-surface Documentation Depth ✅

**MDS Category:** Interface  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Remove all references to `hkask-surface` from active documentation. The concept is deferred to v1.1+ with an ADR if surface generation becomes necessary.

**Rationale:** No `hkask-surface` crate exists. Maintaining references to non-existent crates violates P6 (delete stubs, don't publish them). The MCP/CLI/API equivalence already provides surface generation through utoipa (OpenAPI) and clap (CLI docs).

---

### OQ-2: Federation Documentation Scope ✅

**MDS Category:** Composition  
**Status:** **Resolved — Option 1**  
**Resolution Date:** 2026-05-29

Document federation as a deferred architectural direction (no dedicated ADR yet); federation as a first-class concept is deferred until essential.

**Rationale:** Inter-system agent communication was explored (see ADR-028 in the archive) but is not currently in the codebase. True federation (discovery, resource negotiation, capability composition across independent hKask instances) exceeds the current essential architecture scope. No dedicated federation crate, deferred-design doc, or ADR exists yet; a forward ADR will be authored if/when federation becomes essential.

---

### OQ-3: Arsenal Crate Documentation Ownership ✅

**MDS Category:** Capability  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document MCP servers as a catalog with common pattern description and per-crate README for implemented servers. A unified catalog exists at `docs/status/mcp-tools-inventory.md` (formerly `mcp-server-audit.md`, archived 2026-06-07). Individual README files live in each `mcp-servers/hkask-mcp-*/README.md`.

**Rationale:** Each MCP server having its own specification entry in REQUIREMENTS.md (Option 1) creates 19 × ~2KB = ~38KB of spec overhead — disproportionate. Option 2 keeps the catalog as a single source of truth with per-crate detail for the specific tool surface. Note: The term "arsenal" is not part of the hKask vocabulary — the project has 11 core crates and 21 MCP servers, all in a single workspace.

---

### OQ-4: Cross-Workspace Dependency Visualization ✅

**MDS Category:** Observability  
**Status:** **Resolved — Current approach**  
**Resolution Date:** 2026-05-29

**Decision:** Maintain manual Mermaid dependency diagrams in architecture docs as the primary visualization. CI automation (cargo-depgraph) is a v1.1+ enhancement if dependency complexity warrants it.

**Rationale:** The workspace crate map is stable (11 core + 21 MCP servers). Manual Mermaid in `subsystem-erds.md` §12 and `ports-inventory.md` provides adequate visualization. The DIAGRAM_ALIGNMENT mechanism (PS-09) already catches drift.

---

### OQ-5: Automation and Drift Prevention ✅

**MDS Category:** Curation  
**Status:** **Resolved — Option 1, active**  
**Resolution Date:** 2026-05-29

**Decision:** CI checks for security invariants, constraint compliance, and metadata consistency are active in `.github/workflows/ci.yml` (`security-invariants` job). Further automation (link integrity, citation density, diagram alignment) expands incrementally.

**Rationale:** The `security-invariants` CI job added in this sprint (TASK-08) covers: no `unwrap()` on hot paths, no wildcard capabilities, no hardcoded secrets, no stubs (P6), no deprecated (P7), and no visual UI. This is the foundational tier of Option 1.

---

### OQ-6: ADR Gaps ✅

**MDS Category:** Lifecycle  
**Status:** **Resolved — Option 3**  
**Resolution Date:** 2026-05-29

**Decision:** Create retroactive ADRs for the 5 most impactful decisions. Forward-only ADRs for future decisions.

**Retroactive ADR Target List (5):**
1. **ADR-024**: Unified Registry Decision — single registry with `template_type` discriminator
2. **ADR-025**: 7-Level Attenuation Depth Limit — rationale for max 7 delegation levels
3. **ADR-026**: Bitemporal Triple Schema — valid-time + transaction-time semantics
4. **ADR-027**: Argon2id + HKDF-SHA256 Master Key Derivation — deterministic secrets
5. **ADR-028**: ACP Protocol Design — JSON-RPC 2.0 over stdio for agent communication (now archived — ACP transport layer removed)

**Rationale:** These 5 decisions are the most frequently referenced and most impactful architectural choices. Creating retroactive ADRs provides decision traceability without requiring ADRs for every implementation detail.

---

### OQ-7: Template Refresh ✅

**MDS Category:** Composition  
**Status:** **Resolved — Deferred**  
**Resolution Date:** 2026-05-29

**Decision:** Defer template regeneration to the next documentation refresh cycle. The current templates in `docs/artifacts/` and `registry/templates/spec/` are functional. MDS metadata requirements are documented in `MDS.md` and the four architecture specifications.

**Update (2026-05-29):** A documentation portal ([`README.md`](README.md)) was added that indexes every active document by MDS category and demonstrates the compliant metadata header. When OQ-7 is taken up, the portal and the four architecture specifications are the recommended "best example" sources from which to regenerate the artifact templates.

---

### OQ-8: hkask-mcp-spec Self-Application ✅

**MDS Category:** Curation  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document the self-application concept without executing it. The `hkask-mcp-spec` tools (11 MDS tools) are validated against the existing specification corpus. Self-application (using spec tools to capture/decompose/curate the spec tools themselves) is deferred to a future meta-curation exercise.

**Rationale:** There is no circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. Self-application is deferred only because it has not been implemented yet, not because of any logical problem.

---

### OQ-9: Stub MCP Server Completion ✅

**MDS Category:** Capability  
**Status:** **Resolved** (already confirmed 2026-05-28; reaffirmed 2026-05-29 MCP audit)  
**Resolution Date:** 2026-05-28

**Decision:** Both servers are fully implemented: `hkask-mcp-condenser` (1,744 LOC, 7 tools, 51 tests), `hkask-mcp-web` (3,389 LOC). No stubs remain. MCP tools inventory confirms completeness (see `docs/status/mcp-tools-inventory.md`).

---

## Open Crossroads (Future)

#### F1: OCAP Secret Generation vs. HKDF Derivation ✅ RESOLVED

**MDS Category:** Trust  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** HKDF-SHA256 derivation per WebID (`"hkask:ocap-secret:<webid>"`).

`AgentPod::new()` now calls `derive_ocap_secret(&webid)` which uses `HKDF-SHA256(master_key, "hkask:ocap-secret:" || webid)` to produce a deterministic, per-agent OCAP signing key. This eliminates `SecretRef::Generated` from the pod creation hot path (ADR-027 compliance) while preserving per-agent key isolation (Miller designation).

- Same passphrase + same WebID → same OCAP secret (restart-safe)
- Different WebIDs → cryptographically independent sub-keys (HKDF domain separation)
- No keystore dependency per pod — only the master key needs storage

**See:** `crates/hkask-agents/src/pod/mod.rs::derive_ocap_secret()`, ADR-027

### F3: Memory Pipeline Completeness ✅ RESOLVED

**MDS Category:** Persistence  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** AgentPod now persists lifecycle events as bitemporal episodic triples on every state transition (Populated→Registered→Activated→Deactivated).

`AgentPod::new_with_memory()` accepts an optional `MemoryStoragePort`. `PodManager::create_pod()` wires its `memory_storage` into pod creation. Each lifecycle method calls `record_lifecycle_event()` which stores `{entity: "pod:{id}", attribute: "lifecycle_state", value: state}` as an episodic_triple with private visibility. Persistence failures are non-fatal (logged with `tracing::warn`).

**See:** `crates/hkask-agents/src/pod/mod.rs::record_lifecycle_event()`

### F4: unwrap() Remediation Priority ✅ RESOLVED

**MDS Category:** Trust  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** Standard adopted and enforced: zero `unwrap()` on hot paths; `expect("reason")` preferred over `unwrap()` everywhere; legitimate infallible calls documented with `expect()`.

**Result:** 139 → 0 `unwrap()` calls in production code across all 11 core crates. All 139 converted to `expect("reason")` with explicit invariant documentation. CI `security-invariants` job enforces this permanently.

### F6: Goal Capability — Revocation and Lineage Unification ✅ RESOLVED

**MDS Category:** Trust  
**Status:** **Resolved** — `GoalCapabilityToken` entirely removed in v0.27.0  
**Raised:** 2026-05-29 · **Resolved:** 2026-06-04

**Resolution:** In v0.27.0, `GoalCapabilityToken` was **entirely removed** — the
type, its HMAC signing, epoch-based revocation, and attenuation were all
deleted. Goal operations now use `&WebID` for owner scoping instead of token
verification. The entire token infrastructure (HMAC, revocation, attenuation,
ADR-029) was removed as over-engineered ceremony with no functional payoff.
ADR-029 is now archived.

All sub-questions are **moot**:

1. **Revocation** — No longer applicable; no token to revoke. Owner scoping
   via `WebID` is the authority mechanism.
2. **API/MCP parity** — ✅ Previously resolved; parity still holds with `WebID`.
3. **Operation-set canonicalization encoding** — No longer applicable; no
   token signature to bind an operation set.
4. **Single vs. dual capability primitive** — No longer applicable;
   `GoalCapabilityToken` no longer exists. ADR-029 is archived (superseded).
5. **Persistence corruption response** — Remains relevant for
   `GoalRepositoryError::Corrupt` but is decoupled from token concerns.
6. **Recursion-bound coherence** — `SYSTEM_MAX_ATTENUATION` still applies to
   `CapabilityToken` attenuation, but the goal-specific recursion question is
   moot.

**See:** `crates/hkask-storage/src/goals.rs`, `crates/hkask-cli/src/commands/goal.rs`, `crates/hkask-api/src/routes/goal.rs`, `mcp-servers/hkask-mcp-goal/src/main.rs`, `docs/architecture/reference/subsystem-erds.md` §13, ADR-025. ~~ADR-029 is archived (superseded — `GoalCapabilityToken` type no longer exists).~~

### F5: 41,339 LOC vs. 35K Budget ✅ DEPRECATED

**MDS Category:** Lifecycle  
**Status:** **Deprecated**  
**Resolution Date:** 2026-05-29

**Decision:** The LOC budget has been deprecated. All code budget references removed from `docs/architecture/MDS.md`, `docs/architecture/reference/hKask-erd.md`, and `docs/architecture/PRINCIPLES.md`.

**Replacement discipline:** Every component must be essential and minimal — ask "is this necessary?" before "how big is it?" Code size is an output, not a constraint.

---

### P3-a: ACP Transport Abstraction ⚠️ DEFERRED

**MDS Category:** Interface  
**Status:** Deferred (no current need)  
**Raised:** 2026-05-29 (Loop Distillation)

Current ACP is JSON-RPC 2.0 over stdio (child process). For networked agents or in-process, a transport abstraction would be needed. However, no current consumer requires this — `AcpRuntime` works in-process. When networked ACP becomes necessary, define a transport trait in `hkask-types` and implement for stdio, HTTP, and in-process. (ADR-028, which documented the ACP protocol design, is archived — the transport layer was removed.)

### P3-b: CyberneticsToken/CurationToken Runtime Enforcement ⚠️ DEFERRED

**MDS Category:** Trust  
**Status:** Deferred (structural foundation in place)  
**Raised:** 2026-05-29 (Loop Distillation)

Tokens are now minted at loop construction (9b/9c) but not yet presented to capability gates. The OCAP authority chain exists structurally but is not enforced at runtime. This is by design — the token minting establishes the structural pattern. Runtime enforcement should be added when capability gates are introduced at the point of use (e.g., `ConsolidationBridge` checks for `ConsolidationToken`, `CyberneticsLoop` checks for `CyberneticsToken`).

### P3-d: Episodic vs Semantic Encryption Keys ⚠️ DEFERRED

**MDS Category:** Trust  
**Status:** Deferred (same master key, different visibility enforcement)  
**Raised:** 2026-05-29 (Loop Distillation)

Currently same master key for both. Episodic (private) and semantic (shared) have different threat models. Separate keys would add defense-in-depth but also key management complexity. The current visibility enforcement (`SemanticMemory::store()` rejects non-Shared, `EpisodicMemory::store()` rejects Shared) provides logical separation. Physical separation (separate encryption keys) should be revisited if cross-visibility attacks become a concern.

### P3-e: Loop Membrane Persistence ⚠️ DEFERRED

**MDS Category:** Persistence  
**Status:** Deferred (acceptable data loss for v0.27.0)  
**Raised:** 2026-05-29 (Loop Distillation)

Loop inboxes and variety counters are in-memory. On crash, all pending directives are lost. For v0.27.0, this is acceptable — directives are advisory (Curation suggests, doesn't command). If crash resilience becomes critical, add a WAL or periodic checkpoint mechanism to `MessageDispatch` and `VarietyTracker`. Priority: low.

### P3-f: Semantic Loop MCP Server ⚠️ RESOLVED

**MDS Category:** Interface  
**Status:** Resolved — intentional gap  
**Resolution Date:** 2026-06-03

Semantic Memory (Loop 2b) has no direct MCP server. Queries go through `hkask-mcp-cns` or `hkask-mcp-registry`. This is intentional — semantic queries are lower-level than what MCP tools expose. The CNS and Registry servers provide higher-level access patterns that compose semantic memory with other subsystems. Adding a dedicated semantic MCP server would be premature.

### P3-h: CNS Set-point Configuration ⚠️ DEFERRED

**MDS Category:** Interface  
**Status:** Deferred (hardcoded defaults sufficient for v0.27.0)  
**Raised:** 2026-05-29 (Loop Distillation)

CNS thresholds, gas budgets, variety set-points are currently hardcoded. Need YAML/env configuration for deploy-time tuning. Low priority for v0.27.0 — defaults work for development. Add `SetPointsConfig` YAML parsing when deployment scenarios require tuning.

### 8g: WebSearchPort Extraction ⚠️ DEFERRED

**MDS Category:** Composition  
**Status:** Deferred (no current consumer outside `hkask-mcp-web`)  
**Resolution Date:** 2026-06-03

`WebSearchPort` trait and `ProviderPool` are only consumed within `mcp-servers/hkask-mcp-web`. No other crate references them. Extracting the trait to `hkask-types` and the pool to a new `hkask-web` crate would be premature — it moves code without enabling new capabilities. If a consumer outside the MCP server needs web search (e.g., a new crate that orchestrates search + memory), extract then. The MCP server becoming a thin shim is the right long-term goal, but not today.

### 9d: AgentKind Behavioral Dispatch ⚠️ RESOLVED — Keep Cosmetic

**MDS Category:** Domain  
**Status:** Resolved — `AgentKind` remains a cosmetic enum  
**Resolution Date:** 2026-06-03

**Decision:** `AgentKind` (Bot/Replicant) remains a simple enum with no behavioral dispatch. Behavioral differences between Bot and Replicant are handled at the call site level (e.g., `chat_with_agent()` selects model based on `AgentKind`, privacy enforcement in `SemanticMemory`/`EpisodicMemory` uses `Visibility`). Converting `AgentKind` to a trait with associated types would change it from a 2-variant enum to a type-level dispatch mechanism, affecting every pod, agent registration, and template selection. The current design correctly separates identity (AgentKind) from behavior (site-level decisions). This is the right granularity for v0.27.0.

---

### TQ-1: Mechanical vs. LLM Completeness Evaluation

**MDS Category:** Curation  
**Status:** Open  
**Opened:** 2026-06-06

Can `CompletenessCheck::is_complete()` be evaluated mechanically, or does it require LLM-assisted judgment for natural-language goals? If mechanical, implement as `#[test]`. If LLM-assisted, delegate to `CurateEvaluate`. The first tracer-bullet test is: "Given a `GoalSpec` with one criterion `satisfied: true`, `is_complete()` returns `true`." This is mechanical and should pass. Natural-language goals (e.g., "User can chat with agents") require LLM evaluation.

---

### TQ-2: Coherence Threshold Calibration

**MDS Category:** Curation  
**Status:** Open  
**Opened:** 2026-06-06

The 0.7 coherence threshold (`default_coherence_threshold` in `CurationThresholdConfig`) is a starting guess per MDS §9.2 gap #13. What is the empirical coherence score for a well-curated test invariant set? Calibrate after operational data from at least one full crate's test rewrite.

---

### TQ-3: Skill Enforcement vs. Guidance

**MDS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-06

Should skills be enforced mechanically (pre-commit hooks, CI checks, `spec/skill/evaluate` returning violations that block merge) or treated as guidance (curation decisions overridable per sovereignty principle)? The architecture supports both — this is a social contract, not a technical constraint.

---

### TQ-4: Property-Based Testing Boundaries

**MDS Category:** Capability  
**Status:** Open  
**Opened:** 2026-06-06

Where do `proptest` and `cargo fuzz` fit? MDS invariants are natural property candidates (e.g., "\forall CurationDecision, coherence_score in [0, 1]"). But property testing is not tracer-bullet — it's a different cycle. Should it be governed by its own skill (`property-testing`) or folded into the TDD skill as a specialized cycle type?

---

### TQ-5: Integration Test Isolation

**MDS Category:** Composition  
**Status:** Open  
**Opened:** 2026-06-06

MCP server tests require `rmcp` transport. Should integration tests use the existing `McpTestServer` pattern from `hkask-mcp-markitdown`, or should a shared test fixture crate (`hkask-test-utils`) be extracted? Per C4 ("repetition is a missing primitive"), if 3+ MCP servers duplicate test setup, extract. Current count: 2 servers with test modules. Threshold not yet met.

---

### TQ-6: CNS Variety Counters for Test Diversity

**MDS Category:** Observability  
**Status:** Open  
**Opened:** 2026-06-06

Should `cns.test.*` spans track test diversity (number of distinct seams tested per MDS category) and emit algedonic alerts when test variety drops below threshold? This would make test coverage a homeostatic concern — cybernetically coherent, but requires defining thresholds per MDS category.

---

### TQ-7: Skill-Bundler Composition with TDD

**MDS Category:** Composition  
**Status:** Open  
**Opened:** 2026-06-06

When multiple skills are active (skill-bundler), does the TDD cycle apply per-skill (trace each skill's invariants individually) or per-task (trace the composite behavior)? If two skills govern the same behavior, which invariant wins? Resolution: Curation decides, per `CurateReconcile`.

---

### TQ-8: hkask-keystore Has Zero Tests

**MDS Category:** Trust  
**Status:** Open — CRITICAL  
**Opened:** 2026-06-06

`hkask-keystore` is security-critical (AES-256-GCM, HKDF-SHA256, OS keychain integration) and has zero test modules. This is a P0 gap per the test inventory. Priority: behavioral tests at the `Keychain` and `Encryption` seams.

---

### TQ-9: hkask-mcp-spec Has Zero Tests

**MDS Category:** Interface  
**Status:** Open — HIGH  
**Opened:** 2026-06-06

`hkask-mcp-spec` is the MDS governance surface (8+4 tool surfaces) and has zero test modules. Priority: behavioral tests at the `SpecStore` port and `SpecServer` tool handler seams.

---

### Q11: DelegationResource extensibility

**MDS Category:** Capability  
**Status:** Open  
**Opened:** 2026-06-07

`DelegationResource` enum covers 3 of 11 resource patterns in the capability grant table. Remaining 8 are handled by string matching in `capabilities_match()`. Recommendation: add `from_grant_table()` parse method that validates against known resource patterns.

---

## Open Questions from Review Findings (2026-06-07)

*Questions surfaced during the v0.23 documentation review that were not previously tracked in OPEN_QUESTIONS.md.*

### FUT-002: Should `lambda_for_category` be public?

**MDS Category:** Capability  
**Status:** Open  
**Opened:** 2026-06-07

`lambda_for_category` is a private `fn` with a fixed 5-category dispatch table. If the MDS category set expands (as it did from 5 to 9), the dispatch table silently diverges from `SpecCategory::all()`. Recommendation: either make it `pub` with a documented contract, or replace it with `SpecCategory::all()` iteration.

---

### FUT-003: Is `Dampener.override_cooldown` per-issuer or global?

**MDS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-07

Currently, `Dampener.override_cooldown` is global — any issuer's metacognitive override suppresses all subsequent overrides for 120s. This means a low-trust issuer can starve a high-trust issuer. Should cooldown be per-issuer (per-`WebID`)? Per-issuer would be more principled but requires a `HashMap<WebID, Instant>` and introduces memory growth proportional to active issuers.

---

### FUT-004: Is the MCP gateway a membrane for all servers, or a passthrough for some?

**MDS Category:** Capability, Trust  
**Status:** Open  
**Opened:** 2026-06-07

Only 2 of 21 MCP servers currently gate capabilities through `GovernedTool`. The remaining 19 pass tool calls through without OCAP checks. This means the "capability membrane" described in `MDS.md §7.1-7.2` §5.5 is selectively permeable. Should all servers gate? Should servers without side effects (e.g., `hkask-mcp-spec` read-only queries) be exempted by design?

---

### FUT-005: Is `SpecId` a brand or a plain `String`?

**MDS Category:** Domain  
**Status:** Open  
**Opened:** 2026-06-07

`SpecId` is currently a type alias, not a newtype. If it is a plain `String`, anyone can construct a `Spec` referencing any other `Spec` without provenance. If it is a brand type (newtype with private constructor), only the spec creation tools can mint valid `SpecId`s. This affects spec composition and curation authority.

---

### FUT-007: Per-issuer override cooldown (sibling of FUT-003)

**MDS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-07

Sibling question to FUT-003. If FUT-003 resolves as "per-issuer," this question asks: what is the per-issuer cooldown semantics? Should it scale with trust tier? Should it have a ceiling and floor? Blocked by FUT-003's resolution.

---

### FUT-009: Span namespace `cns.cli.*` vs `cns.cybernetics.*` — ADR needed

**MDS Category:** Observability  
**Status:** Open  
**Opened:** 2026-06-07

The canonical CNS span listing in `PRINCIPLES.md` §1.4 uses `cns.cybernetics.*` for some spans, while `AGENTS.md` and code use `cns.cli.*` for CLI-specific spans. This inconsistency should be resolved via an ADR before renaming spans in production code.

---

### FUT-010: MCP≡CLI≡API equivalence verification

**MDS Category:** Interface  
**Status:** Open  
**Opened:** 2026-06-07

The MDS focusing assumption `MCP ≡ CLI ≡ API` is an axiom of the specification, not a verifiable property of the codebase. Whether the code currently satisfies this equivalence is a code-implementation question, not a spec-document question. The spec document (`MDS.md §7.2`) correctly states the axiom. Code-level verification that all three surfaces exercise the same capability set is tracked here as a code task.

---

### FUT-011: SpecStore bitemporal query methods

**MDS Category:** Persistence  
**Status:** Resolved  
**Opened:** 2026-06-07  
**Resolved:** 2026-06-08

Added 4 bitemporal query methods to `SpecStore` trait + `SqliteSpecStore` impl:
- `list_valid_at(at)` — specs valid at a point in time
- `list_valid_in_range(from, to)` — specs with valid-time window overlap
- `list_since(since)` — specs recorded since a given timestamp (transaction-time)
- `expire(id, valid_to)` — set valid_to to expire a spec

Also added `recorded_at` column to `spec_curation_records` table and `list_curation_records_since(since)` method to `SqliteCurationRecordStore` (transaction-time query for curation audit trail). 6 tests verify all methods.

---

### FUT-012: Curation record persistence wiring

**MDS Category:** Curation  
**Status:** Open  
**Opened:** 2026-06-07

`SqliteCurationRecordStore` exists for persisting curation decisions, but it is not wired into the `spec_curate_evaluate` call path. The spec document correctly describes the intended behavior. This is a code-implementation gap.

---

### FUT-013: Coherence threshold calibration

**MDS Category:** Curation  
**Status:** Resolved  
**Opened:** 2026-06-07  
**Resolved:** 2026-06-08

Added calibration procedure to MDS §5.9: collect ≥10 SpecCurationRecord coherence scores, compute 25th percentile (nearest-rank) as empirical threshold. Code: `DefaultSpecCurator::calibrate_from_history(SqliteCurationRecordStore)`. Also added `load_all_curation_records()` to `SqliteCurationRecordStore` for retrieval of all historical scores. Recalibration recommended after every curation cycle.

---

## Resolution Summary

| OQ | Status | Decision | Date |
|----|--------|----------|------|
| OQ-1 | Resolved | Remove hkask-surface references | 2026-05-29 |
| OQ-2 | Resolved | Document federation as deferred | 2026-05-29 |
| OQ-3 | Resolved | Catalog + per-crate README | 2026-05-29 |
| OQ-4 | Resolved | Manual Mermaid (current) | 2026-05-29 |
| OQ-5 | Resolved | CI checks active | 2026-05-29 |
| OQ-6 | Resolved | 5 retroactive ADRs targeted | 2026-05-29 |
| OQ-7 | Deferred | Next doc refresh cycle | 2026-05-29 |
| OQ-8 | Resolved | Document concept, defer execution | 2026-05-29 |
| OQ-9 | Resolved | Confirmed fully implemented | 2026-05-28 |
| F6 | Resolved | GoalCapabilityToken removed; WebID-based owner scoping replaces token infrastructure | 2026-06-04 |

**MDS completeness:** 8/8 open questions resolved, 1 deferred with documented rationale. F6 resolved post-sprint. Code-implementation gaps (FUT-010 through FUT-013) reclassified from spec-document gaps per the MDS axiom `Spec-document completeness ⊥ Code-implementation completeness`.

---

## MDS Round 3 Deferred Items

*Tracked per MDS Semantic Alignment Audit (2026-06-06) remediation R11. These items were deferred in MDS §11 Round 3 but not previously tracked in OPEN_QUESTIONS.md. Now includes all 10 original MDS §11 R3 items cross-referenced.*

| # | Item | Category | Status | MDS §11 Ref |
|---|------|----------|--------|---------------|
| R3.1 | Span::Spec variant gap | Observability | **Resolved** (added in audit) | Audit R1 |
| R3.2 | SpecStore bitemporal semantics | Persistence | **Resolved** — 4 bitemporal query methods + recorded_at column + list_curation_records_since. 6 tests. Updated 2026-06-08 | MDS §11 #2 |
| R3.3 | Spec signing (Ed25519) | Trust | **Resolved** — Ed25519SpecSigner implemented | MDS §11 #3 |
| R3.4 | Spec capability tokens (spec:read, spec:write, spec:compose) | Capability | **Resolved** — CapabilityChecker::grant_spec() implemented | MDS §11 #4 |
| R3.5 | hLexicon spec-curation terms bootstrapping | Domain | **Resolved** (partially bootstrapped) | Audit §2.3 |
| R3.6 | MCP≡CLI≡API cross-surface equivalence test | Interface | ⚠️ Deferred | MDS §11 #6 |
| R3.7 | Curation authority OCAP boundary integration | Trust | ⚠️ Deferred | — |
| R3.8 | Curation records persistence | Persistence | **Resolved** (partial) — SqliteCurationRecordStore exists. Not yet wired into evaluate(). | MDS §11 #8 |
| R3.9 | Coherence threshold calibration (0.7) | Curation | ⚠️ Deferred (uncalibrated) | MDS §11 #7 |
| R3.10 | Spec version replacement (post version_sha removal) | Lifecycle | **Resolved** — Spec.version: Option<String> added | Audit R15 |
| R3.11 | `SpecStore` needs `Send + Sync` bounds on the trait itself | Trust | ⚠️ Deferred — breaking change to trait signature; bounds currently on field type only | MDS §11 #1 |
| R3.12 | `SpecObserver` → CNS span integration depth | Observability | ⚠️ Deferred — currently emits `tracing::info!`; needs SpanEmitter variety counters and algedonic alert triggers | MDS §11 #5 |
| R3.13 | Spec drift detection (`cns.spec.drift` span) | Observability | ⚠️ Deferred — drift magnitude metric specified but not implemented; requires comparing `Spec` goals against implementation state | MDS §11 #10 |

---

## MDS Audit Remediation Tracking (R4–R18)

*Remediation items from the 2026-06-06 MDS Semantic Alignment Audit that are now resolved but were not previously tracked in this document.*

| # | Item | Category | Status | Audit Ref |
|---|------|----------|--------|----------|
| R4 | MDS §9.1 self-application matrix labels | Observability, Persistence, Lifecycle, Curation | **Resolved** — matrix updated with :partial and :drift labels | Audit R4 |
| R6 | CNS span listing consolidation | Domain | **Resolved** — AGENTS.md and MDS.md §7.1-7.2 now cross-reference PRINCIPLES.md §1.4 | Audit R6 |
| R8 | TemplateType vocabulary mapping | Composition | **Resolved** — as_spec_name() method added, mapping table documented in MDS.md §7.2 §3.3 | Audit R8 |
| R13 | SpecDriftAlert not in CNS loop | Observability | **Resolved** — DefaultSpecCurator dispatches SpecDriftAlert through Communication Loop to CurationLoop inbox | Audit R13 |

---

## Documentation Alignment Open Questions (2026-06-07)

*Questions surfaced during the v0.27.0 documentation alignment and consolidation effort.*

### DA-1: Spec-document vs code-implementation boundary decision rule

**MDS Category:** Curation  
**Status:** Resolved  
**Opened:** 2026-06-07  
**Resolved:** 2026-06-08

The corrected MDS establishes that spec-document completeness is orthogonal to code-implementation completeness. The decision rule is now codified: spec completeness and code completeness are orthogonal predicates; drift items are classified by the curation gradient (Merge/Revise/Defer/Discard). Full drift set in `docs/status/spec-code-drift.yaml`, curation decisions in `docs/status/curation-decisions.yaml`. The MDS_SCAFFOLD.md §4 now has a two-column completeness predicate reflecting this axiom.

---

### DA-2: Status file population

**MDS Category:** Capability, Observability  
**Status:** Partially resolved  
**Opened:** 2026-06-07

Producing real content for `docs/status/` files. Two new status files have been created: `spec-code-drift.yaml` (the exhaustive drift set) and `curation-decisions.yaml` (the curation decision records). Remaining status files (Fowler audit, adversarial simplification, PROJECT_STATUS) stay open. Tracked in `docs/plans/TODO.md` (P2-11 through P2-15).

---

### DA-3: hkask-agents build regression

**MDS Category:** Domain, Capability  
**Status:** Resolved  
**Opened:** 2026-06-07  
**Resolved:** 2026-06-08

The `hkask-agents` crate build regression has been resolved. All 9 code drift items (P2-06-D1 through P2-06-D9) have been resolved via the curation gradient: spec_ahead items received stubs with FocusingAssumptions, divergent items received spec updates or type aliases. See `docs/status/spec-code-drift.yaml` and `docs/status/curation-decisions.yaml`.

---

### DA-4: Spec server self-application

**MDS Category:** Curation  
**Status:** Open (deferred, not blocked)  
**Opened:** 2026-06-07

The `hkask-mcp-spec` server can be used to capture and curate the specification corpus itself. There is no circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. Self-application is deferred only because it has not been implemented yet, not because of any logical problem.

**Note (2026-06-08, updated 2026-06-09):** The spec-code drift curation (Tasks 1–4) could be performed via `hkask-mcp-spec` tools (`spec/goal/capture`, `spec/require/writing-quality`, `spec/graph/coherence`) once self-application is implemented. The drift set and curation decisions were produced manually this cycle; future cycles should use the spec server. This requires SpecStore persistence wiring (FUT-011, FUT-012). Curation tools (evaluate, reconcile, cultivate) were deleted per MDS §3 — curation is external to the spec server.

---

### DA-5: Coherence threshold calibration as spec-document gap

**MDS Category:** Curation  
**Status:** Resolved  
**Opened:** 2026-06-07  
**Resolved:** 2026-06-08

Resolved by adding a `calibration` section to the `coherence_metric` block in MDS §5.9 Curation Spec Template. The calibration procedure is now documented: collect ≥10 SpecCurationRecord coherence scores, compute the 25th percentile (nearest-rank), use that as the empirical threshold. Code implementation: `DefaultSpecCurator::calibrate_from_history(SqliteCurationRecordStore)` in `crates/hkask-agents/src/curator_agent/spec_curator.rs`. This closes the spec-document gap — the spec now states the calibration method, not just the threshold value.

---

## References

[^ddmvss]: hKask Team. (2026). *MDS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/MDS.md`.