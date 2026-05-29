---
title: "hKask Open Questions and Underspecified Aspects"
audience: [architects, developers, decision-makers]
last_updated: 2026-05-29
version: "1.2.0"
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

**Decision:** Document federation as a deferred architectural direction with an ADR. The bidirectional ACP bridge via `RussellAcpAdapter` (606 LOC) provides practical cross-system agent communication without requiring dedicated federation crates. Federation as a first-class concept (separate crates, discovery protocol, resource negotiation) is deferred.

**Rationale:** The Russell ACP bridge demonstrates that inter-system communication works. True federation (discovery, resource negotiation, capability composition across independent hKask instances) is a complexity that exceeds the current 35k LOC budget. See `docs/architecture/deferred/federation.md` (ADR-024).

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

### F1: OCAP Secret Generation vs. HKDF Derivation

**DDMVSS Category:** Trust  
**Status:** Open  

`AgentPod::new()` generates random OCAP secrets via `rand::rng()` when no keystore entry exists. This contradicts ADR-027 (HKDF-SHA256 deterministic derivation). Tradeoff: determinism (cluster-safe, same passphrase → same secrets) vs. forward secrecy (new pod → new random secret).

### F2: Russell ACP Bridge Provenance

**DDMVSS Category:** Trust  
**Status:** Open  

The trust root for cross-system delegation between hKask and Russell via `RussellAcpAdapter` is underspecified. Whether both systems derive from a shared bridge secret or each signs the other's tokens independently.

### F3: Memory Pipeline Completeness

**DDMVSS Category:** Persistence  
**Status:** Open  

The episodic/semantic memory pipeline wiring from `AgentPod` lifecycle events to the bitemporal triple store is incomplete. When should episodic memory begin? Should semantic promotion be automatic or Curator-mediated?

### F4: unwrap() Remediation Priority

**DDMVSS Category:** Trust  
**Status:** Open  

122 remaining `unwrap()` calls down from 139. Approximately 30 are legitimate (lock poisoning in operating-system-backed mutexes). The remainder are distributed across CLI, templates, and storage modules. Should the standard be "zero unwrap on hot paths" with documented exceptions or "zero unwrap period"?

### F5: 41,339 LOC vs. 35K Budget

**DDMVSS Category:** Lifecycle  
**Status:** Open  

The architecture claims a "35k LOC budget" but the actual count is 41,339 (core crates ~29K + MCP servers ~12K). If the budget is for core crates only, the project is within budget. Clarification needed.

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