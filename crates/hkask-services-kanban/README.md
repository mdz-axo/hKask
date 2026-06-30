# hkask-services-kanban — Kanban Board Service

Kanban board lifecycle management: board creation, column management (with WIP limits), task CRUD (with consent-gated assignment), LLM-mediated verification, spawn specs, capability packages, and unjam detection for stuck workflows.

Kanban is the **tool/board/framework** for applying the Kata process (`hkask-services-kata`) to work. PDCA phases map directly to Kanban task statuses (Plan→Backlog, Do→InProgress, Check→Review, Act→Done).

**Version:** v0.31.0 | **Crate:** `hkask-services-kanban`

## Modules

| Module | Purpose |
|--------|---------|
| `kanban` | Core types — `Board`, `Task`, `TaskSpec`, `SpawnSpec`, `CapabilityPackage`, `TaskContract`, etc. |
| `kanban_impl` | `KanbanService` implementation — CRUD, consent-gated assignment, verification, de-jam |
| `kanban_impl::kata` | Kata prompt generation for coaching, improvement, and starter kata cycles on tasks |
| `kanban_impl::decompose` | Task decomposition into INVEST-compliant sub-tasks |
| `kanban_impl::dejam` | Unjam detection and auto-fix for stuck tasks |
| `kanban_impl::spawn` | Sub-replicant spawning with OCAP capability packages |
| `kanban_impl::verification` | LLM-mediated two-step verification (prompt → JSON → pass/fail) |
| `kanban_impl::phases` | Kanban phase management |
| `kanban_impl::comments` | Task comment threading |
| `socratic` | Socratic inquiry cycle — 4-role interrogation for task diagnosis |

## Key Types

- `Board` — board with columns (WIP limits), phases, owner
- `ColumnDef` — column definition with status mapping and WIP limit
- `Task` — task with status, assignee, criteria, verification, gas/rJoule tracking
- `TaskSpec` — builder for task creation (title, description, criteria, priority, budgets)
- `TaskStatus` — Backlog / Ready / InProgress / Review / Done with valid transitions
- `Priority` — Low / Medium / High / Critical
- `TaskFilter` — filter tasks by status, assignee, priority
- `TaskContract` — OCAP contract for task delegation with pre/post conditions
- `ContractState` — Pending / Active / Completed / Violated
- `SpawnSpec` / `CapabilityPackage` — sub-replicant spawning with delegated capabilities
- `GasEntry` — gas spend and refill tracking entries
- `Verification` / `VerificationCriterion` — LLM-mediated task verification
- `ConsentProof` — agent consent record for task assignment
- `KanbanPhase` — named workflow phase with ordering
- `KanbanService` — primary service (board CRUD, task CRUD, assignment, verification, de-jam)
- `KanbanError` — error taxonomy (InvalidInput, NotFound, InvalidTransition, WipLimitExceeded, etc.)
- `UnjamItem` / `UnjamFix` — de-jam detection results
- `QualityGate` (socratic) — socratic inquiry quality gate

## Key Features

- **WIP limits** per column (Anderson, 2010: "limit WIP to expose problems")
- **CNS behavioral contracts:** task assignment uses `expect:` + `[P{N}]` with pre/post conditions
- **Kata integration:** coaching, improvement, and starter katas available as task primitives
- **Capability packages:** reusable OCAP delegation bundles stored as YAML
- **Board templates:** `software-project`, `writing-project`, `scientific-research`, `investment-research`
- **De-jamming:** auto-detects and fixes stuck tasks, stale assignments, unverified completions
- **LLM-mediated verification:** two-step prompt → JSON → structured pass/fail
- **Persistence:** boards and tasks stored as RDF triples via `TripleStore` (MDS §2)

## Dependencies

- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-storage` — persistent board state via `TripleStore`
- `hkask-types` — CNS span types, RDF types
- `hkask-agents` — ActivePods for pod management
- `hkask-capability` — OCAP delegation tokens
