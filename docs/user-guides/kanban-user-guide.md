---
title: "Kanban User Guide"
audience: [users, replicants, agents]
last_updated: 2026-06-24
version: "0.31.0"
status: "Active"
domain: "Coordination"
mds_categories: [domain, composition, lifecycle]
---

# Kanban User Guide

Headless kanban for agent coordination. Boards, tasks, WIP limits, comments,
deliverables, verification, gas/rJoule budgets, Socratic inquiry, and
replicant spawning — all from CLI or REPL. Anchored on the PKO (Procedural
Knowledge Ontology) standard with PROV-O provenance.

## Quick Start

```bash
# Create a board (from template)
/kanban board create "My Project" --template software-project

# Decompose a project into tasks
/kanban decompose <board-id> "Build a CLI tool for CSV to JSON conversion"
  → copy the prompt to your LLM
  → paste the JSON response:
/kanban populate <board-id> '<json-from-llm>'

# View the board
/kanban view <board-id>

# Move a task through the workflow
/kanban move <task-id> ready
/kanban move <task-id> in_progress

# Set gas/rJoule budgets on task creation
/kanban task create <board> "Refactor auth" --gas 10000 --rjoules 250000

# Add a comment (agent ↔ replicant communication)
/kanban note <task-id> "Parser module complete, working on type inference"

# Link a deliverable
/kanban deliver <task-id> ./src/parser.rs

# Add more gas to a running task
/kanban task add-gas <task-id> 5000

# Submit for verification
/kanban submit <task-id> "Parser handles all CSV dialects, tests pass"

# Reopen a completed/gas-exhausted task
/kanban reopen <task-id> --gas 10000
```

## Board Templates

| Template | Use |
|----------|-----|
| `software-project` | Standard dev: Backlog→Ready→InProgress→Review→Done |
| `writing-project` | Content: Ideas→Outlining→Drafting→Editing→Published |
| `scientific-research` | Science: Questions→Hypothesis→Experiment→Analysis→Conclusions |
| `investment-research` | MAIA framework: Discovery→Business→Financial→Valuation→Decision→Monitoring |

Create with: `/kanban board create <name> --template <name>`

## Task Lifecycle

```
Backlog → Ready → InProgress → Review → Done
  ↑        ↑         ↑           ↑
  │        │         │           └── verify/submit
  │        │         └── agent works, adds notes + deliverables
  │        └── accept (consent to assignment)
  └── create
```

## De-Jamming

When things get stuck:

```bash
/kanban unjam <board-id>        # scan for stuck tasks
/kanban unjam <board-id> --fix  # auto-fix: unassign stale, reopen unverified
```

The de-jammer detects:
- Tasks stuck in InProgress > 2x estimated hours
- Assignments in Backlog/Ready > 24h with no movement
- Tasks in Done with no verification

## Troubleshooting & Error Recovery

### Common Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| **Board creation fails** | Missing or invalid template name | List available templates: `/kanban board templates`. Use one of: `software-project`, `writing-project`, `scientific-research`, `investment-research`. |
| **Invalid template** | Typo or unsupported template | Check exact name from `/kanban board templates`. Templates are case-sensitive. |
| **Populate with malformed JSON** | LLM returned non-JSON or invalid structure | Re-run `/kanban decompose <board-id> "..."` and copy the prompt to a different model if needed. Expected schema: array of task objects with `title`, `description`, `status`, `priority`. |
| **Permission denied on assignment** | Agent lacks capability token (P4 OCAP) | Assign via Curator: `/kanban assign <task-id> <agent-webid>`. Self-assignment (`/kanban accept <task-id>`) requires board delegation token. |
| **Verify fails (LLM rejection)** | Evidence insufficient | Review rejection reason. Add evidence and re-submit: `/kanban verify <task-id> <updated-evidence>`. Escalate: `/kanban escalate <task-id>`. |
| **Agent unresponsive** | Agent pod crashed or disconnected | Check pod health: `/agent status <webid>`. Restart: `/agent restart <webid>`. Unassign: `/kanban unassign <task-id>`. |
| **WIP limit exceeded** | Column at capacity | Complete or move tasks. Override (Curator): `/kanban override-wip <board-id> <column> <new-limit>`. |

### General Recovery Workflow

1. **Diagnose:** `/kanban unjam <board-id>` (scan for stuck tasks)
2. **Inspect:** `/kanban view <board-id>` (full board), `/kanban view <board-id> blocked` (blocked filter)
3. **Fix:** `/kanban unjam <board-id> --fix` (auto), `/kanban unassign <task-id>` (manual), `/kanban move <task-id> backlog` (reset)
4. **Verify:** `/kanban cns health` (CNS health check)

### CNS Spans for Kanban

Every kanban operation emits CNS spans (from `crates/hkask-types/src/cns.rs`):

| Span | Emitted When |
|------|-------------|
| `TaskCreated` | New task added to board |
| `TaskMoved` | Task moves between columns |
| `TaskAssigned` | Agent assigned to task (P12 consent) |
| `TaskVerified` | Task passes LLM verification |
| `BoardCreated` | New board created from template |

Monitor with: `kask cns spans --filter kanban`

## Katas on Tasks

Scientific thinking tools available per task:

```bash
/kanban coach <task-id>        # Coaching Kata — 5-question dialogue
/kanban improve <task-id>      # Improvement Kata — PDCA experiment
/kanban practice <task-id>     # Starter Kata — Observation drill
```

## Capability Packages

Reusable OCAP delegation bundles for spawning:

| Package | Skills | Tools | Use |
|---------|--------|-------|-----|
| `backend-dev` | coding-guidelines, tdd, kanban | kanban, research | Backend development |
| `docs-writer` | kanban | kanban, docproc, research | Documentation/writing |

```bash
/kanban spawn <task-id> backend-dev
```

## Filtered Views

```bash
/kanban view <board-id>              # full board
/kanban view <board-id> in_progress  # status filter
/kanban view <board-id> critical     # priority filter
/kanban view <board-id> auth         # label filter
/kanban view <board-id> <webid>     # assignee filter
```

## CLI Equivalents

All REPL commands have CLI equivalents:

```bash
kask kanban board-create <name> [--template <t>]
kask kanban board-view <id> [filter]
kask kanban task-create <board> <title> [-d desc] [-c criteria] [--assign agent]
kask kanban task-list <board> [-s status]
kask kanban task-move <task> <status>
kask kanban task-assign <task> <agent>
kask kanban task-verify <task> -e "evidence"
```

## Socratic Inquiry

Structured 4-stage Socratic exploration using kanban tasks and kata prompts.
Each stage uses a different kata type to guide the dialogue. Quality gates
ensure responses meet structural expectations before advancing.

```bash
# Start a Socratic inquiry
/kanban socratic start <board> "What is knowledge?"
  → Creates inquiry task, shows Elicit stage prompt (Coaching Kata)

# Respond and advance (with quality gate)
/kanban socratic continue <task> "Knowledge is justified true belief."
  → Quality gate checks response → posts comment → advances to next stage

# Check inquiry status
/kanban socratic status <task>
  → Shows current stage, comment count, and quality gate assessment

# Spawn a 4-role inquiry team for coordinated exploration
/kanban socratic team <board> "What is knowledge?"
  → Creates Planner, Diagnoser, Tutor, and Assessor tasks
  → Each role brings a different perspective on the topic
```

### Stages

| Stage | Status | Kata Type | Focus |
|-------|--------|-----------|-------|
| Elicit | Backlog | Coaching Kata | Target condition, actual condition, obstacles, experiment |
| Structure | Ready | Improvement Kata | Direction, current state, target, PDCA iteration |
| Test | InProgress | Practice Kata | Observations vs. interpretations on a sub-problem |
| Summarize | Review | Custom prompt | Synthesize: what learned, evidence, uncertainty |

### Quality Gates

Each stage has structural expectations evaluated by `quality_check()`:

| Stage | Gate | Requirement |
|-------|------|------------|
| Elicit | Substance | Response ≥ 30 characters |
| Structure | Direction + Current | Must identify target AND present state |
| Test | Facts vs. Interpretations | Must distinguish observations from assumptions |
| Summarize | Always passes | Delegated to task_verify |

## Gas & rJoule Budgets

Two separate resource budgets for subagent execution:

| Currency | Purpose | Rate |
|----------|---------|------|
| **Gas** | Software compute (templates, tool dispatch) | Arbitrary units |
| **rJoules** | Inference/API costs | 250,000 rJ ≈ $1 inference spend |

```bash
# Set at task creation
/kanban task create <board> "Analyze logs" --gas 10000 --rjoules 250000

# Add more during execution
/kanban task add-gas <task-id> 5000
/kanban task add-rjoules <task-id> 125000

# View in task list
/kanban task list <board>
  → shows gas_remaining and rjoule_remaining per task
```

Gas consumption is tracked via `gas_spend` audit trail — each consumption
records what operation consumed gas and when. When gas hits zero, the
subagent posts a comment asking for more. If ignored for > 1 hour, the
unjam flow auto-completes the task.

## Ontology Anchoring (PKO)

All kanban operations carry PKO (Procedural Knowledge Ontology) concept
annotations in their JSON responses and CNS spans:

| Kanban Type | PKO Concept |
|------------|-------------|
| Board | `pko:Procedure` |
| Task | `pko:Step` |
| Task move | `pko:ChangeOfStatus` |
| Verification | `pko:StepVerification` |
| Comment | `pko:UserFeedbackOccurrence` |
| Kata prompt | `pko:UserQuestionOccurrence` |
| Unjam item | `pko:IssueOccurrence` |
| Deliverable | `prov:wasGeneratedBy` |
| Gas consumption | `prov:used` |
| Assignment | `prov:wasAssociatedWith` |

CNS spans include `ontology` field for type-aware feedback routing.
PKO reference: <https://w3id.org/pko> — Carriero et al., arXiv:2503.20634

## Architecture

- **Persistence:** RDF triples via `TripleStore` (kanban:board, kanban:task, kanban:board_tasks:{id})
- **P12:** Every action carries `owner: WebID`
- **P1:** Assignment requires `ConsentProof`
- **P3:** Clear Boundaries — verification is evidence-driven, not keyword-gated
- **OCAP:** Spawning delegates capability tokens with attenuation
- **Gas/rJoule:** Two separate budgets for compute vs. inference, with `gas_spend` audit trail
- **Socratic inquiry:** 4-stage kata-powered dialogue with structural quality gates
- **Multi-role coordination:** Planner/Diagnoser/Tutor/Assessor as parallel inquiry tasks
- **Ontology anchoring:** Every response carries `pko` concept field; CNS spans carry `ontology` field
- **PKO reference:** <https://w3id.org/pko> (Carriero et al., ISWC 2024 / PERKS project)
- **CNS:** 5 span types for observability, now ontology-tagged
