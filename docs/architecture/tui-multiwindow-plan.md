# Plan: Dynamic TUI Subwindow System

**Status:** Active · **Created:** 2026-07-22 · **Method:** Improvement Kata (4-step PDCA)

## Context

The hKask TUI was architected as a Zed-style multi-window workspace (split trees, tabs,
command palette, 16 window kinds) but was stripped to a single Chat window across three
commits on 2026-07-21/22. The scaffolding (SplitNode tree, Window trait, WorkspaceAction
enum, layout persistence, factory) was retained but reduced to single-variant enums.

**No backward compatibility requirement** — we can freely refactor, clean up abandoned
paths, and evolve interfaces.

## Architecture Decision (O4 — Resolved)

**Decision:** Add a generic `invoke_mcp_tool` method to `ReplBridge` (async start/poll
pattern). Do NOT restore the 15 per-domain bridge traits.

**Rationale:** `McpRuntime` already implements `ToolPort::invoke(server, tool, args, token)`
— a generic, governed dispatch path. `hkask-repl` already depends on `hkask-mcp` and
`hkask-capability`. Calling through `ToolPort::invoke` preserves OCAP/gas/Regulation
governance. The old 15-bridge design was over-engineered (deleted as "unused" in commit
d28f2521). A single generic method replaces 15 traits + a 1218-line adapter.

**Blocking concern:** `block_on` freezes the TUI. Use a start/poll pattern mirroring
`start_inference`/`poll_inference`: spawn a thread, poll for completion on tick.

## Phased Implementation

### Phase 1 — Foundation Refactoring (no behavior change)

- [ ] Refactor `Leaf(Option<Box<dyn Window>>)` → `Leaf(Box<dyn Window>)`; delete 6 `unreachable!` guards
- [ ] Switch `pending_action: Option<WorkspaceAction>` → `pending_actions: VecDeque<WorkspaceAction>` on ChatWindow; update `drain_action` to drain all
- [ ] Extend `WorkspaceAction` enum: `OpenWindow(WindowKind)`, `CloseFocused`, `Split(SplitDirection)`, `FocusNext`, `FocusPrev`, `NewTab`
- [ ] Add `WindowKind` variants: `Kanban`, `Companies`, `Scenarios` (+ META entries)
- [ ] Add `WindowKind::from_str()` for slash command parsing (`"kanban"` → `WindowKind::Kanban`)
- [ ] Update `window_kind_from_title` and `create_window` factory with new arms
- [ ] Fix `meta_covers_all_enum_variants` test (no longer asserts `len == 1`)

### Phase 2 — Workspace Operations

- [ ] Implement `SplitNode::replace_leaf(target, new_node)` — takes ownership of old leaf, inserts split
- [ ] Implement `SplitNode::close_window(target)` — collapse-on-close (replace split with surviving sibling)
- [ ] Implement `Workspace::split_focused(dir)`, `close_focused()`, `new_tab()`, `close_tab()`
- [ ] Implement `Workspace::apply_action(action)` dispatch
- [ ] Wire `apply_action` into `tick()` action loop
- [ ] Move `focused_window` to per-tab (on `Tab` struct)

### Phase 3 — Generic MCP Bridge

- [ ] Add `start_mcp_tool_invoke(server, tool, args) -> McpInvokeRequestId` to `ReplBridge`
- [ ] Add `poll_mcp_tool_invoke(id) -> McpInvokeState` to `ReplBridge`
- [ ] Implement in `TuiReplBridge` using `ToolPort::invoke` + `DelegationToken` + thread spawn
- [ ] Add stub to `MockReplBridge`

### Phase 4 — Slash Commands + Keybindings

- [ ] Add `/open <kind>`, `/close`, `/split h|v`, `/focus`, `/tab` to `chat.rs::execute_slash_command`
- [ ] Add `Ctrl-W` prefix-mode keymap state machine to `handle_global_key` (h/j/k/l/v/s/c/w)
- [ ] Add `Ctrl-T` for new tab, `Ctrl+Tab`/`Ctrl+Shift+Tab` for tab cycling
- [ ] Fix status bar hints to match actual keybindings

### Phase 5 — Window Implementations

- [ ] `KanbanWindow` — board list, task columns, move/accept actions via MCP bridge
- [ ] `CompaniesWindow` — company search, profile/quotes/financials via MCP bridge
- [ ] `ScenariosWindow` — scenario list, forecast tracker via MCP bridge
- [ ] Each: implement `Window` trait, use `start_mcp_tool_invoke`/`poll_mcp_tool_invoke` for data

### Phase 6 — Layout Persistence + Validation

- [ ] Verify `SavedLayout`/`SavedLeaf` handles new window kinds (string-based, already generic)
- [ ] Test layout save/restore with multi-window splits
- [ ] Integration test: `/open kanban /split v /open companies /close /focus`
- [ ] `cargo test -p hkask-repl` + `cargo clippy -D warnings`

## Keybindings

| Key | Action |
|---|---|
| `Ctrl+Q` | Quit (existing) |
| `Ctrl+W h` | Focus left |
| `Ctrl+W j` | Focus down |
| `Ctrl+W k` | Focus up |
| `Ctrl+W l` | Focus right |
| `Ctrl+W v` | Split vertical (side-by-side) |
| `Ctrl+W s` | Split horizontal (stacked) |
| `Ctrl+W c` | Close focused pane |
| `Ctrl+W w` | Cycle focus next |
| `Ctrl+T` | New tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |

## Slash Commands (TUI-only)

| Command | Action |
|---|---|
| `/open <kind>` | Open window of kind as split from focused |
| `/close` | Close focused window |
| `/split h\|v` | Split focused window |
| `/focus` | Cycle focus to next window |
| `/tab new [name]` | Create new tab |
| `/tab next\|prev` | Switch tabs |

## Metrics

| Metric | Before | Target |
|---|---|---|
| WindowKind variants | 1 | ≥4 |
| WorkspaceAction variants | 1 | ≥6 |
| Functional slash window commands | 0 | ≥4 |
| MCP windows available | 0 | ≥3 |
| Status bar hint accuracy | 40% | 100% |