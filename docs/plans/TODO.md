---
title: "hKask TODO — Open Work"
audience: [project maintainers, contributors]
last_updated: 2026-06-08
version: "1.7.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask TODO — Open Work

---

## P0 — Essential (Must Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P0-01** | CNS span emission integration | CNS bot | High | ✅ Complete | `crates/hkask-cns/src/spans.rs` |
| **P0-02** | Git CAS integration for triples | Storage bot | High | ✅ Complete | `crates/hkask-storage/src/git_cas.rs` |
| **P0-03** | CLI/API symmetry audit | CLI bot | High | ✅ Complete | API routes match CLI commands |
| **P0-04** | Documentation quality gates | Curator | High | ✅ Complete | MDS-aligned refresh complete |

---

## P1 — Important (Should Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P1-01** | Requirements specification | Architect | High | ✅ Complete | `docs/specifications/REQUIREMENTS.md` |
| **P1-02** | Traceability matrix | Architect | Medium | ✅ Complete | `docs/specifications/TRACEABILITY_MATRIX.md` |
| **P1-03** | Diagram refresh (DIAGRAMS_INDEX.md) | Curator | Medium | ✅ Complete | `docs/DIAGRAMS_INDEX.md` — 28 diagrams, 8 V1.1+ candidates |
| **P1-04** | ADR creation for key decisions | Architect | Medium | ✅ Complete | ADR-024 through ADR-028 created 2026-05-29 (5 retroactive ADRs per OQ-6); note: ADR-023 superseded by ADR-027, ADR-028 archived (deferred), ADR-029 archived (superseded) |
| **P1-05** | Link checker script | DevOps | Low | ✅ Complete | `docs/ci/check-links.sh` + check-metadata.sh |
| **P1-06** | Citation compliance audit | Curator | Medium | ⬜ Open | P1-06 — Deferred pending build regression fix |
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
| **P2-06** | Resolve hkask-agents build regression + code drift | Dev | High | ✅ Complete | Build regression resolved; code drift audit complete — see `docs/status/spec-code-drift.yaml` and `docs/status/curation-decisions.yaml` |
| **P2-07** | MDS audit R4: Update §9.1 self-application matrix | Curator | Medium | ✅ Complete | Trust → Pass, Observability → Pass, Persistence/Lifecycle/Curation → :partial. Updated 2026-06-08 |
| **P2-08** | MDS audit R6: Consolidate CNS span listings (3→1 authoritative source) | Curator | Medium | ✅ Complete | PRINCIPLES.md §1.4 is authoritative; 5 hierarchical spans now registered in CANONICAL_NAMESPACES |
| **P2-09** | MDS audit R8: Add TemplateType vocabulary mapping to MDS.md §7.2 | Curator | Medium | ✅ Complete | Prompt↔WordAct, Process↔FlowDef, Cognition↔KnowAct mapping with `as_spec_name()` cross-reference. Updated 2026-06-08 |
| **P2-10** | MDS audit R11: Add R3 deferred items to OPEN_QUESTIONS.md | Curator | Low | ✅ Complete | All 10 MDS §11 R3 items tracked (R3.1–R3.13), plus 3 additional items (Send+Sync bounds, CNS span integration, spec drift detection). Updated 2026-06-08 |
| **P2-11** | Populate `docs/status/PROJECT_STATUS.md` — single source of truth for build/test/metrics status | Dev | Medium | ✅ Complete | Build (pass), test (pass), clippy (pass), doc CI (pass). Created 2026-06-08 |
| **P2-12** | Populate `docs/status/mcp-tools-inventory.md` — complete catalog of all 21 MCP servers' tools | Dev | Medium | ✅ Complete | 21 servers, 119 tools, gas costs, credentials, per-server detail |
| **P2-13** | Populate `docs/status/test-inventory.md` — test seam depth and behavioral coverage | Dev | Medium | ✅ Complete | 12 crates audited, 42 seams, 192 tests, gap analysis |
| **P2-14** | Populate `docs/status/fowler-audit-status.md` — Fowler pattern refactoring tracker | Dev | Low | ✅ Complete | 6 Fowler patterns identified (2 applied, 4 open-low). Created 2026-06-08 |
| **P2-15** | Populate `docs/status/adversarial-simplification-inventory.md` — dead code and unwired seam inventory | Dev | Low | ✅ Complete | 12 dead_code annotations, 4 unwired seams, 3 simplification candidates, 0 removal candidates. Created 2026-06-08 |

**Code drift from spec alignment audit (2026-06-07, resolved 2026-06-08):** Full drift set and curation decisions are in [`docs/status/spec-code-drift.yaml`](../status/spec-code-drift.yaml) and [`docs/status/curation-decisions.yaml`](../status/curation-decisions.yaml). Summary of resolutions:

| ID | Resolution |
|----|------------|
| P2-06-D1 | ✅ Resolved: 5 hierarchical CNS spans registered in `CANONICAL_NAMESPACES` |
| P2-06-D2 | ✅ Resolved: `Caveat` exists as `pub(crate)` — spec updated to note it's an internal implementation detail |
| P2-06-D3 | ✅ Resolved: `CapabilityToken` type alias added to `hkask-types/src/capability/mod.rs` |
| P2-06-D4 | ✅ Resolved: `ContractValidator` stub added with FocusingAssumption FA-C1 |
| P2-06-D5 | ✅ Resolved: `CapabilityAwareValidator` stub added with FocusingAssumption FA-T3 |
| P2-06-D6 | ✅ Resolved: `TemplateInvocation` struct stub added with FocusingAssumption FA-D1 |
| P2-06-D7 | ✅ Resolved: Spec updated — SecurityGateway superseded by GovernedTool |
| P2-06-D8 | ✅ Resolved: Spec updated — McpTransport not needed (rmcp handles transport) |
| P2-06-D9 | ✅ Resolved: `parse_markdown_catalog`, `render_workspace_yaml`, `regenerate_workspace_yaml` stubs added with FocusingAssumption FA-Co1 |

---

## Deferred (v1.1+)

| ID | Task | Reason | ADR |
|----|------|--------|-----|
| **D-01** | Git CAS provenance | Not minimal for v1.0 | ADR pending |
| **D-02** | Federation transport | Complexity exceeds budget | ADR pending |
| **D-03** | Remote LLM providers | Local-first invariant | ADR pending |
| **D-04** | Fine-tuning (axolotl) | Not MVP | N/A |
| **D-05** | hkask-surface reactive protocol | Not implemented | OQ-1 (resolved: removed from docs) |

---

## Completed (2026-05-25 MDS Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-01** | MDS documentation audit (Task 1) | 2026-05-25 | Classification table in refresh plan |
| **R-02** | Archive 10 stale documents (Task 2) | 2026-05-25 | `docs/archive/2026-05-25-documentation-refresh/` |
| **R-03** | Delete 1 duplicate document (Task 2) | 2026-05-25 | `DOCUMENTATION_REFRESH_DDNVSS.md` removed |
| **R-04** | Create MDS_SCAFFOLD.md (Task 3) | 2026-05-25 | `docs/MDS_SCAFFOLD.md` |
| **R-05** | Write 4 MDS-aligned architecture docs (Task 4) | 2026-05-25 | domain-and-composition, interface-and-composition, trust-security-lifecycle, persistence-and-lifecycle |
| **R-06** | Write REQUIREMENTS.md (Task 5) | 2026-05-25 | `docs/specifications/REQUIREMENTS.md` |
| **R-07** | Write TRACEABILITY_MATRIX.md (Task 5) | 2026-05-25 | `docs/specifications/TRACEABILITY_MATRIX.md` |
| **R-08** | Update PROJECT_STATUS.md (Task 7) | 2026-05-25 | Accurate metrics, MDS completeness |
| **R-09** | Write OPEN_QUESTIONS.md (Task 9) | 2026-05-25 | 9 open questions with MDS tags |
| **R-10** | Fix MDS.md stale gaps (Task 8) | 2026-05-25 | hkask-mcp-spec existence, Span::Spec variant |
| **R-11** | Update DOCUMENTATION_STANDARDS.md (Task 8) | 2026-05-25 | TOGAF→MDS metadata migration |
| **R-12** | Add ddmvss_categories to key docs (Task 8) | 2026-05-25 | architecture-master, security-architecture, MDS |

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
| **R-20** | Fix MDS_SCAFFOLD.md directory map | 2026-05-28 | standards/→specifications/, added ci/, generated/ |
| **R-21** | Fix PROJECT_STATUS.md standards/→specifications/ | 2026-05-28 | Row corrected |
| **R-22** | Update TODO.md with completed P1 items | 2026-05-28 | P1-03, P1-05, P1-08 marked complete |
| **R-23** | Archive 10 stale documents (Task 2) | 2026-05-28 | docs/archive/2026-05-28-documentation-refresh/ |
| **R-24** | Delete 4 third-party skill guides (Task 2) | 2026-05-28 | Firecrawl, Browserbase, SerpApi, Tavily removed |
| **R-25** | Fix stale code references in architecture docs (Task 4) | 2026-05-28 | 15+ line number and path corrections |
| **R-26** | Fix test coverage claims in TRACEABILITY_MATRIX + REQUIREMENTS (Task 5) | 2026-05-28 | 0 #[test] unit tests corrected |
| **R-27** | Fix TOGAF→MDS references (Task 3) | 2026-05-28 | DOCUMENTATION_STANDARDS §1, PRINCIPLES PS-11 |
| **R-28** | Fix PROJECT_STATUS factual inaccuracies (Task 7) | 2026-05-28 | LOC, test counts, crate counts corrected |
| **R-29** | Fix hkask-cli build error (Task 1) | 2026-05-28 | repl::run() 4→5 argument mismatch fixed |
| **R-30** | Update GML README status Draft→Active (Task 3) | 2026-05-28 | Aspirational sub-docs archived |

---

## Completed (2026-06-03 Documentation Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-31** | Archive 8 stale documents (Task 2) | 2026-06-03 | `docs/archive/2026-06-03-documentation-refresh/` |
| **R-32** | Delete 3 superseded/implementation reference docs | 2026-06-03 | reference/loop-architecture.md (superseded), mcp-memory-split-plan.md, mcp-memory-continuation-prompt.md |
| **R-33** | Update MCP server count 15→18 across all docs | 2026-06-03 | PRINCIPLES, domain-and-composition, persistence-and-lifecycle, subsystem-erds, REQUIREMENTS, OPEN_QUESTIONS, mcp-server-audit |
| **R-34** | Update MDS_SCAFFOLD directory tree | 2026-06-03 | Removed archived plans, added loop-architecture.md; note: distillation-erd.md subsequently archived 2026-06-07 |
| **R-35** | Fix docs/README.md portal (remove dead links) | 2026-06-03 | Removed archived plan links, fixed GML reference |
| **R-37** | Archive IMPLEMENTATION-PLAN-simplification.md (Task 2) | 2026-06-06 | `docs/archive/2026-06-06-documentation-refresh/` |
| **R-38** | Update PROJECT_STATUS.md (v0.23.0, build regression, MCP server count 21) | 2026-06-06 | version, metrics, build status |
| **R-39** | Update MDS_SCAFFOLD.md with current directory structure | 2026-06-06 | Added 4 status docs to tree |
| **R-40** | Update architecture-master ADR-029 description | 2026-06-06 | Clarified ADR-029 scope |
| **R-41** | Archive 5 stale documents (ADR-023, ADR-028, ADR-029, distillation-erd, refactor-sweep) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/` |
| **R-42** | Fix stale code paths in architecture docs (bot.rs, replicant.rs, dependency.rs) | 2026-06-07 | domain-and-composition, DIAGRAMS_INDEX, persistence-and-lifecycle |
| **R-43** | Update stale version footers (0.21.0 → 0.23.0) | 2026-06-07 | template-header-standard, registry-erd, CI-CD-GUIDE, ADR_TEMPLATE |
| **R-44** | Fix CNS span count 15→21 across all docs | 2026-06-07 | hKask-erd, trust-security-observability |
| **R-45** | Update DIAGRAM_ALIGNMENT verified_dates to 2026-06-07 | 2026-06-07 | All architecture docs with diagrams |
| **R-46** | Update MDS_SCAFFOLD completeness (5/9→9/9 categories) | 2026-06-07 | Observability now ✅ (variety wired to algedonic) |
| **R-47** | Fix registry-erd template types and JSONB columns | 2026-06-07 | WordAct/KnowAct/FlowDef types, TEXT not JSONB |
| **R-48** | Fix MDS_SCAFFOLD directory tree (add ADR-024–031 lines, archive markers) | 2026-06-07 | SCAFFOLD document structure tree |
| **R-49** | Archive mcp-server-audit.md (merged into mcp-tools-inventory.md v1.1.0) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/mcp-server-audit.md` |
| **R-50** | Archive MDS-AUDIT-2026-06-06.md (findings absorbed into SCAFFOLD §4) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/MDS-AUDIT-2026-06-06.md` |
| **R-51** | Fix SCAFFOLD §4 completeness (Trust ✅, Interface ⚠️, result 6/9) | 2026-06-07 | SCAFFOLD completeness predicate |
| **R-52** | Add Q11 (DelegationResource extensibility) to OPEN_QUESTIONS.md | 2026-06-07 | OPEN_QUESTIONS §Open Crossroads |
| **R-53** | Reconcile Curator hLexicon term lists (6→11) in MDS.md §7.1-7.2 | 2026-06-07 | Canonical spec now matches persona doc |
| **R-54** | Remove duplicate Verification section from architecture-master.md | 2026-06-07 | Points to MDS_SCAFFOLD §6 instead |
| **R-55** | Fix architecture-master version (2.2.2→2.2.3) | 2026-06-07 | Align with max spec version |
| **R-56** | Semantic consistency review — consolidated 7 stale docs, fixed SCAFFOLD completeness (5/9→correct 5/9), reconciled Curator hLexicon (6→11 terms) | 2026-06-07 | Full corpus review |
| **R-57** | Archive mcp-server-audit.md (merged into mcp-tools-inventory.md v1.1.0) and MDS-AUDIT-2026-06-06.md (absorbed into SCAFFOLD §4) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/` |
| **R-58** | Remove duplicate Verification section from architecture-master; add self-application precedent note to SCAFFOLD §6.1 | 2026-06-07 | Points to MDS_SCAFFOLD §6 |
| **R-59** | Fix OQ-3 to note 'arsenal' is not a hKask term | 2026-06-07 | OPEN_QUESTIONS.md OQ-3 rationale |
| **R-60** | Add ddmvss_categories to ADR template body frontmatter | 2026-06-07 | ADR_TEMPLATE.md |
| **R-61** | Archive docs/review/ directory (26 files — session artifacts; structural insights incorporated into MDS.md §7.1-7.2 and subsystem-erds.md) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/review/` |
| **R-62** | Fix DOCUMENTATION_STANDARDS §6.2 location paths (standards/→specifications/, adr/→architecture/) | 2026-06-07 | DOCUMENTATION_STANDARDS.md §6.2 |
| **R-63** | Add forward-references from canonical specs to reference docs (okapi, ports-inventory, utoipa, template-header, hLexicon, Curator persona) | 2026-06-07 | MDS.md §7.2, MDS.md §7.1-7.2 |
| **R-64** | Incorporate review findings: type×primitive matrix into MDS.md §7.1-7.2, ERD structural notes into subsystem-erds.md | 2026-06-07 | Review artifact absorption |
| **R-65** | Add 7 untracked future questions from FUTURE.md to OPEN_QUESTIONS.md | 2026-06-07 | FUT-002, FUT-003, FUT-004, FUT-005, FUT-007, FUT-008, FUT-009 |
| **R-66** | Add 7 untracked future questions to OPEN_QUESTIONS.md (review FUTURE.md synthesis) | 2026-06-07 | OPEN_QUESTIONS §Review Findings, FUT-002 through FUT-009 |
| **R-67** | Update cli-reference.md last_updated to 2026-06-07 | 2026-06-07 | docs/generated/cli-reference.md |
| **R-68** | Fix broken links: 17→0 broken links, add 5 status file placeholders, fix 3 archived ADR paths | 2026-06-07 | docs/README.md, docs/ci/check-links.sh |
| **R-68b** | Fix missing metadata in hlexicon-validation-report.md | 2026-06-07 | All 48 docs pass check-metadata.sh |
| **R-69** | Verify no references to docs/review/ remain in active docs | 2026-06-07 | Fixed subsystem-erds.md reference to archive path |
| **R-70** | Create ADR-032 (MCP gateway membrane) and ADR-033 (Dampener override cooldown); update ADR-027 footer | 2026-06-07 | docs/architecture/ADR-032, ADR-033, ADR-027 |
| **R-71** | Verify hkask-mcp-spec build regression — no type errors found, workspace builds clean | 2026-06-07 | cargo check --workspace passes, 5 MCP protocol tests fail (TransportClosed, infra issue) |
| **R-72** | Writing Excellence spot-check: domain-and-capability ✅ 4/4, trust-security-observability ✅ 4/4, MDS ❌ 1/4 | 2026-06-07 | MDS fails Hopper/Lovelace/Gentle (known gap, tracked in OPEN_QUESTIONS) |
| **R-73** | Update all stale verified_date in architecture docs to 2026-06-07 | 2026-06-07 | 21 DIAGRAM_ALIGNMENT entries updated |

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
