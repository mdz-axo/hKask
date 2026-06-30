# hkask-mcp-kata-kanban

Kanban board coordination MCP server — task management with WIP limits, OCAP delegation, and CNS observability.

## Tools (8)

| Tool | Description |
|------|-------------|
| `kanban_board_create` | Create a new kanban board with optional custom columns |
| `kanban_board_list` | List all kanban boards owned by the caller |
| `kanban_task_create` | Create a new task on a kanban board |
| `kanban_task_list` | List tasks on a kanban board, optionally filtered by status |
| `kanban_task_move` | Move a task to a new column (status transition) |
| `kanban_task_assign` | Assign a task to an agent with consent proof (P1 compliance) |
| `kanban_task_verify` | Verify a task against its acceptance criteria |
| `contract_propose_expect` | Create kanban tasks for contracts missing expect: annotations. Takes JSON from propose_missing_expect_annotations. |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |

## Quick Start

```bash
# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-kata-kanban
```

## Usage

```
"Create a kanban board for my writing project"  → kanban_board_create
"Add a task to draft chapter 3"                 → kanban_task_create
"Move the review task to Done"                  → kanban_task_move
```
