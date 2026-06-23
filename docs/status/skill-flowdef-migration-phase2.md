# Skill FlowDef Migration — Phase 2 (Batch 1)

Date: 2026-06-23

## Goal

Upgrade foundational one-shot skill templates to explicit FlowDef PDCA processes that:

1. compose templates as steps,
2. declare `convergence` targets,
3. enforce `gas.cap`,
4. include explicit `loop` actions,
5. exit through convergence rails (`converged | maxed_out | escalated`).

## Phase 1 Re-check Summary

Phase 1 meta-skill manifests were re-verified for:

- valid `convergence` blocks,
- loop wiring,
- template reference resolution,
- runtime compile compatibility (`cargo check -p hkask-templates`).

## Batch 1 Conversions (implemented)

The following skills now have FlowDef manifests and convergence-check templates:

1. `dokkodo-mindset`
2. `pragmatic-laziness`
3. `essentialist`
4. `constraint-forces`
5. `diagnose`
6. `deep-module`
7. `pragmatic-semantics`
8. `pragmatic-cybernetics`

## New Process Manifests

- `registry/manifests/dokkodo-mindset.yaml`
- `registry/manifests/pragmatic-laziness.yaml`
- `registry/manifests/essentialist.yaml`
- `registry/manifests/constraint-forces.yaml`
- `registry/manifests/diagnose.yaml`
- `registry/manifests/deep-module.yaml`
- `registry/manifests/pragmatic-semantics.yaml`
- `registry/manifests/pragmatic-cybernetics.yaml`

## New Convergence Templates

- `registry/templates/dokkodo-mindset/dokkodo-convergence-check.j2`
- `registry/templates/pragmatic-laziness/pragmatic-laziness-convergence-check.j2`
- `registry/templates/essentialist/essentialist-convergence-check.j2`
- `registry/templates/constraint-forces/constraint-forces-convergence-check.j2`
- `registry/templates/diagnose/diagnose-convergence-check.j2`
- `registry/templates/deep-module/deep-module-convergence-check.j2`
- `registry/templates/pragmatic-semantics/semantics-convergence-check.j2`
- `registry/templates/pragmatic-cybernetics/cybernetics-convergence-check.j2`

## Registry Manifest Updates

Each corresponding `registry/templates/<skill>/manifest.yaml` was extended with a `...-convergence-check` template id.

## Docs Updated

- `docs/user-guides/skill-user-guide.md`
  - Summary table type updates for Batch 1 skills: `KnowAct` → `FlowDef`.

## Notes

These conversions establish FlowDef process skeletons and convergence rails for Batch 1.
Further tuning can improve domain-specific convergence metrics once runtime traces are observed.

## Batch 2 Conversions (implemented)

The following skills now have FlowDef manifests and convergence-check templates:

1. `review`
2. `rust-expertise`
3. `strangler-fig`
4. `improve-codebase-architecture`
5. `refactor-service-layer`
6. `goal-analysis`
7. `document-update`
8. `zoom-out`

### New Process Manifests

- `registry/manifests/review.yaml`
- `registry/manifests/rust-expertise.yaml`
- `registry/manifests/strangler-fig.yaml`
- `registry/manifests/improve-codebase-architecture.yaml`
- `registry/manifests/refactor-service-layer.yaml`
- `registry/manifests/goal-analysis.yaml`
- `registry/manifests/document-update.yaml`
- `registry/manifests/zoom-out.yaml`

### New/Updated Templates

- `registry/templates/review/review-structured-eval.j2`
- `registry/templates/review/review-convergence-check.j2`
- `registry/templates/rust-expertise/rust-convergence-check.j2`
- `registry/templates/strangler-fig/strangler-convergence-check.j2`
- `registry/templates/improve-codebase-architecture/arch-convergence-check.j2`
- `registry/templates/refactor-service-layer/rsl-convergence-check.j2`
- `registry/templates/goal-analysis/goal-convergence-check.j2`
- `registry/templates/document-update/doc-structured-pass.j2`
- `registry/templates/document-update/doc-convergence-check.j2`
- `registry/templates/zoom-out/zoom-out-convergence-check.j2`

### Registry Template Manifest Updates

Updated `registry/templates/<skill>/manifest.yaml` for all Batch 2 skills to register convergence-check templates and new structured wrapper templates where required.

### Docs Updated

- `docs/user-guides/skill-user-guide.md`
  - Summary table type updates for Batch 2 skills: `KnowAct` → `FlowDef`.

## Phase 3 Conversions (implemented)

The following skills were migrated or recomposed to explicit FlowDef+PDCA:

1. `bug-hunt` (new dedicated skill manifest)
2. `condenser-continuation` (new dedicated skill manifest)
3. `caveman` (new dedicated skill manifest)
4. `magna-carta-verifier` (new dedicated skill manifest)
5. `handoff` (recomposed manifest with explicit convergence rails)

### New/Updated Process Manifests

- `registry/manifests/bug-hunt.yaml`
- `registry/manifests/condenser-continuation.yaml`
- `registry/manifests/caveman.yaml`
- `registry/manifests/magna-carta-verifier.yaml`
- `registry/manifests/handoff.yaml` (recomposed)

### New Convergence Templates

- `registry/templates/bug-hunt/bug-hunt-convergence-check.j2`
- `registry/templates/condenser-continuation/condenser-convergence-check.j2`
- `registry/templates/caveman/caveman-convergence-check.j2`
- `registry/templates/magna-carta-verifier/mc-convergence-check.j2`
- `registry/templates/handoff/handoff-convergence-check.j2`

### Registry Template Manifest Updates

Updated template crate manifests to register new convergence-check templates:

- `registry/templates/bug-hunt/manifest.yaml`
- `registry/templates/condenser-continuation/manifest.yaml`
- `registry/templates/caveman/manifest.yaml`
- `registry/templates/magna-carta-verifier/manifest.yaml`
- `registry/templates/handoff/manifest.yaml`

### Docs Updated (Phase 3)

- `docs/user-guides/skill-user-guide.md`
  - Summary table type updates: `caveman`, `condenser-continuation`, `handoff`, `magna-carta-verifier` moved to `FlowDef`.

## Phase 4 Conversions (implemented)

The following legacy one-shot manifests were recomposed to explicit FlowDef+PDCA:

1. `coding-guidelines`
2. `grill-me`
3. `mcda`
4. `scenario-planning` (backs `scenario-builder` skill)
5. `gentle-lovelace`

### New/Updated Process Manifests

- `registry/manifests/coding-guidelines.yaml`
- `registry/manifests/grill-me.yaml`
- `registry/manifests/mcda.yaml`
- `registry/manifests/scenario-planning.yaml`
- `registry/manifests/gentle-lovelace.yaml`

### New Convergence Templates

- `registry/templates/coding-guidelines/guidelines-convergence-check.j2`
- `registry/templates/grill-me/grill-me-convergence-check.j2`
- `registry/templates/mcda/mcda-convergence-check.j2`
- `registry/templates/scenario-builder/scenario-convergence-check.j2`
- `registry/templates/gentle-lovelace/gentle-convergence-check.j2`

### Registry Template Manifest Updates

Updated template crate manifests to register new convergence-check templates:

- `registry/templates/coding-guidelines/manifest.yaml`
- `registry/templates/grill-me/manifest.yaml`
- `registry/templates/mcda/manifest.yaml`
- `registry/templates/scenario-builder/manifest.yaml`
- `registry/templates/gentle-lovelace/manifest.yaml`

### Docs Updated (Phase 4)

- `docs/user-guides/skill-user-guide.md`
  - Summary table type updates: `coding-guidelines`, `grill-me`, `mcda`, `scenario-builder`, `gentle-lovelace` moved to `FlowDef`.

## Phase 5 Conversions (implemented)

The following skills were recomposed to standardized FlowDef+PDCA with explicit convergence rails:

1. `tdd`
2. `decision-journal`
3. `self-critique-revision`
4. `structured-extraction`
5. `superforecasting`

### New/Updated Process Manifests

- `registry/manifests/tdd.yaml`
- `registry/manifests/decision-journal.yaml`
- `registry/manifests/self-critique-revision.yaml`
- `registry/manifests/structured-extraction.yaml`
- `registry/manifests/superforecasting.yaml`

### New Convergence Templates

- `registry/templates/tdd/tdd-convergence-check.j2`
- `registry/templates/decision-journal/decision-journal-convergence-check.j2`
- `registry/templates/self-critique-revision/self-critique-convergence-check.j2`
- `registry/templates/structured-extraction/structured-extraction-convergence-check.j2`
- `registry/templates/superforecasting/superforecasting-convergence-check.j2`

### Registry Template Manifest Updates

Updated template crate manifests to register new convergence-check templates:

- `registry/templates/tdd/manifest.yaml`
- `registry/templates/decision-journal/manifest.yaml`
- `registry/templates/self-critique-revision/manifest.yaml`
- `registry/templates/structured-extraction/manifest.yaml`
- `registry/templates/superforecasting/manifest.yaml`

### Docs Updated (Phase 5)

- `docs/user-guides/skill-user-guide.md`
  - Summary table type updates: `decision-journal`, `self-critique-revision`, `structured-extraction`, `superforecasting`, and `tdd` moved to `FlowDef`.

## Phase 6 Conversions (implemented)

The following skills were recomposed to standardized FlowDef+PDCA with explicit convergence rails:

1. `adversarial-red-team`
2. `chain-of-density`
3. `falstaffian-perspective`
4. `kata-coaching`
5. `kata-improvement`
6. `kata-starter`

### New/Updated Process Manifests

- `registry/manifests/adversarial-red-team.yaml`
- `registry/manifests/chain-of-density.yaml`
- `registry/manifests/falstaffian-perspective.yaml`
- `registry/manifests/kata-coaching.yaml`
- `registry/manifests/kata-improvement.yaml`
- `registry/manifests/kata-starter.yaml`

### New Convergence Templates

- `registry/templates/adversarial-red-team/adversarial-convergence-check.j2`
- `registry/templates/chain-of-density/cod-convergence-check.j2`
- `registry/templates/falstaffian-perspective/falstaffian-convergence-check.j2`
- `registry/templates/kata-coaching/kata-coaching-convergence-check.j2`
- `registry/templates/kata-improvement/kata-improvement-convergence-check.j2`
- `registry/templates/kata-starter/kata-starter-convergence-check.j2`

### Registry Template Manifest Updates

Updated template crate manifests to register new convergence-check templates:

- `registry/templates/adversarial-red-team/manifest.yaml`
- `registry/templates/chain-of-density/manifest.yaml`
- `registry/templates/falstaffian-perspective/manifest.yaml`
- `registry/templates/kata-coaching/manifest.yaml`
- `registry/templates/kata-improvement/manifest.yaml`
- `registry/templates/kata-starter/manifest.yaml`

### Docs Updated (Phase 6)

- `docs/user-guides/skill-user-guide.md`
  - Summary table type updates: `adversarial-red-team`, `chain-of-density`, `falstaffian-perspective`, `kata-coaching`, `kata-improvement`, and `kata-starter` moved to `FlowDef`.

### Phase 6 Addendum — skill-translator normalization

To remove the final non-FlowDef catalog outlier, skill translation was normalized to the same convergent process model:

- Rewrote `registry/manifests/skill-translation.yaml` as standardized FlowDef+PDCA with convergence rails and explicit loop.
- Added new template crate `registry/templates/skill-translator/` with:
  - `manifest.yaml`
  - `translate-skill.j2`
  - `translation-convergence-check.j2`
- Updated `docs/user-guides/skill-user-guide.md` summary table entry for `skill-translator` to `FlowDef`.

## Phase 7 Consistency Closure (implemented)

### Objective

Resolve remaining skill-model drift discovered during consistency/smell checks and close the primary outlier (`improv`) under the FlowDef+PDCA definition.

### Changes

- Replaced legacy `registry/manifests/improv.yaml` with standardized FlowDef+PDCA structure:
  - added `functional_role: flowdef`
  - added explicit `convergence` rails and `convergence_field`
  - replaced legacy dynamic template dispatch wiring with stable `step_n_result` wiring
  - added explicit `loop` action
  - aligned `ocap` capabilities with actual rendered templates
- Added new convergence template:
  - `registry/templates/improv/improv-convergence-check.j2`
- Updated improv template crate manifest registration:
  - `registry/templates/improv/manifest.yaml`

### Consistency/Smell Check Outcomes

- Skill summary table in `docs/user-guides/skill-user-guide.md` remains consistent:
  - all listed skills are `FlowDef` except `kata` (`Bundle`) by design.
- `improv` now includes both:
  - `functional_role: flowdef`
  - explicit `action: loop`
- Dynamic improv template-ref smell removed:
  - no remaining `template_ref: improv/improv-{{ ... }}` in manifests.

### Validation

- `cargo check -p hkask-templates` ✅
- `cargo test -p hkask-templates --test yaml_schema_validation` ✅
