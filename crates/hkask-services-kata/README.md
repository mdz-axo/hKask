# hkask-services-kata — Toyota Kata Engine

Improvement Kata and Coaching Kata engine: target conditions, PDCA cycles, obstacle parking lots, and scientific thinking habit tracking. Drives the continuous improvement feedback loop across all agent pods.

Kata is the **process**; Kanban (`hkask-services-kanban`) is the **tool/board** for applying that process to work. PDCA phases map directly to Kanban task statuses (Plan→Backlog, Do→InProgress, Check→Review, Act→Done).

**Version:** v0.31.0 | **Crate:** `hkask-services-kata`

## Modules

| Module | Purpose |
|--------|---------|
| `kata_impl::coaching` | 5-question Coaching Kata cycle with CNS span emission and gas tracking |
| `kata_impl::improvement` | 4-step PDCA Improvement Kata with before/after metrics and improvement signals |
| `kata_impl::starter` | Starter Kata drills — observation, PDCA, five-questions practice |
| `kata_impl::execution` | Template-based step execution with inference routing |
| `kata_impl::manifest` | `KataManifest` loading and validation |
| `kata_impl::state` | `KataState` — current step, outputs, gas, step experiences |
| `kata_impl::history` | `KataHistory` persistence and automaticity tracking |
| `kata_impl::metrics` | CNS variety counter integration and metric collection |
| `kata_impl::error` | `KataError` taxonomy |

## Key Types

- `KataEngine` — primary engine: runs coaching, improvement, and starter cycles with inference, CNS observation, and consent gates
- `KataManifest` — kata definition (type, steps/questions, gas budget, CNS config, consent)
- `KataState` — mutable execution state (current step, step outputs, gas consumed, step experiences)
- `KataResult` — completion result (steps completed, gas consumed, outcome, improvement signal)
- `KataStep` — a single step within a kata (ordinal, template ref, description, classifier flag)
- `KataHistory` / `StepExperience` / `PracticeEntry` — practice history and automaticity tracking
- `KataError` — error taxonomy (InferenceFailed, TemplateNotFound, GasExceeded, NoSteps, etc.)
- `ImprovementDirection` / `ImprovementSignal` — PDCA outcome classification

## Key Features

- **PDCA cycle with before/after metrics:** Captures `metric_before` from CNS counters, executes 4-step PDCA, captures `metric_after`, computes `ImprovementSignal`
- **Automaticity tracking:** Linearly approaches 1.0 over 21 consecutive practice days; 3+ day gaps trigger habit decay alerts
- **CNS variety counters:** `kata.practices.completed`, `kata.automaticity.score`, `kata.habit.formation`
- **OCAP consent gates:** kata-starter (self-consent), kata-improvement (Curator), kata-coaching (Learner)
- **Memory integration:** Every step produces a `StepExperience` recorded to episodic memory
- **Kanban integration:** PDCA experiments map to kanban tasks; coaching 5 questions map to task fields; improvement cycles tracked as task state transitions

## Dependencies

- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-cns` — CNS span emission for kata events
- `hkask-storage` — persistent kata state
- `hkask-templates` — template registry and Jinja2 rendering
- `hkask-types` — CNS span types
- `hkask-ports` — hexagonal port traits
- `hkask-inference` — Inference router
- `minijinja` — Template rendering
