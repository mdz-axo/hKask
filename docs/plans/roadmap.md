---
title: "hKask Implementation Roadmap"
audience: [project maintainers, contributors, stakeholders]
last_updated: 2026-05-20
togaf_phase: "E"
version: "1.0.0"
status: "Active"
domain: "Business"
---

<!-- TOGAF_DOMAIN: Business -->
<!-- VERSION: 1.0.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-20 -->

# hKask Implementation Roadmap

**Purpose:** Phase 1-4 implementation timeline, deferred work categorization, and v1.1+ feature candidates.

**Related:** Future work tracked in project backlog  
**TOGAF Phase:** E — Opportunities & Solutions[^togaf-e]

---

## 1. Executive Summary

hKask development follows a phased approach prioritizing minimal viable functionality (v0.21.0 MVP) before operational enhancements (v1.1+) and feature expansion (v1.2+).

**Current State:** v0.21.0 MVP in progress — Phase 5 (Security Hardening & Integration Tests) complete  
**Tests:** 237 passing across workspace

**Timeline:**
- **Phase 1-2:** Core types, storage, CNS (complete)
- **Phase 3-4:** Templates, agents, MCP runtime (complete)
- **Phase 5:** Security hardening, integration tests (complete 2026-05-20)
- **Phase 6:** API/CLI completion (in progress)
- **v1.0 Release:** Pending documentation overhaul, operational hardening

---

## 2. Phase 1-4 Implementation Status

### 2.1 Completed Phases

| Phase | Components | Status | Date |
|-------|------------|--------|------|
| **Phase 1: Core Types** | `hkask-types` (WebID, ν-event, hLexicon, visibility) | ✅ Complete | 2026-04 |
| **Phase 2: Storage** | `hkask-storage` (SQLite, SQLCipher, triples, vectors), `hkask-keystore` | ✅ Complete | 2026-04 |
| **Phase 3: CNS** | `hkask-cns` (variety counters, algedonic alerts, cns.* spans) | ✅ Complete | 2026-05 |
| **Phase 4: Templates** | `hkask-templates` (registry, cascade, hLexicon grounding) | ✅ Complete | 2026-05 |
| **Phase 5: Agents** | `hkask-agents` (pods, ACP, manifests), `hkask-ensemble` | ✅ Complete | 2026-05 |
| **Phase 6: MCP Runtime** | `hkask-mcp` (dispatch, security adapter), 19 MCP servers | ✅ Complete | 2026-05 |
| **Phase 7: Security** | Capability attenuation, path blocking, Jinja2 sanitization | ✅ Complete | 2026-05-20 |
| **Phase 8: Testing** | 237 tests (hkask-types: 50, hkask-cns: 49, hkask-templates: 138) | ✅ Complete | 2026-05-20 |

### 2.2 In Progress

| Component | Status | Blockers | ETA |
|-----------|--------|----------|-----|
| `hkask-cli` | 80% complete | Documentation overhaul | 2026-05-25 |
| `hkask-api` | 60% complete | OpenAPI spec, utoipa integration | 2026-05-30 |
| Documentation | 60% complete | TOGAF Phases E/F pending | 2026-05-25 |

---

## 3. Deferred Work (v1.0 → v1.1)

### 3.1 Registry/Templating Deferred

| Task | Description | Priority | Rationale |
|------|-------------|----------|-----------|
| **Git CAS Bootstrap** | Load templates from Git CAS with provenance | Medium | v1.0: convention-based fixed paths sufficient |
| **Template Provenance** | Track Git SHA, WebID, timestamp per template | Medium | v1.0: in-memory registry adequate |
| **Dependency Graph** | Parse `{% include %}`, detect cycles | High | v1.0: free composition with matroshka limit only |
| **Hot-Reload via Git** | Auto-reload on `kask bot manifest push` | Low | v1.0: explicit `kask template reload` |
| **Cross-Registry Declaration** | Explicit `allowed_callee_types` in templates | Low | v1.0: free composition |

**Reference:** Deferred work tracked in project backlog

### 3.2 Security Model Deferred

| Task | Description | Priority | Rationale |
|------|-------------|----------|-----------|
| **Capability Revocation Lists** | Bloom filter for revoked tokens | Medium | v1.0: short expiration sufficient |
| **Security Adapter Configuration** | Per-deployment policies via YAML | Medium | v1.0: hardcoded patterns adequate |
| **Jinja2 Sandboxing** | minijinja sandbox features | Low | v1.0: pattern blocking sufficient |

**Reference:** [`../architecture/security-architecture.md`](../architecture/security-architecture.md) §7

### 3.3 CNS Operational Deferred

| Task | Description | Priority | Rationale |
|------|-------------|----------|-----------|
| **Variety State Persistence** | SQLite vs in-memory strategy | Medium | v1.0: in-memory adequate for MVP |
| **Algedonic Alert Routing** | Email, webhook, dashboard | Medium | v1.0: CLI `kask cns alerts` only |
| **Curator Intervention Workflow** | Explicit request vs automatic | Low | v1.0: explicit `/escalate` command |

### 3.4 Resolved Questions (2026-05-20)

| Question | Resolution | Implementation Status |
|----------|------------|----------------------|
| Lexicon binding intelligence | Hybrid (exact + LLM fallback) | ✅ Implemented |
| Recursive template composition | Depth-limited (max 3) | ✅ Implemented |
| Dynamic energy cap calibration | Static (manifest-declared) | ✅ Implemented |
| Manifest step grammar extensibility | Hybrid (Rust core + MCP extensions) | ✅ Implemented |
| hLexicon validation timing | Load-time (production), hybrid (dev) | ✅ Implemented |
| Cross-registry composition | Free (MVP), explicit (production) | ✅ Implemented |
| Selector failure handling | External fallback | ✅ Implemented |
| Template hot-reload | Explicit signal (MVP), Git-driven (production) | ✅ Implemented |
| Bootstrap loading order | Convention (MVP), Git CAS (production) | ✅ Implemented |

**Reference:** Resolved items archived in `docs/archive/`

---

## 4. v1.1+ Feature Candidates

### 4.1 Security Enhancements

| Feature | Description | Effort | Impact |
|---------|-------------|--------|--------|
| **Revocation Lists** | Bloom filter for compromised tokens | Medium | High (security) |
| **Per-Deployment Policies** | YAML-configurable security adapter | Low | Medium (flexibility) |
| **Jinja2 Sandboxing** | minijinja sandbox features | Medium | High (security) |

### 4.2 CNS Operationalization

| Feature | Description | Effort | Impact |
|---------|-------------|--------|--------|
| **Variety Persistence** | SQLite-backed variety counters | Medium | High (audit) |
| **Alert Routing** | Email/webhook integration | Medium | High (operations) |
| **CNS Dashboard** | Real-time variety visualization | High | Medium (observability) |

### 4.3 Registry Enhancements

| Feature | Description | Effort | Impact |
|---------|-------------|--------|--------|
| **Git CAS Integration** | Full Git-backed template storage | High | High (versioning) |
| **Dependency Graph UI** | Visualize template composition | Medium | Low (DX) |
| **Provenance Tracking** | Git SHA, WebID, timestamp | Medium | Medium (audit) |

### 4.4 v1.2+ Candidates (Not Committed)

| Feature | Description | Rationale |
|---------|-------------|-----------|
| **Checkpoint Fallback** | Failure recovery for long cascades | v1.0: fail fast |
| **Multi-Trigger Escalation** | Variety + confidence + timeout | v1.0: explicit `/escalate` only |
| **Embedding Model Versioning** | Awareness in similarity comparisons | v1.0: Embedding MCP responsibility |
| **Curator Retirement Workflow** | Handoff to user-created replicant | v1.0: Curator fixed |

---

## 5. Migration Timeline

### 5.1 Terminology Migration (v0.21.0 → v0.22.0)

| Deprecated Term | Replacement | Status |
|-----------------|-------------|--------|
| νKask | CNS (Cybernetic Nervous System) | ✅ Code updated, docs pending archival |
| OKH spans | `cns.*` spans | ✅ Complete |
| Three registries | Unified registry with `template_type` | ✅ Complete |
| Feedback crate | CNS spans | ✅ Complete |

**Verification:**
```bash
# Check for remaining deprecated terms
grep -r "νKask\|OKH\|three registries" docs/ --include="*.md" --exclude-dir=archive
# Expected: Only vKask-*.md files (to be archived)
```

### 5.2 Document Lifecycle Migration

| Action | Files | Status |
|--------|-------|--------|
| Archive superseded | `vKask-cybernetic-constant.md`, `vKask-erd.md` | ⏳ Pending (Task 7) |
| Add metadata headers | 18 documents | ⏳ Deferred |
| Add citations | 16 documents | ⏳ Deferred |
| Add DIAGRAM_ALIGNMENT | 6 diagrams | ⏳ Deferred |

---

## 6. Success Metrics

### 6.1 v1.0 Release Criteria

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| **Test Coverage** | ≥200 tests | 237 | ✅ Pass |
| **TOGAF Coverage** | 9 phases | 7 of 10 (70%) | ⚠️ In progress |
| **Writing Excellence** | ≥80% passing | 92% | ✅ Pass |
| **Security Hardening** | All P0 tasks | Complete | ✅ Pass |
| **Documentation Quality** | 3 of 4 dimensions | 77% | ✅ Pass |

### 6.2 Operational Metrics (v1.1+)

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Template Selection Latency** | <100ms (fast model) | `cns.prompt.select` span duration |
| **Cascade Execution Time** | <5s (95th percentile) | `cns.prompt.outcome` span duration |
| **Variety Deficit** | <50 (normal operation) | CNS variety counter |
| **Algedonic Alerts** | <1/day (stable system) | `nu_events.algedonic_alert = TRUE` |

---

## 7. References

[^togaf-e]: The Open Group. (2011). *TOGAF Standard, Version 9.1*. Phase E: Opportunities & Solutions. <https://pubs.opengroup.org/architecture/togaf9-doc/arch/chap16.html>.

---

*This roadmap is updated quarterly. Next review: 2026-08-20.*

**Next:** Task 3.7 — Create `migration/strategy.md` (TOGAF Phase F).
