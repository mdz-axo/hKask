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
