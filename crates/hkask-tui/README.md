# hkask-tui — Terminal UI (ratatui)

Terminal user interface built with ratatui. Provides windowed REPL with live bridge connections to all hKask services: chat, CNS monitor, Curator, pods, wallet, config, backup, registry, skills, memory, kanban (interactive multi-column board), Matrix, media, training, terminal (PTY), and editor.

**Version:** v0.31.0 | **Crate:** `hkask-tui`

## Features

- 22 window types via workspace splits
- 9 domain-specific bridge traits with mock + live implementations
- Interactive kanban board: 5-column layout, keyboard navigation (h/j/k/l), task moving (m), Tab-to-chat
- PTY-backed interactive shell (`portable-pty`, bash/fish)
- File editor with open/save (`Ctrl+S`/`Ctrl+O`)
- Logo window rasterized from SVG at half-block Unicode
- Live bridges: Chat, CNS, Curator, Pods, Wallet, Config, Backup, Registry, Skills, Memory, Kanban, Matrix, Media, Training

## Key Controls

| Key | Action |
|-----|--------|
| `Ctrl+N` | New window |
| `Ctrl+P` | Command palette (fuzzy search 22 window kinds) |
| `Ctrl+S` | Save file |
| `Ctrl+O` | Open file |
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
`hkask-mcp-kanban` MCP server (8 tools).

## Dependencies

- `ratatui` — terminal rendering framework
- `portable-pty` — PTY backend for terminal window
- `hkask-services-context` — AgentService for live bridge data
- `hkask-mcp` — MCP runtime for tool dispatch in bridges
