---
title: "hKask Open Questions and Underspecified Aspects"
audience: [architects, developers, decision-makers]
last_updated: 2026-06-07
version: "1.3.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [interface, composition, capability, observability, curation, lifecycle]
---

# hKask Open Questions and Underspecified Aspects

**Purpose:** Unresolved aspects requiring decision-making before they can be addressed. Each question is tagged with its DDMVSS category and includes the decision options under consideration.

**Related:** [`DDMVSS.md`](architecture/DDMVSS.md), [`domain-and-capability.md`](architecture/domain-and-capability.md), [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md)

---

## Resolved Questions (2026-05-29 Sprint)

### OQ-1: hKask-surface Documentation Depth ✅

**DDMVSS Category:** Interface  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Remove all references to `hkask-surface` from active documentation. The concept is deferred to v1.1+ with an ADR if surface generation becomes necessary.

**Rationale:** No `hkask-surface` crate exists. Maintaining references to non-existent crates violates P6 (delete stubs, don't publish them). The MCP/CLI/API equivalence already provides surface generation through utoipa (OpenAPI) and clap (CLI docs).

---

### OQ-2: Federation Documentation Scope ✅

**DDMVSS Category:** Composition  
**Status:** **Resolved — Option 1**  
**Resolution Date:** 2026-05-29

**Decision:** Document federation as a deferred architectural direction (no dedicated ADR yet). The bidirectional ACP bridge via `RussellAcpAdapter` (606 LOC) provides practical cross-system agent communication without requiring dedicated federation crates. Federation as a first-class concept (separate crates, discovery protocol, resource negotiation) is deferred until essential, at which point a forward ADR will record the decision.

**Rationale:** The Russell ACP bridge demonstrates that inter-system communication works. True federation (discovery, resource negotiation, capability composition across independent hKask instances) is a complexity that exceeds the current essential architecture scope. No dedicated federation crate, deferred-design doc, or ADR exists yet; a forward ADR will be authored if/when federation becomes essential. The ACP protocol design was recorded in ADR-028 (now archived — ACP transport layer removed; see [`ADR-028`](architecture/ADR-028-acp-protocol-design.md)).

---

### OQ-3: Arsenal Crate Documentation Ownership ✅

**DDMVSS Category:** Capability  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document MCP servers as a catalog with common pattern description and per-crate README for implemented servers. A unified catalog exists at `docs/status/mcp-tools-inventory.md` (formerly `mcp-server-audit.md`, archived 2026-06-07). Individual README files live in each `mcp-servers/hkask-mcp-*/README.md`.

**Rationale:** Each MCP server having its own specification entry in REQUIREMENTS.md (Option 1) creates 19 × ~2KB = ~38KB of spec overhead — disproportionate. Option 2 keeps the catalog as a single source of truth with per-crate detail for the specific tool surface. Note: The term "arsenal" is not part of the hKask vocabulary — the project has 11 core crates and 21 MCP servers, all in a single workspace.

---

### OQ-4: Cross-Workspace Dependency Visualization ✅

**DDMVSS Category:** Observability  
**Status:** **Resolved — Current approach**  
**Resolution Date:** 2026-05-29

**Decision:** Maintain manual Mermaid dependency diagrams in architecture docs as the primary visualization. CI automation (cargo-depgraph) is a v1.1+ enhancement if dependency complexity warrants it.

**Rationale:** The workspace crate map is stable (11 core + 21 MCP servers). Manual Mermaid in `subsystem-erds.md` §12 and `ports-inventory.md` provides adequate visualization. The DIAGRAM_ALIGNMENT mechanism (PS-09) already catches drift.

---

### OQ-5: Automation and Drift Prevention ✅

**DDMVSS Category:** Curation  
**Status:** **Resolved — Option 1, active**  
**Resolution Date:** 2026-05-29

**Decision:** CI checks for security invariants, constraint compliance, and metadata consistency are active in `.github/workflows/ci.yml` (`security-invariants` job). Further automation (link integrity, citation density, diagram alignment) expands incrementally.

**Rationale:** The `security-invariants` CI job added in this sprint (TASK-08) covers: no `unwrap()` on hot paths, no wildcard capabilities, no hardcoded secrets, no stubs (P6), no deprecated (P7), and no visual UI. This is the foundational tier of Option 1.

---

### OQ-6: ADR Gaps ✅

**DDMVSS Category:** Lifecycle  
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

**DDMVSS Category:** Composition  
**Status:** **Resolved — Deferred**  
**Resolution Date:** 2026-05-29

**Decision:** Defer template regeneration to the next documentation refresh cycle. The current templates in `docs/artifacts/` and `registry/templates/spec/` are functional. DDMVSS metadata requirements are documented in `DDMVSS.md` and the four architecture specifications.

**Update (2026-05-29):** A documentation portal ([`README.md`](README.md)) was added that indexes every active document by DDMVSS category and demonstrates the compliant metadata header. When OQ-7 is taken up, the portal and the four architecture specifications are the recommended "best example" sources from which to regenerate the artifact templates.

---

### OQ-8: hkask-mcp-spec Self-Application ✅

**DDMVSS Category:** Curation  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document the self-application concept without executing it. The `hkask-mcp-spec` tools (8 DDMVSS tools) are validated against the existing specification corpus. Self-application (using spec tools to capture/decompose/curate the spec tools themselves) is deferred to a future meta-curation exercise.

**Rationale:** Self-application is philosophically appealing but introduces circularity that requires careful design. The spec tools work correctly on the existing corpus — that's sufficient for v0.21.0.

---

### OQ-9: Stub MCP Server Completion ✅

**DDMVSS Category:** Capability  
**Status:** **Resolved** (already confirmed 2026-05-28; reaffirmed 2026-05-29 MCP audit)  
**Resolution Date:** 2026-05-28

**Decision:** Both servers are fully implemented: `hkask-mcp-condenser` (761 LOC), `hkask-mcp-web` (3,389 LOC). No stubs remain. MCP tools inventory confirms completeness (see `docs/status/mcp-tools-inventory.md`).

---

## Open Crossroads (Future)

#### F1: OCAP Secret Generation vs. HKDF Derivation ✅ RESOLVED

**DDMVSS Category:** Trust  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** HKDF-SHA256 derivation per WebID (`"hkask:ocap-secret:<webid>"`).

`AgentPod::new()` now calls `derive_ocap_secret(&webid)` which uses `HKDF-SHA256(master_key, "hkask:ocap-secret:" || webid)` to produce a deterministic, per-agent OCAP signing key. This eliminates `SecretRef::Generated` from the pod creation hot path (ADR-027 compliance) while preserving per-agent key isolation (Miller designation).

- Same passphrase + same WebID → same OCAP secret (restart-safe)
- Different WebIDs → cryptographically independent sub-keys (HKDF domain separation)
- No keystore dependency per pod — only the master key needs storage

**See:** `crates/hkask-agents/src/pod/mod.rs::derive_ocap_secret()`, ADR-027

### F2: Russell ACP Bridge Provenance ✅ RESOLVED

**DDMVSS Category:** Trust  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** HKDF-SHA256 derivation with context `"hkask:russell-bridge-secret"`.

`RussellAcpAdapter::new()` now derives the bridge secret from the master key via `SecretRef::derived()`. The constructor no longer takes a raw `bridge_secret` parameter — it resolves the key from HKDF-SHA256(master_key, "hkask:russell-bridge-secret"). Both hKask and Russell must share the same master passphrase and derivation context. Callers updated to remove bridge secret resolution boilerplate.

**See:** `crates/hkask-types/src/secret.rs::RUSSELL_BRIDGE_SECRET`, `crates/hkask-agents/src/adapters/russell_acp.rs::new()`

### F3: Memory Pipeline Completeness ✅ RESOLVED

**DDMVSS Category:** Persistence  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** AgentPod now persists lifecycle events as bitemporal episodic triples on every state transition (Populated→Registered→Activated→Deactivated).

`AgentPod::new_with_memory()` accepts an optional `MemoryStoragePort`. `PodManager::create_pod()` wires its `memory_storage` into pod creation. Each lifecycle method calls `record_lifecycle_event()` which stores `{entity: "pod:{id}", attribute: "lifecycle_state", value: state}` as an episodic_triple with private visibility. Persistence failures are non-fatal (logged with `tracing::warn`).

**See:** `crates/hkask-agents/src/pod/mod.rs::record_lifecycle_event()`

### F4: unwrap() Remediation Priority ✅ RESOLVED

**DDMVSS Category:** Trust  
**Status:** **Resolved**  
**Resolution Date:** 2026-05-29

**Decision:** Standard adopted and enforced: zero `unwrap()` on hot paths; `expect("reason")` preferred over `unwrap()` everywhere; legitimate infallible calls documented with `expect()`.

**Result:** 139 → 0 `unwrap()` calls in production code across all 11 core crates. All 139 converted to `expect("reason")` with explicit invariant documentation. CI `security-invariants` job enforces this permanently.

### F6: Goal Capability — Revocation and Lineage Unification ✅ RESOLVED

**DDMVSS Category:** Trust  
**Status:** **Resolved** — `GoalCapabilityToken` entirely removed in v0.23.0  
**Raised:** 2026-05-29 · **Resolved:** 2026-06-04

**Resolution:** In v0.23.0, `GoalCapabilityToken` was **entirely removed** — the
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

**DDMVSS Category:** Lifecycle  
**Status:** **Deprecated**  
**Resolution Date:** 2026-05-29

**Decision:** The LOC budget has been deprecated. All code budget references removed from `docs/architecture/DDMVSS.md`, `docs/architecture/reference/hKask-erd.md`, and `docs/architecture/PRINCIPLES.md`.

**Replacement discipline:** Every component must be essential and minimal — ask "is this necessary?" before "how big is it?" Code size is an output, not a constraint.

---

### P3-a: ACP Transport Abstraction ⚠️ DEFERRED

**DDMVSS Category:** Interface  
**Status:** Deferred (no current need)  
**Raised:** 2026-05-29 (Loop Distillation)

Current ACP is JSON-RPC 2.0 over stdio (child process). For networked agents or in-process, a transport abstraction would be needed. However, no current consumer requires this — `AcpRuntime` works in-process, `RussellAcpAdapter` works over stdio. When networked ACP becomes necessary, define a transport trait in `hkask-types` and implement for stdio, HTTP, and in-process. (ADR-028, which documented the ACP protocol design, is archived — the transport layer was removed.)

### P3-b: CyberneticsToken/CurationToken Runtime Enforcement ⚠️ DEFERRED

**DDMVSS Category:** Trust  
**Status:** Deferred (structural foundation in place)  
**Raised:** 2026-05-29 (Loop Distillation)

Tokens are now minted at loop construction (9b/9c) but not yet presented to capability gates. The OCAP authority chain exists structurally but is not enforced at runtime. This is by design — the token minting establishes the structural pattern. Runtime enforcement should be added when capability gates are introduced at the point of use (e.g., `ConsolidationBridge` checks for `ConsolidationToken`, `CyberneticsLoop` checks for `CyberneticsToken`).

### P3-d: Episodic vs Semantic Encryption Keys ⚠️ DEFERRED

**DDMVSS Category:** Trust  
**Status:** Deferred (same master key, different visibility enforcement)  
**Raised:** 2026-05-29 (Loop Distillation)

Currently same master key for both. Episodic (private) and semantic (shared) have different threat models. Separate keys would add defense-in-depth but also key management complexity. The current visibility enforcement (`SemanticMemory::store()` rejects non-Shared, `EpisodicMemory::store()` rejects Shared) provides logical separation. Physical separation (separate encryption keys) should be revisited if cross-visibility attacks become a concern.

### P3-e: Loop Membrane Persistence ⚠️ DEFERRED

**DDMVSS Category:** Persistence  
**Status:** Deferred (acceptable data loss for v0.22)  
**Raised:** 2026-05-29 (Loop Distillation)

Loop inboxes and variety counters are in-memory. On crash, all pending directives are lost. For v0.22, this is acceptable — directives are advisory (Curation suggests, doesn't command). If crash resilience becomes critical, add a WAL or periodic checkpoint mechanism to `MessageDispatch` and `VarietyTracker`. Priority: low.

### P3-f: Semantic Loop MCP Server ⚠️ RESOLVED

**DDMVSS Category:** Interface  
**Status:** Resolved — intentional gap  
**Resolution Date:** 2026-06-03

Semantic Memory (Loop 2b) has no direct MCP server. Queries go through `hkask-mcp-cns` or `hkask-mcp-registry`. This is intentional — semantic queries are lower-level than what MCP tools expose. The CNS and Registry servers provide higher-level access patterns that compose semantic memory with other subsystems. Adding a dedicated semantic MCP server would be premature.

### P3-h: CNS Set-point Configuration ⚠️ DEFERRED

**DDMVSS Category:** Interface  
**Status:** Deferred (hardcoded defaults sufficient for v0.22)  
**Raised:** 2026-05-29 (Loop Distillation)

CNS thresholds, gas budgets, variety set-points are currently hardcoded. Need YAML/env configuration for deploy-time tuning. Low priority for v0.22 — defaults work for development. Add `SetPointsConfig` YAML parsing when deployment scenarios require tuning.

### 8g: WebSearchPort Extraction ⚠️ DEFERRED

**DDMVSS Category:** Composition  
**Status:** Deferred (no current consumer outside `hkask-mcp-web`)  
**Resolution Date:** 2026-06-03

`WebSearchPort` trait and `ProviderPool` are only consumed within `mcp-servers/hkask-mcp-web`. No other crate references them. Extracting the trait to `hkask-types` and the pool to a new `hkask-web` crate would be premature — it moves code without enabling new capabilities. If a consumer outside the MCP server needs web search (e.g., a new crate that orchestrates search + memory), extract then. The MCP server becoming a thin shim is the right long-term goal, but not today.

### 9d: AgentKind Behavioral Dispatch ⚠️ RESOLVED — Keep Cosmetic

**DDMVSS Category:** Domain  
**Status:** Resolved — `AgentKind` remains a cosmetic enum  
**Resolution Date:** 2026-06-03

**Decision:** `AgentKind` (Bot/Replicant) remains a simple enum with no behavioral dispatch. Behavioral differences between Bot and Replicant are handled at the call site level (e.g., `chat_with_agent()` selects model based on `AgentKind`, privacy enforcement in `SemanticMemory`/`EpisodicMemory` uses `Visibility`). Converting `AgentKind` to a trait with associated types would change it from a 2-variant enum to a type-level dispatch mechanism, affecting every pod, agent registration, and template selection. The current design correctly separates identity (AgentKind) from behavior (site-level decisions). This is the right granularity for v0.22.

---

### TQ-1: Mechanical vs. LLM Completeness Evaluation

**DDMVSS Category:** Curation  
**Status:** Open  
**Opened:** 2026-06-06

Can `CompletenessCheck::is_complete()` be evaluated mechanically, or does it require LLM-assisted judgment for natural-language goals? If mechanical, implement as `#[test]`. If LLM-assisted, delegate to `CurateEvaluate`. The first tracer-bullet test is: "Given a `GoalSpec` with one criterion `satisfied: true`, `is_complete()` returns `true`." This is mechanical and should pass. Natural-language goals (e.g., "User can chat with agents") require LLM evaluation.

---

### TQ-2: Coherence Threshold Calibration

**DDMVSS Category:** Curation  
**Status:** Open  
**Opened:** 2026-06-06

The 0.7 coherence threshold (`default_coherence_threshold` in `CurationThresholdConfig`) is a starting guess per DDMVSS §9.2 gap #13. What is the empirical coherence score for a well-curated test invariant set? Calibrate after operational data from at least one full crate's test rewrite.

---

### TQ-3: Skill Enforcement vs. Guidance

**DDMVSS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-06

Should skills be enforced mechanically (pre-commit hooks, CI checks, `spec/skill/evaluate` returning violations that block merge) or treated as guidance (curation decisions overridable per sovereignty principle)? The architecture supports both — this is a social contract, not a technical constraint.

---

### TQ-4: Property-Based Testing Boundaries

**DDMVSS Category:** Capability  
**Status:** Open  
**Opened:** 2026-06-06

Where do `proptest` and `cargo fuzz` fit? DDMVSS invariants are natural property candidates (e.g., "\forall CurationDecision, coherence_score in [0, 1]"). But property testing is not tracer-bullet — it's a different cycle. Should it be governed by its own skill (`property-testing`) or folded into the TDD skill as a specialized cycle type?

---

### TQ-5: Integration Test Isolation

**DDMVSS Category:** Composition  
**Status:** Open  
**Opened:** 2026-06-06

MCP server tests require `rmcp` transport. Should integration tests use the existing `McpTestServer` pattern from `hkask-mcp-markitdown`, or should a shared test fixture crate (`hkask-test-utils`) be extracted? Per C4 ("repetition is a missing primitive"), if 3+ MCP servers duplicate test setup, extract. Current count: 2 servers with test modules. Threshold not yet met.

---

### TQ-6: CNS Variety Counters for Test Diversity

**DDMVSS Category:** Observability  
**Status:** Open  
**Opened:** 2026-06-06

Should `cns.test.*` spans track test diversity (number of distinct seams tested per DDMVSS category) and emit algedonic alerts when test variety drops below threshold? This would make test coverage a homeostatic concern — cybernetically coherent, but requires defining thresholds per DDMVSS category.

---

### TQ-7: Skill-Bundler Composition with TDD

**DDMVSS Category:** Composition  
**Status:** Open  
**Opened:** 2026-06-06

When multiple skills are active (skill-bundler), does the TDD cycle apply per-skill (trace each skill's invariants individually) or per-task (trace the composite behavior)? If two skills govern the same behavior, which invariant wins? Resolution: Curation decides, per `CurateReconcile`.

---

### TQ-8: hkask-keystore Has Zero Tests

**DDMVSS Category:** Trust  
**Status:** Open — CRITICAL  
**Opened:** 2026-06-06

`hkask-keystore` is security-critical (AES-256-GCM, HKDF-SHA256, OS keychain integration) and has zero test modules. This is a P0 gap per the test inventory. Priority: behavioral tests at the `Keychain` and `Encryption` seams.

---

### TQ-9: hkask-mcp-spec Has Zero Tests

**DDMVSS Category:** Interface  
**Status:** Open — HIGH  
**Opened:** 2026-06-06

`hkask-mcp-spec` is the DDMVSS governance surface (8+4 tool surfaces) and has zero test modules. Priority: behavioral tests at the `SpecStore` port and `SpecServer` tool handler seams.

---

### Q11: DelegationResource extensibility

**DDMVSS Category:** Capability  
**Status:** Open  
**Opened:** 2026-06-07

`DelegationResource` enum covers 3 of 11 resource patterns in the capability grant table. Remaining 8 are handled by string matching in `capabilities_match()`. Recommendation: add `from_grant_table()` parse method that validates against known resource patterns.

---

## Open Questions from Review Findings (2026-06-07)

*Questions surfaced during the v0.23 documentation review that were not previously tracked in OPEN_QUESTIONS.md.*

### FUT-002: Should `lambda_for_category` be public?

**DDMVSS Category:** Capability  
**Status:** Open  
**Opened:** 2026-06-07

`lambda_for_category` is a private `fn` with a fixed 5-category dispatch table. If the DDMVSS category set expands (as it did from 5 to 9), the dispatch table silently diverges from `SpecCategory::all()`. Recommendation: either make it `pub` with a documented contract, or replace it with `SpecCategory::all()` iteration.

---

### FUT-003: Is `Dampener.override_cooldown` per-issuer or global?

**DDMVSS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-07

Currently, `Dampener.override_cooldown` is global — any issuer's metacognitive override suppresses all subsequent overrides for 120s. This means a low-trust issuer can starve a high-trust issuer. Should cooldown be per-issuer (per-`WebID`)? Per-issuer would be more principled but requires a `HashMap<WebID, Instant>` and introduces memory growth proportional to active issuers.

---

### FUT-004: Is the MCP gateway a membrane for all servers, or a passthrough for some?

**DDMVSS Category:** Capability, Trust  
**Status:** Open  
**Opened:** 2026-06-07

Only 2 of 21 MCP servers currently gate capabilities through `GovernedTool`. The remaining 19 pass tool calls through without OCAP checks. This means the "capability membrane" described in `domain-and-capability.md` §5.5 is selectively permeable. Should all servers gate? Should servers without side effects (e.g., `hkask-mcp-spec` read-only queries) be exempted by design?

---

### FUT-005: Is `SpecId` a brand or a plain `String`?

**DDMVSS Category:** Domain  
**Status:** Open  
**Opened:** 2026-06-07

`SpecId` is currently a type alias, not a newtype. If it is a plain `String`, anyone can construct a `Spec` referencing any other `Spec` without provenance. If it is a brand type (newtype with private constructor), only the spec creation tools can mint valid `SpecId`s. This affects spec composition and curation authority.

---

### FUT-007: Per-issuer override cooldown (sibling of FUT-003)

**DDMVSS Category:** Trust  
**Status:** Open  
**Opened:** 2026-06-07

Sibling question to FUT-003. If FUT-003 resolves as "per-issuer," this question asks: what is the per-issuer cooldown semantics? Should it scale with trust tier? Should it have a ceiling and floor? Blocked by FUT-003's resolution.

---

### FUT-008: Russell bridge revocation granularity — global vs independent

**DDMVSS Category:** Interface  
**Status:** Open  
**Opened:** 2026-06-07

When a Russell bridge is revoked, is revocation global (all bridges for that agent) or independent (per-bridge)? Currently, revocation appears to be all-or-nothing. Independent revocation for multiple bridges is not supported, limiting multi-bridge agent configurations.

---

### FUT-009: Span namespace `cns.cli.*` vs `cns.cybernetics.*` — ADR needed

**DDMVSS Category:** Observability  
**Status:** Open  
**Opened:** 2026-06-07

The canonical CNS span listing in `PRINCIPLES.md` §1.4 uses `cns.cybernetics.*` for some spans, while `AGENTS.md` and code use `cns.cli.*` for CLI-specific spans. This inconsistency should be resolved via an ADR before renaming spans in production code.

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

**DDMVSS completeness:** 8/8 open questions resolved, 1 deferred with documented rationale. F6 resolved post-sprint.

---

## DDMVSS Round 3 Deferred Items

*Tracked per DDMVSS Semantic Alignment Audit (2026-06-06) remediation R11. These items were deferred in DDMVSS §11 Round 3 but not previously tracked in OPEN_QUESTIONS.md.*

| # | Item | Category | Status | Audit Ref |
|---|------|----------|--------|-----------|
| R3.1 | Span::Spec variant gap | Observability | **Resolved** (added in audit) | Audit R1 |
| R3.2 | SpecStore bitemporal semantics | Persistence | **Resolved** (partial) — valid_from/valid_to fields and columns exist. No recorded_at or bitemporal query methods yet. | Audit R14 |
| R3.3 | Spec signing (Ed25519) | Trust | **Resolved** — Ed25519SpecSigner implemented | Audit R12 |
| R3.4 | Spec capability tokens (spec:read, spec:write, spec:compose) | Capability | **Resolved** — CapabilityChecker::grant_spec() implemented | Audit R16 |
| R3.5 | hLexicon spec-curation terms bootstrapping | Domain | **Resolved** (partially bootstrapped) | Audit §2.3 |
| R3.6 | MCP≡CLI≡API cross-surface equivalence test | Interface | ⚠️ Deferred | — |
| R3.7 | Curation authority OCAP boundary integration | Trust | ⚠️ Deferred | — |
| R3.8 | Curation records persistence | Persistence | **Resolved** (partial) — SqliteCurationRecordStore exists. Not yet wired into evaluate(). | Audit R17 |
| R3.9 | Coherence threshold calibration (0.7) | Curation | ⚠️ Deferred (uncalibrated) | — |
| R3.10 | Spec version replacement (post version_sha removal) | Lifecycle | **Resolved** — Spec.version: Option<String> added | Audit R15 |

---

## DDMVSS Audit Remediation Tracking (R4–R18)

*Remediation items from the 2026-06-06 DDMVSS Semantic Alignment Audit that are now resolved but were not previously tracked in this document.*

| # | Item | Category | Status | Audit Ref |
|---|------|----------|--------|----------|
| R4 | DDMVSS §9.1 self-application matrix labels | Observability, Persistence, Lifecycle, Curation | **Resolved** — matrix updated with :partial and :drift labels | Audit R4 |
| R6 | CNS span listing consolidation | Domain | **Resolved** — AGENTS.md and domain-and-capability.md now cross-reference PRINCIPLES.md §1.4 | Audit R6 |
| R8 | TemplateType vocabulary mapping | Composition | **Resolved** — as_spec_name() method added, mapping table documented in interface-and-composition.md §3.3 | Audit R8 |
| R13 | SpecDriftAlert not in CNS loop | Observability | **Resolved** — DefaultSpecCurator dispatches SpecDriftAlert through Communication Loop to CurationLoop inbox | Audit R13 |

---

## References

[^ddmvss]: hKask Team. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.