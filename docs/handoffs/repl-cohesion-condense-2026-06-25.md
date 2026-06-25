# Handoff — REPL State Cohesion + Condensation Path Unification

**Date:** 2026-06-25
**Session:** REPL hardening — memory architecture, thread registry, tool pipeline, auto-start
**Status:** ~95% complete. Two deferred architectural items remain.

---

## 1. Session Context

Hardened the hKask REPL from a single-server inference-only shell into a full autonomous agent runtime. Added 9 auto-started MCP servers (autonomous nervous system), three-layer memory architecture (thread + episodic + semantic), chat thread registry with active/archived lifecycle, saliency-based auto-condense with configurable settings, cold-start thread history injection, self-healing error recovery patterns, and CNS observability for all operations. Two deferred items remain — neither blocks the current functionality.

---

## 2. What Was Done

All changes compile cleanly (`cargo check -p hkask-cli -p hkask-services` passes). 9 tests pass (`cargo test -p hkask-cli --lib -- threads::tests`).

### Files changed (this session)

| File | Change |
|------|--------|
| `crates/hkask-cli/src/repl/init.rs` | Auto-start 9 core MCP servers; project root propagation |
| `crates/hkask-cli/src/repl/builtin_servers.rs` | Added filesystem to builtin list |
| `crates/hkask-cli/src/repl/display.rs` | Updated banner for core servers + degraded warnings |
| `crates/hkask-cli/src/repl/handlers/ask.rs` | Routed /ask through single_agent_turn with agent_override |
| `crates/hkask-cli/src/repl/handlers/repl_settings.rs` | Added pressure/saliency/stm_life settings; context_turns aliased to saliency_window |
| `crates/hkask-cli/src/repl/handlers/thread.rs` | /thread list|switch|new|archive commands |
| `crates/hkask-cli/src/repl/threads.rs` | ThreadRegistry — per-agent short-term memory streams, CNS spans, atomic writes |
| `crates/hkask-cli/src/repl/turn.rs` | agent_override parameter; cold-start thread injection; CNS turn spans; append_turn after each exchange |
| `crates/hkask-cli/src/repl/mod.rs` | ReplState gained thread_registry, degraded_servers; removed thread_seeded; KaskHelper takes ThreadRegistry for tab completion |
| `crates/hkask-cli/src/repl/helper.rs` | Thread ID tab completion for /thread switch|archive |
| `crates/hkask-cli/src/repl/tool_augmented.rs` | Self-correction pattern + error recovery instructions in system prompt |
| `crates/hkask-cli/src/repl/commands.rs` | /thread command registration |
| `crates/hkask-services/src/chat.rs` | TurnRequest gained thread_history, condense_pressure_threshold, condense_saliency_window; invariant context assembly (thread → semantic → episodic); saliency-based condensation split |
| `crates/hkask-types/src/cns.rs` | ToolSubsystem::Filesystem variant |
| `crates/hkask-types/src/agent_paths.rs` | threads/ directory in agent layout |
| `mcp-servers/hkask-mcp-filesystem/src/lib.rs` | CNS spans + path sandboxing added to existing 7 tools |
| `mcp-servers/hkask-mcp-filesystem/src/main.rs` | project_root + capability_tier wiring |
| `mcp-servers/hkask-mcp-skill/src/lib.rs` | Tool-awareness context in skill_execute system prompt |
| `registry/templates/heal/self-heal.j2` | Self-healing skill template (new) |
| `docs/specifications/specs/REPL-specification.md` | Updated memory architecture (§9), settings table (§8), slash commands (§5), auto-start policy (§12) |

---

## 3. What Remains

### HIGH — Item 1: ReplState Cohesion (`crates/hkask-cli/src/repl/mod.rs`)

**Problem:** `ReplState` has 23 fields. Some are tightly coupled pairs that could be sub-structs, improving locality and reducing the God Object surface.

**Specific work:**

1. Extract `TalkConfig { enabled: bool, voice_design: Option<String> }` from `talk_enabled` + `voice_design`. These are always accessed together in the talk handler and turn pipeline.

2. Extract `ManifestState { executor: Option<ManifestExecutor>, manifest: Option<BundleManifest> }` from `manifest_executor` + `process_manifest`. These are paired — one is None when the other is None, but this isn't enforced by the type system. A sub-struct with a constructor that takes both or neither would make the invariant explicit.

3. Extract `ToolPrompt { section: String, definitions: Vec<ChatToolDefinition> }` from `tool_prompt_section` + `tool_definitions`. These are always refreshed together in `refresh_tool_section()` and `init_repl_state()`.

**Strategy:** Group into sub-structs, update all field accesses (search for `state.talk_enabled`, `state.voice_design`, etc.). This is a mechanical refactor — no logic changes. Verify with `cargo check -p hkask-cli` after each extraction.

**Skills to load:** `coding-guidelines` (surgical changes), `deep-module` (deletion test on sub-structs), `essentialist` (verify each sub-struct earns its existence).

### MEDIUM — Item 4: Condensation Path Unification

**Problem:** There are two separate condensation paths with no shared configuration:
- **Auto-condense:** `ChatService::condense_history` calls `hkask_condenser::inference` library functions directly. Controlled by `/repl pressure` and `/repl saliency`.
- **Agent-initiated:** MCP `condenser_compress` and `condenser_thread_summary` tools. These have their own independent configuration (the condenser server's internal profile settings).

The two paths can produce different results for the same content because they use different algorithms and configurations.

**Specific work:**

1. Audit what configuration the MCP condenser server uses vs. what the auto-condense path uses. Key files:
   - `crates/hkask-services/src/chat.rs` — `condense_history` (auto-condense)
   - `mcp-servers/hkask-mcp-condenser/src/lib.rs` — MCP condenser tools
   - `crates/hkask-condenser/src/` — the condenser engine library

2. Determine whether the MCP condenser tools SHOULD respect the REPL's condensation settings, or whether they should remain independently configurable (they serve different use cases — auto-condense is transparent, agent-initiated is explicit).

3. If unifying: the MCP condenser server needs access to ReplSettings. Options:
   - Pass settings via environment variables (simplest — set `HKASK_CONDENSE_*` vars in REPL init)
   - Pass settings via the `skill_execute` context when the agent invokes condenser tools (more surgical)
   - Add a shared configuration service that both paths read from (architectural — defer)

**Recommendation:** Start with option A (env vars). Set `HKASK_CONDENSE_PRESSURE_THRESHOLD` and `HKASK_CONDENSE_SALIENCY_WINDOW` in `init.rs` when `repl_settings` is loaded. Update the condenser server to read these env vars as overrides for its defaults. This is ~10 lines of code.

**Skills to load:** `pragmatic-cybernetics` (two control paths for one function = variety issue), `coding-guidelines` (simplicity first), `zoom-out` (understand the full condensation architecture before touching it).

---

## 4. Recommended Skills and Tools

For the next session:
```
skill coding-guidelines
skill deep-module
skill essentialist
skill pragmatic-cybernetics
skill zoom-out
```

Build command: `cargo check -p hkask-cli -p hkask-services -p hkask-mcp-condenser`
Test command: `cargo test -p hkask-cli --lib -- threads::tests`

---

## 5. Key Decisions to Preserve

1. **Thread history is cold-start-only.** Thread history injection happens only on session start and thread switch. After the first turn, episodic recall handles conversation context. Rationale: avoids redundant triple-injection (thread + episodic + what the engine just processed). See `turn.rs` `thread_history: if state.thread_registry.seeded { None }`.

2. **Context assembly is structurally invariant.** All three memory layers coexist — the agent can't "choose" to have short-term but not long-term memory. The 8-arm match in `execute_turn` exists because each layer may be empty, not because the structure is optional. See `chat.rs` line 1105.

3. **`context_turns` is dead — aliased to `condense_saliency_window`.** The setting still exists in `ReplSettings` for backward compat with existing settings.json files, but it routes to `saliency_window`. `/repl context 7` sets `saliency_window` to 7. See `repl_settings.rs` line 270.

4. **Thread registry uses atomic writes.** `write_thread_file` writes to `.tmp` then renames. Prevents corruption on crash. See `threads.rs` line 297.

5. **Thread seeding is owned by ThreadRegistry, not ReplState.** After review, `thread_seeded` was moved from `ReplState` into `ThreadRegistry.seeded`. `switch_to()` and `create_thread()` reset it; `mark_seeded()` sets it. Rationale: the registry should be self-contained.

6. **CNS spans use `cns_domain` + `operation` convention, not new CnsSpan variants.** Thread operations emit via `tracing::info!(target: "cns", cns_domain = "cns.thread", operation = "switched", ...)` rather than adding new enum variants. Rationale: avoids bloating the CNS enum for domain-specific operations.

7. **No messages-array API refactor yet.** The `InferencePort` trait only accepts prompt strings. Switching to chat message arrays would require changes to all inference backends. Deferred until the inference boundary is ready for it.
