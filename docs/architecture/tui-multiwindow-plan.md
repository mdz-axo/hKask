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

### Phase 1 — Foundation Refactoring (no behavior change) ✅

- [x] Refactor `Leaf(Option<Box<dyn Window>>)` → `Leaf(Box<dyn Window>)`; deleted all `unreachable!` guards
- [x] Switch `pending_action: Option<WorkspaceAction>` → `pending_actions: Vec<WorkspaceAction>` on ChatWindow; `drain_action` → `drain_actions` returning `Vec`
- [x] Extend `WorkspaceAction` enum: `OpenWindow(WindowKind)`, `CloseFocused`, `Split(SplitDirection)`, `FocusNext`, `FocusPrev`, `NewTab(Option<String>)`, `NextTab`, `PrevTab`
- [x] Add `WindowKind` variants: `Kanban`, `Companies`, `Scenarios` (+ META entries)
- [x] Add `WindowKind::from_str()` for slash command parsing (`"kanban"` → `WindowKind::Kanban`)
- [x] Update `window_kind_from_title` and `create_window` factory with new arms
- [x] Fix `meta_covers_all_enum_variants` test (no longer asserts `len == 1`)

### Phase 2 — Workspace Operations ✅

- [x] Implement `SplitNode::take_leaf(target)` — by-value, takes old window out, returns `(Option<Box<dyn Window>>, SplitNode)`
- [x] Implement `SplitNode::remove_window(target)` — by-value, collapses splits to surviving sibling, returns `Option<SplitNode>`
- [x] Implement `Workspace::split_focused(dir)`, `close_focused()`, `new_tab()`, `close_tab()`, `next_tab()`, `prev_tab()`
- [x] Implement `Workspace::apply_action(action)` dispatch
- [x] Wire `apply_action` into `tick()` action loop
- [x] `Ctrl-W` prefix-mode keymap state machine in `handle_global_key`
- [x] `Ctrl-T` new tab, `Ctrl-Tab`/`Ctrl-Shift-Tab` tab cycling
- [x] Fix status bar hints (^Q quit ^W window ^T tab)

### Phase 3 — Window Implementations ✅ (scoped inference path)

- [x] Create `McpScopedState` shared base for MCP-scoped windows
- [x] `KanbanWindow` — scoped to `kanban` MCP server
- [x] `CompaniesWindow` — scoped to `companies` MCP server
- [x] `ScenariosWindow` — scoped to `scenarios` MCP server
- [x] Each: implements `Window` trait, uses `start_scoped_inference` for queries, supports `/open`, `/close`, `/split`, `/focus` slash commands

### Phase 4 — Slash Commands ✅

- [x] Add `/open <kind>`, `/close`, `/split h|v`, `/focus`, `/tab new|next|prev` to `chat.rs::execute_slash_command`
- [x] Add same commands to MCP window `mcp_scoped.rs::handle_slash`
- [x] Update `/help` text to list new commands

### Phase 5 — Generic MCP Bridge ✅

- [x] Add `ToolInvokeBridge` trait (separate from `ReplBridge` to stay ≤7 surface)
- [x] Add `McpInvokeRequestId` and `McpInvokeState` types
- [x] Implement `start_mcp_tool_invoke` / `poll_mcp_tool_invoke` in `TuiReplBridge` using `ToolPort::invoke` + `DelegationToken` + thread spawn
- [x] Add `with_tool_invoke_bridge` setter via `with_bridges!` macro
- [x] Wire `tool_invoke_bridge` through `WorkspaceBridges` → `WindowBridges` → MCP windows
- [x] Add stub to `MockReplBridge` in `test_util.rs`
- [x] MCP windows now support direct tool invocation (`tool_name arg=value`) AND scoped inference (natural language)

### Phase 7 — Adversarial Review Fixes ✅

- [x] Step 1: `:` sigil prefix for direct tool calls; natural language falls through to scoped inference (P0 functional bug)
- [x] Step 2: `focus_window` validates target exists in active tab via `contains_window` (CRITICAL)
- [x] Step 3: `close_focused` validates focused window is in active tab (CRITICAL)
- [x] Step 4: Keymap timeout (60-tick countdown auto-resets `AwaitWindow` state) (HIGH)
- [x] Step 5: State lock extracted and dropped before async `rt.block_on(invoke(...))` call (HIGH)
- [x] Step 6: `format_json_result_depth` caps recursion at 5, truncates output to 5000 chars (HIGH)
- [x] Step 8: Collapsed `kanban.rs`/`companies.rs`/`scenarios.rs` into single `McpScopedWindow` (~130 lines deleted)
- [x] Step 9: `McpInvokeState::Error(String)` → `Error(McpInvokeError)` with structured variants
- [x] Step 10: `WindowKind::META` const table → direct `match` expressions (compiler-enforced exhaustiveness)
- [x] Step 11: Deleted `WindowKind::all()` dead code
- [x] Step 12: Consolidated `system_bridge` + `repl_bridge` into `WorkspaceBridges` (9→7 fields)
- [x] Removed dead `McpInvokeError::InvalidArgs` variant
- [x] Fixed stale doc comments in `mcp_scoped.rs`, `repl_bridge.rs`
- [~] Step 7: Skipped — `PlaceholderWindow` is necessary for Rust's ownership model (can't move out of `&mut T` without replacement value)

## Remaining Follow-Up Items

### Documentation
- ~~`docs/explanation/tui-architecture.md` is comprehensively stale~~ ✅ Rewritten (2026-07-23)

### Test Coverage
- ~~`mcp_scoped.rs` has no test module~~ ✅ Added 18 tests (`:` sigil parsing, JSON/key=value/bool/float args, `format_json_result` recursion cap + truncation)
- ~~Keymap timeout untested~~ ✅ Added `keymap_timeout_resets_await_window` test
- ~~Cross-tab focus guard untested~~ ✅ Added `focus_window_rejects_cross_tab_target` and `close_focused_rejects_cross_tab_target` tests
- ~~`McpInvokeError` mapping from `ToolPortError` in `lib.rs` remains untested~~ ✅ Extracted `map_tool_port_error` function + 4 tests covering all `ToolPortError` variants

### Minor
- ~~Status bar hint omits `Ctrl+Tab`/`Ctrl+Shift+Tab`~~ — accepted as expected (one-line display can't show all bindings)

- [x] Verify `SavedLayout`/`SavedLeaf` handles new window kinds (string-based, already generic)
- [x] Test layout save/restore with multi-window splits (`extract_layout_contains_new_kinds`)
- [x] Test layout validation with Kanban, Companies, Scenarios kinds
- [x] Integration tests: open/close/split/focus/tab operations (12 workspace tests)
- [x] Keybinding tests: Ctrl-W prefix mode, Ctrl-T new tab
- [x] `cargo test -p hkask-repl --features tui` — **120 passed, 0 failed**

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

| Metric | Before | After (All Phases) | Target |
|---|---|---|---|
| WindowKind variants | 1 | 4 | ≥4 ✅ |
| WorkspaceAction variants | 1 | 8 | ≥6 ✅ |
| Functional slash window commands | 0 | 5 (/open /close /split /focus /tab) | ≥5 ✅ |
| MCP windows available | 0 | 3 (Kanban, Companies, Scenarios) | ≥3 ✅ |
| Direct MCP tool invocation | 0 | 1 (ToolInvokeBridge trait + impl) | ≥1 ✅ |
| Status bar hint accuracy | 40% | 100% | 100% ✅ |
| Tests passing | 105 | 141 | 105+ ✅ |
| New tests added | 0 | 36 | ≥10 ✅ |
| Pre-existing bugs fixed | — | 5 (missing Arc import, missing tools param, duplicate PendingCalibration, broken chat_protocol, broken service.rs) | — |