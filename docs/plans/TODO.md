---
title: "hKask TODO — Open Work"
audience: [project maintainers, contributors]
last_updated: 2026-05-25
version: "1.2.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask TODO — Open Work

---

## P0 — Essential (Must Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P0-01** | CNS span emission integration | CNS bot | High | ✅ Complete | `crates/hkask-cns/src/spans.rs` |
| **P0-02** | Git CAS integration for triples | Storage bot | High | ✅ Complete | `crates/hkask-storage/src/git_cas.rs` |
| **P0-03** | CLI/API symmetry audit | CLI bot | High | ✅ Complete | API routes match CLI commands |
| **P0-04** | Documentation quality gates | Curator | High | ✅ Complete | DDMVSS-aligned refresh complete |

---

## P1 — Important (Should Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P1-01** | Requirements specification | Architect | High | ✅ Complete | `docs/specifications/REQUIREMENTS.md` |
| **P1-02** | Traceability matrix | Architect | Medium | ✅ Complete | `docs/specifications/TRACEABILITY_MATRIX.md` |
| **P1-03** | Diagram refresh (DIAGRAMS_INDEX.md) | Curator | Medium | Pending | New diagrams in 4 architecture docs |
| **P1-04** | ADR creation for key decisions | Architect | Medium | Pending | New ADRs in `docs/architecture/adr/` |
| **P1-05** | Link checker script | DevOps | Low | Pending | `docs/ci/check-links.sh` |
| **P1-06** | Citation compliance audit | Curator | Medium | Pending | Grep for uncited sections |
| **P1-07** | Complete stub MCP servers | Dev | Medium | Deferred | condenser, scholar, web (5 LOC each) |
| **P1-08** | Metadata migration for legacy docs | Curator | Low | Pending | Add `ddmvss_categories` to older docs |

---

## P2 — Optional (Nice to Have)

| ID | Task | Owner | Priority | Status |
|----|------|-------|----------|--------|
| **P2-01** | Federation implementation | Federation bot | Low | Deferred to v1.1 |
| **P2-02** | Remote LLM fallback | Inference bot | Low | Deferred |
| **P2-03** | GPU acceleration (CUDA) | Infrastructure | Low | Optional |
| **P2-04** | Qdrant vector search | Storage bot | Low | Contingency |
| **P2-05** | CI automation for doc quality | DevOps | Low | Pending |

---

## Deferred (v1.1+)

| ID | Task | Reason | ADR |
|----|------|--------|-----|
| **D-01** | Git CAS provenance | Not minimal for v1.0 | ADR pending |
| **D-02** | Federation transport | Complexity exceeds budget | ADR pending |
| **D-03** | Remote LLM providers | Local-first invariant | ADR pending |
| **D-04** | Fine-tuning (axolotl) | Not MVP | N/A |
| **D-05** | hkask-surface reactive protocol | Not implemented | OQ-1 |

---

## Completed (2026-05-25 DDMVSS Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-01** | DDMVSS documentation audit (Task 1) | 2026-05-25 | Classification table in refresh plan |
| **R-02** | Archive 10 stale documents (Task 2) | 2026-05-25 | `docs/archive/2026-05-25-documentation-refresh/` |
| **R-03** | Delete 1 duplicate document (Task 2) | 2026-05-25 | `DOCUMENTATION_REFRESH_DDNVSS.md` removed |
| **R-04** | Create DDMVSS_SCAFFOLD.md (Task 3) | 2026-05-25 | `docs/DDMVSS_SCAFFOLD.md` |
| **R-05** | Write 4 DDMVSS-aligned architecture docs (Task 4) | 2026-05-25 | domain-and-capability, interface-and-composition, trust-security-observability, persistence-and-lifecycle |
| **R-06** | Write REQUIREMENTS.md (Task 5) | 2026-05-25 | `docs/specifications/REQUIREMENTS.md` |
| **R-07** | Write TRACEABILITY_MATRIX.md (Task 5) | 2026-05-25 | `docs/specifications/TRACEABILITY_MATRIX.md` |
| **R-08** | Update PROJECT_STATUS.md (Task 7) | 2026-05-25 | Accurate metrics, DDMVSS completeness |
| **R-09** | Write OPEN_QUESTIONS.md (Task 9) | 2026-05-25 | 9 open questions with DDMVSS tags |
| **R-10** | Fix DDMVSS.md stale gaps (Task 8) | 2026-05-25 | hkask-mcp-spec existence, Span::Spec variant |
| **R-11** | Update DOCUMENTATION_STANDARDS.md (Task 8) | 2026-05-25 | TOGAF→DDMVSS metadata migration |
| **R-12** | Add ddmvss_categories to key docs (Task 8) | 2026-05-25 | architecture-master, security-architecture, DDMVSS |

---

## Completed (Prior Sessions)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-01** | Documentation audit | 2026-05-22 | `docs/standards/WRITING_EXCELLENCE_AUDIT.md` |
| **C-02** | Archive stale documents (73 files) | 2026-05-22 | `docs/archive/2026-05-22-documentation-refresh/` |
| **C-03** | Fix hkask-testing compilation failures | 2026-05-22 | All tests passing |
| **C-04** | Resolve clippy warnings | 2026-05-22 | `cargo clippy -- -D warnings` passes |
| **C-05** | Fix cargo fmt issues | 2026-05-22 | `cargo fmt --check` passes |
| **C-06** | Fix hardcoded cryptographic key | 2026-05-22 | `OKAPI_DEV_KEY` const with migration path |
| **C-07** | ADV-REVIEW-F2 security hardening (T01-T22) | 2026-05-24 | `docs/archive/2026-05-25-documentation-refresh/plans/` |

---

## Verification

```bash
# Check P0 completion status
cargo test -p hkask-cns
cargo test -p hkask-storage

# Verify documentation
find docs -type f -name "*.md" ! -path "docs/archive/*" | wc -l
grep -L "^Version:\|^version:" docs/**/*.md 2>/dev/null

# Run quality gates
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

---

*This TODO is the single source of truth for open work. Last updated after 2026-05-22 documentation refresh.*
