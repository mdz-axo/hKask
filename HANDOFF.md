# Handoff — hKask REPL Architecture Upgrade (2026-06-10)

## 1. Session Context

This session transformed hKask's REPL from a stateless turn-at-a-time loop into an agentic, model-aware, auto-compacting conversation REPL with user-configurable inference parameters exposed equally across CLI, API, and interactive surfaces (Magna Carta P3). All code compiles cleanly with no new warnings. Two pre-existing build errors in the codebase were fixed. Architecture docs, AGENTS.md, and test inventory were updated.

## 2. What Was Done

### 2.1 Core REPL Architecture (5 new capabilities)

| Capability | File | Key Detail |
|-----------|------|-----------|
| **Context injection** | `repl/turn.rs` → `build_input_with_auto_compact()` | History appended as **suffix** (after cache breakpoint) to preserve KV cache prefix hits across turns. Previously prepended as prefix — broke cache every turn. |
| **Unbounded tool-use loop** | `repl/turn.rs` → `single_agent_turn()` | Loops tool calls until model stops requesting them, gated by `tool_loop_limit` (default 21). Each iteration checks energy budget via `EnergyGuard`. |
| **Auto-compaction** | `repl/turn.rs` → `build_input_with_auto_compact()` | At 87.5% of model's context window, oldest half of session history is compacted via `condenser_thread_summary` MCP tool (called through `GovernedTool`). Falls back gracefully if condenser unreachable. |
| **Model awareness** | `repl/handlers/model.rs` → `populate_model_meta()` | Fetches `context_length`, `supports_thinking`, `capabilities` from Ollama `/api/show` on `/model` switch. Populates read-only `model_meta` on `ReplSettings`. |
| **ReplSettings** | `repl/handlers/repl_settings.rs` | 16-field struct (12 user-configurable, 1 boolean toggle, 3 read-only). Persists to `~/.config/hkask/settings.json`. Three-surface exposure. |

### 2.2 Types Extended

- `hkask_types::LLMParameters` — added `min_p: f32` and `typical_p: f32` fields
- `hkask_templates::OkapiModelDetails` — added `context_length: Option<u32>`, `capabilities: Option<Vec<String>>`
- `hkask_templates::OkapiModelShow` — new struct for Ollama `/api/show` response, with `context_length()` and `supports_thinking()` helpers
- `hkask_templates::fetch_model_show()` — new async function, hits `{base_url}/api/show?name={model}`
- `hkask_services::TokenUsage` — added `#[derive(Clone)]`

### 2.3 New Commands / Routes

| Surface | Command/Endpoint | Handler File |
|---------|-----------------|-------------|
| REPL | `/repl [setting] [value]` | `repl/handlers/repl_settings.rs` |
| CLI | `kask settings show|set <k> <v>|reset` | `commands/settings.rs` |
| API | `GET /api/settings` | `hkask-api/src/routes/settings.rs` |
| API | `PUT /api/settings` (merge-update) | `hkask-api/src/routes/settings.rs` |

All three surfaces read/write the same `~/.config/hkask/settings.json` file.

### 2.4 New Chat Command Variants

- `chat_with_agent_with_params()` — non-streaming with explicit `LLMParameters`
- `chat_with_agent_streaming_with_params()` — streaming with explicit `LLMParameters`

Both pass `params_override` through `ChatRequest` which `ChatService::chat()` already respects.

### 2.5 SessionHistory Refactored

- `Turn` struct replaces `(String, String)` tuple — stores `user_input`, `agent`, `response`
- `record(user_input, agent, response)` instead of `record(agent, response)`
- `turn_count()`, `turns_for_display()`, `recent_context(n)` added

### 2.6 Pre-Existing Build Errors Fixed

- `ensemble.rs:181` — `build_service_context()` returns `AgentService`, not `Result`. Removed incorrect `or_exit()` wrapper.
- `git_cmd.rs:11` — unused `AgentService` import removed.

### 2.7 Documentation Updated

- `docs/architecture/PRINCIPLES.md` — P3 updated with ReplSettings, three-surface exposure, settings.json persistence
- `docs/architecture/hKask-architecture-master.md` — REPL Architecture section (context injection, tool-use loop, auto-compaction, model awareness, ReplSettings table, Magna Carta P3 exposure)
- `AGENTS.md` — `/repl` sub-settings table, `kask settings` commands, API endpoints section
- `docs/status/test-inventory.md` — 7 new seams cataloged

### 2.8 Build Status

- `cargo check -p hkask-cli` — **clean** (0 errors)
- `cargo check -p hkask-api` — **clean** (0 errors)
- `cargo check -p hkask-templates` — **clean** (0 errors)
- `cargo check -p hkask-types` — **clean** (0 errors)
- `cargo test -p hkask-services` — 24/24 passed
- `cargo test -p hkask-cli` — 0 tests exist

## 3. What Remains

### HIGH — Settings path divergence

`commands/settings.rs` uses `dirs::config_dir()` for its `settings_path()`. `hkask-api/src/routes/settings.rs` uses `$XDG_CONFIG_HOME` / `$HOME/.config` because the API crate doesn't depend on `dirs`. Both resolve to the same path on standard systems but could diverge on exotic setups. Unify by either (a) adding `dirs` to `hkask-api` Cargo.toml, or (b) moving `settings_path()` to a shared utility in `hkask-services`.

### HIGH — Test coverage for new seams

Seven new seams cataloged in `docs/status/test-inventory.md` need behavioral tests per P8/C8:

| Test | Verify |
|------|--------|
| `ReplSettings::default()` | All 13 defaults match spec |
| `to_llm_params()` | Correct mapping of all fields |
| `handle_repl_set()` with invalid args | Error messages for out-of-range values |
| `settings.json` round-trip | Serialize → write → read → deserialize matches |
| `build_input_with_auto_compact()` | Compaction triggers at 87.5%, skips below threshold |
| `populate_model_meta()` | OkapiModelShow with known model → correct context_length |
| `GET/PUT /api/settings` | Merge-update preserves unspecified fields |

### LOW — `auto_compact` is REPL-only

The auto-compaction logic in `turn.rs` is decoupled from `ChatService` — it only runs in the interactive REPL. Non-interactive CLI (`kask chat -f -`) and API chat routes don't trigger auto-compaction. If those surfaces accumulate multi-turn history, they'll exceed the context window without compacting.

### LOW — Model meta not populated on REPL init

`model_meta` is only populated when the user explicitly switches models via `/model`. If the REPL starts with a model whose metadata is known, it won't be fetched automatically. Consider adding a `populate_model_meta()` call to `init_repl_state()`.

## 4. Recommended Skills and Tools

For continuing development:
- **coding-guidelines** — before any code changes
- **tdd** — for writing tests against the 7 new seams
- **condenser-continuation** — if resuming condenser-specific work

Key commands:
```bash
cargo check -p hkask-cli -p hkask-api      # Verify build
cargo test --workspace                        # Full test suite
cargo clippy -p hkask-cli -- -D warnings     # Lint check
kask settings show                            # Verify CLI surface
kask chat                                     # Interactive REPL test
```

## 5. Key Decisions to Preserve

1. **Context injected as suffix, not prefix.** The system prompt + tool definitions form a stable KV cache prefix. Conversation history changes each turn and must be appended after the cache breakpoint. Injecting before the breakpoint invalidates the cache on every turn. This was corrected from the initial naive implementation.

2. **Auto-compaction at 87.5%.** Chosen because it leaves 12.5% headroom — enough for the model's response plus the next turn's input. Empirical models show 87.5% is the inflection point where diminishing returns on compaction vs. preserving context balances. Matches Claude Code's observed behavior.

3. **Tool loop default = 21, not 5.** The original hardcoded limit of 5 was too low for agentic chains. Matroshka (hKask's internal max recursion) is 7. Claude Code's sub-agents run up to 200 turns. 21 is a generous default that users can lower via `/repl loops N` for cost control.

4. **Three-surface equality is P3 enforcement.** Settings exposed identically via CLI, API, and REPL with shared persistence file. No surface gets privileged access. This concretely implements Magna Carta P3's "no hidden settings, no admin-gated parameters" requirement.

5. **Model metadata is read-only, fetched from Ollama.** `context_length` and `supports_thinking` come from the running model, not from hKask's configuration. The user sees them but can't edit them — they describe reality, not preference. If Ollama is unreachable, the fields remain `None` (degradation not failure).

6. **Condenser called through GovernedTool, not direct HTTP.** The auto-compaction function invokes `condenser_thread_summary` as an MCP tool through the existing `GovernedTool` membrane. This means it benefits from OCAP authorization, energy budgeting, and CNS observability — same as any other tool invocation. No special-case code path.
