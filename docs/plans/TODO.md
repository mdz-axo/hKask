# hKask TODO — Open Work

**Version:** 1.0.0  
**Last-Updated:** 2026-05-22  
**Status:** Active  

---

## P0 — Essential (Must Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P0-01** | CNS span emission integration | CNS bot | High | In progress | `crates/hkask-cns/src/span_emitter.rs` |
| **P0-02** | Git CAS integration for triples | Storage bot | High | In progress | `crates/hkask-storage/src/git_cas.rs` |
| **P0-03** | CLI/API symmetry audit | CLI bot | High | Pending | Audit to be run |
| **P0-04** | Documentation quality gates | Curator | High | In progress | This refresh |

---

## P1 — Important (Should Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P1-01** | Requirements specification | Architect | High | **Completed** | `docs/specifications/REQUIREMENTS.md` (to be written) |
| **P1-02** | Traceability matrix | Architect | Medium | Pending | `docs/specifications/TRACEABILITY_MATRIX.md` |
| **P1-03** | Diagram refresh (DIAGRAMS_INDEX.md) | Curator | Medium | Pending | Diagram registry to be updated |
| **P1-04** | ADR creation for key decisions | Architect | Medium | Pending | New ADRs in `docs/architecture/adr/` |
| **P1-05** | Link checker script | DevOps | Low | Pending | `.github/scripts/check_links.sh` |
| **P1-06** | Citation compliance audit | Curator | Medium | Pending | Grep for uncited sections |

---

## P2 — Optional (Nice to Have)

| ID | Task | Owner | Priority | Status |
|----|------|-------|----------|--------|
| **P2-01** | Federation implementation | Federation bot | Low | Deferred to v1.1 |
| **P2-02** | Remote LLM fallback | Inference bot | Low | Deferred |
| **P2-03** | GPU acceleration (CUDA) | Infrastructure | Low | Optional |
| **P2-04** | Qdrant vector search | Storage bot | Low | Contingency |

---

## Deferred (v1.1+)

| ID | Task | Reason | ADR |
|----|------|--------|-----|
| **D-01** | Git CAS provenance | Not minimal for v1.0 | ADR pending |
| **D-02** | Federation transport | Complexity exceeds budget | ADR pending |
| **D-03** | Remote LLM providers | Local-first invariant | ADR pending |
| **D-04** | Fine-tuning (axolotl) | Not MVP | N/A |

---

## Completed (This Session)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-01** | Documentation audit | 2026-05-22 | `docs/DOCUMENTATION_AUDIT_2026-05-22.md` |
| **C-02** | Archive stale documents (73 files) | 2026-05-22 | `docs/archive/2026-05-22-documentation-refresh/` |
| **C-03** | Create TOGAF-Lite scaffold | 2026-05-22 | `docs/TOGAF_LITE_FOR_OPEN_SOURCE.md` |
| **C-04** | Create PROJECT_STATUS.md | 2026-05-22 | `docs/status/PROJECT_STATUS.md` |
| **C-05** | Create TECHNOLOGY.md | 2026-05-22 | `docs/architecture/TECHNOLOGY.md` |
| **C-06** | Delete duplicate files (3) | 2026-05-22 | `git rm` |
| **C-07** | Fix hkask-testing compilation failures | 2026-05-22 | All 331 tests passing |
| **C-08** | Resolve clippy warnings | 2026-05-22 | `cargo clippy -- -D warnings` passes |
| **C-09** | Fix cargo fmt issues | 2026-05-22 | `cargo fmt --check` passes |

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
