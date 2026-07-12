---
title: "How to Use the Terminal UI — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Use the Terminal UI

**Goal:** Use the ratatui-based Terminal UI workspace for multi-window agent interaction.

**Prerequisites:** The `tui` feature must be enabled (it is on by default in `hkask-cli`).

## 1. Start the TUI

```bash
kask chat --tui
```

Or from within a chat session:

```
kask> /tui
```

The TUI workspace opens with a default layout: chat window (primary), CNS health window (sidebar), and status bar (bottom).

## 2. Window Management

| Action | Keybinding |
|--------|-----------|
| Focus next window | `Tab` |
| Focus previous window | `Shift+Tab` |
| Split vertically | `Ctrl+V` |
| Split horizontally | `Ctrl+H` |
| Close focused window | `Ctrl+W` |
| Toggle fullscreen | `Ctrl+F` |

## 3. Available Bridges

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

## 4. Command Palette

Press `Ctrl+P` to open the command palette. Type to filter commands:

- `chat` — Switch to chat window
- `cns` — Open CNS health window
- `wallet` — Open wallet window
- `memory search` — Search episodic memory
- `skill invoke` — Invoke a skill by name

## 5. Inference State

The TUI displays model information in the status bar:
- Current model name
- Token usage (current session)
- Circuit breaker state (Closed, HalfOpen, Open)

## 6. Splash Screen

On first launch, the TUI displays a splash screen with the hKask logo and version information. Press any key to dismiss.

## 7. Exit

Press `Ctrl+C` or `Esc` to exit the TUI. The session state is preserved — re-opening the TUI restores your window layout.

## Notes

- The TUI is built with `ratatui` and `crossterm`.
- Window layouts are persisted to `~/.hkask/tui-layout.yaml`.
- The TUI shares the same inference and memory backends as the REPL — switching between TUI and REPL preserves conversation context.
