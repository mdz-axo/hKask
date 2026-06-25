# Handoff: Doc Consolidation — 2026-06-24

## Session Context

Ontological anchoring sprint complete. Established dual-axis framework (PKO = process, DC+BIBO = state), created 7 bridge modules, wired all 13 MCP servers, bumped workspace to v0.31.0, ran Task 0 doc reconciliation. Tasks 1–5 of the document-update skill remain: consolidate ~73 active docs down to target ≤40.

## What Was Done

### Principles (architecture/code)
- `docs/architecture/core/PRINCIPLES.md` — P5.2 (5W1H ontological core), P5.3 (minimalist test gate), P5.4 (dual-axis framework with full server→bridge registry table), P8.1 (ontological bridging rules — fibo.rs pattern, bridge hierarchy)
- Grounded in Norouzi et al. (2025, arXiv:2509.23776) and PKO paper (Carriero et al., arXiv:2503.20634)

### Shared bridge crates (2 new)
- `crates/hkask-bridge-dublincore/` — 50+ DC + BIBO + CiTO constants, `mime_to_dc_type()`, `kind_to_bibo()`. 128 lines. 2 tests passing.
- `crates/hkask-bridge-pko/` — 30+ PKO constants, `kanban_status_to_pko_execution()`, `docproc_stage_to_pko_step()`, `research_stage_to_pko()`. 174 lines. 3 tests passing.
- Both have READMEs, workspace members, zero dependencies, no reasoners, no OWL.

### Server-specific bridge modules (4 new)
- `mcp-servers/hkask-mcp-memory/src/cogat.rs` — Cognitive Atlas: 16 concepts (episodic/semantic/working memory, encoding, recall, consolidation, forgetting, chunking, salience, priming). 154 lines. 2 tests.
- `mcp-servers/hkask-mcp-replica/src/golem.rs` — GOLEM: 9 concepts (Character, Event, Setting, NarrativeFunction, CreativeWork, Author). 99 lines. 2 tests.
- `mcp-servers/hkask-mcp-training/src/mlschema.rs` — ML-Schema: 10 concepts (Model, Run, Data, HyperParameter, Evaluation). 113 lines. 2 tests.
- `mcp-servers/hkask-mcp-media/src/omc.rs` — OMC v2.8: 12 concepts (Image/Audio/CG, CameraMetadata, Version, Participant, Task, CreativeWork, Scene/Shot/Sequence/Set). 127 lines. 2 tests.
- All have `pub mod` declarations in their server's lib.rs. All tests passing.

### Pre-existing bridges (unchanged)
- `mcp-servers/hkask-mcp-companies/src/fibo.rs` — FIBO (pre-existing)
- `mcp-servers/hkask-mcp-kanban/src/pko.rs` — PKO kanban mappings (pre-existing, predates shared crate)

### Dependency wiring
- All 13 MCP servers declare both `hkask-bridge-pko` and `hkask-bridge-dublincore` as dependencies
- No server code actually imports the bridges yet — vocabulary is available, integration is unwritten

### Version + doc reconciliation
- `Cargo.toml` workspace version bumped from 0.30.0 to 0.31.0
- All 61+ doc frontmatter versions aligned to 0.31.0
- No stale crate/MCP references in docs
- 3 undocumented components noted: `hkask-services-wallet`, `hkask-mcp-curator`, `hkask-mcp-skill` (last two are in P5.4 table by bare name, not crate name)
- `hkask-mcp-cloud-gateway` missing README (internal transport adapter — valid deferral)

## What Remains

### HIGH — Document consolidation (Tasks 1–5 of document-update skill)

The skill is at `.agents/skills/document-update/SKILL.md`. Activate it via `skill` tool. Current state: Task 0 complete, Tasks 1–5 not started.

**Starting count:** 73 active docs (`find docs/ -name '*.md' -not -path '*/archive/*' | wc -l`)  
**Target count:** ≤40

Consolidation targets (these absorb related docs — never delete these):
| # | Target | Absorbs |
|---|--------|---------|
| 1 | `docs/architecture/hKask-architecture-master.md` | Architecture patterns, loop-architecture, energy architecture, matrix-integration, PUBLIC_SURFACE |
| 2 | `docs/architecture/core/PRINCIPLES.md` | P12-replicant-host-mandate (if fully contained) |
| 3 | `docs/architecture/core/MDS.md` | MDS_SCAFFOLD |
| 4 | `docs/architecture/core/TESTING_DISCIPLINE.md` | Testing methodology docs |
| 5 | `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` | CNS-domain-spec, CNS memory verb contracts |
| 6 | `docs/specifications/standards/DOCUMENTATION_STANDARDS.md` | DOCUMENT_OWNERSHIP, template-header-standard |
| 7 | `docs/plans/deployment-and-backup.md` | DEPLOYMENT guide, admin-install-guide |

**Task 1:** Read all 73 docs, classify each as MERGE / ARCHIVE / KEEP. Output a merge plan.

**Task 2:** For each MERGE candidate, identify unique content not present in its target.

**Task 3:** Extract unique content and merge into targets with `> **Incorporated from:** <path>` attribution.

**Task 4:** Archive merged docs (`git rm` or move to `docs/archive/`), update all cross-references. Run `bash docs/ci/check-links.sh`.

**Task 5:** Refresh `docs/README.md`, `docs/architecture/hKask-architecture-master.md`, `docs/DIAGRAMS_INDEX.md` — remove archived entries, update counts.

### MEDIUM — Bridge integration

No server code calls the bridges yet. The vocabulary is available but unused. When integration happens:
- Add `use hkask_bridge_pko::...` and `use hkask_bridge_dublincore::...` to server code
- Consider `cns.bridge` CNS span target for bridge usage observability
- DC graph functions for condenser saliency need deliberate research — deferred

### LOW — Undocumented components

- `hkask-services-wallet` — never referenced in docs
- `hkask-mcp-curator` / `hkask-mcp-skill` — in P5.4 table by bare name but not by crate name
- `hkask-mcp-cloud-gateway` — internal transport adapter, missing README (valid deferral)

## Key Decisions to Preserve

1. **Dual-axis framework (P5.4):** PKO = master process ontology (flow/verb), DC+BIBO = master state ontology (entity/noun). No single source of truth. Every artifact carries both identities.

2. **Planck/Heisenberg anchoring:** You cannot reduce one axis to the other, and the more precisely you sample one, the less you can know about the other. Bridges are sampling instruments, not truth claims. This resolves the PKO duplication "issue" — kanban's `pko.rs` and the shared crate having different constants (e.g., `prov:Agent` vs `pko:Agent`) are two valid samples from different perspectives. No consolidation needed.

3. **Every server uses both axes.** PKO + DC+BIBO are universal dependencies. Domain-specific bridges (FIBO, GOLEM, CogAT, ML-Schema, OMC) are layered on top where DC+BIBO isn't specific enough — they are supplements, not alternatives.

4. **Curator is Socratic guide, not regulator.** Anchored to 5W1H core only. No domain bridge. The curator's domain IS the user's situation, surfaced through Who/What/When/Where/Why/How. This is a Magna Carta requirement (P1, P3). Never propose governance/regulatory/panopticon ontologies for the curator.

5. **Spec and skill are PKO-covered.** Specifications and skill manifests are knowledge production procedures. They share the PKO bridge with kanban/docproc/research.

6. **Condenser uses DC as graph connective tissue.** Dublin Core provides shared type vocabulary for structural saliency calculation across graph fragments. Specific graph-theoretic DC functions need deliberate research — not speculative implementation.

7. **Bridge pattern (`fibo.rs`):** All bridges are zero-dependency pure Rust modules ≤150 lines. Concept URI constants + field mapping functions + tests. No reasoners, no OWL parsing, no graph databases. Thin vocabulary layers only.

8. **Version 0.31.0:** Workspace and all docs aligned to this version. The dual-axis framework was the architectural change that justified the bump.

## Recommended Skills for Continuation

- **document-update** — activate first. Tasks 1–5 are the primary remaining work. The skill's SKILL.md at `.agents/skills/document-update/SKILL.md` contains the full workflow.
- **coding-guidelines** — before making any doc changes
- **gentle-lovelace** — to evaluate writing quality of merged sections

## Build Status

All crates compile clean: `cargo check` passes for all bridge crates and all 13 MCP servers. All 9 bridge tests pass.
