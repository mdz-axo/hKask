---
title: "How to Use the Kanban System — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Use the Kanban System

hKask includes a headless kanban board system in `crates/hkask-services-kata-kanban/src/kanban/` for agent task coordination. Every type carries `owner: WebID` for P12 compliance (no anonymous agency). This guide covers creating boards, managing tasks, WIP limits, task transitions, and CNS integration.

## Creating a Board

Boards are created through `KanbanService` (in `service_impl/`). Each board is owned by a replicant and contains columns, phases, and tasks.

```rust
use hkask_services_kata_kanban::kanban::{Board, ColumnDef, TaskStatus};

let columns = vec![
    ColumnDef::new("Backlog".into(), TaskStatus::Backlog, 0)
        .with_wip_limit(20),
    ColumnDef::new("Ready".into(), TaskStatus::Ready, 1)
        .with_wip_limit(5),
    ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2)
        .with_wip_limit(3),
    ColumnDef::new("Review".into(), TaskStatus::Review, 3)
        .with_wip_limit(3),
    ColumnDef::new("Done".into(), TaskStatus::Done, 4),
];

let board = Board::new("Sprint 12".into(), owner_webid, columns);
```

## Creating Tasks

Tasks are created from a `TaskSpec` and always start in `Backlog`:

```rust
let spec = TaskSpec {
    title: "Add clipboard support".into(),
    description: Some("Implement system clipboard integration for the TUI".into()),

    criteria: vec![
        VerificationCriterion { description: "Copy works on Linux".into() },
        VerificationCriterion { description: "Copy works on macOS".into() },
    ],
    gas_budget: Some(5000),
    rjoule_budget: Some(250_000),
};

let task = Task::new(board.id, spec, owner_webid);
```

Tasks are created unassigned. An accepting agent must claim the task using its own authenticated `WebID`; callers cannot assign another agent during creation. Tasks carry gas budgets (`gas_remaining`) and rJoule budgets (`rjoule_remaining`, where 250k ≈ $1 spend). When these deplete, the task auto-completes via the gas exhaustion path.

## WIP Limits

WIP (Work In Progress) limits are set per column via `ColumnDef::with_wip_limit()`. Per kanban discipline, WIP limits expose system problems and stimulate collaboration. The column ordering is strict — tasks move forward one step at a time.

```rust
// Set a WIP limit of 3 for the InProgress column
let in_progress = ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2)
    .with_wip_limit(3);
```

## Task Status Transitions

The workflow has five states, with transitions constrained to adjacent columns only:

```
Backlog → Ready → InProgress → Review → Done
```

Transitions are validated by `TaskStatus::can_transition_to()`:

```rust
// Forward: Backlog → Ready → InProgress → Review → Done
assert!(TaskStatus::Backlog.can_transition_to(TaskStatus::Ready));
assert!(TaskStatus::Ready.can_transition_to(TaskStatus::InProgress));
assert!(TaskStatus::InProgress.can_transition_to(TaskStatus::Review));
assert!(TaskStatus::Review.can_transition_to(TaskStatus::Done));

// Backward (regression): allowed one step only
assert!(TaskStatus::InProgress.can_transition_to(TaskStatus::Ready));

// Skipping columns is prohibited
assert!(!TaskStatus::Backlog.can_transition_to(TaskStatus::InProgress));

// Done is terminal
assert!(!TaskStatus::Done.can_transition_to(TaskStatus::Review));
```

## Task Lifecycle Features

- **Assignment**: Tasks are assigned to agents with consent required (P1 sovereignty).
- **Comments**: Each task has a mini-REPL thread via `Task::comments` — agents append `Comment` entries as they work.
- **Deliverables**: File paths or URLs pointing to work outputs.
- **Verification**: Tasks carry acceptance criteria (`VerificationCriterion`) and a `Verification` result.
- **Priority**: Optional `Priority` level for sorting.
- **Phases**: Tasks can be grouped into `KanbanPhase` for work reassembly.
- **Filtering**: `TaskFilter` supports filtering by `status`, `assignee`, and `priority`.

## CNS Integration

Kanban operations emit CNS spans for observability. The `KanbanKataBridge` in `crates/hkask-services-kata-kanban/src/bridge.rs` connects the kanban and kata subsystems, enabling kata cycles (coaching, improvement, starter) to run directly on kanban tasks with full CNS observability, gas tracking, and automaticity computation.

## TUI Access

The kanban board is accessible in the TUI through the Kanban window (`crates/hkask-tui/src/windows/kanban.rs`), connected via `KanbanDataBridge`.
