# Task 4 — Premature Bundle Pruning Log

**Date:** 2026-06-16  
**Policy:** Pre-release — direct deletion, git is the revert mechanism.

---

## Deleted bundles

### 1. `.agents/skills/pragmatics/` — Zed-only composition bundle

**Reason:** The `SKILL.md` explicitly describes itself as a "Meta-cognitive codebase review bundle" that "Composes pragmatic-semantics, pragmatic-cybernetics, pragmatic-laziness, essentialist, and coding-guidelines." It had no registry-layer counterpart, so it was a bundle with no executable runtime artifact. The same guidance can be reintroduced later as a `.yaml` `BundleManifest` once all five constituent skills are active.

**Constituent primary skills (to recalibrate before reintroduction):**
1. `pragmatic-semantics` (active, 1.0)
2. `pragmatic-cybernetics` (active, 1.0)
3. `pragmatic-laziness` (stale, 0.55) ← needs calibration
4. `essentialist` (stale, 0.6) ← needs calibration
5. `coding-guidelines` (active, 1.0)

### 2. `registry/templates/flowdef/` — `.j2` files declaring `template_type: FlowDef`

**Reason:** This directory contained `.j2` wrappers (`dispatch.j2`, `memory_recall.j2`) that declared `template_type: FlowDef`. The hKask runtime type system says `FlowDef` maps to `.yaml` files, not `.j2`. These were category errors acting as pass-through process wrappers.

**Files removed:**
- `registry/templates/flowdef/dispatch.j2`
- `registry/templates/flowdef/memory_recall.j2`

---

## Remaining bundle candidates (not yet deleted)

The following registry manifests are process orchestrations that may be bundles or may become legitimate primary FlowDefs once their underlying templates are calibrated. They were **not** deleted because they are `.yaml` `FlowDef` manifests, not `.j2` wrappers, and the prompt prioritizes deleting `.j2` wrapper bundles. They are flagged for re-evaluation after primary-skill calibration.

| Manifest | What it orchestrates | Current blocker |
|----------|---------------------|-----------------|
| `registry/manifests/pragmatic-composition/process_manifest.yaml` | prompt/selector, pragmatic-* | `prompt/selector` not active; pragmatic-laziness stale |
| `registry/manifests/ensemble-orchestration.yaml` | ensemble/selectors, ensemble/templates | `ensemble` registry directory does not exist; manifest comment says ensemble was deferred 2026-06-14 |
| `registry/manifests/standing-ensemble-session.yaml` | ensemble, coaching kata | ensemble is critical; kata skills are stale/critical |
| `registry/manifests/composition.yaml` | composition templates | `registry/templates/composition/` does not exist |
| `registry/manifests/dct-pipeline.yaml` | dct-pipeline templates | `dct-pipeline` is critical (0.2) |

**Decision:** Keep these `.yaml` files under observation. Delete or re-author them once every referenced primary skill is `active` (health ≥ 0.8).

---

## Prioritized list of primary skills to calibrate before bundles are reintroduced

1. `pragmatic-laziness` (0.55) — unblock `pragmatics` bundle
2. `essentialist` (0.6) — unblock `pragmatics` bundle
3. `deep-module` (0.7) — widely referenced architecture skill
4. `refactor-service-layer` (0.75) — widely referenced refactoring skill
5. `strangler-fig` (0.65) — migration pattern skill
6. `condenser-continuation` (0.5) — needed for continuation workflows
7. `kata-starter` (0.5), `kata-improvement` (0.5), `kata-coaching` (0.4), `kata` (0.3) — calibrate in dependency order
8. `improv` (0.4) — interaction grammar skill

Only after these reach ≥ 0.8 should the deleted bundles be restored as `.yaml` `BundleManifest` files.
