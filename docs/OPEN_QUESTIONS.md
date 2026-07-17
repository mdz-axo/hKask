---
title: "hKask Open Questions and Underspecified Aspects"
audience: [architects, developers, decision-makers]
last_updated: 2026-07-17
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Open Questions and Underspecified Aspects

**Purpose:** Unresolved aspects requiring decision-making before they can be addressed. Each question is tagged with its MDS category and includes the decision options under consideration.

**Related:** [`MDS.md`](architecture/core/MDS.md), [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md)

---

## Resolved Questions (2026-05-29 Sprint)

<details>
<summary>9 resolved questions — click to expand (historical record)</summary>

### OQ-1: hKask-surface Documentation Depth ✅

**MDS Category:** Interface  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Remove all references to ``kask` CLI surface` from active documentation. The concept is deferred to v1.1+ with an ADR if surface generation becomes necessary.

**Rationale:** No ``kask` CLI surface` crate exists. Maintaining references to non-existent crates violates P6 (delete stubs, don't publish them). The MCP/CLI/API equivalence already provides surface generation through utoipa (OpenAPI) and clap (CLI docs).

---

### OQ-2: Federation Documentation Scope ✅

**MDS Category:** Composition  
**Status:** **Resolved — Active in v0.31.0**  
**Resolution Date:** 2026-05-29 | **Revision Date:** 2026-06-30

Federation is now active in v0.31.0. The `hkask-federation` crate exists and `FEDERATION_V2.md` is an active proposal.

**Rationale:** Federation was initially deferred as exceeding the essential architecture scope. Since then, inter-system agent communication has been implemented: the `hkask-federation` crate provides discovery, resource negotiation, and capability composition across independent hKask instances. See `FEDERATION_V2.md` for the active proposal and forward design.

---

### OQ-3: Arsenal Crate Documentation Ownership ✅

**MDS Category:** Capability  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document MCP servers as a catalog with common pattern description and per-crate README for implemented servers. A unified catalog exists at `do../status/PROJECT_STATUS.md`. Individual README files live in each `mcp-servers/hkask-mcp-*/README.md`.

**Rationale:** Each MCP server having its own specification entry in REQUIREMENTS.md (Option 1) creates 19 × ~2KB = ~38KB of spec overhead — disproportionate. Option 2 keeps the catalog as a single source of truth with per-crate detail for the specific tool surface. Note: The term "arsenal" is not part of the hKask vocabulary — the project has 11 core crates and 21 MCP servers, all in a single workspace.

---

### OQ-4: Cross-Workspace Dependency Visualization ✅

**MDS Category:** Observability  
**Status:** **Resolved — Current approach**  
**Resolution Date:** 2026-05-29

**Decision:** Maintain manual Mermaid dependency diagrams in architecture docs as the primary visualization. CI automation (cargo-depgraph) is a v1.1+ enhancement if dependency complexity warrants it.


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
3. **ADR-026**: Bitemporal hMem Schema — valid-time + transaction-time semantics
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

### OQ-8: hkask-mcp-docproc Self-Application ✅

**MDS Category:** Curation  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document the self-application concept without executing it. The `hkask-mcp-docproc` tools (11 MDS tools) are validated against the existing specification corpus. Self-application (using spec tools to capture/decompose/curate the spec tools themselves) is deferred to a future meta-curation exercise.

**Rationale:** There is no circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. Self-application is deferred only because it has not been implemented yet, not because of any logical problem.

---

### OQ-9: Stub MCP Server Completion ✅

**MDS Category:** Capability  
**Status:** **Resolved** (already confirmed 2026-05-28; reaffirmed 2026-05-29 MCP audit)  
**Resolution Date:** 2026-05-28

**Decision:** Both servers are fully implemented: `hkask-mcp-condenser` (1,744 LOC, 7 tools, 51 tests), `hkask-mcp-research` (1,044 LOC). No stubs remain. MCP tools inventory confirms completeness (see `do../status/PROJECT_STATUS.md`).

---

</details>

## Pod Architecture Resolved Questions (ζ Group — v0.30.0)

> **Incorporated from:** `docs/architecture/core/OPEN_QUESTIONS_POD.md`

<details>
<summary>5 ζ-group questions — click to expand (resolved during multi-pod architecture design session)</summary>

### ζ.1 — Cross-Pod A2A Protocol

**Question:** What is the minimal viable cross-pod A2A protocol that preserves OCAP gating?

**Status:** **Deferred** (trigger: cross-server deployment use case)

**Analysis:** Current A2A (`A2ARuntime`) assumes same-process agents. Cross-pod A2A requires a network boundary. Use Matrix (Conduit homeserver) as the communication fabric. OCAP tokens carried in message metadata, verified by the receiving pod's `CapabilityChecker`.

---

### ζ.2 — Pod Portability Across Servers

**Question:** Is exporting a SQLCipher file sufficient for "move my pod to another server"?

**Status:** **Resolved** (design) — acceptance test deferred to v0.31.0.

**What transfers:** SQLCipher database file, deterministic passphrase (ADR-027), `.webid` sidecar, pod persona and capabilities.

**What does NOT transfer:** CNS variety counters (temporal state), Curator cursor state, MCP server API keys, active A2A sessions.

**Import procedure:** `kask pod export <pod_id>` → `kask pod import <pod_id> {pod}.db {pod}.webid`.

---

### ζ.3 — Pod Lifecycle Across Containers

**Question:** If a pod IS a Docker container, does `kask pod activate` become `docker start {pod_id}`?

**Status:** **Deferred** (trigger: container deployment use case)

**Mapping:** Populated→Image built, Registered→Container created, Activated→Container started, Deactivated→Container stopped. `ActivePods` becomes a thin status tracker querying Docker/Podman container state.

---

### ζ.4 — Curator Aggregation Model

**Question:** Polling vs push for per-pod CNS aggregation?

**Status:** **Resolved** — polling model implemented in `CuratorSync` (1-second interval).

**Decision:** Polling handles restarts naturally via cursor-based catch-up. Push requires Curator to be alive and reachable at write time — fragile. CNS events serve as observability signals, not the sync trigger.

---

### ζ.5 — PodFactory Deletion Test

**Question:** Does `PodFactory` earn its existence if pods are created via `docker build`?

**Status:** **Resolved** — PodFactory survives essentialist G1 test.

**Verdict:** PodFactory is a deep module (1 public method, stateless). Delete it and pod creation becomes impossible. With container deployment, PodFactory's `deploy()` gains `--output containerfile` mode.

| ζ | Question | Status |
|---|----------|--------|
| ζ.1 | Cross-pod A2A protocol | Deferred |
| ζ.2 | Pod portability | Resolved |
| ζ.3 | Pod lifecycle across containers | Deferred |
| ζ.4 | Curator aggregation model | Resolved |
| ζ.5 | PodFactory deletion test | Resolved |

</details>

## Open Crossroads (Future)

#### F1: OCAP Secret Generation vs. HKDF Derivation ✅ RESOLVED

**MDS Category:** Trust  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** HKDF-SHA256 derivation per WebID (`"hkask:ocap-secret:<webid>"`).

OCAP signing is anchored to the system and A2A signing authorities. SQLCipher database encryption is deliberately separate and uses the canonical `HKASK_DB_PASSPHRASE` resolver across services, pods, synchronization, and MCP servers.

- Same passphrase + same WebID → same OCAP secret (restart-safe)
- Different WebIDs → cryptographically independent sub-keys (HKDF domain separation)
- No keystore dependency per pod — only the master key needs storage

**See:** `crates/hkask-agents/src/pod/mod.rs::system_ocap_signing_key()`, `crates/hkask-keystore/src/keychain.rs::resolve_db_passphrase_string()`

### F3: Memory Pipeline Completeness ✅ RESOLVED

**MDS Category:** Persistence  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** AgentPod now persists lifecycle events as bitemporal episodic hMems on every state transition (Populated→Registered→Activated→Deactivated).

`AgentPod::new_with_memory()` accepts an optional `MemoryStoragePort`. `ActivePods::create_pod()` wires its `memory_storage` into pod creation. Each lifecycle method calls `record_lifecycle_event()` which stores `{entity: "pod:{id}", attribute: "lifecycle_state", value: state}` as an episodic_triple with private visibility. Persistence failures are non-fatal (logged with `tracing::warn`).

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


### F5: 41,339 LOC vs. 35K Budget ✅ DEPRECATED

**MDS Category:** Lifecycle  
**Status:** **Deprecated**  
**Resolution Date:** 2026-05-29


**Replacement discipline:** Every component must be essential and minimal — ask "is this necessary?" before "how big is it?" Code size is an output, not a constraint.

---

### P3-a: ACP Transport Abstraction ⚠️ DEFERRED

**MDS Category:** Interface  
**Status:** Deferred (no current need)  
**Raised:** 2026-05-29 (Loop Distillation)

Current ACP is JSON-RPC 2.0 over stdio (child process). For networked agents or in-process, a transport abstraction would be needed. However, no current consumer requires this — `A2ARuntime` works in-process. When networked ACP becomes necessary, define a transport trait in `hkask-types` and implement for stdio, HTTP, and in-process. (ADR-028, which documented the ACP protocol design, is archived — the transport layer was removed.)

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

Loop inboxes and variety counters are in-memory. On crash, all pending messages on `tokio::mpsc` channels are lost along with any pending directives. For v0.27.0, this is acceptable — directives are advisory (Curation suggests, doesn't command). If crash resilience becomes critical, add a WAL or periodic checkpoint mechanism to channel state and `VarietyTracker`. Priority: low.

### P3-f: Semantic Loop MCP Server ⚠️ RESOLVED

**MDS Category:** Interface  
**Status:** Resolved — intentional gap  
**Resolution Date:** 2026-06-03

Memory (Episodic + Semantic, Loop 2) has no direct MCP server — queries go through `hkask-mcp-memory`. This is intentional — semantic queries are lower-level than what MCP tools expose. The Memory server provides higher-level access patterns that compose semantic memory with other subsystems. Adding a dedicated semantic MCP server would be premature.

### P3-h: CNS Set-point Configuration ⚠️ DEFERRED

**MDS Category:** Interface  
**Status:** Deferred (hardcoded defaults sufficient for v0.27.0)  
**Raised:** 2026-05-29 (Loop Distillation)

CNS thresholds, gas budgets, variety set-points are currently hardcoded. Need YAML/env configuration for deploy-time tuning. Low priority for v0.27.0 — defaults work for development. Add `SetPointsConfig` YAML parsing when deployment scenarios require tuning.

### 8g: WebSearchPort Extraction ⚠️ DEFERRED

**MDS Category:** Composition  
**Status:** Deferred (no current consumer outside `hkask-mcp-research`)  
**Resolution Date:** 2026-06-03

`WebSearchPort` trait and `ProviderPool` are only consumed within `mcp-servers/hkask-mcp-research`. No other crate references them. Extracting the trait to `hkask-types` and the pool to a new `hkask-api` crate would be premature — it moves code without enabling new capabilities. If a consumer outside the MCP server needs web search (e.g., a new crate that orchestrates search + memory), extract then. The MCP server becoming a thin shim is the right long-term goal, but not today.

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

MCP server tests require `rmcp` transport. Should integration tests use the existing `McpTestServer` pattern from `hkask-mcp-docproc`, or should a shared test fixture crate (`hkask-test-harness`) be extracted? Per C4 ("repetition is a missing primitive"), if 3+ MCP servers duplicate test setup, extract. Current count: 2 servers with test modules. Threshold not yet met.

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

### TQ-9: hkask-mcp-docproc Has Zero Tests

**MDS Category:** Interface  
**Status:** Open — HIGH  
**Opened:** 2026-06-06

`hkask-mcp-docproc` is the MDS governance surface (8+4 tool surfaces) and has zero test modules. Priority: behavioral tests at the `SpecStore` port and `SpecServer` tool handler seams.

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

Only 2 of 21 MCP servers currently gate capabilities through `GovernedTool`. The remaining 19 pass tool calls through without OCAP checks. This means the "capability membrane" described in `MDS.md §7.1-7.2` §5.5 is selectively permeable. Should all servers gate? Should servers without side effects (e.g., `hkask-mcp-docproc` read-only queries) be exempted by design?

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

The canonical CNS span registry (`crates/hkask-types/src/cns.rs`, `CnsSpan`) uses `cns.cybernetics.*` for some spans, while `AGENTS.md` and code use `cns.cli.*` for CLI-specific spans. This inconsistency should be resolved via an ADR before renaming spans in production code.

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
| OQ-1 | Resolved | Remove `kask` CLI surface references | 2026-05-29 |
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
| R3.5 | Spec-curation terms bootstrapping | Domain | **Resolved** (partially bootstrapped) | Audit §2.3 |
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
| R6 | CNS span listing consolidation | Domain | **Resolved** — AGENTS.md and MDS.md §7.1-7.2 now cross-reference canonical CNS span registry: `crates/hkask-types/src/cns.rs` (`CnsSpan`) | Audit R6 |
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

The corrected MDS establishes that spec-document completeness is orthogonal to code-implementation completeness. The decision rule is now codified: spec completeness and code completeness are orthogonal predicates; drift items are classified by the curation gradient (Merge/Revise/Defer/Discard). Full drift set in `do../status/corpus_inventory.yaml`, curation decisions in `do../status/corpus_inventory.yaml`. The MDS_SCAFFOLD.md §4 now has a two-column completeness predicate reflecting this axiom.

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

The `hkask-agents` crate build regression has been resolved. All 9 code drift items (P2-06-D1 through P2-06-D9) have been resolved via the curation gradient: spec_ahead items received stubs with FocusingAssumptions, divergent items received spec updates or type aliases. See `do../status/corpus_inventory.yaml` and `do../status/corpus_inventory.yaml`.

---

### DA-4: Spec server self-application

**MDS Category:** Curation  
**Status:** Open (deferred, not blocked)  
**Opened:** 2026-06-07

The `hkask-mcp-docproc` server can be used to capture and curate the specification corpus itself. There is no circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. Self-application is deferred only because it has not been implemented yet, not because of any logical problem.

**Note (2026-06-08, updated 2026-06-09):** The spec-code drift curation (Tasks 1–4) could be performed via `hkask-mcp-docproc` tools (`spec/goal/capture`, `spec/require/writing-quality`, `spec/graph/coherence`) once self-application is implemented. The drift set and curation decisions were produced manually this cycle; future cycles should use the spec server. This requires SpecStore persistence wiring (FUT-011, FUT-012). Curation tools (evaluate, reconcile, cultivate) were deleted per MDS §3 — curation is external to the spec server.

---

### DA-5: Coherence threshold calibration as spec-document gap

**MDS Category:** Curation  
**Status:** Resolved  
**Opened:** 2026-06-07  
**Resolved:** 2026-06-08

Resolved by adding a `calibration` section to the `coherence_metric` block in MDS §5.9 Curation Spec Template. The calibration procedure is now documented: collect ≥10 SpecCurationRecord coherence scores, compute the 25th percentile (nearest-rank), use that as the empirical threshold. Code implementation: `DefaultSpecCurator::calibrate_from_history(SqliteCurationRecordStore)` in `crates/hkask-agents/src/curator_agent/spec_curator.rs`. This closes the spec-document gap — the spec now states the calibration method, not just the threshold value.

---

## Document Automation (2026-06-12)

Open questions from the documentation corpus sweep and `document-update` skill composition.

### FUT-DOC-1 — Continuous Self-Application

**MDS Category:** Lifecycle, Curation
**Status:** Open
**Opened:** 2026-06-12
**Cross-references:** MDS_SCAFFOLD.md §6.1, FUT-DOC-4

Should the spec server run `spec/graph/coherence` on the document corpus on every merge to main, or only on explicit `kask sovereignty verify` invocation? The MDS_SCAFFOLD §6.1 notes this is "not blocked by circularity" but deferred.

**Options:**
1. **CI gate:** Run `spec/graph/coherence` as a CI check on every merge to main. Failing coherence blocks the merge.
2. **Manual invocation:** Run only on `kask sovereignty verify`. Lower overhead, but drift accumulates between invocations.
3. **Scheduled:** Run on a cron schedule (daily/weekly). Compromise between automation and overhead.

### FUT-DOC-2 — Coherence Threshold Calibration

**MDS Category:** Curation
**Status:** Open
**Opened:** 2026-06-12
**Cross-references:** TQ-2, FUT-013, DA-5

The current threshold of 0.7 (Jaccard similarity) is inherited from MDS §7.5. Is 0.7 the correct threshold for a document corpus (as opposed to a code API surface)?

**Options:**
1. **Keep 0.7:** Inherit from MDS code threshold. Consistent but may be wrong for documents.
2. **Calibrate empirically:** Collect coherence scores from ≥10 corpus sweeps, compute 25th percentile per DA-5 resolution.
3. **Lower to 0.5:** Documents have more natural variance than code APIs. A lower threshold may be more appropriate.

### FUT-DOC-3 — Automated Drift Detection

**MDS Category:** Composition, Curation
**Status:** Open
**Opened:** 2026-06-12
**Cross-references:** spec-code-drift.yaml, MDS.md §3

The current drift detection method (set-difference of named entities) is manual. Can `spec/graph/query` be extended with a `spec/graph/diff` tool that computes the symmetric difference between spec entities and code `pub` surfaces automatically? This would close the MDS self-application loop.

**Options:**
1. **Add `spec/graph/diff` tool:** New MCP tool that parses spec documents for named entities, extracts `pub` API surfaces via `cargo doc --output-format=json`, computes symmetric difference.
2. **CI script:** Bash script that does the same without a new MCP tool. Lower integration cost.
3. **Defer:** Manual drift detection is adequate for current corpus size (47 documents).

### FUT-DOC-4 — Skill Enforcement vs. Guidance

**MDS Category:** Trust, Curation
**Status:** Open
**Opened:** 2026-06-12
**Cross-references:** TQ-3, magna-carta.md P4

Should the `document-update` skill be enforced (agent MUST load it before any document edit) or advisory (agent MAY load it)? Resolution depends on the Magna Carta P4 (Clear Boundaries) constraint force classification.

**Options:**
1. **Enforced (Prohibition):** Agent MUST load `document-update` before any `docs/` edit. Violation blocks the edit. Strongest guarantee but reduces agent autonomy.
2. **Guardrail:** Agent SHOULD load `document-update`; edits without it produce a warning. Balanced.
3. **Guideline:** Agent MAY load `document-update`. The skill is available but not required. Preserves agent autonomy.

### FUT-DOC-5 — Replica Style Enforcement

**MDS Category:** Curation
**Status:** Open
**Opened:** 2026-06-12
**Cross-references:** WRITING_EXCELLENCE.md §3, hkask-mcp-replica

Can `replica_compare` be used as a CI gate — rejecting document edits whose stylistic distance from the Curator centroid exceeds 0.3? This would operationalize the Writing Excellence Mandate as an automated check rather than a manual review.

**Options:**
1. **CI gate:** `replica_compare` runs on every document edit; distance >0.3 blocks merge. Full automation.
2. **Advisory check:** `replica_compare` runs but produces a warning, not a block. Lower friction.
3. **Defer:** Manual review via `spec/require/writing-quality` is adequate. Style enforcement is aspirational.

### FUT-DOC-6 — Semantic Documentation Embedding Space

**MDS Category:** Domain, Curation
**Status:** Open
**Opened:** 2026-06-12
**Cross-references:** WRITING_EXCELLENCE.md §2.3, hkask-services-corpus EmbedService, hkask-storage sqlite-vec, FUT-DOC-5

The current `document-update` skill performs structural analysis (metadata, links, taxonomy alignment) and manual rubric-based quality assessment. It has zero semantic content analysis — no embedding-based similarity search, no structural pattern matching against exemplary technical documentation.

The infrastructure exists across three crates but is wired for author style replication, not documentation quality:
- `hkask-services-corpus::EmbedService` — chunks and embeds Gutenberg prose for author voice replication
- `hkask-storage` — sqlite-vec vector database for embedding storage
- `hkask-mcp-replica` — `replica_compose`/`replica_compare` for stylistic analysis

**Proposed capability:** A documentation snippet base — a curated corpus of exemplary technical documents embedded into sqlite-vec, with metadata tagging by document section type per WRITING_EXCELLENCE §2.3 (Statement, Evidence, Diagram, Implications). This would enable:

1. **Semantic similarity search** — for any hKask document section, retrieve the 3 most similar sections from the exemplary base. "This ADR's Trust section reads like a Kubernetes API reference, not an architecture decision record — consider restructuring."

2. **Structural gap detection** — detect missing WRITING_EXCELLENCE §2.3 elements. "This document has Statement and Evidence sections but no Implications. Here are 5 exemplary Implications sections from similar-domain documents in the snippet base."

3. **Style centroid comparison** — `replica_compare` against a "technical-documentation" centroid trained on the exemplary base, complementing the existing Curator author-voice centroid.

4. **Auto-compose suggestions** — `replica_compose` with a documentation-structure persona that understands RFC structure, ADR conventions, and API documentation patterns.

**Exemplary corpus sources (candidates):**
| Source | Document Type | Why |
|--------|-------------|-----|
| IETF RFCs | Specification | Canonical technical specification structure |
| Kubernetes API docs | API reference | Gold-standard API documentation patterns |
| Rust stdlib docs | API reference + guide | Mixed reference/guide documentation |
| Python PEPs | Design document | Enhancement proposal structure |
| Well-written ADRs | Decision record | ADR conventions (Context → Decision → Consequences) |
| man pages | Reference | Terse, scannable reference format |
| Write the Docs guides | Guide | Community-vetted documentation patterns |

**Architecture sketch:**
```
docs/                                Exemplary corpus
  ├── architecture/                   (RFCs, PEPs, ADRs, man pages)
  ├── specifications/                        │
  └── ...                                    ▼
        │                           hkask-mcp-docproc
        │                           (parse → chunk by section type)
        ▼                                    │
  doc-update skill                           ▼
  (Task 3: quality gate)            EmbedService.embed_corpus()
        │                           (chunk → tag → embed → centroid)
        │                                    │
        ├── structural checks                ▼
        │   (metadata, links)         sqlite-vec
        │                            (doc_snippets table)
        ├── manual rubric                     │
        │   (Hopper/Lovelace/                 ▼
        │    Schriver/Gentle)         replica_compare(section_embedding,
        │                              exemplary_centroid)
        └── NEW: semantic checks              │
            (embedding similarity,            ▼
             structure detection,      writing_quality_report.yaml
             centroid distance)        (now includes semantic_score)
```

**Open design questions:**
- Should the exemplary corpus be static (curated once) or dynamic (updated as new exemplars are discovered)?
- What embedding model? The existing `EmbeddingRouter` supports DeepInfra, OpenRouter — any of these could generate documentation-aware embeddings. (Together AI embeddings not yet implemented.)
- Should the semantic score be a separate dimension (5th dimension alongside Hopper/Lovelace/Schriver/Gentle) or folded into the Gentle (agent-correctness) dimension?
- What's the minimum viable exemplary corpus size? 50 documents? 500? The EmbedService budget gate could help here.

**Options:**
1. **Implement now:** Build the snippet base with 20-50 exemplary documents, wire into `document-update` Task 3 as a 5th quality dimension. High value, moderate effort.
2. **Defer to next cycle:** Complete the structural/document automation loop first (FUT-DOC-1 through FUT-DOC-5), then add semantic analysis. Lower risk, but delays the capability.
3. **Prototype only:** Build the embedding pipeline but don't wire it into the quality gate yet. Prove the concept before committing to it as a gating check.

---

---

## Backup System Implementation (2026-06-14)

*Questions surfaced during the git backup system implementation.*

### BKP-001: Auto-snapshot scheduler daemon integration

**MDS Category:** Lifecycle  
**Status:** ✅ Resolved  
**Opened:** 2026-06-14  
**Resolved:** 2026-06-14

`BackupService::run_daily_snapshot()` provides the daily snapshot capability. The daemon loop calls this on a 24-hour schedule via `BackupLoop`, which implements `HkaskLoop` and is registered in `AgentService::build()` alongside the existing `SnapshotLoop`. The `auto_snapshot` config flag controls whether the loop is active.

**Resolution:** `BackupLoop` created in `hkask-storage/src/backup/loop.rs` (backup code absorbed into `hkask-storage`, v0.31.0). Registered in `AgentService::build()` at section 6c. Runs daily snapshots through `BackupService`, optionally followed by `verify()` and `prune()`.

---

### BKP-002: History rewriting completeness for prune

**MDS Category:** Persistence  
**Status:** Resolved (v0.30.0)  
**Opened:** 2026-06-14  
**Resolved:** 2026-06-22

~~`BackupService::rewrite_history()` creates a new commit chain with only retained commits, but does not garbage-collect the old (pruned) git objects.~~

**Resolution:** Added `delete_blob` to `GitCASPort` (completing CRUD). `rewrite_history` now collects ContentHashes from pruned commits, computes the set difference with retained hashes, and deletes pruned-only blobs from the CAS directory before creating the orphan commit. Pruning is now effective — orphan commits contain only retained blobs.

---

### BKP-003: Encryption key rotation strategy

**MDS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-14

When the encryption passphrase changes, all existing encrypted blobs become unreadable with the new key. A rotation strategy is needed: either re-encrypt all blobs with the new key (expensive), or maintain a key history (stores old salts/keys for decryption, uses new key for encryption). The current implementation does not handle key rotation — changing the passphrase via `enable_encryption()` generates a new salt and key, making old blobs inaccessible.

**Options:**
1. Re-encrypt all blobs on key change (walk all repos, decrypt with old key, re-encrypt with new key)
2. Maintain key history in `BackupConfig.encryption` as a `Vec<EncryptionConfig>` — try each key on decrypt
3. Accept that key rotation requires a full re-backup

---

## Dual-Presence Pattern (merged from `docs/specifications/dual-presence-pattern.md`, 2026-06-15)

**MDS Category:** domain, composition, trust
**Source:** Formerly `docs/specifications/dual-presence-pattern.md` — merged 2026-06-15 per essentialist consolidation.

The dual-presence pattern — where two entities (a sovereign host replicant and a co-participant daemon) share one CLI/REPL conversation loop — requires five architectural decisions:

### DP-1: Presence Model
**What does "being present" actually mean?**
- **Continuous:** Curator observes every message (enables proactive CNS regulation but raises P2 consent concerns)
- **Invoked:** Curator only engages when addressed (simpler but makes Curator reactive, not regulatory)
- Can presence be toggled? If user disables Curator, who regulates gas budgets?

### DP-2: Sovereignty Boundary
**Where does P1 draw the line between user and system?**
- If Curator observes all messages, is that data access requiring consent?
- If Curator stores observations, whose memory is it?

### DP-3: Authority Model
**Who has final say when user and Curator disagree?**
- P1 (User Sovereignty) vs P9 (System Self-Regulation) tension
- Escalation path: Curator warns → user overrides → CNS records override → pattern detection

### DP-4: Addressing Model
**How does the user address the Curator vs. other agents?**
- `@curator` prefix? Separate `/curator` command? Implicit context detection?

### DP-5: Memory Model
**Who owns the record of dual-presence interactions?**
- Episodic (private, per-agent) vs semantic (public, shared)
- If Curator observes, does it encode its observations as its own episodic memory?

**Recommended answer order:** Presence Model → Sovereignty Boundary → Authority Model → Addressing Model → Memory Model → Generalization (don't generalize an unstable pattern).

---


## Training System Open Questions

### TRN-001: Harness-specific optimizer naming layer

**MDS Category:** Capability
**Status:** Open
**Opened:** 2026-06-16

Axolotl supports `adamw_bnb_8bit`, `lion_8bit`, `adafactor`. Unsloth defaults to `adamw_8bit`. The canonical `TrainingParams` exposes `optimizer: Option<String>` — a free-form string. How should optimizer choice be exposed without coupling to specific harness implementations? A harness-aware optimizer mapping layer is needed.

**Options:**
1. Harness-agnostic optimizer enum (e.g., `AdamW8Bit`, `Lion`, `Adafactor`) with per-harness mapping
2. Free-form string with validation at config-generation time
3. Separate optimizer params per harness (adds surface, violates unification)

### TRN-002: Multi-LoRA composition at inference time

**MDS Category:** Composition
**Status:** Open
**Opened:** 2026-06-16

The architecture doc mentions adapters can be composed. How should two skill adapters (e.g., `coding-guidelines + deep-module`) be served simultaneously? `hkask-inference` supports one adapter per request (`LLMParameters.adapter`), not multiple.

**Options:**
1. Weight interpolation (blend adapter weights)
2. Sequential application (pass through adapter A, then adapter B)
3. MoE-style gating (router selects which adapter to activate per token)
4. Prompt-level composition (include both skill instructions in system prompt)

### TRN-003: Adapter version drift detection

**MDS Category:** Lifecycle
**Status:** Open
**Opened:** 2026-06-16

When a SKILL.md is updated, the adapter trained on the old version may produce incorrect decompositions. Should the system detect skill-document drift and flag adapters as stale? This is a CNS drift-detection problem already solved for specs (`DefaultSpecCurator`). Can the same mechanism apply to skill→adapter drift?

### TRN-004: HuggingFace dataset versioning

**MDS Category:** Domain
**Status:** Open
**Opened:** 2026-06-16

The `DatasetPipeline` uses local JSONL files. Should it also support loading datasets directly from HuggingFace Hub via `datasets.load_dataset` with version pinning? This would enable reproducible training with canonical datasets.

### TRN-005: Cloud harness config embedding

**MDS Category:** Composition
**Status:** Open
**Opened:** 2026-06-16

Axolotl and Unsloth are currently local-only; cloud dispatch returns `Unavailable`. For cloud hosts (Together, Runpod, Baseten), the harness field is tracked in `TrainingJob` but the actual axolotl YAML or unsloth Python script is not embedded in the dispatch payload. Should cloud hosts generate and include harness-specific config, or is the cloud host native training API sufficient?

### TRN-006: Training CNS span registration

**MDS Category:** Observability
**Status:** Open
**Opened:** 2026-06-16

CNS spans like `cns.training.sweep.iteration` and `cns.training.retrain.ab` are emitted via `tracing::info!` with string targets but not registered in `crates/hkask-types/src/cns.rs` `CnsSpan` enum. They work for logging but cannot be subscribed to programmatically via CNS variety tracking. Which of these spans meet the deletion test for programmatic observability?

### ADT-001: Adapter format normalization ✅ RESOLVED

**MDS Category:** Composition
**Status:** **Resolved**
**Resolution Date:** 2026-06-17

**Original question:** LoRA adapters from different training frameworks (PEFT, Axolotl, Unsloth) may have different serialization formats. Should `TrainedLoRAAdapter` normalize to a canonical format, or delegate format-awareness to the `InferenceProvider`?

**Decision:** Format validation lives in `ProviderCapability` and `AdapterConfig`. The `AdapterConfig` type parses `adapter_config.json` (PEFT standard). Provider backends validate compatibility through `ProviderCapability::can_compose()`. Normalization is the training pipeline's responsibility, not the storage layer's.

**Implementation:** `hkask-adapter::adapter_config::AdapterConfig::validate_base_model()`, `hkask-adapter::provider_cost::ProviderCapability::can_compose()`

### ADT-002: Adapter versioning ✅ RESOLVED

**MDS Category:** Lifecycle
**Status:** **Resolved**
**Resolution Date:** 2026-06-17

**Original question:** If a user retrains the same expertise, does the new adapter supersede the old one, or coexist?

**Decision:** Caller-managed versioning. `TrainedLoRAAdapter` carries an optional `version: Option<String>` field. Never implicitly superseded — P2 requires user consent for any change. Together AI and vLLM don't support native adapter versioning, so versions become distinct named adapters at the provider level.

**Implementation:** `TrainedLoRAAdapter.version` field

### ADT-003: Adapter distribution source ✅ RESOLVED

**MDS Category:** Composition
**Status:** **Resolved**
**Resolution Date:** 2026-06-17

**Original question:** Where do adapters live for provider access? S3? HuggingFace? Local disk?

**Decision:** `AdapterSource` enum with `HuggingFace { repo: String }` as the initial variant. All three inference providers (Together, Runpod, Baseten) can pull adapters from Hugging Face Hub natively. The enum is designed for extension — adding a second source is just an enum arm with no schema migration.

**Implementation:** `hkask-adapter::adapter_store::AdapterSource`

### ADT-004: Cross-model-family safety ✅ RESOLVED

**MDS Category:** Trust
**Status:** **Resolved**
**Resolution Date:** 2026-06-17

**Original question:** Can an adapter trained on Llama-3.3-70B be composed with Qwen2.5-7B?

**Decision:** `ProviderCapability::can_compose()` checks the provider's supported base model families against the adapter's `base_model_family`. Incompatible compositions are rejected at `create_endpoint()` time. The system does not warn about cross-family matches — it prevents them.

**Implementation:** `hkask-adapter::provider_cost::ProviderCapability::can_compose()`

### ADT-005: Adapter sharing between users ✅ RESOLVED

**MDS Category:** Trust
**Status:** **Resolved**
**Resolution Date:** 2026-06-17

**Original question:** Should one user's adapter be deployable by another user?

**Decision:** Sovereign-scoped by default (P1). `TrainedLoRAAdapter` carries an `owner: WebID` and `AdapterStore::list_owner()` filters by WebID. Sharing requires explicit consent (P2) through a `DelegationToken` with `adapter:read` capability, which is the existing OCAP model. No separate sharing mechanism needed.

**Implementation:** `TrainedLoRAAdapter.owner`, `AdapterStore::list_owner()`, `AdapterPort` trait methods accept `&DelegationToken`


## Pod Architecture

> **Incorporated from:** `docs/architecture/core/OPEN_QUESTIONS_POD.md`

Questions raised by the Solid Pod isomorphism that require future design work.

### POD-1 — Pod-to-Pod Communication (Cross-Pod A2A)

**Current state:** `ActivePods` registry tracks active deployments. `PodRegistry` provides filesystem-based discovery.

**Options:**

| Transport | Pros | Cons |
|-----------|------|------|
| **Matrix (Conduit)** | Already in deployment model. Built-in federation. | Adds Conduit as hard dependency. |
| **gRPC** | High performance. Bidirectional streaming. | New infrastructure. No federation. |
| **mpsc over TCP** | Simple. Matches CNS channel pattern. | No authentication, discovery, routing. |
| **WebSocket + JSON** | Browser-compatible. | No built-in federation. |

**Key constraint:** Cross-pod A2A must preserve OCAP gating. Capability tokens must extend across pod boundaries.

**Question:** What is the minimal viable cross-pod A2A protocol that preserves OCAP gating?

### POD-2 — Pod Portability Across Servers

**Current state:** Backup exports the SQLCipher file. All SQLCipher databases use the canonical installation passphrase resolved from `HKASK_DB_PASSPHRASE` or its keychain fallback.

**Open sub-questions:**
- **CNS state:** Reset variety counters on migration (60-second window).
- **API keys:** Travel with the pod (user-scoped, not server-scoped).
- **Addressable identity:** The pod's WebID is the identity; any server can host any pod.

### POD-3 — Pod Lifecycle Across Containers

**Target:** Pod IS a Docker/Podman container.

| Current (ActivePods) | Containerized (Proposed) | Question |
|---------------------|--------------------------|----------|
| `kask pod activate <id>` | `docker start <pod_id>` | Should `kask pod activate` wrap Docker? |
| `PodLifecycleState::Activated` | Container running | Is Activated a logical state or process state? |
| `PodLifecycleState::Deactivated` | Container stopped | Can pods reactivate from Deactivated? |

**Proposed:** `PodLifecycleState` remains logical. The container is an implementation detail. Deactivation is terminal — restart requires re-registration.

### POD-4 — Curator Aggregation Model

**Current state:** `CnsRuntime` is server-global. Per-pod CNS means N runtimes.

| Model | Description | Pros | Cons |
|-------|-------------|------|------|
| **Poll (Curator pulls)** | Curator queries each pod's CNS | Simple, Curator controls sampling | Polling overhead, delayed alerts |
| **Push (Pods emit)** | Pods push to shared Curator channel | Real-time, matches existing pattern | Weaker per-pod isolation |

**Constraint:** Algedonic pathway is unidirectional (CNS → Curator). Per-pod boundary means no cross-pod CNS observation.

**Question:** Poll (stronger isolation) or push (stronger real-time)?

### POD-5 — Essentialist Deletion Test on PodFactory

**G1 (Exist):** Is `PodFactory` a Rust type, or a CLI command that shells out to Docker?

| Future | PodFactory Role | Verdict |
|--------|----------------|---------|
| **In-process pods** | `PodFactory::deploy()` constructs `PodDeployment` in-process | **KEEP** — canonical constructor |
| **Containerized pods** | `kask pod export-container` generates Dockerfile | **DELETE** — replace with CLI |

**Question:** Is PodFactory a necessary intermediate step, or skip directly to container-native deployment?


## hKask Communication — Implementation Status & Open Questions

> **MDS Category:** Composition, Curation
> **Last Updated:** 2026-06-27

The `hkask-communication` crate was originally classified as RESOLVED (stubs replaced with tests). It now has a full operational pipeline beyond tests: the 7R7 Listener polls Matrix rooms, the CNS bridge persists NuEvents, CurationLoop.sense() filters communication events from NuEventStore and pushes directly to curation context, and the CAT engagement gate gates agent activation.

### COMM-001: hKask-Communication Pipeline Operational ✅ RESOLVED

**Original status (2026-06-14):** RESOLVED — stubs replaced with tests.

**Current status (2026-06-27):** RESOLVED — full pipeline operational. The crate provides:
- `MatrixTransport` (596 LOC) — matrix-sdk client lifecycle, messaging, rooms, files
- `SevenR7Listener` (191 LOC) — passive room observer, polls on configurable interval, persists CNS NuEvents
- `AgentRegistry` (152 LOC) — WebID↔UserId mapping, thread watchlists
- CNS bridge — NuEvent persistence → CurationLoop.sense() filters communication events from NuEventStore and pushes directly to curation context
- CAT engagement gate — `convergence_bias` scalar per agent
- Response dispatch — agent → Matrix room via `MatrixTransport::send_message()`
- 652 LOC of tests (Conduit-dependent integration + MXID derivation unit tests)

**Deferred:** E2EE (SQLCipher/SQLite conflict), continuous sync (v1 uses on-demand polling).

### COMM-002: Condenser Saliency Refactoring

**MDS Category:** Curation
**Status:** Open

**Context:** The `hkask-condenser` crate (761 LOC) provides context window condensation with `classify`, `compress`, `persist`, `thread_summary`, and related tools. Messages arriving via the Matrix transport pipeline follow the same condensation path as REPL/CLI messages, but the condensation strategy is uniform — it does not consider message saliency (priority, urgency, or relevance weighting).

**Question:** Should the condenser apply saliency-based weighting to messages based on their provenance (Matrix room context, sender identity, CAT convergence bias, or explicit priority markers), rather than treating all messages as equally important for condensation?

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A — Uniform (current)** | All messages condensed equally | Simple, predictable | May discard high-value content during window pressure |
| **B — Room-weighted** | Weight by Matrix room priority (Curator room > agent room > general) | Room-level priority, easy to implement | Doesn't consider message content |
| **C — Sender-weighted** | Weight by sender identity (human owner > replicant > bot > unknown) | Respects sovereignty hierarchy | Requires sender identity verification |
| **D — CAT-informed** | Weight by `convergence_bias` scalar from CAT engagement gate | Integrates with existing engagement model | Adds coupling between communication and condenser crates |
| **E — Hybrid (B + C + D)** | Weighted fusion with configurable coefficients | Most adaptive, user-configurable | Highest complexity |

**Key constraint:** Any saliency model must remain user-visible (P3 — Generative Space). Weights cannot be hidden parameters (P3 Prohibition #4).

**Recommended decision:** Defer. Implement Option A (uniform) as the stable baseline. Gather telemetry on condensation quality under realistic Matrix message volume before committing to a weighting strategy. Revisit when continuous sync is implemented and message volume increases.


## Fusion Mode — Deferred Items & Open Questions

> **MDS Category:** Domain, Composition, Trust
> **Last Updated:** 2026-07-17

Items deferred from the fusion-mode design study, implementation (T1–T4 + Proposal A), and adversarial code review (F1–F10). Each is tagged with its origin and the condition for resolution.

### FUS-001: Best-of-N Judge Position-Bias Measurement (F4)

**Status:** ⚠️ DEFERRED — instrument built, live measurement pending

**Context:** `mode_best_of_n` runs swap-revote (two judge calls in reversed display order, compared via `identify_pick`/Jaccard) to detect position bias (Zheng et al. 2024, arXiv:2406.07791). The adversarial review (F4) flagged the 2× token cost as potentially unjustified without evidence that the judge has position bias severe enough to warrant it. The two judge calls now run concurrently (`futures_util::join!`), halving latency, but token cost remains 2×.

**Instrument:** The harness in `fusion_orchestrator::tests::best_of_n_bias_harness_*` proves the measurement mechanism with two mock judges — `FixedPick` (bias-free, swap-revote agrees) and `FirstDisplayed` (position-biased, swap-revote disagrees). The protocol is documented in `docs/how-to/fusion-mode.md` under `best-of-n` → "Measuring whether swap-revote is justified".

**Decision (pending live measurement):** Run the harness with a real judge on a fixed panel-output fixture in 2 display orderings. If the pick changes → bias is present, keep swap-revote. If the pick is stable across orderings → simplify to order-randomization (1 judge call, permuted display order) to halve token cost. Re-measure when the judge model or panel composition changes. The ongoing `agree`/`disagree` verdicts logged at `cns.fusion` provide observational data: a sustained ~100% `agree` rate is evidence the second call is not earning its cost.

### FUS-002: Fusion Study Falsification Hypotheses (H1–H5)

**Status:** ⚠️ DEFERRED — none of the 5 hypotheses have been run

**Context:** The fusion-mode design study proposed 5 falsification hypotheses. None have been A/B-tested on a fixed benchmark. Each should be validated before the corresponding feature is considered production-confirmed:

| ID | Hypothesis | What it would falsify |
|----|-----------|----------------------|
| **H1** | Structured judge stabilization verdict > string-prefix self-report for convergence detection | The `deliberation` mode's structured-verdict parser (replaces former `FOLLOW_UP:` prefix) |
| **H2** | Swap-revote position-bias mitigation changes the pick on a measurable fraction of inputs | The `best-of-n` swap-revote mechanism (see FUS-001) |
| **H3** | Vote/tally (`algo:vote`) ≠ synthesis for heterogeneous panel outputs | The `algo:vote` method's existence vs. `algo:merge` alone |
| **H4** | Heterogeneous panels (mixed providers) outperform homogeneous panels on diverse tasks | The panel-diversity default |
| **H5** | Skill anchoring reduces judge bias / improves rubric adherence | The `skills` field on `FusionConfig` |

**Decision:** Defer until a fixed fusion benchmark fixture exists. Each hypothesis is independently testable once the fixture is in place.

### FUS-003: Fusion MCP Server (Q-A)

**Status:** ❌ REJECTED (P7) — deferred until external-agent demand materializes

**Context:** A fusion MCP server (exposing fusion modes as MCP tools to external agents) was considered during the design study. Per P7 (Generative Space — no speculative surfaces), this is deferred until there is concrete external-agent demand for fusion-as-a-service. The fusion orchestrator remains an internal inference-path component, not an MCP-exposed surface.

### FUS-004: Fusion as Default Inference Path (Q-B)

**Status:** ❌ REJECTED (P1/P5/P9)

**Context:** Making fusion the default inference path was considered and rejected. Fusion is opt-in and disabled by default — it activates only when a judge and panel are explicitly configured (`HKASK_FUSION_JUDGE_MODEL` + `HKASK_FUSION_PANEL_MODELS`). Rationale: P1 (User Sovereignty — the user chooses their inference path), P5 (No speculative defaults — fusion multiplies token cost), P9 (Homeostatic Self-Regulation — single-model inference is the stable baseline). This decision is final unless the user explicitly requests a change.


## References

[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification*. `docs/architecture/core/MDS.md`.

---

## 9. Documentation Alignment Audit (2026-07-01)

Generated by TASK 0–TASK 7 documentation alignment and completion initiative. Full report at `docs/status/documentation-alignment-2026-07-01.md`.

### DA-FUT-001 — Incorporated-from Annotations: Provenance or Noise?

**Context:** The architecture master contains 16 "Incorporated from:" annotations referencing files that were merged/absorbed in the 2026-06-24 consolidation. These files no longer exist on disk.

**Epistemic mode:** Subjunctive | **Constraint force:** Guideline

**Question:** Should these annotations remain as provenance markers (showing the lineage of absorbed content), or should they be deleted (violating the spirit of P5: no dead references)?

**Resolution needed:** Governance decision. Current state: annotations remain as provenance.

---

### DA-FUT-002 — Documentation Maintenance: Periodic vs. Continuous

**Context:** The MDS §4 defines a cybernetic feedback loop for system regulation. Documentation drift detection currently relies on manual `verify-docs.sh` + `check-links.sh` runs. The CNS infrastructure (`cns.documentation.drift` spans) could be wired for continuous monitoring.

**Epistemic mode:** Subjunctive | **Constraint force:** Hypothesis

**Meta-question:** What is the target steady-state for documentation maintenance — is it a periodic audit cycle (per-release `verify-docs.sh`), or a continuous CNS-monitored feedback loop (SeamWatcher detecting stale `verified-against` paths + algedonic alerts)?

**Resolution needed:** Governance decision — architecture team preference on documentation maintenance model.

---

### DA-FUT-003 — Undocumented API Routes: Intentional or Defect?

**Context:** 8 route files in `crates/hkask-api/src/routes/` lack `#[utoipa::path]` annotations: `admin.rs`, `auth.rs`, `export.rs`, `landing.rs`, `pods.rs`, `replicant.rs`, `settings.rs`, `terminal.rs`. This means 33% of routes are absent from the generated `openapi.json`.

**Epistemic mode:** Subjunctive | **Constraint force:** Hypothesis

**Question:** Are these routes intentionally undocumented (internal endpoints, no consumer-facing API contract), or is this a coverage gap that should be filled?

**Resolution needed:** Route owner confirmation. If intentional, document the policy. If gap, add `#[utoipa::path]` annotations.

---

### DA-FUT-004 — ACP-ZED-CONFIGURATION.md Recovery

**Context:** `docs/user-guides/ACP-ZED-CONFIGURATION.md` was referenced in the README but does not exist on disk. The ACP crate (`hkask-acp`) exists and is functional.

**Epistemic mode:** Declarative | **Constraint force:** Guideline

**Question:** Should this file be recovered from git history (`git log --diff-filter=D -- docs/user-guides/ACP-ZED-CONFIGURATION.md`) and restored, or rewritten from scratch against the current ACP crate?

**Resolution needed:** ACP crate developer input.

---

### DA-FUT-005 — Corpus Inventory Regeneration

**Context:** `docs/status/corpus_inventory.yaml` references paths under `docs/architecture/reference/`, `docs/guides/`, `docs/specifications/standards/`, `docs/specifications/specs/`, and `docs/specifications/policies/` — all directories that no longer exist. The inventory was last updated 2026-06-17 and needs regeneration.

**Epistemic mode:** Declarative | **Constraint force:** Guideline

**Question:** Should the inventory be regenerated to reflect current on-disk state, or should it be retired in favor of a simpler approach (e.g., deriving from README.md + file system scan)?

**Resolution needed:** Documentation steward decision.

---

### DA-FUT-006 — OPEN_QUESTIONS.md Resolved Item Archiving

**Context:** `OPEN_QUESTIONS.md` is ~1150 lines. Approximately 30 items are marked ✅ RESOLVED or ✅ DONE but remain in the active file. The resolved sections include OQ-1 through OQ-9, ζ.1–ζ.5, F1/F3/F4/F6, 8.1–8.5, COMM-001, and ADT-001–005.

**Epistemic mode:** Declarative | **Constraint force:** Guideline

**Question:** Should resolved items be moved to an archive section or removed entirely (preserving traceability via git history), per the DOCUMENTATION_STANDARDS.md lifecycle policy?

**Resolution needed:** Documentation steward scope decision.

---

### DA-FUT-007 — DOCUMENTATION_STANDARDS.md Self-Application Fixes

**Context:** `DOCUMENTATION_STANDARDS.md` §9 and §11.4 reference `WRITING_EXCELLENCE.md` (absorbed into Appendix A), `DOCUMENT_OWNERSHIP.md` (absorbed into §12), and `HANDOFF_LIFECYCLE.md` (absorbed into Appendix B). These internal references are broken.

**Epistemic mode:** Declarative | **Constraint force:** Guideline

**Question:** Should these references be updated to point to their appendix locations, or should the text be edited to remove the dependency on the absorbed files?

**Resolution needed:** Self-evident fix — requires an edit pass on §9 and §11.4.

---

### Items Promoted to GitHub Issues

| ID | Description | Priority |
|----|-------------|----------|
| DA-ISSUE-001 | Add `#[utoipa::path]` to 8 undocumented route files | P2 |
| DA-ISSUE-002 | Recover or rewrite `ACP-ZED-CONFIGURATION.md` | P2 |
| DA-ISSUE-003 | Regenerate `corpus_inventory.yaml` against current disk state | P3 |
| DA-ISSUE-004 | Fix DOCUMENTATION_STANDARDS.md self-application (broken internal refs) | P3 |
| DA-ISSUE-005 | Archive resolved OPEN_QUESTIONS.md items | P3 |
| DA-ISSUE-006 | Register `cns.documentation.drift` span type in CNS registry | P3 |
| DA-ISSUE-007 | Remove "Incorporated from:" annotations or convert to provenance block | P3 |

---

*Generated by the hKask documentation alignment audit (TASK 0–TASK 7).*
*2026-07-01 — v0.31.0*