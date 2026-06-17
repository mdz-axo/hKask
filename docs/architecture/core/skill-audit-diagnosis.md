---
title: "Dual-Layer Skill System — Diagnosis Report"
audience: "hKask developers and architects"
last_updated: "2026-06-17"
version: "0.27.0"
status: "Active"
domain: "architecture"
mds_categories: ["domain", "curation"]
---

# Dual-Layer Skill System — Diagnosis Report

**Date:** 2026-06-16  
**Workspace version:** 0.27.0  
**Scope:** `.agents/skills/*/SKILL.md`, `registry/templates/*/manifest.yaml` + `*.j2`, canonical Rust types.

## Executive Summary

The hKask dual-layer skill system is structurally confused. The **canonical types** (`hkask_types::lexicon::TemplateType`, `hkask_types::ports::Skill`, `hkask_templates::SkillLoader`) define a clear two-layer model, but the **on-disk corpus** does not follow it. Prior agents produced:

- Orphan template directories without `SKILL.md` or `manifest.yaml`.
- `.j2` frontmatter that contradicts the `skill-manager` / `skill-translator` specs.
- `FlowDef` templates stored as `.j2` while `lexicon.rs` declares `.yaml`.
- A runtime bootstrap registry (`bootstrap-registry.yaml`) that bypasses per-skill `manifest.yaml` files.
- A `SkillLoader` that registers every `SKILL.md` as `TemplateType::FlowDef`, regardless of content.

This report classifies each finding by pragmatic semantic force.

## 1. Root-Cause Findings

### 1.1 DDMVSS aliases were treated as runtime template types

**Classification:** Prohibition / Guardrail violation.  
**Evidence:**

- `skill-manager/SKILL.md` L20: "**TemplateType** (`hkask-types/src/lexicon.rs`): `WordAct` | `KnowAct` | `FlowDef` only. DDMVSS names (Cognition, Prompt, Process) are spec aliases — **never** use them in .j2 frontmatter."
- `skill-manager/SKILL.md` L66-76 R5: rejects `template_type: Cognition`/`Prompt`/`Process`.
- `crates/hkask-types/src/lexicon.rs` L75-81: `TemplateType::as_spec_name()` maps `WordAct→Prompt`, `KnowAct→Cognition`, `FlowDef→Process`; there is no runtime variant for the aliases.

**Actual corpus state:** A grep of all `.j2` and `manifest.yaml` files found **zero** occurrences of `Cognition`/`Prompt`/`Process` as `template_type` values. The aliases are absent from frontmatter.

**Verdict:** The current corpus does **not** currently violate this rule in data, but the type system and skills repeatedly had to re-state the prohibition, indicating prior agents did not internalize it. The risk remains because `lexicon.rs` exposes `as_spec_name()` without enforcing that it is only used for spec serialization.

### 1.2 Bundles were created before constituent primary skills were calibrated

**Classification:** Prohibition / architectural violation.  
**Evidence:**

- `essentialist/SKILL.md` L190: "When running within hKask, use the `essentialist/essentialist-flow` FlowDef template, which orchestrates: Iteration over `[G1, G2, G3]` gates, Delegation to `deep-module/deep-module-delete` (G1), `deep-module/deep-module-assess` (G2), `coding-guidelines/guidelines-verify` (G3)."
- `pragmatics/SKILL.md` is a composed bundle of `pragmatic-semantics`, `pragmatic-cybernetics`, `pragmatic-laziness`, `essentialist`, and `coding-guidelines`.
- `pragmatics` is **Zed-only** — there is no `registry/templates/pragmatics/` directory.
- `registry/templates/composition/` contains style-mashup `.j2` files but no `manifest.yaml` and no `SKILL.md`.
- `registry/templates/pragmatic-composition/prompt_template.j2` exists without a `manifest.yaml` or `SKILL.md`, references terms (`bind`, `render`, `format`, `inject`) not in `hlexicon-workspace.yaml` (verified by audit).
- `registry/templates/ensemble/` contains templates without a `SKILL.md` or `manifest.yaml`.

**Verdict:** `composition/`, `pragmatic-composition/`, and `ensemble/` are premature/incomplete bundle artifacts. They were created as template buckets rather than curated compositions of active primary skills. `pragmatics` is a valid bundle concept but is incomplete (no registry layer) and was created before its primary skills were fully calibrated.

### 1.3 `SKILL.md` instructions contradict `.j2` contracts and Rust types

**Classification:** Guardrail / Evidence.  
**Evidence:**

| Spec source | What it says | What the corpus does | File evidence |
|-------------|--------------|----------------------|---------------|
| `skill-manager/SKILL.md` L47-57 | `.j2` `[inference]` block has `contract.input`, `contract.output`, `energy_cap`, `visibility` at **top level** under `[inference]`. | Nearly every `.j2` nests `energy_cap` and `visibility` **under `contract`**. | `registry/templates/coding-guidelines/guidelines-apply.j2` L1-17; `registry/templates/essentialist/essentialist-flow.j2` L1-?; audit script flags 40+ files. |
| `skill-manager/SKILL.md` L11-15 | Registry layer is `registry/templates/<name>/manifest.yaml + *.j2`, loaded by `SqliteRegistry`. | 39 of 68 registry directories have **no `manifest.yaml`**. The runtime instead loads from `registry/templates/bootstrap-registry.yaml`. | `tmp/skill-audit.json`: `manifest_present: false` for 39 skills. |
| `skill-discovery/SKILL.md` L89-99 | `energy_cap` must be in `[2048, 8192]`. | Many templates use 2000, 4096, etc. within range, but the placement under contract is wrong. | Audit flags `energy_cap_nested_under_contract`. |
| `crates/hkask-types/src/lexicon.rs` L64-69 | `FlowDef` file extension is `.yaml`. | All `FlowDef` templates in the corpus are `.j2` (`flowdef/dispatch.j2`, `essentialist/essentialist-flow.j2`, `kata-coaching/coaching-cycle.j2`, etc.). | `registry/templates/flowdef/dispatch.j2`; `bootstrap-registry.yaml` L305 references `flowdef/dispatch.j2` with `template_type: FlowDef`. |
| `skill-manager/SKILL.md` L66-76 R5 | `template_type` must be `WordAct`/`KnowAct`/`FlowDef`. | This is obeyed, but the type/file-extension mismatch suggests the prior model was confused. | Grep found zero invalid `template_type` values. |

**Verdict:** The SKILL.md layer and the Rust types define one model; the on-disk templates define another. The runtime (bootstrap registry + minijinja rendering) silently accepts the on-disk model, which prevents the documented model from being enforced.

### 1.4 Flow logic is duplicated across layers instead of registry `FlowDef` being the single source of truth

**Classification:** Guardrail / architectural debt.  
**Evidence:**

- `pragmatics/SKILL.md` L31-104 describes a full 4-phase cascade in prose, duplicating the orchestration that should live in a `FlowDef` template or `BundleManifest`.
- `essentialist/SKILL.md` L126-184 describes the G1→G2→G3 recursive loop in prose, while `essentialist/essentialist-flow.j2` exists but is a single `.j2` without explicit calls to separate G1/G2/G3 templates.
- `kata-coaching/SKILL.md` L107-115 lists `coaching-cycle.j2` as `FlowDef` and five `coaching-q*.j2` as `WordAct`, but `coaching-cycle.j2` lacks `[inference]` frontmatter (audit flags it) and does not appear to call the individual question templates by id.
- `registry/manifests/pragmatic-composition/process_manifest.yaml` implements a 5-step flow, but `registry/templates/pragmatic-composition/prompt_template.j2` is a standalone prompt renderer that does not reference the process manifest.

**Verdict:** The Zed layer documents flows in prose; the registry layer either has incomplete FlowDef templates or unrelated process manifests. There is no single source of truth for process orchestration.

### 1.5 The runtime loader contradicts the dual-layer model

**Classification:** Prohibition / architectural violation.  
**Evidence:**

- `crates/hkask-templates/src/skill_loader.rs` L173: `let mut skill = Skill::new(&id, TemplateType::FlowDef);` — every `SKILL.md` is registered as `FlowDef`, regardless of whether the skill is a WordAct/KnowAct/FlowDef capability.
- `crates/hkask-templates/src/registry.rs` L337-357: `Registry::bootstrap()` loads from `registry/templates/bootstrap-registry.yaml`, not from per-skill `manifest.yaml` files.
- `crates/hkask-templates/src/registry_sqlite.rs`: `SqliteRegistry` implements the index traits but the CLI uses it as an in-memory store (`SqliteRegistry::new(None)` in `template.rs` L30).

**Verdict:** The runtime design hardcodes every Zed-layer skill as a FlowDef and ignores per-skill manifests. This makes the dual-layer model described in the skills effectively unenforceable at runtime.

## 2. Pragmatic Semantic Force Classification

| Finding | Force | Rationale |
|---------|-------|-----------|
| DDMVSS aliases in runtime template types | **Prohibition** | `lexicon.rs` and `skill-manager` explicitly forbid it. |
| Creating bundles before primary skill calibration | **Prohibition** | Bundles are compositions of active (≥0.8) primary skills per dual-layer model. |
| `SKILL.md` registered as `FlowDef` regardless of content | **Prohibition** | `SkillLoader` hardcodes the domain, violating the typed model. |
| `manifest.yaml` absent in most registry directories | **Guardrail** | Runtime still works via bootstrap registry, but the documented dual-layer contract is broken. |
| `energy_cap`/`visibility` nested under `contract` | **Guardrail** | Contradicts `skill-manager` spec; must be top-level under `[inference]`. |
| `FlowDef` stored as `.j2` while `lexicon.rs` says `.yaml` | **Guardrail** | Type/runtime inconsistency. |
| hLexicon terms not in workspace | **Guardrail** | Violates grounding requirement; easy to fix. |
| Prose flow duplication in SKILL.md | **Guideline** | Makes maintenance hard; should be delegated to FlowDef. |
| Version strings stale (v0.21.0, v0.22.0) | **Evidence** | Observable drift from workspace v0.27.0. |
| `composition/`, `pragmatic-composition/`, `ensemble/` are not real skills | **Hypothesis → Evidence** | Audit shows no SKILL.md, no manifest.yaml, no typed contracts. |

## 3. Canonical Live Positions

- `Skill` / `SkillZone` / `RegistryIndex`: `crates/hkask-types/src/ports/registry.rs`
- `TemplateType`: `crates/hkask-types/src/lexicon.rs` L26-93
- `CnsSpan`: `crates/hkask-types/src/cns.rs` L84-202
- `SkillLoader`: `crates/hkask-templates/src/skill_loader.rs`
- `Registry`: `crates/hkask-templates/src/registry.rs`
- `ContractValidator`: `crates/hkask-templates/src/contract_validator.rs`
- hLexicon workspace: `registry/hlexicon/hlexicon-workspace.yaml`

## 4. Recommended Remediation Order

1. **Delete premature bundle directories** (`composition/`, `pragmatic-composition/`, `ensemble/` in `registry/templates/`). Keep git history as revert mechanism.
2. **Align `.j2` frontmatter** in all active primary skills: move `energy_cap` and `visibility` to top-level under `[inference]` per `skill-manager` spec.
3. **Add `manifest.yaml`** to every registry-only directory that is meant to be a skill, or delete it if it is just an experiment.
4. **Decide `FlowDef` file format**: either update `lexicon.rs` to say `.j2` (matching the corpus and runtime) or migrate all `FlowDef` `.j2` to `.yaml`.
5. **Fix `SkillLoader`** so it infers `domain` from the registry layer or manifest, instead of hardcoding `FlowDef`.
6. **Replace `bootstrap-registry.yaml`** with a loader that reads per-skill `manifest.yaml` files and validates them against `ContractValidator`.
7. **Re-audit** after each wave until all intended primary skills score ≥ 0.8.

## 5. Audit Data

Full machine-readable audit: `tmp/skill-audit.json`.  
Human-readable summary: `tmp/skill-audit.md`.

Summary counts:

- Total skill names (union of both layers): 69
- Complete (both layers): 29
- Zed-only: 1 (`pragmatics`)
- Registry-only: 39
- Active (≥0.8): 23
- Stale warning: 11
- Critical: 7
- Recommend deprecation: 28
