---
name: task-breakdown
visibility: public
description: "Decompose work into small, verifiable, vertically-sliced tasks with explicit acceptance criteria and checkpoints. Convergent PDCA loop: gather read-only context and dependency graph, slice vertically, write tasks with verification, evaluate against sizing/red-flag/checkpoint criteria, iterate until the plan is stable, then finalize tasks/plan.md and tasks/todo.md. Adapted from addyosmani/planning-and-task-breakdown."
---


# Task Breakdown

Decompose work into small, verifiable tasks with explicit acceptance criteria. The skill runs a convergent PDCA loop: gather read-only context and a dependency graph, slice work vertically into feature paths, write tasks with acceptance criteria and verification steps, evaluate the plan against six weighted quality criteria, and iterate until the plan is stable. Finalizes `tasks/plan.md` (the implementation plan) and `tasks/todo.md` (the checklist). Adapted from addyosmani's planning-and-task-breakdown.

## When to Use

- You have a spec or clear requirements and need to break work into implementable units
- A task feels too large or vague to start
- Work needs to be parallelized across multiple agents or sessions
- You need to communicate scope to a human
- The implementation order isn't obvious
- You want iterative quality convergence — plans are scored and refined until they meet a quality threshold (≤ 0.15 weighted total across six criteria)

**When NOT to use:** Single-file changes with obvious scope, or when the spec already contains well-defined tasks.

## Instructions

1. **Enter plan mode (read-only).** Read the spec and relevant codebase sections. Identify existing patterns and conventions. Map dependencies between components into a dependency graph — implementation order follows the graph bottom-up (build foundations first). Note risks and unknowns, surfacing every assumption as an open question. Do NOT write code during planning. The output is a context summary, dependency graph, and risk register.

2. **Slice vertically.** Decompose the work into vertical feature paths, not horizontal layers. A vertical slice delivers one complete, testable feature end-to-end (schema + API + UI), e.g. "User can create an account" — not "Build entire database schema". Reject horizontal slicing. If a slice title contains "and", it is almost certainly two tasks — split it. Schedule high-risk slices early (fail fast). Respect the dependency graph: a slice may not precede its foundation slice. Accept refinement directives from the evaluate step on re-entry.

3. **Write tasks.** Turn each slice into one or more small, verifiable tasks. Each task has: description, acceptance criteria (specific, testable, ≤3 bullets), verification steps (tests pass, build succeeds, manual check), dependencies, likely-touched files, and estimated scope (XS/S/M/L/XL). Enforce: no XL tasks — break them down; no task touches more than ~5 files; no "and" in titles. Arrange tasks so dependencies are satisfied and each task leaves the system in a working state. Group tasks into phases (Foundation, Core Features, Polish) with checkpoints between phases.

4. **Evaluate against six weighted criteria.** Score the plan: task sizing (0.25), vertical-slice integrity (0.20), acceptance-criteria specificity (0.20), dependency ordering (0.15), checkpoint presence (0.10), red-flag absence (0.10). Score each criterion 0 (perfect) to 1 (severely deficient). Be honest — inflated scores produce worse plans. Emit specific, actionable refinement directives for any criterion scored above 0.00; each directive names the criterion, states what is wrong, and describes the expected fix.

5. **Check convergence.** Compute the normalized convergence metric from the evaluation's weighted total. Threshold is 0.15 — a metric of ≤ 0.15 means CONVERGED. Range 0.16–0.25 is NEAR (one more iteration should resolve). Range 0.26–0.50 is DRIFTING (refinement directives should target specific weaknesses). Above 0.50 is DIVERGED (consider re-slicing). Maximum 3 iterations.

6. **Write the final plan.** On convergence, produce two files. `tasks/plan.md`: overview, architecture decisions, phased task list with checkpoints, risks table, and open questions. `tasks/todo.md`: a flat checklist-style task list grouped by phase, scannable. Create the `tasks/` directory if it does not exist. These paths are the convention expected by downstream tooling.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `task-breakdown-plan.j2` | `KnowAct` | PLAN phase — read-only mode. Read the spec and relevant codebase sections, identify existing patterns/conventions, build the dependency graph, and note risks/unknowns. No code is written. Produces context summary, dependency graph, and risk register. |
| `task-breakdown-slice.j2` | `KnowAct` | DO phase — slice vertically into candidate tasks. Each slice delivers one end-to-end testable feature path (schema + API + UI), not horizontal layers. Reject horizontal slicing. Accepts refinement_directives from the evaluate step. |
| `task-breakdown-write-tasks.j2` | `KnowAct` | DO phase — write each task with description, acceptance criteria, verification steps, dependencies, likely-touched files, and estimated scope (XS/S/M/L/XL). Enforce: L+ tasks must be broken down; no task touches more than ~5 files; no "and" in titles. |
| `task-breakdown-evaluate.j2` | `KnowAct` | CHECK phase — score the plan against six weighted criteria: task sizing (0.25), vertical-slice integrity (0.20), AC specificity (0.20), dependency ordering (0.15), checkpoint presence (0.10), red-flag absence (0.10). Emits specific refinement_directives for criteria above threshold. |
| `task-breakdown-convergence.j2` | `KnowAct` | Compute normalized convergence metric from the evaluation scores. Weighted sum across six criteria, normalized to [0,1] where 0 = plan is complete, correctly ordered, properly sized, and free of red flags. Threshold: 0.15 (same band as diataxis-diagram / idiomatic-rust — "maximally correct"). |
| `task-breakdown-write-plan.j2` | `KnowAct` | ACT phase — finalize the plan into tasks/plan.md (overview, architecture decisions, phased task list with checkpoints, risks, open questions) and tasks/todo.md (checklist-style task list). |

## Constraints

- All templates are `visibility: Public` — no restricted spans generated
- Energy caps: plan=6144, slice=6144, write-tasks=8192, evaluate=6144, convergence=2048, write-plan=4096
- Plan mode is read-only — no code is written during the PLAN phase
- Vertical slicing is mandatory — horizontal layers (build all X, then all Y) are rejected
- No task may be XL; no task touches more than ~5 files; no "and" in task titles
- Every task must have acceptance criteria and a verification step
- Dependency order must be respected: a task may not precede a task it depends on
- Maximum 3 iterations before forced convergence exit
- Convergence threshold: 0.15 weighted total across six criteria
- Output paths are `tasks/plan.md` and `tasks/todo.md` — the convention expected by downstream tooling
- Registry is authoritative — when this SKILL.md disagrees with registry templates, the registry wins
- NOTE: This SKILL.md is a hand-authored placeholder following the derived-companion format. Regenerate via `kask skill derive task-breakdown` once the binary is built to stay P5.1-compliant.