---
title: "Kata PDCA Lifecycle State Machine"
audience: [architects, developers, agents, replicants]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Kata"
mds_categories: [domain, curation, lifecycle]
---

# Kata PDCA Lifecycle State Machine

## Description

The Improvement Kata PDCA cycle in `hkask-services-kata-kanban` executes as a 5-step **single-pass** sequential pipeline within the `KataEngine` that maps to four conceptual PDCA phases. Each step runs an LLM inference via the registered template (e.g., `kata-improvement/improvement-step1-direction`), validates output against the step's `output_schema`, records a `StepExperience`, and emits CNS spans. The `KataEngine::run_improvement_from()` iterates through steps **exactly once** (`for step in &manifest.steps` — no re-entry loop). The cycle is bounded by `gas.cap` (default 15,000). Metric capture flanks the execution: `metric_before` is captured pre-cycle and `metric_after` post-cycle, yielding an `ImprovementSignal` (Positive/Negative/Stalled/NotMeasured). CNS algedonic alerts fire if variety deficit exceeds threshold.

**Convergence iteration lives elsewhere:** The convergence loop (`max_iterations`, threshold, re-entry with updated data) is implemented in `ManifestExecutor::execute_manifest()` in `crates/hkask-templates/src/executor.rs` — the Pattern A Skills Model execution engine. The kata engine is a *step executor* called within that loop; it does not drive convergence itself. Kanban task mapping through `KanbanKataBridge` translates PDCA phases to `TaskStatus`: Plan→Backlog, Do→InProgress, Check→Review, Act→Done (or Backlog if convergence unmet at the ManifestExecutor level).

**Key source:** `crates/hkask-services-kata-kanban/src/kata/mod.rs:333-486` (`execute` — single-pass orchestration), `crates/hkask-services-kata-kanban/src/kata/improvement.rs:20-121` (`run_improvement_from` — single-pass `for` loop, no re-entry), `crates/hkask-services-kata-kanban/src/bridge.rs:43-56` (`run_improvement_on_task`), `crates/hkask-services-kata-kanban/src/kata/metrics.rs:6-133` (metric capture + signal).

**Convergence loop source:** `crates/hkask-templates/src/executor.rs:267-679` (`execute_manifest` — `'cascade: loop` with convergence check and re-entry at step 0), `crates/hkask-templates/src/executor.rs:746-799` (`check_convergence` — threshold + improvement ratio gating).

```mermaid
stateDiagram-v2
    [*] --> Init : enter(learner_bot, context)

    Init --> Plan : consent_check("improvement")

    state Plan {
        [*] --> Step1_Direction
        Step1_Direction --> Step2_Current : gas_check
        Step2_Current --> Step3_Target : gas_check
        Step3_Target --> PlanDone : step 3 output schema valid
        --
        note left of Step1_Direction
            Step 1: understand
            direction/challenge
            template_ref: step1-direction
            CNS: improv.direction
        end note
        note left of Step2_Current
            Step 2: grasp current
            condition with data
            template_ref: step2-current
            CNS: improv.current_condition
        end note
        note left of Step3_Target
            Step 3: establish target
            condition
            template_ref: step3-target
            CNS: improv.target_set
        end note
    }

    Plan --> Do : capture_before_metrics

    state Do {
        [*] --> Step4_Experiment
        Step4_Experiment --> DoDone : step 4 output valid
        --
        note left of Step4_Experiment
            Step 4: define next
            experiment
            template_ref: step4-experiment
            CNS: improv.experiment_run
            improv posture: Yes But
        end note
    }

    Do --> Check : execute_step returns Ok

    state Check {
        [*] --> Step5_Convergence
        Step5_Convergence --> SchemaValid : check_step_output
        SchemaValid --> ConvergenceComputed : convergence metric
        ConvergenceComputed --> CheckDone
        --
        note left of Step5_Convergence
            Step 5: convergence check
            template_ref: convergence-check
            threshold: 0.15
        end note
    }

    Check --> Act : capture_after_metrics

    state Act {
        [*] --> ComputeSignal : improvement_signal
        ComputeSignal --> CNS_Alerts : check_cns_alerts
        CNS_Alerts --> RecordResult : kata.practices completed
        RecordResult --> ActDone
        --
        note left of ComputeSignal
            Signal: Positive/Negative
            /Stalled/NotMeasured
            CNS: has_signal emitted
        end note
        note left of CNS_Alerts
            CNS algedonic check
            threshold: 100 variety
            escalation: Curator
        end note
    }

    Act --> Done : single-pass complete

    state Termination {
        Done: all steps executed, task->Done
        GasExceeded: gas > cap, abort (single pass)
    }

    note right of Done
        Single-pass execution: the KataEngine
        runs steps 1-5 exactly once.
        Convergence iteration (max 3, threshold)
        lives in ManifestExecutor::execute_manifest()
        (crates/hkask-templates).
    end note

    Plan --> GasExceeded : gas > cap
    Do --> GasExceeded : gas > cap
    Check --> GasExceeded : gas > cap

    Done --> [*]
    GasExceeded --> [*]
```

## Transition Summary

| From | To | Trigger | Source Location |
|------|----|---------|-----------------|
| `[*]` | `Init` | `KataEngine::execute("improvement", learner_bot, context)` | `kata/mod.rs:345` |
| `Init` | `Plan` | `consent_check("improvement", learner_bot)` passes | `kata/mod.rs:360-361` |
| `Step1_Direction` | `Step2_Current` | `execute_step()` returns, `gas + step_gas <= cap`, `check_step_output()` | `improvement.rs:54` |
| `Step2_Current` | `Step3_Target` | Same gas + output validation gates | `improvement.rs:54` |
| `Plan` | `Do` | `capture_before_metrics()` records CNS counters | `kata/mod.rs:364` |
| `Do` | `Check` | `execute_step()` returns step 4 output | `improvement.rs:54` |
| `Check` | `Act` | `capture_after_metrics()` records post-cycle CNS counters | `kata/mod.rs:379` |
| `Act` | `Done` | `improvement_signal` computed, `KataResult` returned (single pass complete) | `kata/mod.rs:380-398` |
| Any phase | `GasExceeded` | `gas_consumed + step_gas > gas.cap` | `improvement.rs:47-48` |
| *(Convergence iteration)* | — | Loop handled by `ManifestExecutor::execute_manifest()` (`crates/hkask-templates/src/executor.rs:267-679`), **not** in kata engine | `executor.rs:267` |

## PDCA → Kanban State Mapping

| PDCA Phase | Kanban `TaskStatus` | CNS Event | Trigger |
|------------|---------------------|-----------|---------|
| **Plan** | `Backlog` | `cns.tool.kanban` (task created) | `KanbanKataBridge::run_improvement_on_task()` |
| **Do** | `InProgress` | `cns.tool.kanban` (task moved) | Coaching Q4: "What is your next step?" |
| **Check** | `Review` | `cns.tool.kanban` (task verified) | Coaching Q5: task transitions to Review |
| **Act** | `Done` | `cns.tool.kanban` (task completed) | Verification passes |
| **Act** (fail) | `InProgress` | `cns.kata.improv.effectiveness` (degradation) | Verification fails → rework |

## Guard Conditions

- **Init → Plan:** Curator consent required for Improvement Kata; `consent_check` must return `Ok(())`. Self-consent suffices for Starter; Learner consent for Coaching.
- **Gas gate (any step):** `state.gas_consumed + step_gas > manifest.gas.cap` → `Err(KataError::GasExceeded)`. No soft continue; hard abort per `error_handling.on_gas_exceeded: abort`.
- **Output schema check:** If step has `output_schema`, all `properties` keys must exist in the inference output JSON. Missing keys → CNS `debug!` log, check returns `false`.
- **Convergence threshold:** Default 0.15; `improvement_gate: threshold_only`. On `not_reached: escalate`, Curator is notified. **Note:** Convergence checking and re-iteration (`max_iterations: 3`, `min_iterations: 1`) are performed by `ManifestExecutor::execute_manifest()` in `crates/hkask-templates`, **not** by the kata engine. The kata engine is a single-pass executor consumed within that outer loop.
- **CNS algedonic: `algedonic_threshold: 100` variety deficit → warning emitted to `cns.kata` target with `escalation_target: Curator`.

## CNS Span Diagram

```
cns.prompt.kata.improvement
├── [pre-cycle]  kata_type="improvement", bot=<learner>
├── [per-step]   step=<ordinal>, action=<action>, bot=<learner>
├── [per-step]   step=<ordinal>, passed_check=<bool>
├── [post-step]  step=<ordinal>, gas=<consumed>
├── [post-cycle] steps=<completed>, gas=<consumed>, has_signal=<bool>
└── [algedonic]  namespace=<...>, severity, deficit, threshold
```

---

<!-- DIAGRAM_ALIGNMENT
id: DIAG-FW-005
verified_date: 2026-07-01
verified_against: crates/hkask-services-kata-kanban/src/kata/mod.rs (execute:333-486), crates/hkask-services-kata-kanban/src/kata/improvement.rs (run_improvement_from:20-121 — single-pass for loop, no re-entry), crates/hkask-services-kata-kanban/src/bridge.rs (KanbanKataBridge:18-73), crates/hkask-services-kata-kanban/src/kata/metrics.rs (capture_before/after:6-105, compute_improvement_signal:76-105), crates/hkask-services-kata-kanban/src/kata/manifest.rs (KataStep, KataManifest, convergence config), crates/hkask-services-kata-kanban/src/kanban/types/status.rs (TaskStatus transitions), registry/manifests/kata-improvement.yaml (step definitions, convergence parameters, CNS spans:150-160), crates/hkask-templates/src/executor.rs (execute_manifest:209-686 — convergence loop, check_convergence:746-799)
status: VERIFIED (v2 — corrected: kata engine is single-pass; convergence loop is ManifestExecutor concern)
-->

## Cross-Reference

- [`hKask-architecture-master.md` § Kata — Cybernetic Capability Development](architecture/hKask-architecture-master.md#kata--cybernetic-capability-development)
- [`PRINCIPLES.md` § P6 — Space for Replicants & Bots](architecture/core/PRINCIPLES.md#p6--space-for-replicants--bots)
- [`kata/mod.rs`](crates/hkask-services-kata-kanban/src/kata/mod.rs) — `KataEngine::execute()` dispatch (L333-486)
- [`kata/improvement.rs`](crates/hkask-services-kata-kanban/src/kata/improvement.rs) — `run_improvement_from()` single-pass step loop (L20-121)
- [`executor.rs`](crates/hkask-templates/src/executor.rs) — `ManifestExecutor::execute_manifest()` convergence loop (L209-686), `check_convergence()` (L746-799)
- [`kata/metrics.rs`](crates/hkask-services-kata-kanban/src/kata/metrics.rs) — before/after capture, signal computation (L6-133)
- [`kata/manifest.rs`](crates/hkask-services-kata-kanban/src/kata/manifest.rs) — `KataStep`, `KataManifest`, convergence config
- [`bridge.rs`](crates/hkask-services-kata-kanban/src/bridge.rs) — `KanbanKataBridge` PDCA→task mapping (L18-73)
- [`kanban/types/status.rs`](crates/hkask-services-kata-kanban/src/kanban/types/status.rs) — `TaskStatus` column-ordered transitions
- [`registry/manifests/kata-improvement.yaml`](registry/manifests/kata-improvement.yaml) — canonical step definitions, convergence params, CNS spans
