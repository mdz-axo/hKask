---
title: "hKask Open Questions and Underspecified Aspects"
audience: [architects, developers, decision-makers]
last_updated: 2026-05-25
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [interface, composition, capability, observability, curation, lifecycle]
---

# hKask Open Questions and Underspecified Aspects

**Purpose:** Unresolved aspects requiring decision-making before they can be addressed. Each question is tagged with its DDMVSS category and includes the decision options under consideration.

**Related:** [`DDMVSS.md`](architecture/DDMVSS.md), [`domain-and-capability.md`](architecture/domain-and-capability.md), [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md)

---

## OQ-1: hKask-surface Documentation Depth

**DDMVSS Category:** Interface  
**Status:** Undecided  
**Impact:** Documentation completeness

The `hkask-surface` crate is not present in the current workspace. Reactive surface protocol generation was discussed in earlier architecture documents but has not been implemented.

**Decision options:**
1. Document the concept as a deferred feature with ADR
2. Remove all references to `hkask-surface` from active documentation
3. Implement a minimal surface generation crate

**Recommendation:** Option 2 — remove references, defer to v1.1+ with ADR.

---

## OQ-2: Federation Documentation Scope

**DDMVSS Category:** Composition  
**Status:** Undecided  
**Impact:** Architecture document scope

No federation crates (`hkask-federation`, `hkask-federation-transport`, `hkask-federation-agent`) exist in the current workspace. Federation was discussed in earlier documents but has not been implemented.

**Decision options:**
1. Document federation as a deferred architectural direction with ADR
2. Remove all federation references from active documentation
3. Create stub crates with interface definitions only

**Recommendation:** Option 1 — document as deferred with ADR explaining the complexity/budget tradeoff.

---

## OQ-3: Arsenal Crate Documentation Ownership

**DDMVSS Category:** Capability  
**Status:** Undecided  
**Impact:** Documentation maintenance burden

15 MCP servers exist (scholar removed; was an empty stub). Documentation granularity needs decision.

**Decision options:**
1. Each MCP server gets its own specification entry in REQUIREMENTS.md
2. Document as a catalog with common pattern description and per-crate README
3. Only document fully-implemented servers; stubs listed as deferred

**Recommendation:** Option 2 — catalog approach with common pattern, per-crate README for implemented servers.

---

## OQ-4: Cross-Workspace Dependency Visualization

**DDMVSS Category:** Observability  
**Status:** Undecided  
**Impact:** Architecture documentation quality

Workspace dependency information exists in `Cargo.toml` and can be generated via `cargo metadata`. Whether to promote automated dependency diagrams to first-class architecture artifacts.

**Decision options:**
1. Automated regeneration via CI (cargo-depgraph) into `docs/generated/`
2. Manual Mermaid diagrams updated during documentation refresh
3. Keep as `Cargo.toml` only — no visual representation

**Recommendation:** Option 2 — manual Mermaid in architecture docs (current approach), with CI automation as v1.1+ enhancement.

---

## OQ-5: Automation and Drift Prevention

**DDMVSS Category:** Curation  
**Status:** Undecided  
**Impact:** Long-term documentation quality

The Sourced-Ideas Mandate and Mermaid-First bias create ongoing maintenance burden. Automation to prevent documentation drift needs formalization.

**Decision options:**
1. CI checks for metadata compliance, link integrity, citation density
2. Pre-commit hooks for DIAGRAM_ALIGNMENT verification
3. Periodic agent sweeps (monthly documentation audit)
4. All of the above (comprehensive governance)

**Recommendation:** Option 1 initially (CI checks), expanding to option 4 over time.

---

## OQ-6: ADR Gaps

**DDMVSS Category:** Lifecycle  
**Status:** Undecided  
**Impact:** Decision traceability

Several architectural decisions are implied by code but lack formal ADRs:
- Choice of Argon2id + ChaCha20-Poly1305 for encryption
- ACP server protocol design (JSON-RPC 2.0 over stdio)
- Bitemporal triple schema design
- Unified registry vs. separate registries decision
- 7-level attenuation depth limit rationale

**Decision options:**
1. Create retroactive ADRs for all significant decisions
2. Only create ADRs for future decisions
3. Create ADRs for the top 5 most impactful decisions

**Recommendation:** Option 3 — retroactive ADRs for the 5 most impactful decisions, forward-only for the rest.

---

## OQ-7: Template Refresh

**DDMVSS Category:** Composition  
**Status:** Undecided  
**Impact:** New document quality

Templates in `docs/artifacts/` and `registry/templates/spec/` may need updating to reflect the refreshed document structure and DDMVSS metadata requirements.

**Decision options:**
1. Regenerate templates from best examples in refreshed corpus
2. Keep current templates, add DDMVSS metadata examples
3. Create new DDMVSS-specific template set

**Recommendation:** Option 1 — regenerate from best examples (the four new architecture documents).

---

## OQ-8: hkask-mcp-spec Self-Application

**DDMVSS Category:** Curation  
**Status:** Undecided  
**Impact:** Methodology validation

The `hkask-mcp-spec` MCP server implements 8 DDMVSS tools. Whether the documentation refresh workflow itself should be executed through these tools, establishing a self-application precedent.

**Decision options:**
1. Execute refresh through spec tools (self-application validation)
2. Document the self-application concept without executing it
3. Defer self-application until spec tools are more mature

**Recommendation:** Option 2 — document the concept, defer execution to next refresh cycle.

---

## OQ-9: Stub MCP Server Completion

**DDMVSS Category:** Capability  
**Status:** Deferred  
**Impact:** Feature completeness

Two MCP servers are stubs (5 LOC each): `hkask-mcp-condenser`, `hkask-mcp-web`.

**Decision options:**
1. Implement in next development phase
2. Remove from workspace if not needed for MVP
3. Keep as stubs with documented intent

**Recommendation:** Option 3 — keep as stubs with documented intent, implement in v1.1+.

---

## References

[^ddmvss]: hKask Team. (2026). *DDMVSS — Domain-Driven Minimum Viable Specification Set*. `docs/architecture/DDMVSS.md`.
