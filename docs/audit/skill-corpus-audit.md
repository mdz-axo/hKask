# Task 3 — Dual-Layer Corpus Enumeration and Validation

**Date:** 2026-06-16  
**Scope:** `.agents/skills/*` (Zed layer) and `registry/templates/*` (runtime layer).  
**Source data:** `REGISTRY_AUDIT_REPORT.md`, `registry_audit_results.json`, and direct filesystem scans.

---

## 1. Corpus Overview

| Metric | Count |
|--------|-------|
| Zed-layer skill directories | 30 |
| Registry-layer template crates | 71 |
| Complete skills (both layers) | 29 |
| Registry-only skills | 42 |
| Zed-only skills | 1 (pragmatics) |
| Total `.j2` templates scanned | 244 |
| Manifests (`manifest.yaml`) | 31 |
| Bundle/FlowDef manifests in `registry/manifests/` | 66 |

---

## 2. Health Distribution

| Status | Count |
|--------|-------|
| active (score ≥ 0.8) | 17 |
| stale_warning (0.5–0.79) | 20 |
| critical (0.2–0.49) | 13 |
| recommend_deprecation (< 0.2) | 21 |

---

## 3. Complete Skills (Both Layers Present)

| Skill | Registry manifest | Zed SKILL.md | Score | Status | Top defects |
|-------|-------------------|--------------|-------|--------|-------------|
| coding-guidelines | ✓ | ✓ | 1.0 | active | — |
| condenser-continuation | ✓ | ✓ | 0.5 | stale_warning | 3 manifest errors; 5 J2 flags |
| constraint-forces | ✓ | ✓ | 1.0 | active | — |
| deep-module | ✓ | ✓ | 0.7 | stale_warning | 3 manifest errors; 3 J2 flags |
| diagnose | ✓ | ✓ | 1.0 | active | — |
| document-update | ✓ | ✓ | 0.0 | recommend_deprecation | 15 manifest errors; 7 J2 errors |
| essentialist | ✓ | ✓ | 0.6 | stale_warning | 2 manifest errors; 4 J2 flags |
| grill-me | ✓ | ✓ | 1.0 | active | — |
| handoff | ✓ | ✓ | 1.0 | active | — |
| improv | ✓ | ✓ | 0.4 | critical | 6 J2 errors |
| improve-codebase-architecture | ✓ | ✓ | 1.0 | active | — |
| kata | ✓ | ✓ | 0.3 | critical | 7 J2 errors |
| kata-coaching | ✓ | ✓ | 0.4 | critical | 6 J2 errors |
| kata-improvement | ✓ | ✓ | 0.5 | stale_warning | 5 J2 errors |
| kata-starter | ✓ | ✓ | 0.5 | stale_warning | 5 J2 errors |
| magna-carta-verifier | ✓ | ✓ | 1.0 | active | — |
| pragmatic-cybernetics | ✓ | ✓ | 1.0 | active | — |
| pragmatic-laziness | ✓ | ✓ | 0.55 | stale_warning | 7 J2 flags |
| pragmatic-semantics | ✓ | ✓ | 1.0 | active | — |
| refactor-service-layer | ✓ | ✓ | 0.75 | stale_warning | 1 manifest error; 2 J2 flags |
| rust-expertise | ✓ | ✓ | 1.0 | active | — |
| skill-bundler | ✓ | ✓ | 1.0 | active | — |
| skill-discovery | ✓ | ✓ | 1.0 | active | — |
| skill-maintenance | ✓ | ✓ | 1.0 | active | — |
| skill-manager | ✓ | ✓ | 1.0 | active | — |
| skill-translator | ✓ | ✓ | 1.0 | active | — |
| strangler-fig | ✓ | ✓ | 0.65 | stale_warning | 3 manifest errors; 4 J2 flags |
| tdd | ✓ | ✓ | 0.8 | active | 4 J2 flags |
| zoom-out | ✓ | ✓ | 1.0 | active | — |

---

## 4. Registry-Only Skills (No Zed Counterpart)

These are **incomplete** per the dual-layer model. They cannot be discovered/used by the Zed coding agent.

| Skill | Score | Status | Why it matters |
|-------|-------|--------|----------------|
| caveman | 0.75 | stale_warning | Single WordAct, no companion guide |
| chain-of-density | 0.05 | recommend_deprecation | No manifest, 10 J2 flags |
| chat-template/templates | 0.65 | stale_warning | No manifest |
| cns/selectors | 0.65 | stale_warning | No manifest |
| curator | 0.15 | recommend_deprecation | No manifest, 6 J2 errors |
| dct-pipeline | 0.2 | critical | No manifest, 11 J2 flags |
| decision-journal | 0.0 | recommend_deprecation | No manifest, 29 J2 flags |
| ensemble | 0.35 | critical | No manifest, 4 J2 errors |
| ensemble/selectors | 0.65 | stale_warning | No manifest |
| flowdef | 0.0 | recommend_deprecation | No manifest, 12 J2 flags |
| gentle-lovelace | 0.65 | stale_warning | No manifest |
| git/selectors | 0.65 | stale_warning | No manifest |
| gml | 0.0 | recommend_deprecation | No manifest, 13 J2 errors |
| goal | 0.35 | critical | No manifest, 4 J2 errors |
| inference/selectors | 0.65 | stale_warning | No manifest |
| knowact | 0.0 | recommend_deprecation | No manifest, 15 J2 flags |
| mcda | 0.0 | recommend_deprecation | No manifest, 33 J2 flags |
| mcp | 0.0 | recommend_deprecation | No manifest, 15 J2 flags |
| mcp/selectors | 0.65 | stale_warning | No manifest |
| media | 0.35 | critical | No manifest, 4 J2 errors |
| memory/selectors | 0.65 | stale_warning | No manifest |
| memory/templates | 0.3 | critical | No manifest, 4 J2 errors |
| metacognition | 0.4 | critical | No manifest, 5 J2 flags |
| pragmatic-composition | 0.3 | critical | No manifest, 8 J2 flags |
| prompt-defense | 0.0 | recommend_deprecation | No manifest, 25 J2 flags |
| rag | 0.0 | recommend_deprecation | No manifest, 24 J2 flags |
| rca | 0.0 | recommend_deprecation | No manifest, 36 J2 flags |
| reasoning | 0.2 | critical | No manifest, 11 J2 flags |
| registry/selectors | 0.65 | stale_warning | No manifest |
| replica | 0.45 | critical | No manifest, 3 J2 errors |
| review | 0.6 | stale_warning | No manifest, 3 J2 flags |
| scenario | 0.0 | recommend_deprecation | No manifest, 46 J2 flags |
| self-critique-revision | 0.0 | recommend_deprecation | No manifest, 17 J2 flags |
| spec | 0.15 | recommend_deprecation | No manifest, 6 J2 errors |
| structured-extraction | 0.0 | recommend_deprecation | No manifest, 20 J2 flags |
| superforecasting | 0.0 | recommend_deprecation | No manifest, 8 J2 errors |
| templates/doc-knowledge | 0.75 | stale_warning | No .j2, registry-only doc crate |
| templates/spandrel | 0.0 | recommend_deprecation | 14 J2 errors |
| web | 0.25 | critical | No manifest, 5 J2 errors |
| wordact | 0.0 | recommend_deprecation | No manifest, 15 J2 flags |

---

## 5. Zed-Only Skill (No Registry Counterpart)

| Skill | Why it matters |
|-------|----------------|
| **pragmatics** | Zed-layer bundle/companion guide for `pragmatic-semantics`, `pragmatic-cybernetics`, `pragmatic-laziness`, `essentialist`, `coding-guidelines`. Has no executable registry crate; should be treated as a meta-skill companion or deleted if the runtime bundle already exists elsewhere. |

---

## 6. Invalid `template_type` Flags

The following registry crates contain `.j2` files declaring `template_type: FlowDef` or DDMVSS aliases (`Cognition`/`Prompt`/`Process`). These are **Prohibition-level** defects.

| Crate | Affected `.j2` files (sample) | Invalid type |
|-------|------------------------------|--------------|
| adversarial-red-team | `generate-adversarial.j2`, `multi-turn-attack.j2`, `select-target.j2`, `test-against-target.j2` | FlowDef |
| chain-of-density | all `.j2` | FlowDef / aliases |
| decision-journal | all `.j2` | FlowDef |
| dct-pipeline | all `.j2` | FlowDef / aliases |
| flowdef | all `.j2` | FlowDef / aliases (by directory name) |
| knowact | all `.j2` | aliases |
| mcda | all `.j2` | FlowDef / aliases |
| mcp | all `.j2` | FlowDef / aliases |
| media | all `.j2` | FlowDef / aliases |
| pragmatic-composition | all `.j2` | FlowDef / aliases |
| prompt-defense | all `.j2` | FlowDef / aliases |
| rag | all `.j2` | FlowDef / aliases |
| rca | all `.j2` | FlowDef / aliases |
| reasoning | all `.j2` | FlowDef / aliases |
| scenario | all `.j2` | FlowDef / aliases |
| self-critique-revision | all `.j2` | FlowDef / aliases |
| structured-extraction | all `.j2` | FlowDef / aliases |
| superforecasting | all `.j2` | FlowDef / aliases |
| web | all `.j2` | FlowDef / aliases |
| wordact | all `.j2` | aliases |

---

## 7. Premature Bundle Candidates

A bundle is premature if it composes skills that are not yet `active` (health < 0.8) or if it is expressed as a `.j2` wrapper rather than a `.yaml` `BundleManifest`.

### `.j2` wrapper bundles (delete)

| Location | Why it is a bundle |
|----------|--------------------|
| `registry/templates/flowdef/` | Entire directory appears to be `.j2` wrappers around other skills |
| `registry/templates/pragmatic-composition/*.j2` | Wrapper around pragmatic composition; the real process manifest lives in `registry/manifests/pragmatic-composition/process_manifest.yaml` |
| `registry/templates/ensemble/*.j2` | Wrapper around ensemble orchestration |

### `.yaml` manifests that may be premature

| Manifest | Referenced / implied skills | Health of constituents |
|----------|------------------------------|------------------------|
| `registry/manifests/ensemble-orchestration.yaml` | ensemble, multiple selectors | ensemble: critical (0.35); selectors: stale (0.65) |
| `registry/manifests/standing-ensemble-session.yaml` | ensemble, coaching kata | ensemble: critical; kata-improvement: stale |
| `registry/manifests/pragmatic-composition/process_manifest.yaml` | prompt/selector, pragmatic-* | pragmatic-laziness: stale; no active `prompt/selector` |
| `registry/manifests/composition.yaml` | composition templates | `registry/templates/composition/` deprecated (0.0) |
| `registry/manifests/dct-pipeline.yaml` | dct-pipeline | dct-pipeline: critical (0.2) |

**Recommendation:** Delete the `.j2` wrappers now. Revisit `.yaml` manifests only after every referenced skill reaches health ≥ 0.8.

---

## 8. Summary by Severity

| Severity | Count | Action |
|----------|-------|--------|
| High — invalid `template_type` on `.j2` | ~20 crates | Delete `.j2` files or convert to `.yaml` FlowDef |
| High — single-layer (registry-only) | 42 skills | Add Zed layer or delete |
| High — single-layer (Zed-only) | 1 skill | Add registry layer or delete |
| Medium — hLexicon term drift | Hundreds of flags | Replace unknown terms with canonical terms |
| Medium — missing `manifest.yaml` | 40 crates | Add crate manifest |
| Low — manifest/contract misalignment | ~30 skills | Calibrate descriptions and contracts |

---

## 9. JSON Report Location

A machine-readable report is maintained alongside this document:

- `registry_audit_results.json` — raw per-skill, per-template audit records (generated by prior `audit_registry.py`).
- This markdown file is the interpreted dual-layer summary.

For CI integration, the Rust audit harness (Task 7) will consume the same rules and emit a structured `SkillAuditReport`.
