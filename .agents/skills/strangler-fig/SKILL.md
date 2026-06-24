---
name: strangler-fig
visibility: public
description: "Incremental architectural migration via Martin Fowler's Strangler Fig pattern. Introduce new implementation alongside old, migrate one domain at a time, both paths delegate before any deletion. System fully functional at every intermediate step. Use when migrating architecture, replacing legacy code, or extracting a service layer."
composes_skills: [coding-guidelines, tdd, constraint-forces]
---

# Strangler Fig Migration

Incremental architectural migration using Martin Fowler's Strangler Fig pattern (2004). The old code is the "tree." The new implementation is the "fig" that gradually wraps and replaces it. At every intermediate step, the system is fully functional.

## When to Activate

- User says "migrate architecture", "strangler fig", "incremental replacement", "wrap then replace"
- Replacing a legacy subsystem with a new implementation
- Extracting a shared service layer from duplicated surface code
- Any architectural change where big-bang rewrites are too risky

## Do NOT Activate For

- Greenfield projects with no existing code to migrate
- Pure addition of new functionality (use `tdd` skill)
- Refactoring within a single module (use `coding-guidelines`)
- Changes that can be accomplished in a single atomic commit

## Core Pattern

### The Strangler Fig Sequence

```
Step 1: CREATE — Implement new component with domain types
        [old code still running]
Step 2: WIRE A — Route first surface through new component
        [both paths usable, old still present]
Step 3: WIRE B — Route second surface through new component
        [all consumers on new path, old still present]
Step 4: DELETE — Remove duplicated logic from old surfaces
        [only new path remains]
Step 5: VERIFY — Full workspace: build + test + lint
```

The system must be fully functional after **every** step. If a step breaks the system, revert and re-examine the boundary. Never skip verification between steps.

### Why This Works

| Approach | Risk | Rollback | Cognitive Load |
|----------|------|----------|----------------|
| Big-bang rewrite | Catastrophic | Impossible | Overwhelming |
| Strangler Fig | Contained per domain | Trivial (revert step) | One domain at a time |

The fig doesn't kill the tree in one strike. It wraps one branch at a time. The tree continues to function until the fig fully envelops it — then the old wood rots away naturally.

## Principles

### P1 — One Domain Per Migration Step

Each migration step touches exactly one domain operation. No cross-domain refactors in a single step. If you find yourself touching `chat`, `ensemble`, and `cns` in one step, the domain boundaries are wrong — decompose further.

### P2 — Functional at Every Step

```bash
# After EVERY step:
cargo check --workspace && cargo test --workspace
```

If this fails, the step is wrong. Do not proceed to the next step with known breakage. The entire value of strangler fig is the safety net of continuous verification.

### P3 — Both Paths Before Deletion

Never delete old code before the new path is fully wired and verified. The old code is the safety net. Delete only after:

```
[ ] New component implemented and tested in isolation
[ ] Surface A wired and verified (tests pass)
[ ] Surface B wired and verified (tests pass)
[ ] All consumers migrated
```

Only then does the delete step execute.

### P4 — Reversible Steps

Every step must be independently reversible. If Step 3 (WIRE B) fails, you can revert to the end of Step 2 and the system still works. This requires:

- No partial state between steps
- Clean commit boundaries at each step
- The old code path remains intact until explicitly deleted

### P5 — Surgical Changes (via coding-guidelines)

Each commit touches exactly one migration step. No "while we're in the area" refactors. No style changes in adjacent code. Every changed line traces directly to the domain being migrated.

## Process

### Phase 1 — Map the Tree

Before any migration, produce a domain map:

1. **Identify domains** — what are the bounded contexts? Each is a candidate for independent migration.
2. **List consumers** — for each domain, which surfaces or callers consume it?
3. **Classify overlap** — for each consumer, is the logic Identical, Divergent, or Surface-only?
4. **Sequence by risk** — migrate simplest, most isolated domains first. Save cross-cutting domains for last.

### Phase 2 — Select the First Fig

Choose the smallest, most self-contained domain as the proof of concept. Criteria:

- Fewest consumers (ideally 2–3)
- Least divergent logic across consumers
- Lowest blast radius if something goes wrong
- Completable in a single session

Do NOT start with the largest, most complex domain. The first fig proves the pattern works.

### Phase 3 — Execute Per Domain

For each domain, in sequence:

```
[ ] CREATE — Implement new component
    └─ verify: unit tests pass for new component in isolation
[ ] WIRE A — Route first consumer through new component
    └─ verify: consumer A tests pass, old code still intact
[ ] WIRE B — Route second consumer through new component
    └─ verify: consumer B tests pass, old code still intact
[ ] WIRE N — Route remaining consumers
    └─ verify: all consumer tests pass
[ ] DELETE — Remove duplicated logic from all old surfaces
    └─ verify: workspace builds, all tests pass, no dead references
[ ] LINT — cargo clippy --workspace -- -D warnings
```

### Phase 4 — Repeat for Remaining Domains

After the proof of concept succeeds, migrate remaining domains in dependency order:

1. Independent domains first (no dependencies on other domains being migrated)
2. Dependent domains next (consume outputs of already-migrated domains)
3. Cross-cutting infrastructure last (shared state, error types, config)

### Phase 5 — Remove the Dead Tree

After all domains are migrated:

```bash
# Verify no references to old code paths remain
grep -r "old_module\|legacy_path" crates/ --include="*.rs"

# Full verification
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Delete old modules that have no remaining callers. The tree is dead — the fig has fully enveloped it.

## Migration Planning Template

For each domain, fill out before starting:

```
Domain: <name>
Consumers: [A, B, C]
Divergence: Identical | Divergent (explain) | Surface-only
Risk: Low | Medium | High
Order: N of M

Step 1 — CREATE: <new component path + interface>
Step 2 — WIRE A: <consumer A path + what changes>
Step 3 — WIRE B: <consumer B path + what changes>
Step 4 — DELETE: <old code paths to remove>
Step 5 — VERIFY: <specific test commands>
```

## Anti-Patterns (Immediately Flag These)

1. **All consumers at once** — migrating all surfaces in one commit defeats the safety net
2. **Delete before wire** — deleting old code before verifying the new path works
3. **Partial wiring** — wiring consumer A but leaving consumer B half-done
4. **Cross-domain steps** — migrating chat and ensemble in the same commit
5. **Big-bang thinking** — "let's just rewrite it all at once, it'll be faster"
6. **Skipping verification** — not running tests between steps
7. **Starting with the hardest domain** — proof of concept should be the easiest, not the hardest
8. **Feature creep during migration** — adding new functionality to the new path while migrating

## Checklist Per Domain

```
[ ] Domain mapped: consumers identified, divergence classified
[ ] Migration plan documented (template above)
[ ] CREATE: New component implemented, isolated tests pass
[ ] WIRE A: First consumer routed, consumer tests pass
[ ] WIRE B: Second consumer routed, consumer tests pass
[ ] ...remaining consumers wired and verified
[ ] DELETE: Old code removed, no dead references
[ ] VERIFY: cargo check --workspace && cargo test --workspace
[ ] LINT: cargo clippy --workspace -- -D warnings
[ ] Step is independently reversible (git revert possible)
[ ] No cross-domain changes in this step
```

## Registry Templates

This skill's runtime templates live in `registry/templates/strangler-fig/`:

| Template | Type | Purpose |
|----------|------|--------|
| `strangler-fig-plan.j2` | KnowAct | Map domains, classify overlap, sequence migration by risk |
| `strangler-fig-execute.j2` | KnowAct | Execute one domain migration step (create→wire→delete) |
| `strangler-fig-verify.j2` | KnowAct | Verify system functional at intermediate step, detect regressions |

The SKILL.md (this file) teaches the Zed coding agent the strangler fig methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When Migration is Complete

- [ ] Every domain migrated to new architecture
- [ ] All old code paths deleted (no dead references)
- [ ] Full workspace builds and tests pass
- [ ] Clippy clean across workspace
- [ ] No consumers reference the old architecture
- [ ] Each migration step is independently documented and reversible


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/strangler-fig.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 5
- **Convergence meaning:** 0 = migration step integrity is verified and no critical rollback blockers remain

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 22000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
