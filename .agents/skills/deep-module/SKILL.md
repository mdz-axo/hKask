---
name: deep-module
visibility: public
description: "Module design discipline based on John Ousterhout's 'A Philosophy of Software Design.' Apply the deletion test to evaluate whether a module deserves to exist: delete the callers — if complexity reappears, extract. Delete the module — if complexity vanishes, don't create it. Enforces depth (high benefit/cost ratio), interface minimalism (≤7 public functions), and dependency direction."
composes_skills: [coding-guidelines, pragmatic-semantics, zoom-out]
---

# Deep Module Design

Module design discipline from John Ousterhout's *A Philosophy of Software Design* (2018). A module's depth is the ratio of behavior encapsulated (benefit) to interface surface area (cost). Deep modules have small interfaces and much behavior behind them. Shallow modules have large interfaces with thin or non-existent behavior.

## When to Activate

- User says "design module", "deep module", "deletion test", "is this module worth it?", "evaluate module depth"
- Before extracting a new module or crate
- During code review of a new module's interface
- When refactoring and considering whether to split or merge modules
- Auditing existing modules for depth (finding shallow modules to deepen or eliminate)

## Do NOT Activate For

- Pure implementation within an existing module (use `coding-guidelines`)
- Architectural migration decisions (use `strangler-fig`)
- Module discovery/mapping (use `zoom-out`)
- Constraint classification (use `pragmatic-semantics`)

## Core Concept

### The Depth Formula

```
Depth = Behavior Encapsulated / Interface Surface Area

Behavior Encapsulated = lines of implementation logic, complexity managed, invariants preserved
Interface Surface Area = number of public functions + public types + public traits + public constants
```

A module with 3 public functions that encapsulate 500 lines of domain logic is **deep**.
A module with 20 public functions that just delegate to other modules is **shallow**.

### The Deletion Test

Ousterhout's most powerful tool for deciding whether a module should exist:

**Direction 1 — Caller's perspective:**

> Delete the code that uses the module. If the **complexity** reappears in N callers, the module is pulling its weight — extract it.

**Direction 2 — Module's perspective:**

> Delete the proposed module. If the **complexity** vanishes — meaning it was just a pass-through with no real encapsulation — don't create it. Deepen the callers or merge with a sibling.

Both directions must pass. A module must both encapsulate real complexity AND not be replaceable by a single function call to a dependency.

## Principles

### P1 — Depth Over Abstraction

Abstraction is a means to depth, not an end. A module with a clever abstraction that wraps trivial behavior is shallow. A module with a simple interface that handles complex domain logic is deep.

Don't abstract for abstraction's sake. Abstract to hide complexity.

### P2 — The 7-Function Rule

No more than 7 public functions per module. This is a heuristic, not a hard limit, but exceeding it is a strong signal of a shallow module:

- More public functions than private? Red flag.
- More public types than public functions? Data without behavior — red flag.
- Public functions that are all thin delegations? Pass-through module — red flag.

If a module needs more than 7 public functions, it likely represents two or more separate modules trying to coexist — split them.

### P3 — Interface Cost is Real

Every public function, type, trait, and constant is a liability:

- It must be tested
- It must be documented
- It must preserve backward compatibility or trigger a version bump
- It increases the cognitive load on every developer who uses the module
- It constrains internal refactoring

Add public items only when the behavior they enable is worth this cost. If in doubt, keep it private.

### P4 — Deep Modules Compose

Deep modules compose into deep systems. Shallow modules compose into tangled systems with layers of thin delegation. A system's total depth is not the sum of its modules' depths — it's limited by the shallowest module in any critical path.

### P5 — Dependency Direction Preserves Depth

```
Deep:   Consumer → Deep Module → Dependency
        (Consumer gets rich behavior through small interface)

Shallow: Consumer → Shallow Module → Dependency
         (Consumer still has to understand the dependency's interface)
```

A module that depends on a deep module appears deep itself. A module that depends on shallow modules inherits their shallowness.

## Process

### Phase 1 — Assess

For an existing or proposed module:

1. **Count the interface**: List every public function, type, trait, constant.
2. **Estimate behavior**: Roughly how much implementation complexity is behind the interface? Lines of code, state machines, invariants enforced.
3. **Compute depth**: High interface count + low behavior = shallow. Low interface count + high behavior = deep.

### Phase 2 — Apply the Deletion Test

**Delete the callers' usage of the module:**

- Walk each call site. If you inlined the module's logic at the call site, would complexity increase?
- If YES across multiple callers: the module earns its keep. Proceed.
- If NO (a thin delegation): the module is a pass-through. Don't create it.

**Delete the module itself:**

- If you removed the module and replaced it with a direct call to its dependency, would any behavior be lost?
- If YES: the module encapsulates real behavior. Proceed.
- If NO: complexity vanishes → the module was just wiring. Don't create it.

### Phase 3 — Design

If both deletion test directions pass, design the interface:

1. **Minimize public surface**: Start with 1 public function. Only add more when proven necessary.
2. **Hide information**: What invariants, algorithms, data structures, and assumptions can be private?
3. **Design for the caller**: The interface should match how callers think about the problem, not how the implementation works.
4. **Keep configuration out of the interface**: Use a config struct, not per-function parameters.
5. **Unify errors**: One error type per module, not one per function.

### Phase 4 — Verify Depth

After implementation, verify depth with the depth test:

```bash
# Count public items
grep -c "^pub " src/module.rs

# Estimate behavior (lines of non-comment, non-blank implementation)
grep -c -v "^\s*\(//\|$\)" src/module.rs
```

If `public_items > 7` or `implementation_lines / public_items < 10`, the module is likely shallow.

## Depth Score Matrix

For each module, compute a depth score:

```
Depth Score = Behavior Lines / (Public Functions + Public Types + Public Traits)

| Score | Classification | Action |
|-------|---------------|--------|
| 100+  | Deep          | Keep. This is the goal. |
| 50–99 | Adequate      | Acceptable. Monitor for interface creep. |
| 20–49 | Shallow       | Candidate for deepening. Add behavior or reduce interface. |
| 0–19  | Very Shallow  | Merge with sibling or eliminate. |
```

A pass-through module will score 0–5. A deep domain module will score 100+.

## Anti-Patterns (Immediately Flag These)

1. **Pass-through module** — all public functions delegate to a single dependency with no added logic
2. **Data bag module** — more public types than functions (data without behavior)
3. **Interface explosion** — 20+ public functions with thin implementations
4. **Config in signatures** — `fn do_thing(db: &Db, cache: &Cache, config: &Config)` instead of `fn do_thing(ctx: &ServiceContext)`
5. **Error type per function** — `DoThingError`, `DoOtherError`, `DoMoreError` instead of unified `ModuleError`
6. **Leaky abstraction** — callers must understand dependency internals to use the module
7. **Abstraction for one caller** — a module with exactly one consumer (inline it)
8. **Premature generalization** — "what if we need..." without a concrete second caller

## Checklist Per Module

```
[ ] Interface counted: N public functions, M public types, K public traits
[ ] Behavior estimated: ~X lines of implementation
[ ] Depth score: X / (N + M + K) = <score>
[ ] Deletion test — callers: complexity reappears in all consumers?
[ ] Deletion test — module: behavior lost if removed?
[ ] Interface minimized: every public item justifies its cost
[ ] Information hidden: invariants, algorithms, assumptions are private
[ ] Interface matches caller mental model, not implementation details
[ ] Config unified: ServiceConfig/ModuleConfig, not per-function parameters
[ ] Error type unified: one error enum per module
[ ] No pass-through functions (direct delegation with no logic)
[ ] Depth score ≥ 50 (adequate) or ≥ 100 (deep — ideal)
```

## Registry Templates

This skill's runtime templates live in `registry/templates/deep-module/`:

| Template | Type | Purpose |
|----------|------|--------|
| `deep-module-assess.j2` | KnowAct | Assess module depth: count interface, estimate behavior, compute depth score |
| `deep-module-delete.j2` | KnowAct | Execute the deletion test in both directions on a candidate module |
| `deep-module-design.j2` | KnowAct | Design a deep module interface from deletion test results |

The SKILL.md (this file) teaches the Zed coding agent the deep module design methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## Quick Reference

Before extracting a module, ask:
1. **Caller deletion test**: If I delete usage, does complexity reappear? → Extract.
2. **Module deletion test**: If I delete the module, does complexity vanish? → Don't create.
3. **Depth score**: Implementation lines / public items > 50? → Adequate. > 100? → Deep.
4. **Interface count**: ≤ 7 public functions? → Good. > 7? → Split or deepen.
5. **Dependency direction**: Does the module encapsulate its dependency? → Deep. Does it just re-expose it? → Shallow.


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/deep-module.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 5
- **Convergence meaning:** 0 = module passes deletion test, ≤7 public items, interface minimal

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 2 rJ (manifest `rjoule.cap` — see `registry/manifests/deep-module.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
