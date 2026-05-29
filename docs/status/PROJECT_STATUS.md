---
title: "hKask Project Status"
audience: [project maintainers, contributors, stakeholders]
last_updated: 2026-05-29
version: "0.21.4"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Project Status

---

## 1. Executive Summary

hKask (ℏKask — "A Minimal Viable Container for Agents") is a **minimal agent-native container platform** enabling sovereign agents (bots and replicants) to communicate, compose capabilities, and learn through unified template-driven architecture.

**Current Phase:** Phase 8 complete — Documentation refresh (DDMVSS-aligned); documentation portal added and metadata gate fully green 2026-05-29  
**Next Phase:** Operational hardening and stub MCP server completion

---

## 2. Metrics

### 2.1 Code Metrics

| Metric | Value | Status |
|--------|-------|--------|
| **Core LOC (Rust)** | ~40,794 | Measured 2026-05-28 |
| **MCP Server LOC (Rust)** | ~11,178 | Excluded from budget |
| **Total Rust LOC** | ~51,972 | — |
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
| **MCP Servers** | 15 | `hkask-mcp-*` in `mcp-servers/` |
| **Total** | 26 | All in workspace |

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
| `hkask-types` | 5,154 | ID types, ν-event, hLexicon, specs | ✅ Complete |
| `hkask-storage` | 4,010 | SQLite + SQLCipher + sqlite-vec | ✅ Complete |
| `hkask-memory` | 695 | Semantic/episodic pipelines | ✅ Complete |
| `hkask-cns` | 2,039 | CNS, variety counters, algedonic | ✅ Complete |
| `hkask-templates` | 8,259 | Registry, cascade, rendering | ✅ Complete |
| `hkask-agents` | 7,474 | Pods, ACP, bot/replicant | ✅ Complete |
| `hkask-ensemble` | 4,698 | Multi-agent chat | ✅ Complete |
| `hkask-keystore` | 384 | OS keychain, AES-256-GCM | ✅ Complete |
| `hkask-mcp` | 1,911 | MCP runtime, dispatch, security | ✅ Complete |
| `hkask-cli` | 3,741 | CLI commands (14 subcommand groups) | ✅ Complete |
| `hkask-api` | 2,449 | HTTP API (11 route groups), utoipa | ✅ Complete |

### 3.3 MCP Servers (15)

| Server | LOC | Status | Purpose |
|--------|-----|--------|---------|
| `hkask-mcp-inference` | 391 | ✅ Complete | Okapi LLM inference |
| `hkask-mcp-condenser` | 761 | ⚠️ Stub | General-purpose context reranking and condensation |
| `hkask-mcp-web` | 3,389 | ⚠️ Stub | Web search, scrape |
| `hkask-mcp-ocap` | 319 | ✅ Complete | Capability management |
| `hkask-mcp-keystore` | 529 | ✅ Complete | Keystore operations |
| `hkask-mcp-cns` | 280 | ✅ Complete | CNS operations |
| `hkask-mcp-git` | 412 | ✅ Complete | Git CAS |
| `hkask-mcp-registry` | 310 | ✅ Complete | Registry operations |
| `hkask-mcp-gml` | 987 | ✅ Complete | GML allosteric engine |
| `hkask-mcp-spec` | 853 | ✅ Complete | DDMVSS spec tools (8 tools) |
| `hkask-mcp-github` | 459 | ✅ Complete | GitHub integration |
| `hkask-mcp-fmp` | 369 | ✅ Complete | Financial data (FMP) |
| `hkask-mcp-telnyx` | 244 | ✅ Complete | Communications (Telnyx) |
| `hkask-mcp-fal` | 434 | ✅ Complete | Media generation (FAL) |
| `hkask-mcp-rss-reader` | 1,443 | ✅ Complete | RSS feed reader |

**Note:** MCP servers are excluded from count per [`AGENTS.md`](../../AGENTS.md).

---

## 4. Documentation Status

### 4.1 Active Documents (Post Bloat Removal)

| Category | Count | Location |
|----------|-------|----------|
| **Architecture Specs** | 4 | `docs/architecture/` (domain-and-capability, interface-and-composition, trust-security-observability, persistence-and-lifecycle) |
| **Architecture Framework** | 3 | `docs/architecture/` (DDMVSS, PRINCIPLES, magna-carta) |
| **Architecture Index** | 1 | `docs/architecture/hKask-architecture-master.md` |
| **Architecture ADR** | 8 | `docs/architecture/` (ADR-022 through ADR-029; ADR-029 = goal capability primitive) |
| **Reference Artifacts** | 9 | `docs/architecture/reference/` (incl. okapi-integration) |
| **Specifications** | 9 | `docs/specifications/` (REQUIREMENTS, TRACEABILITY, DDMVSS_SCAFFOLD, DOCUMENTATION_STANDARDS, WRITING_EXCELLENCE, DEPENDENCY_POLICY, ADR_TEMPLATE, CI-CD-GUIDE, DEPLOYMENT) |
| **Plans** | 6 | `docs/plans/` (TODO + 5 persona/template drafts: curator, curator-persona, backstory-r7, personas-r7, high-temp-templates) |
| **User Guides** | 2 | `docs/user-guides/` (AGENT-POD-CREATION-GUIDE, COMMON-AGENT-PATTERNS) |
| **GML** | 1 | `docs/gml/` |
| **Status** | 2 | `docs/status/` (PROJECT_STATUS, mcp-server-audit) |
| **Cross-cutting** | 2 | `docs/` root (DIAGRAMS_INDEX, OPEN_QUESTIONS) |
| **Portal** | 1 | `docs/README.md` (documentation portal, indexes all active docs by DDMVSS category) |
| **Artifacts** | 1 | `docs/artifacts/` (README) |
| **Generated** | 1 | `docs/generated/` (cli-reference) |
| **CI Scripts** | 2 | `docs/ci/` (check-links.sh, check-metadata.sh) |
| **Total** | 49 (.md, excl. archive) + 2 CI scripts | — |

### 4.2 Archived Documents

| Archive | Count | Reason |
|---------|-------|--------|
| `2026-05-22-documentation-refresh` | 73 | Initial documentation audit |
| `2026-05-25-documentation-refresh` | 12 | TOGAF→DDMVSS migration |
| `2026-05-25-ddmvss-reset` | 3 | Pre-DDDMVSS docs absorbed into 4 specs |
| `2026-05-25-bloat-removal` | 6 | Content absorbed into DDMVSS specs or stale |
| `2026-05-28-documentation-refresh` | 10 (+ 4 deleted) | Stale/historical docs archived; MODEL_CATALOG, 2 plan files, and 1 other deleted |
| **Total** | 104 | — |

### 4.3 DDMVSS Completeness

| Category | Authoritative Document | Complete? | Curated? |
|----------|----------------------|-----------|----------|
| Domain | `domain-and-capability.md` | ✅ | ✅ |
| Capability | `domain-and-capability.md` | ✅ | ✅ |
| Interface | `interface-and-composition.md` | ✅ | ✅ |
| Composition | `interface-and-composition.md` | ✅ | ✅ |
| Trust & Security | `trust-security-observability.md` | ✅ | ✅ |
| Observability | `trust-security-observability.md` | ✅ | ✅ |
| Persistence | `persistence-and-lifecycle.md` | ✅ | ✅ |
| Lifecycle | `persistence-and-lifecycle.md` | ✅ | ✅ |
| Curation | `DDMVSS.md` + `WRITING_EXCELLENCE.md` | ✅ | ✅ |

**Result:** 9/9 categories satisfied. Corpus is DDMVSS-complete.

### 4.4 Quality Gates

| Gate | Status | Last Run |
|------|--------|----------|
| **Build** (`cargo check --workspace`) | ✅ Pass | 2026-05-29 |
| **Tests** | ✅ Pass | 2026-05-25 |
| **Lint** | ✅ Pass | 2026-05-25 |
| **Format** (`cargo fmt --check`) | ✅ Pass | 2026-05-29 |
| **Metadata Headers** (`docs/ci/check-metadata.sh`) | ✅ 49/49 docs compliant, 0 missing | 2026-05-29 |
| **Citation Compliance** | ✅ New docs have citations | 2026-05-28 |
| **Diagram Alignment** | ✅ 28 diagrams verified in DIAGRAMS_INDEX.md | 2026-05-28 |
| **Link Integrity** (`docs/ci/check-links.sh`) | ✅ 223 links checked, 0 broken, 0 placeholders | 2026-05-29 |

---

## 5. Open Work

### 5.1 P0 — Essential

| ID | Task | Owner | Status |
|----|------|-------|--------|
| **P0-01** | ~~Fix hkask-storage trait mismatches (goals.rs compile errors)~~ — **superseded**: `goals.rs` compiles cleanly; the real defect was a capability-forgery / confused-deputy gap in the goal subsystem (see P0-03) | Storage bot | ✅ Closed (2026-05-29) |
| **P0-02** | Integration tests for inference pipeline | Testing bot | Pending |
| **P0-03** | Harden goal capability subsystem: bind all authority into the HMAC signature, constant-time verify, owner/visibility checks on every write, legal state-transition enforcement, fail-loud persistence read-back | Storage/Security bot | ✅ Complete (2026-05-29) |

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
| None — all compilation errors resolved | — | ✅ Fixed |

**Resolved 2026-05-29 (goal capability hardening, P0-03):**

| Finding | Lens | Resolution |
|---------|------|-----------|
| `GoalCapabilityToken` HMAC omitted `operations` and `expires` → forgeable authority | Security | Signature now binds all authority-bearing fields; constant-time verify |
| Goal write paths verified the token but not the holder's ownership → confused deputy | Capability | Every write enforces `GoalAccess::can_write`/`can_admin` against the holder WebID |
| `update_goal_state` accepted any transition despite an unused `InvalidTransition` variant | Correctness | `GoalState::can_transition_to` total function enforced at the repository boundary |
| `goal_from_row` silently coerced corrupt state/visibility/timestamps to defaults | Persistence | Corruption now surfaces as an error; INSERTs persist RFC3339 `created_at` so timestamps round-trip |
| `delete_goal` panicked via `.expect("mutex lock")` while siblings mapped `LockPoisoned` | Robustness | Unified on the `LockPoisoned` mapping; no panic path remains |

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
