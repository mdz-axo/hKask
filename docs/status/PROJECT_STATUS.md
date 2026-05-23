# hKask Project Status

**Version:** 0.21.0  
**Last-Updated:** 2026-05-23  
**Status:** Pre-alpha MVP — Build passing, 32 tests passing, Phase 2 & 3 complete  
**TOGAF Phase:** C — Application / D — CNS Integration  

---

## 1. Executive Summary

hKask (ℏKask — "Planck's Constant of Agent Systems") is a **minimal agent-native container platform** enabling sovereign agents (bots and replicants) to communicate, compose capabilities, and learn through unified template-driven architecture.

**Current Phase:** Phase 6 — Okapi Integration Hardening & Ensemble/CNS complete  
**Next Phase:** Phase 7 — Multi-agent chat coordination, full CNS span integration

---

## 2. Metrics

### 2.1 Code Metrics

| Metric | Value | Status |
|--------|-------|--------|
| **Core LOC (Rust)** | ~25,800 | — |
| **MCP Server LOC (Rust)** | Included in count | — |
| **Total Rust LOC** | ~25,800 | — |
| **Excluded** | Jinja2 templates, YAML manifests | Not counted |

### 2.2 Test Metrics

| Workspace | Tests | Status |
|-----------|-------|--------|
| **Core Crates** | 32 passing | ✅ |
| **Total** | 32 passing | ✅ |

### 2.3 Build Status

| Command | Status | Warnings |
|---------|--------|----------|
| `cargo check --workspace` | ✅ Pass | None |
| `cargo test --workspace` | ✅ Pass | 32 tests passing |
| `cargo clippy --workspace -- -D warnings` | ✅ Pass | None |
| `cargo fmt --check` | ✅ Pass | — |

### 2.4 Workspace Structure

| Component | Count | Description |
|-----------|-------|-------------|
| **Core Crates** | 11 | `hkask-*` in `crates/` |
| **MCP Servers** | 19 | `hkask-mcp-*` in `mcp-servers/` |
| **Test Crate** | 1 | `hkask-testing` |
| **Total** | 31 | All in workspace |

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
| `hkask-types` | ~2,000 | ID types, ν-event, hLexicon | ✅ Complete |
| `hkask-storage` | ~4,000 | SQLite + SQLCipher | ✅ Complete |
| `hkask-memory` | ~3,000 | Semantic/episodic pipelines | ✅ Complete |
| `hkask-cns` | ~2,000 | CNS, variety counters | ✅ Complete |
| `hkask-templates` | ~5,000 | Registry, cascade | ✅ Complete |
| `hkask-agents` | ~2,500 | Pods, ACP, manifests | ✅ Complete |
| `hkask-ensemble` | ~3,500 | Multi-agent chat, CNS spans | ✅ Complete |
| `hkask-keystore` | ~1,000 | OS keychain, AES-GCM | ✅ Complete |
| `hkask-mcp` | ~2,500 | MCP runtime, dispatch | ✅ Complete |
| `hkask-cli` | ~2,500 | CLI commands | ✅ Complete |
| `hkask-api` | ~2,500 | HTTP API, utoipa | ✅ Complete |

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
| `hkask-mcp-inference` | ✅ Enabled | Okapi LLM |
| `hkask-mcp-storage` | ⚠️ Exists, commented | Storage operations |
| `hkask-mcp-memory` | ⚠️ Exists, commented | Memory pipelines |
| `hkask-mcp-ensemble` | ⚠️ Exists, commented | Chat orchestration |

**Converted to Templates (per AGENTS.md):**
- `hkask-mcp-spandrel` → Graph analysis templates
- `hkask-mcp-doc-knowledge` → Document extraction templates

**Note:** MCP servers are excluded from count per [`AGENTS.md`](../../AGENTS.md).

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
| **P0-01** | Fix hkask-storage/src/goals.rs trait mismatches | Storage bot | Pending |
| **P0-02** | Integration tests for inference pipeline | Testing bot | Pending |

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
