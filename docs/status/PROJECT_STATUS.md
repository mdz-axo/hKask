# hKask Project Status

**Version:** 0.21.0  
**Last-Updated:** 2026-05-22  
**Status:** Pre-alpha MVP in progress — Phase 4 (Templates) complete  
**TOGAF Phase:** C — Application  

---

## 1. Executive Summary

hKask (ℏKask — "Planck's Constant of Agent Systems") is a **minimal agent-native container platform** enabling sovereign agents (bots and replicants) to communicate, compose capabilities, and learn through unified template-driven architecture.

**Current Phase:** Phase 4 — Templates complete, bot-mediated subsystems operational  
**Next Phase:** Phase 5 — CNS integration and algedonic alerts

---

## 2. Metrics

### 2.1 Code Metrics

| Metric | Value | Budget | Status |
|--------|-------|--------|--------|
| **Core LOC (Rust)** | ~6,400 | ≤30,000 | 21% used |
| **MCP Server LOC** | ~3,000 | Excluded | — |
| **Test LOC** | ~2,600 | Excluded | — |
| **Total Crates** | 31 | — | 11 core + 19 MCP + 1 test |

### 2.2 Test Metrics

| Workspace | Tests | Status |
|-----------|-------|--------|
| **Core** | 237 passing | ✅ |
| **MCP Servers** | 11 passing | ✅ |
| **Test Crate** | 6 passing | ✅ |
| **Total** | 254 passing | ✅ |

### 2.3 Build Status

| Command | Status | Warnings |
|---------|--------|----------|
| `cargo check --workspace` | ✅ Pass | Minor unused function warnings |
| `cargo test --workspace` | ✅ Pass | 254 tests passing |
| `cargo clippy --workspace -- -D warnings` | ⚠️ Fix needed | Dead code warnings in CLI |
| `cargo fmt --check` | ✅ Pass | — |

---

## 3. Implementation Progress

### 3.1 Completed Phases

| Phase | Description | Status | Date |
|-------|-------------|--------|------|
| **Phase 1** | Security Foundation | ✅ Complete | 2026-05-18 |
| **Phase 2** | Bot System | ✅ Complete | 2026-05-19 |
| **Phase 3** | A2A Protocol | ✅ Complete | 2026-05-19 |
| **Phase 4** | Templates & Registry | ✅ Complete | 2026-05-20 |

### 3.2 Core Crates (11)

| Crate | LOC | Purpose | Status |
|-------|-----|---------|--------|
| `hkask-types` | ~2,000 | ID types, ν-event, hLexicon | ✅ Complete |
| `hkask-storage` | ~4,000 | SQLite + SQLCipher | ✅ Complete |
| `hkask-memory` | ~3,000 | Semantic/episodic pipelines | ✅ Complete |
| `hkask-cns` | ~2,000 | CNS, variety counters | ✅ Complete |
| `hkask-templates` | ~5,000 | Registry, cascade | ✅ Complete |
| `hkask-agents` | ~2,500 | Pods, ACP, manifests | ✅ Complete |
| `hkask-ensemble` | ~1,500 | Multi-agent chat | ✅ Complete |
| `hkask-keystore` | ~1,000 | OS keychain, AES-GCM | ✅ Complete |
| `hkask-mcp` | ~2,500 | MCP runtime, dispatch | ✅ Complete |
| `hkask-cli` | ~2,000 | CLI commands | ✅ Complete |
| `hkask-api` | ~2,000 | HTTP API, utoipa | ✅ Complete |

### 3.3 MCP Servers (19)

| Server | Status | Purpose |
|--------|--------|---------|
| `hkask-mcp-embedding` | ✅ Enabled | Vector generation |
| `hkask-mcp-condenser` | ✅ Enabled | Template abstraction |
| `hkask-mcp-web` | ✅ Enabled | Search, scrape |
| `hkask-mcp-scholar` | ✅ Enabled | Academic research |
| `hkask-mcp-ocap` | ✅ Enabled | Capability management |
| `hkask-mcp-keystore` | ✅ Enabled | Keystore operations |
| `hkask-mcp-cns` | ✅ Enabled | CNS operations |
| `hkask-mcp-git` | ✅ Enabled | Git CAS |
| `hkask-mcp-registry` | ✅ Enabled | Registry operations |
| `hkask-mcp-gml` | ✅ Enabled | GML operations |
| `hkask-mcp-github` | ✅ Enabled | GitHub integration |
| `hkask-mcp-fmp` | ✅ Enabled | FMP integration |
| `hkask-mcp-telnyx` | ✅ Enabled | Telnyx integration |
| `hkask-mcp-fal` | ✅ Enabled | FAL integration |
| `hkask-mcp-rss-reader` | ✅ Enabled | RSS reader |
| `hkask-mcp-inference` | ⚠️ Exists, commented | Okapi LLM |
| `hkask-mcp-storage` | ⚠️ Exists, commented | Storage operations |
| `hkask-mcp-memory` | ⚠️ Exists, commented | Memory pipelines |
| `hkask-mcp-ensemble` | ⚠️ Exists, commented | Chat orchestration |

**Converted to Templates (per AGENTS.md):**
- `hkask-mcp-spandrel` → Graph analysis templates
- `hkask-mcp-doc-knowledge` → Document extraction templates

**Note:** MCP servers are excluded from the 30,000 LOC budget per [`AGENTS.md`](../../AGENTS.md).

---

## 4. Documentation Status

### 4.1 Active Documents

| Category | Count | Location |
|----------|-------|----------|
| **Standards** | 4 | `docs/standards/` |
| **Architecture** | 12 | `docs/architecture/` |
| **Specifications** | 3 | `docs/specifications/` |
| **Plans** | 2 | `docs/plans/` |
| **User Guides** | 5 | `docs/user-guides/` |
| **GML** | 2 | `docs/gml/` |
| **TOGAF Scaffold** | 1 | `docs/TOGAF_LITE_FOR_OPEN_SOURCE.md` |
| **Project Status** | 1 | `docs/status/PROJECT_STATUS.md` |
| **Audit** | 1 | `docs/DOCUMENTATION_AUDIT_2026-05-22.md` |
| **Total** | 31 | — |

### 4.2 Archived Documents

| Category | Count | Location |
|----------|-------|----------|
| **Completion Reports** | 19 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Decision Records** | 11 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Status Reports** | 9 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Remediation Logs** | 8 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Migration Docs** | 5 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Superseded Plans** | 5 | `docs/archive/2026-05-22-documentation-refresh/` |
| **GML Implementation** | 13 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Integrations** | 3 | `docs/archive/2026-05-22-documentation-refresh/` |
| **Total** | 73 | — |

### 4.3 Quality Gates

| Gate | Status | Last Run |
|------|--------|----------|
| **Metadata Headers** | ⚠️ Partial | 2026-05-22 |
| **Citation Compliance** | ⚠️ Needs verification | — |
| **Diagram Alignment** | ⚠️ Needs verification | — |
| **Link Integrity** | ⚠️ Pending | — |
| **Writing Excellence** | ⚠️ Needs audit | — |

---

## 5. Open Work

### 5.1 P0 — Essential

| ID | Task | Owner | Status |
|----|------|-------|--------|
| **P0-01** | CNS span emission integration | CNS bot | In progress |
| **P0-02** | Git CAS integration for triples | Storage bot | In progress |
| **P0-03** | CLI/API symmetry audit | CLI bot | Pending |
| **P0-04** | Documentation quality gates | Curator | In progress |

### 5.2 P1 — Important

| ID | Task | Owner | Status |
|----|------|-------|--------|
| **P1-01** | Technology architecture document | Architect | Pending |
| **P1-02** | Requirements specification | Architect | Pending |
| **P1-03** | Traceability matrix | Architect | Pending |
| **P1-04** | Diagram refresh (DIAGRAMS_INDEX.md) | Curator | Pending |
| **P1-05** | ADR creation for key decisions | Architect | Pending |

---

## 6. Known Issues

| Issue | Severity | Status |
|-------|----------|--------|
| CLI unused function warnings | Low | Fix needed |
| Documentation metadata headers incomplete | Medium | Fix in progress |
| Diagram alignment verification pending | Medium | Fix in progress |
| Link checker script pending | Low | Script to be created |

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
grep -L "^Version:\|^version:" docs/**/*.md 2>/dev/null
.github/scripts/check_links.sh  # To be created

# Line count verification
find crates -name "*.rs" -type f | xargs wc -l
find mcp-servers -name "*.rs" -type f | xargs wc -l
```

---

## 8. References

- [`AGENTS.md`](../../AGENTS.md) — Agent operating guide
- [`DOCUMENTATION_STANDARDS.md`](../standards/DOCUMENTATION_STANDARDS.md) — Documentation standards
- [`TOGAF_LITE_FOR_OPEN_SOURCE.md`](../TOGAF_LITE_FOR_OPEN_SOURCE.md) — TOGAF-Lite scaffold
- [`hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — Master specification

---

*This is the single source of truth for project status. All other status reports reference this document.*

**Next Update:** 2026-05-29 (weekly cadence)
