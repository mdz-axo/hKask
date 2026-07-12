---
title: "Scenario and Documentation Repair Handoff"
audience: [developers, maintainers, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting and documentation"
mds_categories: [domain, composition, lifecycle, curation]
---

# Scenario and Documentation Repair Handoff

## 1. Session Context

This session audited `hkask-mcp-scenarios`, the related superforecasting/scenario-builder templates, and the documentation corpus with an adversarial architecture-review posture. The scoped implementation and documentation repair are complete. Do not assume the worktree is clean: it contained substantial unrelated user changes across several crates and documents before and during this session.

## 2. What Was Done

### Scenario forecasting implementation

- Strengthened Fermi input validation in `crates/hkask-forecast/src/lib.rs`: estimates and confidence weights must be finite and within `[0, 1]`.
- Corrected event-tree all-events probability handling in `mcp-servers/hkask-mcp-scenarios/src/superforecast.rs`: a dependent node now contributes its parent-true conditional rather than multiplying a marginal that already includes the parent probability.
- Corrected calibration bias semantics in `mcp-servers/hkask-mcp-scenarios/src/superforecast.rs`: bins compare actual hit rate with their mean submitted probability; positive bias denotes overconfidence.
- Made `scenario_calibration` honor its optional subject filter in `mcp-servers/hkask-mcp-scenarios/src/lib.rs`.
- Completed `ScenarioEvent` scaffold fields emitted by `scenario_build` and `scenario_research` so their JSON can deserialize into the server type.
- Added/updated focused regression tests in `crates/hkask-forecast/src/lib.rs` and `mcp-servers/hkask-mcp-scenarios/src/superforecast.rs`.

### Scenario templates and documentation

- Aligned superforecasting manifests with v0.31.0 in `registry/manifests/superforecasting.yaml` and `registry/templates/superforecasting/manifest.yaml`.
- Updated `registry/templates/scenario-builder/implications-indicators.j2`: robust strategies must work in all four scenarios and include a constraint-force classification.
- Updated `registry/templates/scenario-builder/scenario-quality-gate.j2`: a valid 2×2 output must contain exactly four scenarios.
- Relocated crate-local scenarios design documents into the root documentation corpus:
  - `docs/explanation/scenario-forecasting.md`
  - `docs/architecture/scenarios-companies-bridge.md`
  - `docs/status/scenarios-semantic-graph-audit.md`
- Added code-anchored pipeline documentation: `docs/diagrams/flowchart-scenario-forecasting-pipeline.md`, registered as `DIAG-FW-007` in `docs/DIAGRAMS_INDEX.md`.

### Documentation corpus repair

- Repaired all mechanically reported stale crate references and broken relative links throughout the active docs corpus.
- Fixed the verifier numeric-count false positive in `docs/ci/verify-docs.sh` and made it count both `///` and `//!` Rust documentation comments.
- Added a grouped 41-tool README table to `mcp-servers/hkask-mcp-companies/README.md`.
- Normalized malformed `last-verified-against` metadata in older diagrams and removed a non-commit metadata value.
- Removed the retired Kata-Kanban `ConsentProof` type and its stale documentation references.

### Validation

- `cargo test -p hkask-forecast -p hkask-mcp-scenarios` passed: 8 shared-forecast tests and 28 scenarios-server tests.
- `bash docs/ci/verify-docs.sh` passed with 0 errors and 4 expected planned-reference warnings.
- `git diff --check` passed.

## 3. What Remains

### HIGH — Preserve existing user work

Before making any further change, inspect `git status --short` and `git diff`. Do not revert or reformat unrelated edits in communication, storage, companies, Kata-Kanban, training, or architecture documentation.

### MEDIUM — Scenario modeling limits

`mcp-servers/hkask-mcp-scenarios/src/superforecast.rs` still uses an explicit average proxy for multi-parent dependencies. It is not a general joint-distribution or conditional-independence model. If improving this, first define a concrete probability contract and add tests for multi-parent joint cases; do not silently relabel the heuristic.

### MEDIUM — Pipeline ordering

`ScenariosServer::check_sequence` in `mcp-servers/hkask-mcp-scenarios/src/lib.rs` emits informational spans but has no per-session state. The documented workflow is therefore advisory, not enforced. If enforcement is required, first decide whether tool flexibility or strict pipeline integrity is the governing constraint.

### LOW — MCP protocol coverage

Scenario computations are unit-tested, but MCP macro-dispatch/request-response integration tests remain absent. Add only if a reproducible protocol-level behavior needs guarding.

### LOW — Documentation warnings

The documentation gate has four non-blocking `PLANNED` warnings for future crate names in plans/status documents. Keep them if they remain intentional; otherwise update the planning documents when their roadmap changes.

## 4. Recommended Skills and Tools

- Activate `coding-guidelines` before edits; the worktree is shared and changes must be surgical.
- Use `idiomatic-rust` for any probability-model or state/ownership redesign.
- Use `tdd` if replacing the multi-parent probability heuristic or adding MCP integration coverage.
- Use `pragmatic-semantics` to distinguish measured behavior from design recommendations in architecture docs.
- Use `diataxis-diagram` if a new workflow, state model, or data relationship is introduced.

Recommended commands:

```sh
git --no-optional-locks status --short
git --no-pager diff --check
cargo test -p hkask-forecast -p hkask-mcp-scenarios
bash docs/ci/verify-docs.sh
```

## 5. Key Decisions to Preserve

1. **Do not treat multi-parent event calculations as a joint probabilistic model.** The average conditional contribution is a declared approximation; hiding that limitation would overstate epistemic certainty.
2. **Keep shared forecasting math in `hkask-forecast`.** It is the deep, dependency-free computation boundary shared by scenarios and companies; do not duplicate Fermi, Bayes, or Brier calculations in MCP surfaces.
3. **Do not delete public MCP methods based solely on Rust call-graph searches.** Macro-generated tool dispatch makes callers invisible to ordinary static searches.
4. **Scenario scaffolding must serialize into `ScenarioEvent` without hand-added fields.** The producer/consumer contract is now complete and should remain mechanically aligned.
5. **Architecture and audit documents belong under root `docs/`, not within MCP crate directories.** Keep crate-local documentation to the concise `README.md` coding context allowed by project standards.
6. **Documentation verification is currently green.** Preserve `0` errors from `docs/ci/verify-docs.sh`; expected planning warnings are advisory only.
