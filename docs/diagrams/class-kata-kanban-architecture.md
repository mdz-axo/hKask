# Kata-Kanban MCP Server — Architecture Class Diagram

**Diataxis type:** Reference
**Status:** Current (v0.31.0)

This diagram maps the structural relationships in the `hkask-mcp-kata-kanban` MCP server and its backing `hkask-services-kata-kanban` service crate. The MCP server (`KanbanServer`) is a thin tri-surface wrapper that delegates every tool call to `KanbanService`. The service owns an `HMemStore` (board/task persistence), an optional `ActivePods` (subagent spawning), and an optional `KanbanKataBridge` (full kata execution). The bridge delegates to `KataEngine`, which holds the inference port, template registry, and optional CNS/history/metric callbacks.

Two execution paths exist: (1) **prompt generation** — `task_coaching_prompt` / `task_improvement_prompt` / `task_practice_prompt` produce a rendered string for the caller to feed to an LLM; (2) **full kata execution** — `run_coaching_kata` / `run_improvement_kata` / `run_starter_kata` invoke `KataEngine::execute()` end-to-end with inference, gas tracking, and CNS spans. The MCP surface exposes only path (1); path (2) is available only through the REPL `kask kanban kata` commands and the service API.

Cross-links:
- [Kata PDCA Lifecycle State Machine](../how-to/skills-and-composition.md#kata-pdca-lifecycle-state-machine) — single-pass execution flow
- [Kata-Kanban Execution Boundary](../how-to/skills-and-composition.md#kata-kanban-execution-boundary) — sequence diagram of the two paths
- [Architecture Master: Kata](../architecture/core/hKask-architecture-master.md#kata--cybernetic-capability-development) — canonical kata architecture
- [Service Layer Class Diagram](../explanation/architecture-patterns.md#service-layer-class-diagram) — broader service decomposition

```mermaid
classDiagram
    direction TD

    class KanbanServer {
        +webid: WebID
        +replicant: String
        +daemon: Option~DaemonClient~
        +service: KanbanService
        +db: Option~SqlitePool~
        +kanban_board_create() String
        +kanban_board_list() String
        +kanban_task_create() String
        +kanban_task_list() String
        +kanban_task_move() String
        +kanban_task_assign() String
        +kanban_task_verify() String
        +kanban_task_add_gas() String
        +kanban_task_add_rjoules() String
        +kanban_task_comment() String
        +kanban_task_comments_since() String
        +kanban_task_add_deliverable() String
        +kanban_task_reopen() String
        +kanban_task_kata_coaching() String
        +kanban_task_kata_improvement() String
        +kanban_task_kata_practice() String
        +kanban_task_spawn() String
        +contract_propose_expect() String
    }

    class KanbanService {
        +store: HMemStore
        +pod_manager: Option~Arc~ActivePods~~
        +kata_bridge: Option~Arc~KanbanKataBridge~~
        +board_create() Result~Board~
        +board_list() Result~Vec~Board~~
        +board_get() Result~Option~Board~~
        +board_view() Result~String~
        +board_delete() Result~usize~
        +task_create() Result~Task~
        +task_list() Result~Vec~Task~~
        +task_get() Result~Option~Task~~
        +task_move() Result~Task~
        +task_claim() Result~Task~
        +task_verify() Result~(Task, Verification)~
        +task_reopen() Result~Task~
        +task_add_gas() Result~Task~
        +task_add_rjoules() Result~Task~
        +task_consume_gas() Result~u64~
        +task_consume_rjoules() Result~u64~
        +task_gas_exhaust() Result~Task~
        +task_comment() Result~Comment~
        +task_comments() Result~Vec~Comment~~
        +task_comments_since() Result~Vec~Comment~~
        +task_add_deliverable() Result~Task~
        +task_unassign() Result~Task~
        +task_delete() Result~()~
        +task_coaching_prompt() Result~String~
        +task_improvement_prompt() Result~String~
        +task_practice_prompt() Result~String~
        +run_coaching_kata() Result~KataResult~
        +run_improvement_kata() Result~KataResult~
        +run_starter_kata() Result~KataResult~
        +spawn_task() Result~String~
        +unjam_report() Result~Vec~UnjamItem~~
        +unjam_fix() Result~Vec~UnjamFix~~
        +decompose_prompt() Result~String~
        +decompose_populate() Result~(usize, Option~String~)~
        +board_create_from_template() Result~Board~
        +board_add_phase() Result~KanbanPhase~
        +task_set_phase() Result~Task~
        +tasks_by_phase() Result~Vec~Task~~
        +verification_prompt() Result~String~
        +verify_with_llm() Result~(Task, Verification)~
    }

    class KanbanKataBridge {
        +engine: Arc~KataEngine~
        +run_coaching_on_task() Result~KataResult~
        +run_improvement_on_task() Result~KataResult~
        +run_starter_on_task() Result~KataResult~
    }

    class KataEngine {
        +inference: Arc~dyn InferencePort~
        +registry: SqliteRegistry
        +consent_check: Option~ConsentCheckFn~
        +cns_observer: Option~CnsObserverFn~
        +history: Option~KataHistory~
        +history_store: Option~Arc~KataHistoryStore~~
        +metric_collector: Option~MetricCollectorFn~
        +cns_runtime: Option~Arc~RwLock~CnsRuntime~~
        +new() KataEngine
        +from_env() KataEngine
        +execute() Result~KataResult~
        +run_bundle() Result~KataResult~
        +load_manifest() Result~KataManifest~
        +record_history_entry() Result~Option~i64~~
    }

    class HMemStore {
        +driver: Arc~dyn DatabaseDriver~
        +encryptor: Option~Arc~Encryptor~
        +insert() Result~()~
        +update() Result~()~
        +query_by_entity() Result~Vec~HMem~~
        +query_by_entity_attribute() Result~Vec~HMem~~
        +close_by_id() Result~()~
    }

    class Board {
        +id: BoardId
        +name: String
        +owner: WebID
        +columns: Vec~ColumnDef~
        +phases: Vec~KanbanPhase~
        +created_at: DateTime
    }

    class Task {
        +id: TaskId
        +board_id: BoardId
        +title: String
        +description: Option~String~
        +status: TaskStatus
        +owner: WebID
        +assignee: Option~WebID~
        +criteria: Vec~VerificationCriterion~
        +verification: Option~Verification~
        +story_points: Option~u32~
        +estimated_hours: Option~f64~
        +priority: Option~Priority~
        +labels: Vec~String~
        +comments: Vec~Comment~
        +deliverables: Vec~String~
        +phase_id: Option~PhaseId~
        +gas_remaining: Option~u64~
        +rjoule_remaining: Option~u64~
        +gas_spend: Vec~GasEntry~
    }

    class TaskStatus {
        <<enumeration>>
        Backlog
        Ready
        InProgress
        Review
        Done
    }

    class TaskContract {
        -package_name: String
        -delegator: WebID
        -delegate: WebID
        -task_id: TaskId
        -gas_limit: u64
        -timeout: u64
        -max_attenuation: u8
        -state: ContractState
        +check_completion() ContractVerification
    }

    class SocraticRole {
        <<enumeration>>
        Planner
        Diagnoser
        Tutor
        Assessor
    }

    KanbanServer --> KanbanService : delegates
    KanbanService --> HMemStore : persists via
    KanbanService --> KanbanKataBridge : optional
    KanbanKataBridge --> KataEngine : delegates to
    KanbanService --> Board : manages
    KanbanService --> Task : manages
    Board --> ColumnDef : contains
    Board --> KanbanPhase : contains
    Task --> TaskStatus : has
    Task --> TaskContract : creates transiently
    Task --> Comment : contains
    Task --> GasEntry : audit trail
    Task --> Verification : result
    SocraticRole ..> Task : spawns inquiries as
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-017
verified_date: 2026-07-20
verified_against: mcp-servers/hkask-mcp-kata-kanban/src/lib.rs:29-34 (KanbanServer struct), crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:34-38 (KanbanService struct), crates/hkask-services-kata-kanban/src/bridge.rs:18-20 (KanbanKataBridge struct), crates/hkask-services-kata-kanban/src/kata/mod.rs:76-94 (KataEngine struct), crates/hkask-storage/src/hmem.rs:134-138 (HMemStore struct), crates/hkask-services-kata-kanban/src/kanban/types/task.rs:9-55 (Task struct), crates/hkask-services-kata-kanban/src/kanban/types/status.rs:16-27 (TaskStatus enum), crates/hkask-services-kata-kanban/src/kanban/types/contract.rs:17-40 (TaskContract struct), crates/hkask-services-kata-kanban/src/kanban/socratic.rs:265-270 (SocraticRole enum)
status: VERIFIED
-->
