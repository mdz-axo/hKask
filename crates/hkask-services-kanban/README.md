# hkask-services-kanban — Kanban Board Service

Kanban board lifecycle management: board creation, list/column management, task CRUD, and unjam detection for stuck workflows.

**Version:** v0.31.0 | **Crate:** `hkask-services-kanban`

## Modules

| Module | Purpose |
|--------|---------|
| `kanban` | Core kanban types — `KanbanBoard`, `KanbanList`, `KanbanTask` |
| `kanban_impl` | `KanbanService` implementation — CRUD, unjam detection |

## Key Types

- `KanbanBoard` — board with lists and tasks
- `KanbanList` — column/list within a board
- `KanbanTask` — task with status, priority, and CNS tracking
- `KanbanService` — service trait for board operations

## Dependencies

- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-cns` — CNS span emission for workflow events
- `hkask-storage` — persistent board state
