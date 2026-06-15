# Handoff: Plans Audit + Next Steps — 2026-06-15

---

## 1. Session Context

Conducted a comprehensive audit of all 8 plans in `docs/plans/` against current project state (`PROJECT_STATUS.md`, `TODO.md`, build/test results). Identified the only fully-implemented plan (`r7.3-public-seam-watcher`), archived it, and corrected stale statuses in 3 other plans. The session ended with a prioritized next-steps roadmap. **No code changes were made** — this was a documentation/planning session only.

**Build health at session start:** All 18 workspace members compile. 35/35 CNS tests pass. 413 tests total, 396 REQ tags. CI inventory gate passes.

---

## 2. What Was Done

### Plans Audit (8 plans, 7 remaining after archive)

| Plan | Assessment |
|------|-----------|
| `TODO.md` | Living document, ~95% current. P0 and P1 all complete, P2 nearly complete (1 item open). No changes needed. |
| `DOCUMENT_ROADMAP.md` | Draft. Fixed P0 duplicate (P0-2 `corpus_inventory.yaml` already exists, marked done; P0-3 renamed to `curation_decisions.yaml`). Updated Quick Wins (QW-2 done, added status column). Date bumped to 2026-06-15. |
| `code-quality-impact-execution-plan-v0.27.0.md` (Phase 1) | Proposed, nothing started. 6 waves / 10 tasks. |
| `code-quality-impact-execution-plan-phase-2-v0.27.0.md` (Phase 2) | In Progress. Only Task 1/1.1 (public seam inventory) done. Tasks 2-10 open. |
| `code-quality-impact-execution-plan-phase-3-v0.27.0.md` (Phase 3) | Proposed, nothing started. 6 waves / 10 tasks. |
| `mcp-server-roadmap.md` | Active. Fixed §7 discrepancies — tasks 2 and 3 were `⬜ Open` but §3 showed RAG pipeline complete. Now both marked `✅ Complete (2026-06-13)`. Remaining open: media value-add (§2.3), test-utils (§6). |
| `pragmatic-audit-implementation-plan-v0.27.0.md` | Status updated from `Proposed` → `In Progress — Wave 1 R1 ✅ (19 tests)`. Metrics updated: hkask-communication 0→19 tests, REQ tags 345→396. Waves 2-6 not started. |

### Archived

- `docs/plans/r7.3-public-seam-watcher-v0.28.0.md` → `docs/archive/2026-06-15-r7.3-seam-watcher/`
  - All 5 waves implemented + 5 adversarial gaps fixed. 14 files changed, 35/35 CNS tests pass. Completion documented in `PROJECT_STATUS.md`.

---

## 3. What Remains

### HIGH — Critical P8/C8 gaps (Prohibition-level from pragmatic audit)

**R2: `hkask-agents` tests (8 → ≥20)**
- File: `crates/hkask-agents/`
- Current: 8 tests for 77-seam deep module (depth mismatch)
- Target: ≥20 REQ-tagged tests covering mode transitions, pod constraints, public seam groups
- Skill: `pragmatics` for architectural overview → `tdd` for red-green-refactor
- Validation: `cargo test -p hkask-agents`

**R3: `hkask-mcp` tests (5 → ≥15)**
- File: `crates/hkask-mcp/`
- Current: 5 tests for security-critical dispatch (Gate-3, capability, auth boundaries)
- Target: ≥15 REQ-tagged tests covering auth, capability assignment, tool dispatch
- Note: `hkask-mcp` has a pre-existing tracing macro issue — `cargo check` clean but with warnings
- Validation: `cargo test -p hkask-mcp`

**Code-Quality Phase 1 — Task 1: Uniform MCP Gate-3 capability verification**
- File: All 9 MCP server mains (`mcp-servers/hkask-mcp-*/src/main.rs`)
- Current: Startup gate inconsistency creates OCAP drift risk
- Target: Every MCP server applies auth + assignment + capability checks consistently
- Plan reference: `docs/plans/code-quality-impact-execution-plan-v0.27.0.md` §Wave 1
- Skill: `refactor-service-layer` (extract shared verifier helper into `hkask-mcp` or `hkask-services`)
- Validation: `cargo test -p hkask-mcp`; `cargo check --workspace`

### MEDIUM — Quick Wins + Code quality

**DOCUMENT_ROADMAP Quick Wins** (all <15 min each):
- QW-1: Fix `skill-inventory.md` `mds_categories`: `status` → `curation` (1 min)
- QW-3: Fix `corpus_inventory.yaml` — remove check-links.sh + check-metadata.sh from `missing_referenced` (5 min)
- QW-4: Bump 14 document versions → 0.27.0 (15 min)
- QW-5: Create `docs/status/spec-code-drift.yaml` stub (15 min)
- QW-6: Create `docs/status/curation-decisions.yaml` stub (15 min)

**Code-Quality Phase 2 — Task 2: Property/mutation testing for critical parsers**
- Plan: `docs/plans/code-quality-impact-execution-plan-phase-2-v0.27.0.md` §Task 2
- Target seams: capability/token parsing, span namespace parsing, settings merge
- Skill: `tdd`

### LOW — Documentation hygiene + deferred work

**DOCUMENT_ROADMAP P1 items:**
- P1-2: Fix `skill-inventory.md` mds_categories (same as QW-1)
- P1-3: Fix 18 version anomalies (overlaps with QW-4)
- P1-4: Create `docs/ci/sync-versions.sh`
- P1-5: Update `MDS_SCAFFOLD.md` document structure

**MCP Server Roadmap remaining:**
- §2.2: Communication STT, voice design, verbal mode templates
- §2.3: Media value-add layer (image pipelines, batch ops, media library)
- §6: Extract `hkask-test-utils` (deferred — C4 threshold not met)

---

## 4. Recommended Skills and Tools

### Primary Skills for Next Session

| Skill | Why | When |
|-------|-----|------|
| **`pragmatics`** | Ground the next agent in the full P1–P12 principle hierarchy and the current architectural state (`hKask-architecture-master.md`). Essential before touching any code — the principles are the design constraints. | Activate first, before any implementation. |
| **`grill-me`** | Stress-test design assumptions before committing to code. For R2 (agents tests), grill: "What behavioral properties justify 77 public seams with only 8 tests?" For R3 (mcp tests): "What security invariants are currently untested in the MCP dispatch layer?" For Phase 1 Task 1: "Is a shared MCP verifier helper actually needed across all 9 servers, or would per-server checks be simpler?" | Activate before each task to surface assumptions. |
| **`refactor-service-layer`** | For Code-Quality Phase 1 Task 1: extracting the shared MCP startup verifier. For pragmatic audit R9: continuing strangler fig extraction of mid-migration domains. Strangler fig pattern — old path delegates, new path parity, no premature deletion. | Activate when touching cross-crate extraction work. |
| **`condenser-continuation`** | Restore full context if this handoff is read after a context reset. Verifies build health, prioritizes remaining tasks. | Activate on session start if context was reset. |
| **`coding-guidelines`** + **`tdd`** | Standard discipline: Think Before Coding, Simplicity First, Surgical Changes, Goal-Driven Execution. Vertical tracer-bullet RED→GREEN→REFACTOR with `// REQ:` tags. | Activate for all code changes. |
| **`essentialist`** | For any "should I delete this?" or "is this module deep enough?" decisions. Enforces the 3-gate challenge (Exist → Surface → Contract). | Activate when simplifying or removing code. |

### Verification Commands to Run Before Starting Any Task

```bash
# Ensure codebase is still healthy
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Check current state of target crates
cargo test -p hkask-agents -- --list    # should show ~8 tests
cargo test -p hkask-mcp -- --list       # should show ~5 tests
grep -r "// REQ:" crates/hkask-agents/src/ | wc -l
grep -r "// REQ:" crates/hkask-mcp/src/ | wc -l

# P8 traceability gate
bash scripts/audit/public-seam-inventory.sh --check
```

---

## 5. Key Decisions to Preserve

1. **R7.3 is done and archived.** The seam watcher is fully integrated into the CNS cybernetics loop with algedonic escalation. Do not re-open or re-implement it. The archive path is `docs/archive/2026-06-15-r7.3-seam-watcher/`.

2. **Pragmatic audit R1 is satisfied** (hkask-communication: 19 tests, exceeding the ≥10 target). Do not add more communication tests unless a specific gap is identified by the grill-me skill.

3. **REQ tags are now 396, approaching the >400 target.** The pragmatic audit's "all waves" REQ tag target (400) is nearly met. Additional REQ-tagged tests should focus on the R2/R3 target crates (hkask-agents, hkask-mcp), not general coverage inflation.

4. **The Code-Quality Phase 1 plan and pragmatic-audit plan overlap on MCP server quality.** Phase 1 Task 1 (uniform Gate-3 verification) and pragmatic audit R3 (hkask-mcp tests) should be coordinated — improving MCP tests creates the foundation for verifying uniform gate behavior. Consider doing R3 before Phase 1 Task 1.

5. **MCP server roadmap §7 is now accurate.** Tasks 2 and 3 (RAG design + embed integration) are complete as part of the docproc merger. End-to-end Q/R/G is future work tracked in §3.2, not a §7 task. When updating the roadmap, update both the body (§3) and the summary matrix (§7) together — they drifted once already.

6. **DOCUMENT_ROADMAP.md P0-2 is done** (corpus_inventory.yaml exists). P0-3 was a duplicate — it was renamed to `curation_decisions.yaml` and is still pending (QW-6).

7. **The handoff directory was empty before this file.** This is the first handoff document. Future handoffs should follow the same format: `docs/handoffs/[brief-description]-[YYYY-MM-DD].md` with ≤12 char description.

---

## Immediate Next Action (Recommended Priority)

Start with **QW-1 through QW-6** (all under 15 min each, total ~60 min) to clear the DOCUMENT_ROADMAP quick wins. Then proceed to the high-priority work:

1. **QW-1 → QW-3 → QW-5 → QW-6** (document stubs, 1–15 min each)
2. **Activate `grill-me`** on the R2/R3/P1-T1 priority stack to surface design assumptions
3. **Activate `pragmatics`** to ground in architectural principles before touching code
4. **R2 (hkask-agents tests)** — highest P8/C8 gap. 8 tests for 77 public seams.
5. **R3 (hkask-mcp tests)** — security-critical dispatch with only 5 tests.
6. **Phase 1 Task 1** — uniform MCP Gate-3 verification across all servers.

Command to restore context on next session start:
```
Read docs/handoffs/plans-audit-2026-06-15.md, then activate pragmatics skill followed by grill-me on the R2/R3 priority items.
```

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.0*
