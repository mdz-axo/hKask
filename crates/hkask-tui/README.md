# hkask-tui — Terminal UI (ratatui)

Terminal user interface built with ratatui. Provides windowed REPL with live bridge connections to all hKask services: chat, CNS monitor, Curator, pods, wallet, config, backup, registry, skills, memory, kanban, Matrix, media, training, terminal (PTY), and editor.

**Version:** v0.30.0 | **Crate:** `hkask-tui`

## Features

- 19 window types via workspace splits
- 9 domain-specific bridge traits with mock + live implementations
- PTY-backed interactive shell (`portable-pty`, bash/fish)
- File editor with open/save (`Ctrl+S`/`Ctrl+O`)
- Logo window rasterized from SVG at half-block Unicode
- Live bridges: Chat, CNS, Curator, Pods, Wallet, Config, Backup, Registry, Skills, Memory, Kanban, Matrix, Media, Training

## Key Controls

| Key | Action |
|-----|--------|
| `Ctrl+N` | New window |
| `Ctrl+S` | Save file |
| `Ctrl+O` | Open file |
| `Tab` | Switch pane |

## Dependencies

- `ratatui` — terminal rendering framework
- `portable-pty` — PTY backend for terminal window
- `hkask-services-context` — AgentService for live bridge data
- `hkask-mcp` — MCP runtime for tool dispatch in bridges
