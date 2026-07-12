# TUI Workspace State Lifecycle

**Type:** state diagram | **Target:** `Workspace` struct + `SplitNode` tree management | **Diataxis quadrant:** Reference

Tracks the workspace from initialization through tab, split, and focus management. The `SplitNode` binary tree is the core data structure — windows live in leaf nodes, and splits create internal nodes. Layout persistence serializes the tree structure (without window state) to JSON.

```mermaid
stateDiagram-v2
    [*] --> Init: TuiSession::new
    Init --> Splash: show_splash()
    Splash --> Running: dismiss (key/timeout)
    Splash --> Init: build_logo_buffer (once)

    Running --> PaletteOpen: Ctrl+P
    PaletteOpen --> Running: Esc / Ctrl+P / Enter (select)
    PaletteOpen --> WindowOpened: Enter on window kind

    Running --> HelpVisible: '?'
    HelpVisible --> Running: '?' (toggle)

    state Running {
        [*] --> SinglePane: default layout
        SinglePane --> SplitView: Ctrl+Shift+H / Ctrl+Shift+J
        SplitView --> SinglePane: close all but one

        state SplitView {
            [*] --> FocusedLeaf
            FocusedLeaf --> FocusedLeaf: Tab / Ctrl+HJKL (navigate)
            FocusedLeaf --> Resized: Ctrl+= / Ctrl+- (adjust ratio)
            Resized --> FocusedLeaf
        }

        state "Tab Management" as Tabs {
            [*] --> ActiveTab
            ActiveTab --> SwitchedTab: Ctrl+1-9
            ActiveTab --> NewTab: Ctrl+T
            NewTab --> ActiveTab: auto-focus
            ActiveTab --> TabClosed: Ctrl+W
            TabClosed --> ActiveTab: focus prev/next
            TabClosed --> [*]: empty → quit
        }
    }

    WindowOpened --> Running: new window added to split tree

    Running --> LayoutSaved: Ctrl+Q (quit)
    LayoutSaved --> [*]: ratatui::restore()

    note left of Running
        Event loop: poll (16ms) → render → tick
        Focus routing: palette → global → window
        Tick: root.tick() + status bar updates
    end note

    note right of Splash
        SplashScreen renders half-block
        Unicode pixels from build_logo_buffer.
        Dismisses after configurable duration
        or on any key press.
    end note
```

## Split Tree Structure

```
Workspace
  tabs: Vec<Tab>
  active_tab: usize
  focused_window: Option<WindowId>
  │
  └─ Tab
       name: String
       root: SplitNode
            │
            ├─ Leaf(Option<Box<dyn Window>>)  ← window lives here
            ├─ Horizontal { left, right, ratio }
            └─ Vertical   { top, bottom, ratio }
```

## State Transitions by Operation

| Operation | From | To | Side Effects |
|-----------|------|----|-------------|
| `split_focused` | Leaf | Horizontal/Vertical split | Existing window preserved, new Chat window adjacent |
| `new_tab` | Any | New tab with single Chat leaf | Active tab switches to new tab |
| `close_tab` | Multi-tab | N-1 tabs | Focus moves to first window in remaining tab |
| `focus_next` | Focused leaf | Next leaf in tree* | `on_blur` on old, `on_focus` on new |
| `resize_focused` | Any split | Split with adjusted ratio | Ratio clamped to [0.1, 0.9] |
| `restore_layout` | Any | Reconstructed from JSON | All windows recreated, old tabs/windows dropped |

\* Focus order follows depth-first leaf enumeration (`collect_ids`).

## Layout Persistence

Serialized to `~/.config/hkask/agents/{agent_name}/tui_layout.json`:

```json
{
  "version": 1,
  "tabs": [
    {
      "name": "Chat",
      "root": {
        "Horizontal": {
          "left": { "Vertical": { "top": { "Leaf": { "kind": "hKask" } }, ... } },
          "right": { "Leaf": { "kind": "Curator" } },
          "ratio": 0.65
        }
      }
    }
  ],
  "active_tab": 0
}
```

Window state (chat messages, input buffers) is NOT persisted — only the structural layout. The `SavedSplit` enum mirrors `SplitNode` but uses `WindowKind` strings instead of live window objects.

---

*Generated from `crates/hkask-tui/src/workspace.rs`, `layout.rs`, `window.rs` — v0.31.0*
