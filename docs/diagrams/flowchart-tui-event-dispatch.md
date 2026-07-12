# TUI Event Dispatch Pipeline

**Type:** flowchart | **Target:** `TuiSession::run` + key event routing | **Diataxis quadrant:** Reference

Describes how keyboard input flows from crossterm event polling through the TUI session, global keybindings, command palette, and focused window dispatch. The workspace tick loop handles background state updates (CNS polling, gas tracking).

```mermaid
flowchart TD
    A([TuiSession::run]) --> B[Show splash screen]
    B --> C[Restore saved layout]
    C --> D{Event poll}
    D -->|Key press| E{Palette open?}
    D -->|Resize| F[ratatui auto-resize]
    D -->|Other| D
    E -->|Yes| G[palette.handle_key]
    E -->|No| H{Global binding?}
    H -->|Yes| I[execute global action]
    H -->|No| J[route to focused window]
    G --> K[Render frame]
    I --> K
    J --> K
    F --> K
    K --> L[Tick workspace]
    L --> M{should_quit?}
    M -->|No| D
    M -->|Yes| N[Save layout]
    N --> O([Exit])

    subgraph "Global Key Actions"
        I1["Ctrl+Q → quit"] 
        I2["Ctrl+N → new chat"]
        I3["Ctrl+T → new tab"]
        I4["Ctrl+W → close tab"]
        I5["Ctrl+P → command palette"]
        I6["Ctrl+H/J/K/L → focus nav"]
        I7["Ctrl+Shift+H → split H"]
        I8["Ctrl+Shift+J → split V"]
        I9["Ctrl+=/- → resize"]
        I10["Ctrl+1-9 → switch tab"]
        I11["Tab → focus next*"]
        I12["? → help overlay"]
    end

    subgraph "Tab Exception: MCP Windows"
        T1{Window is MCP-tabbed?}
        T2["Tab → toggle Chat/Data"]
        T3["Tab → focus next window"]
    end

    I11 --> T1
    T1 -->|Kanban, Memory, Matrix, Media, Training, Companies, Research, Docproc, Replica, Skills, Terminal| T2
    T1 -->|Other windows| T3

    subgraph "Tick Loop"
        L1["root.tick() → all windows"]
        L2["update gas_remaining"]
        L3["update cns_status"]
        L4["update context_pressure"]
        L5["update model name"]
    end

    L --> L1
    L1 --> L2 --> L3 --> L4 --> L5
```

## Key Decision Points

| Decision | Condition | Action |
|----------|-----------|--------|
| Palette interception | `workspace.palette_open == true` | Route ALL keys to `command_palette.handle_key()` |
| Global vs. window | Key matches global binding table | Execute global action immediately |
| Tab routing | Focused window is MCP-tabbed | Let window handle Tab (Chat/Data toggle) |
| Tab routing | Focused window is not MCP-tabbed | Workspace handles Tab (focus-next) |
| Quit guard | `should_quit == true` | Break event loop, save layout, restore terminal |

## Temporal Properties

| Property | Value | Notes |
|----------|-------|-------|
| Event poll interval | 16ms (~60 FPS) | `tick_rate` from `TuiSession` |
| Tick frequency | Every frame | Background updates: CNS, gas, pressure |
| Splash duration | Configurable | Dismissed by key press or timeout |
| Layout save | On quit | JSON serialized per-agent in `~/.config/hkask/agents/{name}/` |

## Cybernetic Notes

The event dispatch loop is a **negative feedback loop**: input → render → tick (model update) → render. The key routing exception for MCP-tabbed windows (hardcoded list, Finding #5 in architecture review) creates a **variety deficit** — new MCP-tabbed window kinds won't receive Tab key routing unless the list is updated.

---

*Generated from `crates/hkask-tui/src/lib.rs:145-218`, `workspace.rs:543-646`, `mcp_tabbed.rs` — v0.31.0*
