# Open Questions — Archive (Resolved)

> Historical record of resolved questions. Moved from OPEN_QUESTIONS.md during cleanup.

## Resolved Questions (2026-05-29 Sprint)

<details>
<summary>9 resolved questions — click to expand (historical record)</summary>

### OQ-1: hKask-surface Documentation Depth ✅

**MDS Category:** Interface  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Remove all references to ``kask` CLI surface` from active documentation. The concept is deferred to v1.1+ with an ADR if surface generation becomes necessary.

**Rationale:** No ``kask` CLI surface` crate exists. Maintaining references to non-existent crates violates P6 (delete stubs, don't publish them). The MCP/CLI/API equivalence already provides surface generation through utoipa (OpenAPI) and clap (CLI docs).

---


**MDS Category:** Composition  
**Status:** **Resolved — Active in v0.31.0**  
**Resolution Date:** 2026-05-29 | **Revision Date:** 2026-06-30



---

### OQ-3: Arsenal Crate Documentation Ownership ✅

**MDS Category:** Capability  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document MCP servers as a catalog with common pattern description and per-crate README for implemented servers. A unified catalog exists at `docs/status/PROJECT_STATUS.md`. Individual README files live in each `mcp-servers/hkask-mcp-*/README.md`.

**Rationale:** Each MCP server having its own specification entry in REQUIREMENTS.md (Option 1) creates 19 × ~2KB = ~38KB of spec overhead — disproportionate. Option 2 keeps the catalog as a single source of truth with per-crate detail for the specific tool surface. Note: The term "arsenal" is not part of the hKask vocabulary — the project has 11 core crates and 21 MCP servers, all in a single workspace.

---

### OQ-4: Cross-Workspace Dependency Visualization ✅

**MDS Category:** Observability  
**Status:** **Resolved — Current approach**  
**Resolution Date:** 2026-05-29

**Decision:** Maintain manual Mermaid dependency diagrams in architecture docs as the primary visualization. CI automation (cargo-depgraph) is a v1.1+ enhancement if dependency complexity warrants it.


---

### OQ-5: Automation and Drift Prevention ✅

**MDS Category:** Curation  
**Status:** **Resolved — Option 1, active**  
**Resolution Date:** 2026-05-29

**Decision:** CI checks for security invariants, constraint compliance, and metadata consistency are active in `.github/workflows/ci.yml` (`security-invariants` job). Further automation (link integrity, citation density, diagram alignment) expands incrementally.

**Rationale:** The `security-invariants` CI job added in this sprint (TASK-08) covers: no `unwrap()` on hot paths, no wildcard capabilities, no hardcoded secrets, no stubs (P6), no deprecated (P7), and no visual UI. This is the foundational tier of Option 1.

---

### OQ-6: ADR Gaps ✅

**MDS Category:** Lifecycle  
**Status:** **Resolved — Option 3**  
**Resolution Date:** 2026-05-29

**Decision:** Create retroactive ADRs for the 5 most impactful decisions. Forward-only ADRs for future decisions.

**Retroactive ADR Target List (5):**
1. **ADR-024**: Unified Registry Decision — single registry with `template_type` discriminator
2. **ADR-025**: 7-Level Attenuation Depth Limit — rationale for max 7 delegation levels
3. **ADR-026**: Bitemporal hMem Schema — valid-time + transaction-time semantics
4. **ADR-027**: Argon2id + HKDF-SHA256 Master Key Derivation — deterministic secrets
5. **ADR-028**: ACP Protocol Design — JSON-RPC 2.0 over stdio for agent communication (now archived — ACP transport layer removed)

**Rationale:** These 5 decisions are the most frequently referenced and most impactful architectural choices. Creating retroactive ADRs provides decision traceability without requiring ADRs for every implementation detail.

---

### OQ-7: Template Refresh ✅

**MDS Category:** Composition  
**Status:** **Resolved — Deferred**  
**Resolution Date:** 2026-05-29

**Decision:** Defer template regeneration to the next documentation refresh cycle. The current templates in `docs/artifacts/` and `registry/templates/spec/` are functional. MDS metadata requirements are documented in `MDS.md` and the four architecture specifications.

**Update (2026-05-29):** A documentation portal ([`README.md`](README.md)) was added that indexes every active document by MDS category and demonstrates the compliant metadata header. When OQ-7 is taken up, the portal and the four architecture specifications are the recommended "best example" sources from which to regenerate the artifact templates.

---

### OQ-8: hkask-mcp-docproc Self-Application ✅

**MDS Category:** Curation  
**Status:** **Resolved — Option 2**  
**Resolution Date:** 2026-05-29

**Decision:** Document the self-application concept without executing it. The `hkask-mcp-docproc` tools (11 MDS tools) are validated against the existing specification corpus. Self-application (using spec tools to capture/decompose/curate the spec tools themselves) is deferred to a future meta-curation exercise.

**Rationale:** There is no circularity concern — the server's process is defined by its own spec and code; using it on the spec corpus is no more circular than using a compiler to compile itself. Self-application is deferred only because it has not been implemented yet, not because of any logical problem.

---

### OQ-9: Stub MCP Server Completion ✅

**MDS Category:** Capability  
**Status:** **Resolved** (already confirmed 2026-05-28; reaffirmed 2026-05-29 MCP audit)  
**Resolution Date:** 2026-05-28

**Decision:** Both servers are fully implemented: `hkask-mcp-condenser` (1,744 LOC, 7 tools, 51 tests), `hkask-mcp-research` (1,044 LOC). No stubs remain. MCP tools inventory confirms completeness (see `docs/status/PROJECT_STATUS.md`).

---

</details>

## Pod Architecture Resolved Questions (ζ Group — v0.30.0)

> **Incorporated from:** `docs/architecture/core/OPEN_QUESTIONS_POD.md` (file removed post-merge; content preserved here — see DA-ISSUE-007). **Post-pivot note:** pod architecture is legacy infrastructure retained in-tree; the v0.31.0 primary surface is human-user `kask chat` + skills/MCP/LLM.

<details>
<summary>5 ζ-group questions — click to expand (resolved during multi-pod architecture design session)</summary>

### ζ.1 — Cross-Pod A2A Protocol

**Question:** What is the minimal viable cross-pod A2A protocol that preserves OCAP gating?

**Status:** **Deferred** (trigger: cross-server deployment use case)

**Analysis:** Current A2A (`A2ARuntime`) assumes same-process agents. Cross-pod A2A requires a network boundary. Use Matrix (Conduit homeserver) as the communication fabric. OCAP tokens carried in message metadata, verified by the receiving pod's `CapabilityChecker`.

---

### ζ.2 — Pod Portability Across Servers

**Question:** Is exporting a SQLCipher file sufficient for "move my pod to another server"?

**Status:** **Resolved** (design) — acceptance test deferred to v0.31.0.

**What transfers:** SQLCipher database file, deterministic passphrase (ADR-027), `.webid` sidecar, pod persona and capabilities.

**What does NOT transfer:** Regulation variety counters (temporal state), Curator cursor state, MCP server API keys, active A2A sessions.

**Import procedure:** `kask pod export <pod_id>` → `kask pod import <pod_id> {pod}.db {pod}.webid`.

---

### ζ.3 — Pod Lifecycle Across Containers

**Question:** If a pod IS a Docker container, does `kask pod activate` become `docker start {pod_id}`?

**Status:** **Deferred** (trigger: container deployment use case)

**Mapping:** Populated→Image built, Registered→Container created, Activated→Container started, Deactivated→Container stopped. `ActivePods` becomes a thin status tracker querying Docker/Podman container state.

---

### ζ.4 — Curator Aggregation Model

**Question:** Polling vs push for per-pod Regulation aggregation?

**Status:** **Resolved** — polling model implemented in `CuratorSync` (1-second interval).

**Decision:** Polling handles restarts naturally via cursor-based catch-up. Push requires Curator to be alive and reachable at write time — fragile. Regulation events serve as observability signals, not the sync trigger.

---

### ζ.5 — PodFactory Deletion Test

**Question:** Does `PodFactory` earn its existence if pods are created via `docker build`?

**Status:** **Resolved** — PodFactory survives essentialist G1 test.

**Verdict:** PodFactory is a deep module (1 public method, stateless). Delete it and pod creation becomes impossible. With container deployment, PodFactory's `deploy()` gains `--output containerfile` mode.

| ζ | Question | Status |
|---|----------|--------|
| ζ.1 | Cross-pod A2A protocol | Deferred |
| ζ.2 | Pod portability | Resolved |
| ζ.3 | Pod lifecycle across containers | Deferred |
| ζ.4 | Curator aggregation model | Resolved |
| ζ.5 | PodFactory deletion test | Resolved |

</details>


## Resolution Summary

| OQ | Status | Decision | Date |
|----|--------|----------|------|
| OQ-1 | Resolved | Remove `kask` CLI surface references | 2026-05-29 |
| OQ-3 | Resolved | Catalog + per-crate README | 2026-05-29 |
| OQ-4 | Resolved | Manual Mermaid (current) | 2026-05-29 |
| OQ-5 | Resolved | CI checks active | 2026-05-29 |
| OQ-6 | Resolved | 5 retroactive ADRs targeted | 2026-05-29 |
| OQ-7 | Deferred | Next doc refresh cycle | 2026-05-29 |
| OQ-8 | Resolved | Document concept, defer execution | 2026-05-29 |
| OQ-9 | Resolved | Confirmed fully implemented | 2026-05-28 |
| F6 | Resolved | GoalCapabilityToken removed; WebID-based owner scoping replaces token infrastructure | 2026-06-04 |

**MDS completeness:** 8/8 open questions resolved, 1 deferred with documented rationale. F6 resolved post-sprint. Code-implementation gaps (FUT-010 through FUT-013) reclassified from spec-document gaps per the MDS axiom `Spec-document completeness ⊥ Code-implementation completeness`.

---

## MDS Round 3 Deferred Items

*Tracked per MDS Semantic Alignment Audit (2026-06-06) remediation R11. These items were deferred in MDS §11 Round 3 but not previously tracked in OPEN_QUESTIONS.md. Now includes all 10 original MDS §11 R3 items cross-referenced.*

| # | Item | Category | Status | MDS §11 Ref |
|---|------|----------|--------|---------------|
| R3.1 | Span::Spec variant gap | Observability | **Resolved** (added in audit) | Audit R1 |
| R3.2 | SpecStore bitemporal semantics | Persistence | **Resolved** — 4 bitemporal query methods + recorded_at column + list_curation_records_since. 6 tests. Updated 2026-06-08 | MDS §11 #2 |
| R3.3 | Spec signing (Ed25519) | Trust | **Resolved** — Ed25519SpecSigner implemented | MDS §11 #3 |
| R3.4 | Spec capability tokens (spec:read, spec:write, spec:compose) | Capability | **Resolved** — CapabilityChecker::grant_spec() implemented | MDS §11 #4 |
| R3.5 | Spec-curation terms bootstrapping | Domain | **Resolved** (partially bootstrapped) | Audit §2.3 |
| R3.6 | MCP≡CLI≡API cross-surface equivalence test | Interface | ⚠️ Deferred | MDS §11 #6 |
| R3.7 | Curation authority OCAP boundary integration | Trust | ⚠️ Deferred | — |
| R3.8 | Curation records persistence | Persistence | **Resolved** (partial) — SqliteCurationRecordStore exists. Not yet wired into evaluate(). | MDS §11 #8 |
| R3.9 | Coherence threshold calibration (0.7) | Curation | ⚠️ Deferred (uncalibrated) | MDS §11 #7 |
| R3.10 | Spec version replacement (post version_sha removal) | Lifecycle | **Resolved** — Spec.version: Option<String> added | Audit R15 |
| R3.11 | `SpecStore` needs `Send + Sync` bounds on the trait itself | Trust | ⚠️ Deferred — breaking change to trait signature; bounds currently on field type only | MDS §11 #1 |
| R3.12 | `SpecObserver` → Regulation span integration depth | Observability | ⚠️ Deferred — currently emits `tracing::info!`; needs SpanEmitter variety counters and algedonic alert triggers | MDS §11 #5 |
| R3.13 | Spec drift detection (`reg.spec.drift` span) | Observability | ⚠️ Deferred — drift magnitude metric specified but not implemented; requires comparing `Spec` goals against implementation state | MDS §11 #10 |

---

## MDS Audit Remediation Tracking (R4–R18)

*Remediation items from the 2026-06-06 MDS Semantic Alignment Audit that are now resolved but were not previously tracked in this document.*

| # | Item | Category | Status | Audit Ref |
|---|------|----------|--------|----------|
| R4 | MDS §9.1 self-application matrix labels | Observability, Persistence, Lifecycle, Curation | **Resolved** — matrix updated with :partial and :drift labels | Audit R4 |
| R6 | Regulation span listing consolidation | Domain | **Resolved** — AGENTS.md and MDS.md §7.1-7.2 now cross-reference canonical Regulation span registry: `crates/hkask-types/src/regulation.rs` (`RegulationSpan`) | Audit R6 |
| R8 | TemplateType vocabulary mapping | Composition | **Resolved** — as_spec_name() method added, mapping table documented in MDS.md §7.2 §3.3 | Audit R8 |
| R13 | SpecDriftAlert not in Regulation loop | Observability | **Resolved** — DefaultSpecCurator dispatches SpecDriftAlert through Communication Loop to CurationLoop inbox | Audit R13 |

---

