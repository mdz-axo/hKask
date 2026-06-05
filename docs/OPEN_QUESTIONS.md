---
title: "hKask Open Questions and Underspecified Aspects"
audience: [architects, developers, decision-makers]
last_updated: 2026-05-29
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

**Rationale:** The Russell ACP bridge demonstrates that inter-system communication works. True federation (discovery, resource negotiation, capability composition across independent hKask instances) is a complexity that exceeds the current essential architecture scope. No dedicated federation crate, deferred-design doc, or ADR exists yet; a forward ADR will be authored if/when federation becomes essential. The ACP protocol design is recorded in [`ADR-028`](architecture/ADR-028-acp-protocol-design.md).

---

### OQ-3: Arsenal Crate Documentation Ownership ✅

**DDMVSS Category:** Capability  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document MCP servers as a catalog with common pattern description and per-crate README for implemented servers. A unified catalog exists at `docs/status/mcp-server-audit.md`. Individual README files live in each `mcp-servers/hkask-mcp-*/README.md`.

**Rationale:** Each MCP server having its own specification entry in REQUIREMENTS.md (Option 1) creates 19 × ~2KB = ~38KB of spec overhead — disproportionate. Option 2 keeps the catalog as a single source of truth with per-crate detail for the specific tool surface.

---

### OQ-4: Cross-Workspace Dependency Visualization ✅

**DDMVSS Category:** Observability  
**Status:** **Resolved — Current approach**  
**Resolution Date:** 2026-05-29

**Decision:** Maintain manual Mermaid dependency diagrams in architecture docs as the primary visualization. CI automation (cargo-depgraph) is a v1.1+ enhancement if dependency complexity warrants it.

**Rationale:** The workspace crate map is stable (11 core + 19 MCP servers). Manual Mermaid in `subsystem-erds.md` §12 and `ports-inventory.md` provides adequate visualization. The DIAGRAM_ALIGNMENT mechanism (PS-09) already catches drift.

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
5. **ADR-028**: ACP Protocol Design — JSON-RPC 2.0 over stdio for agent communication

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

**Decision:** Both servers are fully implemented: `hkask-mcp-condenser` (761 LOC), `hkask-mcp-web` (3,389 LOC). No stubs remain. MCP server audit `docs/status/mcp-server-audit.md` confirms completeness.

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
**Status:** **Resolved** — `GoalCapabilityToken` entirely removed in v0.22.0  
**Raised:** 2026-05-29 · **Resolved:** 2026-06-04

**Resolution:** In v0.22.0, `GoalCapabilityToken` was **entirely removed** — the
type, its HMAC signing, epoch-based revocation, and attenuation were all
deleted. Goal operations now use `&WebID` for owner scoping instead of token
verification. The entire token infrastructure (HMAC, revocation, attenuation,
ADR-029) was removed as over-engineered ceremony with no functional payoff.

All sub-questions are **moot**:

1. **Revocation** — No longer applicable; no token to revoke. Owner scoping
   via `WebID` is the authority mechanism.
2. **API/MCP parity** — ✅ Previously resolved; parity still holds with `WebID`.
3. **Operation-set canonicalization encoding** — No longer applicable; no
   token signature to bind an operation set.
4. **Single vs. dual capability primitive** — No longer applicable;
   `GoalCapabilityToken` no longer exists. ADR-029 is superseded.
5. **Persistence corruption response** — Remains relevant for
   `GoalRepositoryError::Corrupt` but is decoupled from token concerns.
6. **Recursion-bound coherence** — `SYSTEM_MAX_ATTENUATION` still applies to
   `CapabilityToken` attenuation, but the goal-specific recursion question is
   moot.

**See:** `crates/hkask-storage/src/goals.rs`, `crates/hkask-cli/src/commands/goal.rs`, `crates/hkask-api/src/routes/goal.rs`, `mcp-servers/hkask-mcp-goal/src/main.rs`, `docs/architecture/reference/subsystem-erds.md` §13, ADR-025. ~~ADR-029 is superseded (the `GoalCapabilityToken` type no longer exists).~~

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

Current ACP is JSON-RPC 2.0 over stdio (child process). For networked agents or in-process, a transport abstraction would be needed. However, no current consumer requires this — `AcpRuntime` works in-process, `RussellAcpAdapter` works over stdio. When networked ACP becomes necessary, define a transport trait in `hkask-types` and implement for stdio, HTTP, and in-process.

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

## References

[^ddmvss]: hKask Team. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.