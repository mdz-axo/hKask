# hkask-tui — Terminal UI (ratatui)

Terminal user interface built with Ratatui. The crate owns workspace state, rendering, key routing, window-local state, and presentation-facing bridge traits. Production bridge implementations live in `hkask-repl`; bridge coverage is mixed rather than uniformly live.

**Version:** v0.31.0 | **Crate:** `hkask-tui`

## Features

- 16 window types via workspace splits
- 15 optional domain-specific bridge traits with mock and production adapters
- Interactive kanban board: 5-column layout, keyboard navigation (h/j/k/l), task moving (m), Tab-to-chat
- PTY-backed interactive shell (`portable-pty`, bash/fish)
- File editor with open/save (`Ctrl+S`/`Ctrl+O`)
- Curator chat is the default mode in the Chat window (P12.1 dual-presence)
- Window management via slash commands: `/open`, `/close`, `/split`, `/focus`, `/tab`, `/palette`
- Required system/inference bridges plus optional domain adapters; missing adapters degrade individual windows instead of blocking startup

## Key Controls

| Key | Action |
|-----|--------|
| `Ctrl+N` | New Chat window |
| `Ctrl+P` | Command palette (fuzzy search 16 window kinds) |
| `Ctrl+W` | Close focused window |
| `Tab` | Focus next pane (in Kanban: toggle board/chat) |
| `Ctrl+Q` | Quit |
| `?` | Help overlay |

## Kanban Board

The Kanban window provides an interactive 5-column task board:

| Key | Action |
|-----|--------|
| `h` / `l` / Arrow keys | Switch between columns |
| `j` / `k` | Move selection up/down within column |
| `m` | Advance selected task to next status column |
| `PgUp` / `PgDn` | Jump 5 tasks up/down |
| `Home` / `End` | Jump to first/last task |
| `Tab` | Toggle between board view and scoped chat |

Columns: Backlog, Ready, In Progress, Review, Done. Tasks show priority indicators
(critical/high/medium/low) and assignee when assigned. Selected task is highlighted with
reverse video. Detail bar shows full task info. The Chat tab connects to the
`hkask-mcp-kata-kanban` MCP server (8 tools).

## Dependencies

- `ratatui` — terminal rendering framework
- `portable-pty` — PTY backend for terminal window
- `crossterm` — terminal events and input
- `serde` / `serde_json` — validated structural layout persistence

Architecture and current limitations are documented in [`docs/explanation/tui-architecture.md`](../../docs/explanation/tui-architecture.md).
