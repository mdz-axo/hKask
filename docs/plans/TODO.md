---
title: "hKask TODO — Open Work"
audience: [project maintainers, contributors]
last_updated: 2026-06-03
version: "1.4.0"
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
| **P1-03** | Diagram refresh (DIAGRAMS_INDEX.md) | Curator | Medium | ✅ Complete | `docs/DIAGRAMS_INDEX.md` — 28 diagrams, 8 V1.1+ candidates |
| **P1-04** | ADR creation for key decisions | Architect | Medium | ✅ Complete | ADR-024 through ADR-028 created 2026-05-29 (5 retroactive ADRs per OQ-6) |
| **P1-05** | Link checker script | DevOps | Low | ✅ Complete | `docs/ci/check-links.sh` + check-metadata.sh |
| **P1-06** | Citation compliance audit | Curator | Medium | Pending | P1-06 — Spot check passed, full audit deferred |
| **P1-07** | Complete stub MCP servers | Dev | Medium | ✅ Complete | hkask-mcp-condenser: 761 LOC, hkask-mcp-web: 3,389 LOC (verified 2026-05-28) |
| **P1-08** | Metadata migration for legacy docs | Curator | Low | ✅ Complete | All 47 active docs have ddmvss_categories (2026-05-28) |

---

## P2 — Optional (Nice to Have)

| ID | Task | Owner | Priority | Status |
|----|------|-------|----------|--------|
| **P2-01** | Federation implementation | Federation bot | Low | Deferred to v1.1 |
| **P2-02** | Remote LLM fallback | Inference bot | Low | Deferred |
| **P2-03** | GPU acceleration (CUDA) | Infrastructure | Low | Optional |
| **P2-04** | Qdrant vector search | Storage bot | Low | Contingency |
| **P2-05** | CI automation for doc quality | DevOps | Low | ✅ Complete | docs/ci/check-links.sh + check-metadata.sh operational |

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

## Completed (2026-05-28 Documentation Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-13** | Fix AGENTS.md path rot (3 broken references) | 2026-05-28 | hKask-erd.md→reference/, registry-templating→interface-and-composition, P0_OKAPI→reference/okapi-integration |
| **R-14** | Fix README.md path rot (3 broken references) | 2026-05-28 | Same path corrections as R-13 |
| **R-15** | Fix hKask-architecture-master.md duplicate line | 2026-05-28 | okapi-integration.md listed twice → once |
| **R-16** | Create docs/ci/check-links.sh + check-metadata.sh | 2026-05-28 | Both scripts operational, passing |
| **R-17** | Run metadata check — 34 docs missing ddmvss_categories | 2026-05-28 | `docs/ci/check-metadata.sh` |
| **R-18** | Batch-add ddmvss_categories to all 34 docs | 2026-05-28 | All 47 active docs now pass metadata check |
| **R-19** | Create DIAGRAMS_INDEX.md (28 diagrams, 8 V1.1+ candidates) | 2026-05-28 | `docs/DIAGRAMS_INDEX.md` |
| **R-20** | Fix DDMVSS_SCAFFOLD.md directory map | 2026-05-28 | standards/→specifications/, added ci/, generated/ |
| **R-21** | Fix PROJECT_STATUS.md standards/→specifications/ | 2026-05-28 | Row corrected |
| **R-22** | Update TODO.md with completed P1 items | 2026-05-28 | P1-03, P1-05, P1-08 marked complete |
| **R-23** | Archive 10 stale documents (Task 2) | 2026-05-28 | docs/archive/2026-05-28-documentation-refresh/ |
| **R-24** | Delete 4 third-party skill guides (Task 2) | 2026-05-28 | Firecrawl, Browserbase, SerpApi, Tavily removed |
| **R-25** | Fix stale code references in architecture docs (Task 4) | 2026-05-28 | 15+ line number and path corrections |
| **R-26** | Fix test coverage claims in TRACEABILITY_MATRIX + REQUIREMENTS (Task 5) | 2026-05-28 | 0 #[test] unit tests corrected |
| **R-27** | Fix TOGAF→DDMVSS references (Task 3) | 2026-05-28 | DOCUMENTATION_STANDARDS §1, PRINCIPLES PS-11 |
| **R-28** | Fix PROJECT_STATUS factual inaccuracies (Task 7) | 2026-05-28 | LOC, test counts, crate counts corrected |
| **R-29** | Fix hkask-cli build error (Task 1) | 2026-05-28 | repl::run() 4→5 argument mismatch fixed |
| **R-30** | Update GML README status Draft→Active (Task 3) | 2026-05-28 | Aspirational sub-docs archived |

---

## Completed (2026-06-03 Documentation Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-31** | Archive 8 stale documents (Task 2) | 2026-06-03 | `docs/archive/2026-06-03-documentation-refresh/` |
| **R-32** | Delete 3 superseded/implementation reference docs | 2026-06-03 | reference/loop-architecture.md (superseded), mcp-memory-split-plan.md, mcp-memory-continuation-prompt.md |
| **R-33** | Update MCP server count 15→18 across all docs | 2026-06-03 | PRINCIPLES, domain-and-capability, persistence-and-lifecycle, subsystem-erds, REQUIREMENTS, OPEN_QUESTIONS, mcp-server-audit |
| **R-34** | Update DDMVSS_SCAFFOLD directory tree | 2026-06-03 | Removed archived plans, added loop-architecture.md, distillation-erd.md |
| **R-35** | Fix docs/README.md portal (remove dead links) | 2026-06-03 | Removed archived plan links, fixed GML reference |
| **R-36** | Fix hKask-architecture-master.md references | 2026-06-03 | Removed superseded reference/loop-architecture.md, added distillation-erd.md |

---

## Completed (Prior Sessions)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-01** | Documentation audit | 2026-05-22 | `docs/specifications/WRITING_EXCELLENCE.md` |
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

*This TODO is the single source of truth for open work. Last updated after 2026-06-03 documentation refresh.*
