---
title: "hKask Project Status"
audience: [project maintainers, contributors, stakeholders]
last_updated: 2026-06-07
version: "0.23.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Project Status

---

## 1. Executive Summary

hKask (ℏKask - "A Minimal Viable Container for Agents") is a **minimal agent-native container platform** enabling sovereign agents (bots and replicants) to communicate, compose capabilities, and learn through unified template-driven architecture.

**Current Phase:** Phase 9 complete — DDMVSS-aligned documentation refresh; 5 stale documents archived (ADR-023, ADR-028, ADR-029, distillation-erd, refactor-sweep)
**Next Phase:** Resolve build regression in `hkask-mcp-spec`, complete documentation quality gates

---

## 2. Metrics

### 2.1 Code Metrics

| Metric | Value | Status |
|--------|-------|--------|
| **Core LOC (Rust)** | ~57,730 | Measured 2026-06-05 |
| **MCP Server LOC (Rust)** | ~12,099 | Excluded from budget |
| **Total Rust LOC** | ~69,829 | — |

> **Note:** LOC counts are from the last clean measurement (2026-06-05). A build regression in `hkask-agents` (5 compile errors related to `SovereigntyChecker` / `ConsentManager` import) prevents a fresh count as of 2026-06-06.
| **Excluded** | Jinja2 templates, YAML manifests | Not counted |

### 2.2 Test Metrics

| Workspace | Tests | Status |
|-----------|-------|--------|
| **Core Crates** | Unit + integration tests across crates (incl. goal-capability forgery, confused-deputy, and lifecycle-transition tests added 2026-05-29) plus doctest blocks | ✅ |
| **MCP Servers** | 0 | — |
| **Total** | `cargo test --workspace` green (0 failures) | ✅ |

### 2.3 Build Status

| Command | Status | Warnings |
|---------|--------|----------|
| `cargo check --workspace` | ✅ Pass | None |
| `cargo test --workspace` | ✅ Pass | 3 doctests ok, 3 ignored |
| `cargo clippy --workspace -- -D warnings` | ✅ Pass | None |
| `cargo fmt --check` | ✅ Pass | — |

### 2.4 Workspace Structure

| Component | Count | Description |
|-----------|-------|-------------|
| **Core Crates** | 11 | `hkask-*` in `crates/` |
| **MCP Servers** | 21 | `hkask-mcp-*` in `mcp-servers/` (incl. `hkask-mcp-goal`, `hkask-mcp-ensemble`, `hkask-mcp-episodic`, `hkask-mcp-semantic`, `hkask-mcp-doc-knowledge`, `hkask-mcp-markitdown`) |
| **Total** | 32 | All in workspace |

---

## 3. Implementation Progress

### 3.1 Completed Phases

| Phase | Description | Status | Date |
|-------|-------------|--------|------|
| **Phase 1** | Security Foundation | ✅ Complete | 2026-05-18 |
| **Phase 2** | Bot System | ✅ Complete | 2026-05-19 |
| **Phase 3** | A2A Protocol | ✅ Complete | 2026-05-19 |
| **Phase 4** | Templates & Registry | ✅ Complete | 2026-05-20 |
| **Phase 5** | Security Hardening & Testing | ✅ Complete | 2026-05-22 |
| **Phase 6** | Okapi Integration Hardening | ✅ Complete | 2026-05-23 |
| **Phase 7** | Ensemble & CNS Integration | ✅ Complete | 2026-05-23 |
| **Phase 8** | CLI/API Commands | ✅ Complete | 2026-05-23 |

### 3.2 Core Crates (11)

| Crate | LOC | Purpose | Status |
|-------|-----|---------|--------|
| `hkask-types` | 7,673 | ID types, ν-event, hLexicon, specs | ✅ Complete |
| `hkask-storage` | 4,771 | SQLite + SQLCipher + sqlite-vec | ✅ Complete |
| `hkask-memory` | 2,005 | Semantic/episodic pipelines | ✅ Complete |
| `hkask-cns` | 5,432 | CNS, variety counters, algedonic | ✅ Complete |
| `hkask-templates` | 3,529 | Registry, cascade, rendering | ✅ Complete |
| `hkask-agents` | 10,945 | Pods, ACP, bot/replicant | ✅ Complete |
| `hkask-ensemble` | 3,246 | Multi-agent chat | ✅ Complete |
| `hkask-keystore` | 619 | OS keychain, AES-256-GCM | ✅ Complete |
| `hkask-mcp` | 1,801 | MCP runtime, dispatch, security | ✅ Complete |
| `hkask-cli` | 12,151 | CLI commands (25 subcommand groups) | ✅ Complete |
| `hkask-api` | 5,558 | HTTP API (18 route groups), utoipa | ✅ Complete |

### 3.3 MCP Servers (21)

| Server | LOC | Status | Purpose |
|--------|-----|--------|---------|
| `hkask-mcp-inference` | 328 | ✅ Complete | Okapi LLM inference |
| `hkask-mcp-condenser` | 866 | ⚠️ Stub | Context condensation (reranking and compression of the active conversation window) |
| `hkask-mcp-web` | 3,185 | ⚠️ Stub | Web search, scrape |
| `hkask-mcp-ocap` | 315 | ✅ Complete | Capability management |
| `hkask-mcp-keystore` | 497 | ✅ Complete | Keystore operations |
| `hkask-mcp-cns` | 401 | ✅ Complete | CNS operations |
| `hkask-mcp-git` | 308 | ✅ Complete | Git CAS |
| `hkask-mcp-registry` | 294 | ✅ Complete | Registry operations |
| `hkask-mcp-spec` | 853 | ✅ Complete | DDMVSS spec tools (8 tools) |
| `hkask-mcp-github` | 451 | ✅ Complete | GitHub integration |
| `hkask-mcp-fmp` | 367 | ✅ Complete | Financial data (FMP) |
| `hkask-mcp-telnyx` | 240 | ✅ Complete | Communications (Telnyx) |
| `hkask-mcp-fal` | 414 | ✅ Complete | Media generation (FAL) |
| `hkask-mcp-rss-reader` | 1,432 | ✅ Complete | RSS feed reader |
| `hkask-mcp-goal` | 287 | ✅ Complete | Goal coordination substrate (OCAP-gated, CNS-observed); mirrors CLI/API |
| `hkask-mcp-ensemble` | 391 | ✅ Complete | Multi-agent chat MCP server |
| `hkask-mcp-episodic` | 219 | ✅ Complete | Episodic memory MCP server |
| `hkask-mcp-semantic` | 437 | ✅ Complete | Semantic memory MCP server |

**Note:** MCP servers are excluded from count per [`AGENTS.md`](../../AGENTS.md).

---

## 4. Documentation Status

### 4.1 Active Documents (Post Bloat Removal)

| Category | Count | Location |
|----------|-------|----------|
| **Architecture Specs** | 4 | `docs/architecture/` (domain-and-capability, interface-and-composition, trust-security-observability, persistence-and-lifecycle) |
| **Architecture Framework** | 4 | `docs/architecture/` (DDMVSS, PRINCIPLES, loop-architecture, magna-carta) |
| **Architecture Index** | 1 | `docs/architecture/hKask-architecture-master.md` |
| **Architecture ADR** | 7 (+ 3 archived) | `docs/architecture/` (ADR-022, ADR-024–027, ADR-030, ADR-031 active; ADR-023 superseded by ADR-027, ADR-028 deferred, ADR-029 superseded) |
| **Reference Artifacts** | 6 (+ 1 archived) | `docs/architecture/reference/` (distillation-erd.md archived; canonical ERDs remain active) |
| **Specifications** | 9 | `docs/specifications/` |
| **Plans** | 2 | `docs/plans/` (TODO, high-temp-templates) |
| **User Guides** | 2 | `docs/user-guides/` |
| **Status** | 5 | `docs/status/` (PROJECT_STATUS, mcp-tools-inventory, test-inventory, fowler-audit-status, adversarial-simplification-inventory) |
| **Cross-cutting** | 2 | `docs/` root (DIAGRAMS_INDEX, OPEN_QUESTIONS) |
| **Portal** | 1 | `docs/README.md` |
| **Generated** | 2 | `docs/generated/` (cli-reference, openapi.json) |
| **CI Scripts** | 2 | `docs/ci/` (check-links.sh, check-metadata.sh) |
| **Total** | 42 (.md, excl. archive) + 2 CI scripts | — |

### 4.2 Archived Documents

| Archive | Count | Reason |
|---------|-------|--------|
| `2026-05-22-documentation-refresh` | 73 | Initial documentation audit |
| `2026-05-25-documentation-refresh` | 12 | TOGAF→DDMVSS migration |
| `2026-05-25-ddmvss-reset` | 3 | Pre-DDDMVSS docs absorbed into 4 specs |
| `2026-05-25-bloat-removal` | 6 | Content absorbed into DDMVSS specs or stale |
| `2026-05-28-documentation-refresh` | 10 (+ 4 deleted) | Stale/historical docs archived; MODEL_CATALOG, 2 plan files, and 1 other deleted |
| `2026-06-01-documentation-refresh` | 11 | Audit artifacts + speculative 10-loop feedback-loops-decomposition archived |
| `2026-06-03-documentation-refresh` | 17 | 3 reference docs, 5 plan docs, 9 GML docs archived |
| `2026-06-06-documentation-refresh` | 1 | IMPLEMENTATION-PLAN-simplification.md archived |
| `2026-06-07-documentation-refresh` | 7 | ADR-023 (superseded by ADR-027), ADR-028 (deferred), ADR-029 (superseded), distillation-erd.md (applied), refactor-sweep-2026-06-06.md (captured elsewhere), mcp-server-audit.md (merged into mcp-tools-inventory.md v1.1.0), DDMVSS-AUDIT-2026-06-06.md (absorbed into SCAFFOLD §4) |
| **Total** | 139 | — |

### 4.3 DDMVSS Completeness

| Category | Authoritative Document | Complete? | Curated? |
|----------|----------------------|-----------|----------|
| Domain | `domain-and-capability.md` | ✅ | ✅ |
| Capability | `domain-and-capability.md` | ✅ | ✅ |
| Interface | `interface-and-composition.md` | ⚠️ MCP≡CLI≡API equivalence axiom unverified (OQ R3.6) | ✅ Merge |
| Composition | `interface-and-composition.md` | ✅ | ✅ |
| Trust & Security | `trust-security-observability.md` | ✅ | ✅ |
| Observability | `trust-security-observability.md` | ✅ | ✅ |
| Persistence | `persistence-and-lifecycle.md` | ⚠️ SpecStore not bitemporal | ✅ Merge |
| Lifecycle | `persistence-and-lifecycle.md` | ⚠️ SpecStore not bitemporal | ✅ Merge |
| Curation | `DDMVSS.md` + `WRITING_EXCELLENCE.md` | ⚠️ Coherence threshold uncalibrated | ✅ Merge |

**Result:** 5/9 categories fully confirmed (Domain, Capability, Trust, Composition, Observability). 4/9 have partial gaps. Remaining gaps tracked in `OPEN_QUESTIONS.md`.

### 4.4 Quality Gates

| Gate | Status | Last Run |
|------|--------|----------|
| **Build** (`cargo check --workspace`) | ❌ 2 compile errors in `hkask-mcp-spec` | 2026-06-07 |
| **Tests** | ⚠️ Blocked by build regression | 2026-06-07 |
| **Lint** | ✅ Pass | 2026-05-25 |
| **Format** (`cargo fmt --check`) | ✅ Pass | 2026-05-29 |
| **Metadata Headers** (`docs/ci/check-metadata.sh`) | ✅ 44/44 docs compliant, 0 missing (5 archived 2026-06-07) | 2026-06-07 |
| **Citation Compliance** | ✅ New docs have citations | 2026-06-07 |
| **Diagram Alignment** | ✅ 33 diagrams verified in DIAGRAMS_INDEX.md | 2026-06-07 |
| **Link Integrity** (`docs/ci/check-links.sh`) | ✅ 223 links checked, 0 broken, 0 placeholders | 2026-06-07 |

---

## 5. Open Work

### 5.1 P0 — Essential

| ID | Task | Owner | Status |
|----|------|-------|--------|
| **P0-01** | ~~Fix hkask-storage trait mismatches (goals.rs compile errors)~~ — **superseded**: `goals.rs` compiles cleanly; the real defect was a capability-forgery / confused-deputy gap in the goal subsystem (see P0-03) | Storage bot | ✅ Closed (2026-05-29) |
| **P0-02** | Integration tests for inference pipeline | Testing bot | Pending |
| **P0-03** | Harden goal capability subsystem: bind all authority into the HMAC signature, constant-time verify, owner/visibility checks on every write, legal state-transition enforcement, fail-loud persistence read-back | Storage/Security bot | ✅ Complete (2026-05-29) |
| **P0-04** | Wire goal subsystem into CLI (`kask goal create|list|set-state`) over the shared encrypted DB, with denials emitted to the `NuEventStore` CNS sink (originally ADR-029; ADR-029 archived) | CLI bot | ✅ Complete (2026-05-29) |
| **P0-05** | Goal subsystem API/MCP parity: HTTP routes (`/api/goals`, `/api/goals/{id}/state`) + `hkask-mcp-goal` server (`goal_create`/`goal_list`/`goal_set_state`), satisfying MCP ≡ CLI ≡ API (REQ-IFC-001) | API/MCP bot | ✅ Complete (2026-05-29) |

### 5.2 P1 — Important

| ID | Task | Owner | Status |
|----|------|-------|--------|
| **P1-01** | Phase 4: Production documentation | Curator | Pending |
| **P1-02** | Performance optimization | Performance bot | Pending |
| **P1-03** | Deployment guide | DevOps bot | Pending |

### 5.3 Completed (Phase 2 & 3)

| ID | Task | Owner | Status |
|----|------|-------|--------|
| **P2-01** | Ensemble multi-agent chat coordination | Ensemble bot | ✅ Complete |
| **P2-02** | CNS span integration across all components | CNS bot | ✅ Complete |
| **P2-03** | Confidence escalation spans | Ensemble bot | ✅ Complete |
| **P2-04** | Variety monitoring & algedonic alerts | CNS bot | ✅ Complete |
| **P3-01** | CLI commands (kask chat, kask pod) | CLI bot | ✅ Complete |
| **P3-02** | HTTP API endpoints (templates, bots, pods, CNS, sovereignty) | API bot | ✅ Complete |
| **P3-03** | Ensemble API endpoints (chat, deliberation) | API bot | ✅ Complete |
| **P3-04** | SOAP inference endpoint for Russell | API bot | ✅ Complete |

---

## 6. Known Issues

| Issue | Severity | Status |
|-------|----------|--------|
| 5 compile errors in `hkask-agents` (SovereigntyChecker import, DenyAllConsent, Arc import, missing consent field) | High | ⬜ Open (shifted: now `hkask-mcp-spec` has 2 type errors in main.rs:727,796) |

**Resolved 2026-05-29 (goal capability hardening, P0-03):**

| Finding | Lens | Resolution |
|---------|------|-----------|
| `GoalCapabilityToken` HMAC omitted `operations` and `expires` → forgeable authority | Security | `GoalCapabilityToken` removed in v0.23.0 — goal operations now use WebID-based owner scoping |
| Goal write paths verified the token but not the holder's ownership → confused deputy | Capability | `GoalCapabilityToken` removed in v0.23.0; every write enforces `GoalAccess::can_write`/`can_admin` against the holder WebID |
| `update_goal_state` accepted any transition despite an unused `InvalidTransition` variant | Correctness | `GoalState::can_transition_to` total function enforced at the repository boundary |
| `goal_from_row` silently coerced corrupt state/visibility/timestamps to defaults | Persistence | Corruption now surfaces as an error; INSERTs persist RFC3339 `created_at` so timestamps round-trip |
| `delete_goal` panicked via `.expect("mutex lock")` while siblings mapped `LockPoisoned` | Robustness | Unified on the `LockPoisoned` mapping; no panic path remains |
| Goal subsystem had no live surface (telemetry seam unused) | Interface/Observability | Wired into CLI via `kask goal` with the `NuEventStore` denial sink; OQ-F6 resolved (`GoalCapabilityToken` removed) |

---

## 7. Verification Commands

```bash
# Build verification
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check

# Documentation verification
find docs -type f -name "*.md" ! -path "docs/archive/*" | wc -l
bash docs/ci/check-metadata.sh   # mandatory metadata headers
bash docs/ci/check-links.sh      # link integrity

# Line count verification
find crates -name "*.rs" -type f | xargs wc -l
find mcp-servers -name "*.rs" -type f | xargs wc -l
```

---

## 8. References

- [`AGENTS.md`](../../AGENTS.md) — Agent operating guide
- [`DOCUMENTATION_STANDARDS.md`](../specifications/DOCUMENTATION_STANDARDS.md) — Documentation standards
- [`DDMVSS_SCAFFOLD.md`](../specifications/DDMVSS_SCAFFOLD.md) — DDMVSS category → directory mapping
- [`hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — Master specification
- [`domain-and-capability.md`](../architecture/domain-and-capability.md) — Domain & Capability architecture
- [`interface-and-composition.md`](../architecture/interface-and-composition.md) — Interface & Composition architecture
- [`trust-security-observability.md`](../architecture/trust-security-observability.md) — Trust, Security & Observability architecture
- [`persistence-and-lifecycle.md`](../architecture/persistence-and-lifecycle.md) — Persistence & Lifecycle architecture
- [`REQUIREMENTS.md`](../specifications/REQUIREMENTS.md) — Requirements specification
- [`TRACEABILITY_MATRIX.md`](../specifications/TRACEABILITY_MATRIX.md) — Traceability matrix

---

*This is the single source of truth for project status. All other status reports reference this document.*

**Next Update:** 2026-06-05 (weekly cadence)
