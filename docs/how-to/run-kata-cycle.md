---
title: "How to Run a Kata Cycle — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Run a Kata Cycle

hKask's kata system (`crates/hkask-services-kata-kanban/src/kata/`) implements the Toyota Kata methodology as an inference-driven practice engine. Three kata types are supported: **Starter** (foundational drills), **Coaching** (5-question Socratic dialogue), and **Improvement** (4-step PDCA cycle). All execute through `KataEngine` with CNS observability, gas budgeting, and automaticity tracking.

## The Three Kata Types

| Kata Type | Source File | Description |
|-----------|-------------|-------------|
| Starter | `starter.rs` | Foundational practice routines (observation drills, PDCA practice) |
| Coaching | `coaching.rs` | 5-question Coaching Kata dialogue |
| Improvement | `improvement.rs` | 4-step PDCA Improvement Kata cycle |

## Starting a Kata Cycle

Kata cycles are defined in YAML manifests loaded from `registry/manifests/*.yaml` and deserialized into `KataManifest` (`manifest.rs`).

### Kata Manifests

Each manifest declares:
- **`manifest`**: id, name, kata_type (starter/coaching/improvement), description, editor, visibility
- **`gas`**: cap (default 15000), alert_threshold (0.7), hard_limit
- **`steps`**: Improvement Kata steps (ordinal, action, description, template_ref, gas_cap, output_schema)
- **`questions`**: Coaching Kata questions (number, question, description)
- **`practices`**: Starter Kata routines (name, description, frequency, duration, steps, success_criteria)
- **`cns`**: CNS configuration (emit_spans, span_namespace, variety_monitoring)
- **`error_handling`**: on_gas_exceeded, on_timeout, max_retries, retry_backoff

Loading a manifest:
```rust
let manifest = engine.load_manifest("coaching/five-questions").await?;
```

## The 5 Coaching Questions

The Coaching Kata runs a Socratic dialogue where the coach asks five questions to reveal the learner's thinking:

1. **What is the target condition?** — Define the measurable goal (1 week to 3 months out)
2. **What is the actual condition now?** — Facts and data, not assumptions
3. **What obstacles prevent reaching the target?** — Identify which ONE you're addressing now
4. **What is your next step? What do you expect?** — The PDCA experiment
5. **How quickly can we go and see what we learned?** — Rapid feedback cycle

The coach **never gives solutions**. Never says "you should." Only asks questions. The learner responds with specific data and observations.

```rust
let result = engine.run_coaching_from(&manifest, &mut state).await?;
// KataResult { steps_completed, gas_consumed, step_experiences, ... }
```

## The 4-Step PDCA Cycle

The Improvement Kata runs a Plan-Do-Check-Act cycle:

1. **Plan** — Define the experiment and expected outcome
2. **Do** — Execute the experiment (with gas budget enforcement)
3. **Check** — Compare actual vs. expected results (schema-validated)
4. **Act** — Record learning and decide next step

Each step is rendered from a Jinja2 template (`.j2` files in `registry/templates/`) with context from the kata state. Steps can be marked as `classifier: true` to use the configured classifier model.

```rust
let result = engine.run_improvement_from(&manifest, &mut state).await?;
```

## Starter Kata Practice Routines

Starter kata builds foundational habits before tackling specific capability gaps. Practice routines include:

- **Five Questions Drill**: Practice asking the 5 coaching questions
- **PDCA Cycle**: Run Plan-Do-Check-Act experiments
- **Observation Drill**: Distinguish facts from interpretations

The engine tracks automaticity (habit strength) and streaks:

```rust
let result = engine.run_starter(&manifest, &mut state).await?;
// Tracks automaticity, streak_days, days_since_last in CNS spans
```

## Recording Results

Every kata execution produces a `KataResult` containing:
- `manifest_id`, `kata_type`, `steps_completed`, `total_steps`
- `gas_consumed`, `gas_cap`
- `step_experiences`: Vec of `StepExperience { agent, kata_type, step_label, action, output_summary, gas_used, timestamp }`
- `outcome`, `improvement_signal`, `automaticity_delta`

CNS spans are emitted at `cns.kata` target with namespace from the manifest config. The engine also records history entries via `record_history_entry()` for trend analysis.

## Running Kata on Kanban Tasks

The `KanbanKataBridge` connects kata and kanban subsystems:

```rust
let bridge = KanbanKataBridge::new(engine);
let result = bridge.run_coaching_on_task(&task, &manifest).await?;
let result = bridge.run_improvement_on_task(&task, &manifest).await?;
let result = bridge.run_starter_on_task(&task, "sub-problem desc", &manifest).await?;
```

Task fields (title, description, criteria, comments, deliverables) are mapped into kata context.
