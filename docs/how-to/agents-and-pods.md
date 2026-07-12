---
title: "Agents and Pods"
audience: [operators, developers, users]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle, composition]
---

# Agents and Pods

Create agent pods, use the REPL and TUI for interactive sessions, coordinate work with kanban boards, and run kata practice cycles. Pods are the sovereign runtime containers for agents; the REPL and TUI are the primary interaction surfaces; kanban and kata provide structured task coordination.

---

## Pod Architecture

A pod is a sovereign runtime container for a single agent (replicant or bot). Each pod has its own isolated storage (episodic + semantic memory), CNS runtime, MCP tool bindings, and capability token. Pods are registered with the A2A runtime for Matrix communication and can be activated for MCP access.

Each pod gets:
- **Per-pod storage**: dedicated SQLCipher database, HMem stores, embedding storage
- **Per-pod CNS runtime**: isolated span namespace for observability
- **Per-pod tool bindings**: governed MCP tools with OCAP-gated access
- **WebID**: deterministically derived from the persona definition

Three `PodKind` variants determine isolation:
- **Curator** — singleton pod, owns the SemanticIndex, CNS aggregation
- **Team** — shared workspace for multiple bots
- **Replicant** (default) — per-user sovereign pod

---

## Creating Agent Pods

### Step 1: Write an Agent Persona YAML

Define your agent in a YAML file. The persona is parsed by `AgentPersona::from_yaml()` in `crates/hkask-agents/src/pod/types.rs`.

```yaml
agent:
  name: my-assistant
  type: replicant      # "bot" or "replicant"
  version: "0.1.0"

charter:
  description: "A general-purpose assistant for code review"
  editor: "alice"

capabilities:
  - tool:execute
  - skill:rust-review

rights:
  - read: registry/skills
  - write: episodic/my-assistant

responsibilities:
  - review_pull_requests
  - generate_reports

visibility:
  default: shared
  episodic_override: private

communication_posture:
  convergence_bias: 0.7
  invariant_traits:
    - precise
    - concise
```

Validation rules (enforced by `AgentPersona::validate_fields()`):
- `name`: 1–64 chars, alphanumeric, hyphens, and underscores only
- `agent_type`: must be `bot` or `replicant`
- `version`: 1–32 chars, non-empty
- `description`: max 1000 chars
- `editor`: 1–256 chars, non-empty
- `capabilities`: max 20, each ≤128 chars

### Step 2: Deploy the Pod via CLI

```bash
kask pod create --template <template> --persona <path/to/persona.yaml> [--name <name>]
```

### Step 3: Deploy the Pod Programmatically

Use the `PodFactory` from `crates/hkask-agents/src/pod/deployment.rs`:

```rust
let factory = PodFactory::new(template_loader, consent, data_dir, db_provider);
let pod = factory.deploy(persona, pod_kind).await?;
```

Pods are persisted as files in `~/.config/hkask/pods/` with filename convention `<kind>/<name>.pod.yaml`.

### Step 4: Register and Activate

The pod lifecycle is linear: **Populated → Registered → Activated → Deactivated**.

```rust
// Register with A2A runtime
pod.register(&a2a_runtime).await?;  // Populated → Registered

// Activate for full capability
pod.activate()?;  // Registered → Activated
```

Registration mints a capability token. Activation grants MCP access and enables A2A communication. Agents are initially mutually exclusive between Chat and Server modes — set via `enter_chat_mode()` or `enter_server_mode()`.

### Pod CLI Commands

```bash
# List all active pods
kask pod list

# Check pod status
kask pod status <pod_id>
kask pod status <pod_id> --verbose

# Activate a pod
kask pod activate <pod_id>

# Deactivate a pod (terminal — cannot re-activate)
kask pod deactivate <pod_id>

# Assign a role to a pod
kask pod assign <name> <role>

# Set pod mode (chat or server)
kask pod mode <name> <mode> [--role <role>]

# Export pod as container build context
kask pod export-container <pod_id> [--output ./pod-build]

# Export pod as K8s manifests
kask pod export-k8s <pod_id> [--volume-size-gb 10] [--max-replicas 3] [--output ./k8s-manifests]
```

### Verify Pod Health

```rust
let status = active_pods.get_pod_status(&pod_id).await?;
// PodStatusInfo { pod_id, name, state, webid, agent_type, template, pod_kind, created_at }

let pods = active_pods.list_pods().await?;
let can_exec = active_pods.has_capability(&pod.webid(), "tool:execute").await;
```

---

## Using the REPL

The REPL (`kask chat`) is the primary interactive interface for hKask. It provides a readline-based shell with 30+ slash commands, real-time streaming responses, and full access to the CNS, MCP tools, and skill system.

### Starting the REPL

```bash
kask chat
```

On first run, hKask runs onboarding to resolve your identity (WebID), master key, and A2A secret. After onboarding, you enter the REPL:

```
ℏKask [Curator]>
```

### Options

| Flag | Effect |
|------|--------|
| `kask chat --tui` | Launch the terminal UI (ratatui-based multi-window interface) |
| `HKASK_TUI=1 kask chat` | Same as `--tui` via environment variable |
| `kask chat --agent <name>` | Start with a specific agent (default: Curator) |
| `kask chat --model <name>` | Start with a specific model |
| `kask chat --template <id>` | Start with a specific template loaded |
| `kask chat --input <file>` | Non-interactive: send file content and exit |

### All Slash Commands

Commands start with `/`. The complete command table sorted by category:

#### Help & Session

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/help` | `/h`, `/?` | `[COMMAND]` | Show help, or details for a specific command |
| `/quit` | `/q`, `/exit` | | End the session |
| `/clear` | `/cls` | | Clear the screen |
| `/status` | `/st` | | System status (CNS, agent, pod count) |

#### Agent & Pod Management

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/agent` | `/a` | `[NAME]` | Switch agent or show current |
| `/agents` | `/ls` | | List registered agents |
| `/pods` | | | List agent pods |

#### Model & Inference

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/model` | `/m` | `[NAME\|QUERY]` | Switch model, fuzzy search, or show current |
| `/fusion` | | `[off\|on\|status]` | Show or toggle fusion mode (multi-model deliberation) |
| `/repl` | | `[SETTING] [VALUE]` | Show or set REPL inference settings |

#### MCP & Tools

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/tools` | | | List available MCP tools |
| `/mcp` | | `list\|start <server\|all>` | Manage MCP server connections (P2: opt-in) |
| `/invoke` | `/inv` | `<server>/<tool> [args]` | Invoke an MCP tool through GovernedTool |

#### Skills & Bundles

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/templates` | `/tpl` | | List registered templates |
| `/bundle` | `/b` | `[SKILL1 SKILL2 ...] \| list \| off \| skills` | Compose, apply, or manage skill bundles |

#### Escalations & Metacognition

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/escalations` | `/esc` | | List pending escalations |
| `/resolve` | | `<ID>` | Resolve an escalation |
| `/dismiss` | | `<ID>` | Dismiss an escalation |
| `/metacognition` | `/meta` | | Run a metacognition cycle |

#### Memory & History

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/history` | `/hist` | | Show session history (episodic memory recall) |
| `/consolidate` | `/cons` | `[LIMIT] [--floor CONFIDENCE] [--max MAX_TRIPLES]` | Trigger episodic→semantic consolidation |

#### Thread Management

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/thread` | `/th` | `list\|switch <id>\|new [title]\|archive <id>` | Manage chat threads — short-term memory across sessions |

#### Communication (requires `communication` feature)

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/matrix` | `/mx` | `[ROOM_ID]` | List Matrix rooms, or show messages from a room |
| `/msg` | `/dm` | `<ROOM_ID> <MESSAGE>` | Send a message to a Matrix room |

#### Specialized

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/sovereignty` | `/sov` | | Show sovereignty status |
| `/ask` | | `<AGENT> <MESSAGE>` | Force a specific agent to respond |
| `/improv` | `/imp` | `[plussing\|yes-and\|yes-but\|freestyle\|riff]` | Set or display the active improv interaction mode |
| `/kanban` | `/kb` | `list\|board\|task\|move\|accept\|submit\|decompose\|spawn` | Kanban board and task coordination |
| `/listen` | `/rec`, `/record` | `start [SECONDS] \| stop \| view [FILE]` | Record audio, transcribe, and play back with word-level sync |
| `/talk` | `/speak` | `on \| off \| voice [DESCRIPTION]` | Enable spoken summaries of agent responses (TTS) |
| `/start` | `/tour`, `/onboarding` | | Take a guided tour of hKask's key capabilities |
| `/feedback` | | | Submit onboarding or usability feedback |

### Context Condensation (`/consolidate`)

The REPL provides explicit episodic→semantic memory consolidation. This extracts semantic triples from episodic memory, creating reusable knowledge that persists across sessions:

```
# Show consolidation status
/consolidate

# Run consolidation with defaults (limit=100)
/consolidate run

# Run with custom parameters
/consolidate run --limit 50 --floor 0.33 --max 500
```

Parameters:

| Flag | Description |
|------|-------------|
| `--limit` / `-l` | Maximum episodes to consolidate (default: 100) |
| `--floor` / `-f` | Minimum confidence threshold (0.0–1.0; default: 0.33) |
| `--max` / `-m` | Maximum semantic triples to retain (default: none) |

### Model Switching (`/model`)

Show current model:

```
/model
```

List all available models:

```
/model list
```

Exact match switch:

```
/model deepseek-v3
```

Fuzzy search:

```
/model llama
```

If multiple models match, you get a filtered list. If exactly one matches, the model switches immediately. If no provider is reachable, the model name is stored and used for the next successful inference connection.

### Memory Recall (`/history`)

Recall recent conversation turns from episodic memory:

```
/history
```

Memory recall is scoped to the current agent and requires OCAP authorization (the token must have `memory:read` capability).

### Thread Management (`/thread`)

Threads provide short-term memory that persists across sessions. They are stored in `agents/{name}/threads.json` and auto-archived on session start based on `short_term_memory_life`.

```
/thread list
/thread new "Title for this thread"
/thread switch abc12345
/thread archive def67890
```

### Clean Exit

```
/quit
```

This saves readline history to the history file and exits cleanly.

### Common REPL Workflows

**Start a new coding session:**

```
/model deepseek-v3
/mcp start 1,4-6,9
/tools
/pods
<your prompt>
```

**Audit session:**

```
/sovereignty
/status
/escalations
/metacognition
```

### REPL Troubleshooting

| Issue | Fix |
|-------|-----|
| "No A2A secret resolved" | Run `kask chat` interactively first to complete onboarding |
| Matrix commands not available | Rebuild with `cargo build --features communication` |
| TUI not available | Rebuild with `cargo build --features tui` |
| Consent denied during consolidation | Run `kask sovereignty grant --category episodic_memory` in a separate terminal, then retry |
| Unknown command | The REPL provides fuzzy matching. Type `/help` for the full command list |

---

## Using the Terminal UI

The TUI is a ratatui-based multi-window workspace. The `tui` feature must be enabled (it is on by default in `hkask-cli`).

### Start the TUI

```bash
kask chat --tui
```

The TUI workspace opens with a default layout: chat window (primary), CNS health window (sidebar), and status bar (bottom).

### Window Management

| Action | Keybinding |
|--------|-----------|
| Focus next window | `Tab` |
| Focus previous window | `Shift+Tab` |
| Split vertically | `Ctrl+V` |
| Split horizontally | `Ctrl+H` |
| Close focused window | `Ctrl+W` |
| Toggle fullscreen | `Ctrl+F` |

### Available Bridges

The TUI connects to 14 data bridges, each providing a dedicated window type:

| Bridge | Window Content |
|--------|---------------|
| `wallet` | Wallet balances, transactions, deposit addresses |
| `config` | Current configuration, settings editor |
| `backup` | Backup status, restore options |
| `registry` | Agent registry, pod listings |
| `memory` | Episodic memory search, semantic recall |
| `kanban` | Task boards, WIP limits, task status |
| `matrix` | Matrix rooms, messages, agent presence |
| `media` | Generated images, videos, audio |
| `training` | LoRA training status, adapter lifecycle |
| `companies` | Company research, financial data |
| `research` | Web search results, extracted content |
| `docproc` | Document processing queue, OCR status |
| `replica` | Style replicas, prose composition |
| `skills` | Skill registry, invocation, audit |

### Command Palette

Press `Ctrl+P` to open the command palette. Type to filter commands:

- `chat` — Switch to chat window
- `cns` — Open CNS health window
- `wallet` — Open wallet window
- `memory search` — Search episodic memory
- `skill invoke` — Invoke a skill by name

### Inference State

The TUI displays model information in the status bar:
- Current model name
- Token usage (current session)
- Circuit breaker state (Closed, HalfOpen, Open)

### Exit

Press `Ctrl+C` or `Esc` to exit the TUI. The session state is preserved — re-opening the TUI restores your window layout. Window layouts are persisted to `~/.hkask/tui-layout.yaml`. The TUI shares the same inference and memory backends as the REPL — switching between TUI and REPL preserves conversation context.

---

## Kanban System

hKask includes a headless kanban board system in `crates/hkask-services-kata-kanban/src/kanban/` for agent task coordination. Every type carries `owner: WebID` for P12 compliance (no anonymous agency).

### Kanban CLI Commands

```bash
# Create a board
kask kanban board-create <name> [--columns <columns>]

# List all boards
kask kanban board-list

# View a board as text-based column layout
kask kanban board-view <board_id>

# Create a task
kask kanban task-create <board_id> <title> [--description <desc>] [--criteria <criterion>] [--assign <agent>]

# List tasks on a board
kask kanban task-list <board_id> [--status <status>]

# Show task details
kask kanban task-show <task_id>

# Move a task to a new column
kask kanban task-move <task_id> <status>

# Assign a task to an agent
kask kanban task-assign <task_id> <agent>

# Verify a task against acceptance criteria
kask kanban task-verify <task_id> --evidence <evidence>
```

### Creating a Board Programmatically

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

### Creating Tasks

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

### WIP Limits

WIP (Work In Progress) limits are set per column via `ColumnDef::with_wip_limit()`. Per kanban discipline, WIP limits expose system problems and stimulate collaboration. The column ordering is strict — tasks move forward one step at a time.

### Task Status Transitions

The workflow has five states, with transitions constrained to adjacent columns only:

```
Backlog → Ready → InProgress → Review → Done
```

Transitions are validated by `TaskStatus::can_transition_to()`:

```rust
// Forward transitions
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

### Task Lifecycle Features

- **Assignment**: Tasks are assigned to agents with consent required (P1 sovereignty)
- **Comments**: Each task has a mini-REPL thread via `Task::comments` — agents append `Comment` entries as they work
- **Deliverables**: File paths or URLs pointing to work outputs
- **Verification**: Tasks carry acceptance criteria (`VerificationCriterion`) and a `Verification` result
- **Priority**: Optional `Priority` level for sorting
- **Phases**: Tasks can be grouped into `KanbanPhase` for work reassembly
- **Filtering**: `TaskFilter` supports filtering by `status`, `assignee`, and `priority`

### CNS Integration

Kanban operations emit CNS spans for observability. The `KanbanKataBridge` in `crates/hkask-services-kata-kanban/src/bridge.rs` connects the kanban and kata subsystems, enabling kata cycles to run directly on kanban tasks with full CNS observability, gas tracking, and automaticity computation.

The kanban board is also accessible in the TUI through the Kanban window (`crates/hkask-tui/src/windows/kanban.rs`), connected via `KanbanDataBridge`.

---

## Kata Cycles

hKask's kata system (`crates/hkask-services-kata-kanban/src/kata/`) implements the Toyota Kata methodology as an inference-driven practice engine. Three kata types are supported: **Starter** (foundational drills), **Coaching** (5-question Socratic dialogue), and **Improvement** (4-step PDCA cycle). All execute through `KataEngine` with CNS observability, gas budgeting, and automaticity tracking.

### Kata CLI Commands

```bash
# List available kata manifests
kask kata list

# Show details of a specific kata manifest
kask kata show <name>

# Execute a kata cycle
kask kata start <name> --bot <bot-name> [--ctx key=value]... [--save <path>] [--resume <path>]
```

### The Three Kata Types

| Kata Type | Source File | Description |
|-----------|-------------|-------------|
| Starter | `starter.rs` | Foundational practice routines (observation drills, PDCA practice) |
| Coaching | `coaching.rs` | 5-question Coaching Kata dialogue |
| Improvement | `improvement.rs` | 4-step PDCA Improvement Kata cycle |

### Kata Manifests

Kata cycles are defined in YAML manifests loaded from `registry/manifests/*.yaml` and deserialized into `KataManifest` (`manifest.rs`). Each manifest declares:
- **`manifest`**: id, name, kata_type (starter/coaching/improvement), description, editor, visibility
- **`gas`**: cap (default 15000), alert_threshold (0.7), hard_limit
- **`steps`**: Improvement Kata steps (ordinal, action, description, template_ref, gas_cap, output_schema)
- **`questions`**: Coaching Kata questions (number, question, description)
- **`practices`**: Starter Kata routines (name, description, frequency, duration, steps, success_criteria)
- **`cns`**: CNS configuration (emit_spans, span_namespace, variety_monitoring)
- **`error_handling`**: on_gas_exceeded, on_timeout, max_retries, retry_backoff

### The 5 Coaching Questions

The Coaching Kata runs a Socratic dialogue where the coach asks five questions to reveal the learner's thinking:

1. **What is the target condition?** — Define the measurable goal (1 week to 3 months out)
2. **What is the actual condition now?** — Facts and data, not assumptions
3. **What obstacles prevent reaching the target?** — Identify which ONE you are addressing now
4. **What is your next step? What do you expect?** — The PDCA experiment
5. **How quickly can we go and see what we learned?** — Rapid feedback cycle

The coach **never gives solutions**. Never says "you should." Only asks questions. The learner responds with specific data and observations.

### The 4-Step PDCA Cycle

The Improvement Kata runs a Plan-Do-Check-Act cycle:

1. **Plan** — Define the experiment and expected outcome
2. **Do** — Execute the experiment (with gas budget enforcement)
3. **Check** — Compare actual vs. expected results (schema-validated)
4. **Act** — Record learning and decide next step

Each step is rendered from a Jinja2 template (`.j2` files in `registry/templates/`) with context from the kata state. Steps can be marked as `classifier: true` to use the configured classifier model.

### Starter Kata Practice Routines

Starter kata builds foundational habits before tackling specific capability gaps. Practice routines include:

- **Five Questions Drill**: Practice asking the 5 coaching questions
- **PDCA Cycle**: Run Plan-Do-Check-Act experiments
- **Observation Drill**: Distinguish facts from interpretations

The engine tracks automaticity (habit strength) and streaks. CNS spans are emitted at `cns.kata` target with namespace from the manifest config. The engine also records history entries via `record_history_entry()` for trend analysis.

### Recording Results

Every kata execution produces a `KataResult` containing:
- `manifest_id`, `kata_type`, `steps_completed`, `total_steps`
- `gas_consumed`, `gas_cap`
- `step_experiences`: Vec of `StepExperience { agent, kata_type, step_label, action, output_summary, gas_used, timestamp }`
- `outcome`, `improvement_signal`, `automaticity_delta`

### Running Kata on Kanban Tasks

The `KanbanKataBridge` connects kata and kanban subsystems:

```rust
let bridge = KanbanKataBridge::new(engine);
let result = bridge.run_coaching_on_task(&task, &manifest).await?;
let result = bridge.run_improvement_on_task(&task, &manifest).await?;
let result = bridge.run_starter_on_task(&task, "sub-problem desc", &manifest).await?;
```

Task fields (title, description, criteria, comments, deliverables) are mapped into kata context.

---

## Related

- [Skills and Composition](skills-and-composition.md) — Skill invocation from the REPL
- [Sovereignty and Observability](sovereignty-and-observability.md) — CNS spans and alerts visible from REPL
- [Deployment and Transport](deployment-and-transport.md) — Matrix transport for A2A communication