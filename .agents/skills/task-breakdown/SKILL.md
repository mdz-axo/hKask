---
name: task-breakdown
visibility: public
description: "Decompose work into small, verifiable, vertically-sliced tasks with explicit acceptance criteria and checkpoints. Convergent PDCA loop: gather read-only context and dependency graph, decompose (slice + write tasks in one producer), evaluate against sizing/red-flag/checkpoint criteria, iterate until the plan is stable, then finalize tasks/plan.md and tasks/todo.md with PKO process-axis anchors. Adapted from addyosmani/planning-and-task-breakdown."
---


# Task Breakdown

Decompose work into small, verifiable tasks with explicit acceptance criteria. The skill runs a convergent PDCA loop: gather read-only context and a dependency graph, decompose work vertically into tasks with acceptance criteria and verification steps, evaluate the plan against six weighted quality criteria, and iterate until the plan is stable. Finalizes `tasks/plan.md` (the implementation plan) and `tasks/todo.md` (the checklist), each element carrying a PKO process-axis identity (Procedure / Step / StepVerification). Adapted from addyosmani's planning-and-task-breakdown.

## When to Use

- You have a spec or clear requirements and need to break work into implementable units
- A task feels too large or vague to start
- Work needs to be parallelized across multiple agents or sessions
- You need to communicate scope to a human
- The implementation order isn't obvious
- You want iterative quality convergence — plans are scored and refined until they meet a quality threshold (≤ 0.15 weighted total across six criteria)

**When NOT to use:** Single-file changes with obvious scope, or when the spec already contains well-defined tasks.

## Instructions

1. **Enter plan mode (read-only).** Read the spec and relevant codebase sections. Identify existing patterns and conventions. Map dependencies between components into a dependency graph — implementation order follows the graph bottom-up (build foundations first; e.g. hkask-types → hkask-ports → hkask-services → hkask-cli/hkask-mcp). Note risks and unknowns, surfacing every assumption as an open question. Do NOT write code during planning. The output is a context summary, dependency graph, and risk register.

2. **Decompose (slice + write tasks).** In one producer step, slice the work vertically AND write each task. A vertical slice delivers one complete, testable feature end-to-end (e.g. a port trait + service impl + CLI flag), not a horizontal layer ("build all of hkask-types"). Each task carries: slice_id/feature_path, acceptance criteria (specific, testable, ≤3 bullets), verification steps (tests pass, build succeeds, manual check), dependencies, likely-touched files, and estimated scope (XS/S/M/L/XL). Enforce: no XL tasks — break them down; no "and" in titles; the ~5-files limit is advisory (a correct cross-crate Rust feature may legitimately touch 5-7 files — state the justification rather than forcing an artificial split). Arrange tasks so dependencies are satisfied; group into phases (Foundation, Core Features, Polish) with checkpoints between phases. Accepts task-addressable refinement directives on re-entry.

3. **Evaluate against six weighted criteria.** Score the plan: task sizing (0.25), vertical-slice integrity (0.20), acceptance-criteria specificity (0.20), dependency ordering (0.15), checkpoint presence (0.10), red-flag absence (0.10). Score each criterion 0 (perfect) to 1 (severely deficient). Be honest — inflated scores produce worse plans. Emit specific, task-addressable refinement directives for any criterion scored above 0.00; each directive names the criterion, states what is wrong, and describes the expected fix.

4. **Independent quality gate.** A separate `fusion: false` evaluator scores the plan on the same six criteria WITHOUT self-assessment bias — it re-derives every score from the plan, flags any dimension where it diverges from the producer's evaluate by > 0.2 (a `bias_delta`), and rejects compensation masking (`gate_pass` is false if any single criterion > 0.30, regardless of the weighted total). Mirrors superforecasting's forecast-quality-gate: the judge must not be the generator.

5. **Check convergence.** Compute the normalized convergence metric from the INDEPENDENT gate's scores (not the producer's self-assessment). Threshold is 0.15 — ≤ 0.15 means CONVERGED (`min_iterations: 0` lets a first-pass-perfect plan converge immediately). A materiality guard uses the carried prior metric: if the metric is unchanged (< 0.02 delta) and the plan is stable across iterations, force convergence — the residual gap is taste, not a fixable defect. Range 0.16–0.25 NEAR, 0.26–0.50 DRIFTING, > 0.50 DIVERGED. Maximum 3 iterations. If not converged, loop back to Decompose carrying the prior metric.

5. **Write the final plan.** On convergence, produce two files plus a `pko_anchors` map. `tasks/plan.md`: overview, architecture decisions, phased task list with checkpoints, risks table, open questions. `tasks/todo.md`: a flat checklist-style task list grouped by phase, scannable. `pko_anchors`: each element's PKO process-axis identity — the plan is a `pko:Procedure` targeting a `pko:ProcedureTarget`; tasks are `pko:Step`; phases `pko:MultiStep`; acceptance criteria `pko:requiresAction`; verification `pko:StepVerification`; risks `pko:IssueOccurrence`; open questions `pko:UserQuestionOccurrence`; checkpoints `pko:UserFeedbackOccurrence`. The plan.md *document* itself carries DC+BIBO state metadata (title/creator/date, `bibo:Document`). Create the `tasks/` directory if it does not exist.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `task-breakdown-plan.j2` | `KnowAct` | PLAN phase — read-only mode. Read the spec and relevant codebase sections, identify existing patterns/conventions, build the dependency graph, and note risks/unknowns. No code is written. Produces context summary, dependency graph, and risk register. |
| `task-breakdown-decompose.j2` | `KnowAct` | DO phase — single producer: slice vertically AND write tasks in one step. Each task carries slice_id/feature_path, acceptance criteria, verification, dependencies, files, and scope. The PDCA loop re-enters here so refinement directives are task-addressable and re-slicing + re-writing happen together. Merges the former slice + write-tasks steps (fixes the grain-mismatch; mirrors diataxis-diagram's single generate producer). |
| `task-breakdown-evaluate.j2` | `KnowAct` | CHECK phase — score the plan against six weighted criteria: task sizing (0.25), vertical-slice integrity (0.20), AC specificity (0.20), dependency ordering (0.15), checkpoint presence (0.10), red-flag absence (0.10). Emits task-addressable refinement_directives for criteria above threshold. |
| `task-breakdown-convergence.j2` | `KnowAct` | Compute normalized convergence metric from the evaluation scores. Weighted sum across six criteria, normalized to [0,1] where 0 = plan is complete, correctly ordered, properly sized, and free of red flags. Threshold: 0.15 (same band as diataxis-diagram / idiomatic-rust). |
| `task-breakdown-write-plan.j2` | `KnowAct` | ACT phase — finalize tasks/plan.md + tasks/todo.md, plus a pko_anchors map giving each element a PKO process-axis identity (Procedure, Step, StepVerification, etc.). The plan.md document itself carries DC+BIBO state metadata. |

## Constraints

- All templates are `visibility: Public` — no restricted spans generated
- Energy caps: plan=6144, decompose=8192, evaluate=6144, convergence=2048, write-plan=4096
- `min_iterations: 0` — a first-pass-perfect plan (metric ≤ 0.15) converges immediately; no forced non-converged write-plan execution
- Plan mode is read-only — no code is written during the PLAN phase
- Vertical slicing is mandatory — horizontal layers (build all X, then all Y) are rejected
- No task may be XL; the ~5-files limit is advisory (cross-crate Rust features may legitimately touch 5-7 — justify); no "and" in task titles
- Every task must have acceptance criteria and a verification step
- Dependency order must be respected: a task may not precede a task it depends on
- Maximum 3 iterations before forced convergence exit
- Convergence threshold: 0.15 weighted total across six criteria
- Output paths are `tasks/plan.md` and `tasks/todo.md` — the convention expected by downstream tooling
- PKO grounding is dual-axis (P5.4): PKO for the *structure* (Procedure/Step/StepVerification), DC+BIBO for the *document*; no domain bridge needed (generic procedural knowledge)
- The PKO field mapping lives in `crates/hkask-bridge-pko/src/lib.rs` (`task_breakdown_field_to_pko`), sibling to `kanban_status_to_pko_execution` — task-breakdown owns the specification axis, kanban owns the execution axis
- Registry is authoritative — when this SKILL.md disagrees with registry templates, the registry wins
- NOTE: This SKILL.md is a hand-authored companion following the derived format. Regenerate via `kask skill derive task-breakdown` once the binary is built to stay P5.1-compliant.