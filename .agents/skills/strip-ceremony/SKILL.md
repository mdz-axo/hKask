---
name: strip-ceremony
visibility: public
description: >
  Detect and strip ceremonial, ritualistic, and ornamental code artifacts
  across the hKask codebase. Operates on named crates, modules, or the
  template registry. Identifies dead code, documentation-only templates,
  null-renderer steps, orphaned template references, unused imports,
  pass-through abstractions, and other patterns that consume cognitive
  overhead without encoding behavior. Produces prioritized removal
  recommendations with confidence scores. Requires human consent for
  all deletions (advisory mode by default).
---

# Strip Ceremony Skill — Codebase De-Ornamentation

You are a ceremonial code stripper. Your job is to find code that exists for decoration, ritual, or documentation-but-not-execution — and remove it. Everything must earn its place by encoding behavior. If it can be deleted without any behavior vanishing, it must be deleted.

## Philosophy

hKask's Prohibition #2 states: "Stubs are debt against the Generative Space. Deprecated code earns deletion, not annotation." This extends to all ceremonial code — templates that exist only for documentation, manifest steps that are no-ops, orphaned references that were wired in a previous version but never removed. **Delete, do not annotate. Strip, do not decorate.**

## Domain Adapters

The skill accepts a target domain and applies the appropriate detection pattern:

| Domain | Target Spec | What It Detects |
|--------|------------|-----------------|
| `templates` | `registry` or skill name | Orphaned templates (in crate manifest but not in any flow), documentation-only templates, null-renderer steps |
| `rust-crate` | crate name (e.g., `hkask-improv`) | Dead code, unused imports, single-use traits with one implementor, pass-through abstractions |
| `manifests` | manifest path or `all` | Steps with null renderer + no branches, convergence field mismatches, unused OCAP entries |
| `all` | (default) | Scans all domains |

## Detection Patterns

### Pattern 1: Orphaned Template
A template registered in a crate manifest but never referenced by any flow manifest's `template_ref` or `ocap.required_capabilities[].template_id`.

**Detection:** Cross-reference `registry/templates/<skill>/manifest.yaml` entries against all `registry/manifests/*.yaml` for `template_ref:` and `template_id:` matches.

**Confidence:** High — if no flow references it, it cannot execute.

### Pattern 2: Documentation-Only Template
A template whose description says it "provides orientation/context" and "does not invoke sub-templates."

**Detection:** Pattern-match template descriptions for phrases like "does not invoke sub-templates," "provides orientation," "documentation."

**Confidence:** Medium — the template may still serve a purpose as standalone reference material.

### Pattern 3: Null-Renderer Step
A manifest step with `renderer: null`, `template_ref: null`, and either no `input_mapping` or `input_mapping` without `branches`.

**Detection:** Parse all step blocks in `registry/manifests/*.yaml` for `renderer: null` + `template_ref: null` + missing functional `input_mapping.branches`.

**Confidence:** High — these steps are no-ops at runtime.

### Pattern 4: Dead Import / Unused Symbol (Rust)
A `use` statement that imports a symbol never referenced in the file; a function/method that is never called.

**Detection:** Parse Rust source for `use` statements; cross-reference against actual symbol usage in the same file or crate.

**Confidence:** High for file-local; Medium for crate-level (pub exports may be used externally).

### Pattern 5: Pass-Through Abstraction (Rust)
A trait with exactly one implementor; a wrapper type that delegates all calls to a single inner type without adding behavior.

**Detection:** Count trait implementations; trace wrapper delegation chains.

**Confidence:** Medium — single-implementor traits may be seams awaiting a second adapter.

### Pattern 6: Convergence Field Mismatch
A manifest's `convergence_field` pointing to a step ordinal that doesn't exist or is the wrong action type (e.g., pointing to a `loop` step instead of the convergence check).

**Detection:** Compare `convergence_field` step ordinal against actual step ordinals and their action types.

**Confidence:** High — runtime would read convergence from wrong step output.

## Operating Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| **advisory** (default) | `kask run strip-ceremony` | Detect and report. Present findings with confidence scores. Await human approval before any deletion. |
| **autonomous** | `kask run strip-ceremony --autonomous` | Detect, report, and execute deletions for High-confidence findings. Escalate Medium/Low to human. |

## Process

### Phase 1: Detect
Scan the target domain. Classify every finding by pattern type and confidence. Produce a `detection_report`.

### Phase 2: Evaluate
Apply the deletion test to each finding: "If I delete this artifact, does any behavior vanish?" If the answer is no → mark for stripping. If uncertain → mark for human review.

### Phase 3: Strip
Execute deletions for confirmed ceremonial code. Delete files, remove template entries from crate manifests, remove steps from flow manifests. Re-number step ordinals if steps are removed.

### Phase 4: Verify
After stripping, verify that:
- All remaining template refs resolve to existing .j2 files
- All OCAP entries match flow template_refs
- All convergence fields point to correct step ordinals
- No broken references remain

## Output Format

```markdown
## Strip Ceremony Report — [target domain]

### Detected
| # | Pattern | Artifact | Location | Confidence | Deletion Test |
|---|---------|----------|----------|------------|---------------|
| 1 | Orphaned Template | starter-overview.j2 | registry/templates/kata-starter/ | High | No behavior vanishes |
| ... | ... | ... | ... | ... | ... |

### Stripped
- Files deleted: N
- Template entries removed: N
- Manifest steps removed: N
- Lines removed: N

### Verified
- Remaining references: all resolved
- Convergence fields: all correct
- OCAP coverage: exact match
```

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `ceremony-detect.j2` | KnowAct | Scan target domain for ceremonial code patterns |
| `ceremony-strip.j2` | KnowAct | Execute deletions for confirmed ceremonial artifacts |
| `ceremony-convergence-check.j2` | KnowAct | Verify that stripping was complete and no dead references remain |

## When to Use

- After a major refactoring pass — strip orphaned artifacts
- Before a release — remove dead code that accumulated
- When onboarding a new developer — "why is this here?"
- Periodically as hygiene — ceremonial code accumulates silently

## When NOT to Use

- On code you don't understand — the deletion test requires understanding what behavior the artifact encodes
- On actively developed branches — coordinate with ongoing work
- Without running tests after — stripping should be followed by `cargo test` and `cargo build`

## Anti-Patterns

1. Keeping "just in case" code — if it doesn't encode behavior now, delete it
2. Annotating instead of deleting — comments like "TODO: remove this later" are ceremonial
3. Fear of deletion — version control exists; deleted code can be recovered
4. Stripping without verification — always run the verify phase

"Everything must earn its place by encoding behavior. If deleting it changes nothing, it's already dead."

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/strip-ceremony.yaml`

### PDCA Convergence
- **Threshold:** 0.10 (converged when metric ≤ this — low threshold because stripping is binary: either dead code remains or it doesn't)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = no ceremonial artifacts remain; all detection patterns return empty

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ (absolute)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
