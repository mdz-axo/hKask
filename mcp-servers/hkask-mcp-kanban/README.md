# hkask-mcp-kanban

Kanban board coordination MCP server — task management with WIP limits, OCAP delegation, and CNS observability.

## Tools (9)

| Tool | Description |
|------|-------------|
| `kanban_board_create` | Create a kanban board |
| `kanban_board_list` | List kanban boards |
| `kanban_task_create` | Create a task |
| `kanban_task_list` | List tasks on a board |
| `kanban_task_move` | Move task between columns |
| `kanban_task_assign` | Assign task to agent |
| `kanban_task_verify` | Verify task completion |
| `contract_propose_expect` | Propose contract expectation |
| `run` | Main run loop |

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
hkask-mcp-kanban
```

## Usage

```
"Create a kanban board for my writing project"  → kanban_board_create
"Add a task to draft chapter 3"                 → kanban_task_create
"Move the review task to Done"                  → kanban_task_move
```
