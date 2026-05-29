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

**Rationale:** Each MCP server having its own specification entry in REQUIREMENTS.md (Option 1) creates 15 × ~2KB = ~30KB of spec overhead — disproportionate. Option 2 keeps the catalog as a single source of truth with per-crate detail for the specific tool surface.

---

### OQ-4: Cross-Workspace Dependency Visualization ✅

**DDMVSS Category:** Observability  
**Status:** **Resolved — Current approach**  
**Resolution Date:** 2026-05-29

**Decision:** Maintain manual Mermaid dependency diagrams in architecture docs as the primary visualization. CI automation (cargo-depgraph) is a v1.1+ enhancement if dependency complexity warrants it.

**Rationale:** The workspace crate map is stable (11 core + 15 MCP servers). Manual Mermaid in `subsystem-erds.md` §12 and `ports-inventory.md` provides adequate visualization. The DIAGRAM_ALIGNMENT mechanism (PS-09) already catches drift.

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

### F6: Goal Capability — Revocation and Lineage Unification ⚠️ OPEN

**DDMVSS Category:** Trust  
**Status:** **Open** (surfaced by the 2026-05-29 goal-capability hardening, P0-03)  
**Raised:** 2026-05-29

The goal-capability subsystem was hardened (authority bound into the HMAC,
constant-time verify, owner/visibility checks on writes, legal-transition
enforcement, fail-loud read-back). Several aspects remain underspecified and are
deliberately **not** pre-built (P5 — code not needed today is debt):

1. **Revocation.** `GoalCapabilityToken` carries only `expires`; there is no way
   to revoke a leaked token before expiry. Options: short-TTL-only (current),
   an epoch counter folded into the HMAC, or Miller-style revocable forwarders
   (membranes). Decision needed before goal tokens cross trust boundaries.
2. **Operation-set canonicalization encoding.** The signature now binds a
   sorted, deduplicated, length-delimited operation list. Whether to switch to
   a stable bitset (smaller, ordering-free by construction) should be recorded
   in an ADR if the `GoalOp` set grows.
3. **Single vs. dual capability primitive.** `GoalCapabilityToken` now mirrors
   the canonical `CapabilityToken` (shared `SYSTEM_MAX_ATTENUATION`,
   `can_attenuate()`), but remains a distinct type. Whether to collapse it into
   a typed projection over `CapabilityToken` (true OCAP lineage unification,
   including root-nonce chain verification) is an architectural decision
   warranting its own ADR.
4. **Persistence corruption response.** Corruption now surfaces as
   `GoalRepositoryError::Corrupt`; the system-level policy (quarantine, CNS
   algedonic alert, repair) is unspecified.
5. **Recursion-bound coherence.** Attenuation depth, template cascade depth, and
   subgoal depth all use "7". Confirm whether these should reference one shared
   constant (`SYSTEM_MAX_ATTENUATION`) rather than three coincidental literals.

**See:** `crates/hkask-types/src/goal_capability.rs`, `crates/hkask-storage/src/goals.rs`, `docs/architecture/reference/subsystem-erds.md` §13, ADR-025.

### F5: 41,339 LOC vs. 35K Budget ✅ DEPRECATED

**DDMVSS Category:** Lifecycle  
**Status:** **Deprecated**  
**Resolution Date:** 2026-05-29

**Decision:** The LOC budget has been deprecated. All code budget references removed from `docs/architecture/DDMVSS.md`, `docs/architecture/reference/hKask-erd.md`, and `docs/architecture/PRINCIPLES.md`.

**Replacement discipline:** Every component must be essential and minimal — ask "is this necessary?" before "how big is it?" Code size is an output, not a constraint.

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

**DDMVSS completeness:** 8/8 open questions resolved, 1 deferred with documented rationale.

---

## References

[^ddmvss]: hKask Team. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.