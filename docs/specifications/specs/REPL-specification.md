---
title: "hKask REPL Specification"
audience: [architects, developers, users]
last_updated: 2026-06-18
version: "0.31.0"
status: "Active"
domain: "Surface"
mds_categories: [domain, composition, lifecycle, curation]
---

# hKask REPL Specification — `kask repl`

## 1. Purpose and Scope

This document is the authoritative specification for the hKask interactive REPL (Read-Eval-Print Loop). The REPL is accessed through a browser terminal (xterm.js + WebSocket) or optionally via SSH. The hKask server spawns `kask repl --webid <user>` on a PTY per authenticated user session, providing a terminal-based conversational interface to agents, models, tools, and pods — all governed by the Magna Carta's four principles of User Sovereignty, Affirmative Consent, Generative Space, and Clear Boundaries (OCAP). The project defers multi-agent ensemble sessions (2026-06-14) as a future mode evolving from the dual-presence pattern.

**Audience:** Architects, developers, users, and agents interacting with hKask.

**Scope:** Covers the REPL loop, slash command registry, single-agent turn pipeline, memory infrastructure, gas governance, inference configuration, tool-augmented execution, and future features toward parity with leading AI REPL providers (primarily Zed). Does NOT cover the HTTP API surface (`hkask-api`) or standalone CLI commands (`kask bundle`, `kask sovereignty`, etc.) except where they are directly invoked from the REPL. Multi-agent ensemble turn pipeline is deferred (2026-06-14); see §7 for forward-looking design notes.

## 2. Design Principles

### 2.1 User Sovereignty First (Magna Carta P1–P4)

Every design decision in the REPL grounds itself in the Magna Carta:

| Principle | REPL Implementation |
|-----------|-------------------|
| **P1: User Sovereignty** | Agent-specific SQLCipher-encrypted memory, WebID-scoped access, `/sovereignty` verification command |
| **P2: Affirmative Consent** | OCAP capability tokens minted per operation; GovernedTool membrane blocks unauthorized access |
| **P3: Generative Space** | `/repl` command exposes every inference parameter (temperature, top-p, top-k, min-p, typical-p, seed, max_tokens, gas limits, auto-condense). No hidden settings. No engineer-only options. |
| **P4: Clear Boundaries (OCAP)** | All tool invocations route through `GovernedTool` verifying unforgeable capability tokens; dual enforcement gate (`require_capability` + `require_sovereignty`) |

### 2.2 Familiarity Through Parity

The REPL adopts behavioral patterns from leading AI REPL systems so the interaction model feels natural:

- **Slash commands** (`/model`, `/agent`, `/help`) follow the convention established by ChatGPT, Claude, and others.
- **Tab completion** for slash commands via `rustyline`.
- **Fuzzy matching** on unknown commands (suggests "did you mean /model?") — pattern from Claude CLI.
- **Streaming output** on first inference iteration, with tokens rendered incrementally — pattern from Zed, Aider, GPT CLI.
- **History persistence** in `$XDG_DATA_HOME/hkask/kask_history.txt` — standard readline behavior.
- **Color-coded output** with ANSI escape codes for agent names, model info, CNS alerts, gas budget, and tool results.

### 2.3 Self-Documenting

Every capability is discoverable from `/help`, which displays a categorized command menu. Each command has a detailed `/help <command>` page. Unknown commands trigger fuzzy-match suggestions.

### 2.4 Headless Constraint

Per PRINCIPLES.md P6, the REPL is strictly terminal-based. The browser terminal (xterm.js rendering a PTY over WebSocket) is a terminal emulator, not a visual web application. No dashboards, no Grafana/Prometheus stacks, no graphical UI. Output is rendered with ANSI codes and structured text. Code execution output (Phase 2+) uses terminal-appropriate MIME ranking, not GUI rendering.

## 3. Architecture

### 3.1 Crate Location

```
crates/hkask-cli/src/repl/
├── mod.rs              # ReplState struct, main loop
├── commands.rs         # Slash command registry (SLASH_COMMANDS table)
├── display.rs          # Banner, help, command help
├── helper.rs           # KaskHelper (Completer, Highlighter, Hinter, Validator)
├── init.rs             # Dependency injection — wires CNS, loops, memory, tools
├── turn.rs             # single_agent_turn() (→ ChatService::execute_turn)
├── energy.rs           # EnergyGuard (owned-consumption hold-settle gas pattern)
├── cns_display.rs      # CNS algedonic alert display, loop system tick (read-only)
├── tool_augmented.rs   # Tool call parsing, invocation, response processing
├── builtin_servers.rs  # MCP server startup at REPL boot
└── handlers/
    ├── mod.rs          # Re-exports
    ├── agent.rs        # /agent, /agents
    ├── ask.rs          # /ask (session-aware agent query)
    ├── consolidation.rs # /consolidate
    ├── ensemble.rs     # /ensemble, /filter, /mode, /into — Deferred (2026-06-14)
    ├── escalation.rs   # /escalations, /resolve, /dismiss
    ├── info.rs         # /history, /pods, /templates, /tools
    ├── invoke.rs       # /invoke (OCAP-gated tool invocation)
    ├── model.rs        # /model (list, switch, fuzzy search)
    ├── repl_settings.rs # /repl (ReplSettings, to_llm_params)
    └── status.rs       # /status (agent, model, gas, CNS, loops)
```

### 3.2 ReplState — Central State Object

```rust
pub(crate) struct ReplState {
    pub(crate) inference_port: Arc<dyn InferencePort>,          // Shared Okapi inference port
    pub(crate) inference_loop: Arc<InferenceLoop>,              // CNS-observable energy budget + model tracking
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,  // Private, agent-scoped memory
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,  // Public, shared memory
    pub(crate) agent_webid: WebID,                              // Provided by server via --webid (from OAuth identity)
    pub(crate) current_model: String,                           // Active Okapi model name
    pub(crate) current_agent: String,                           // Active agent name (from onboarding)
    pub(crate) active_session: Option<String>,                  // Deferred (2026-06-14): Future multi-agent session ID (None = single-agent)
    pub(crate) resolved_secrets: Option<ResolvedSecrets>,       // From onboarding (ACP + DB)
    pub(crate) governed_tool: Arc<GovernedTool<RawMcpToolPort>>, // OCAP + CNS governance membrane
    // ── private fields (only accessed within repl/ submodules) ──
    consolidation_service: Option<ConsolidationService>,        // Episodic→semantic consolidation
    persona_constraints: Option<PersonaConstraints>,            // Per-agent persona filter rules
    manifest_executor: Option<ManifestExecutor>,                // Process manifest cascade runner
    process_manifest: Option<BundleManifest>,                   // Agent's resolved process manifest
    // ── public fields ──
    pub(crate) tool_prompt_section: String,                     // Pre-formatted tool section of system prompt
    pub(crate) service_context: Arc<AgentService>,              // Canonical infrastructure assembly
    pub(crate) repl_settings: ReplSettings,                     // User-configurable inference parameters
    pub(crate) is_first_run: bool,                              // true on first OAuth sign-in (onboarding triggered), false on returning sessions
}
```

**Design Intent:** `init::init_repl_state()` initializes `ReplState` once at REPL boot and mutates it in place across turns. Five fields remain private (only accessed within `repl/` submodules): `consolidation_service`, `persona_constraints`, `manifest_executor`, `process_manifest`. The system no longer stores session history in-memory — all history access routes through OCAP-gated episodic storage via `ChatService::recall_recent_turns()`. `is_first_run` is set when the user's OAuth sign-in triggers provisioning of a new WebID and default replicant; it gates the First Steps guide shown in the welcome banner.

### 3.3 Dependency Injection (init.rs)

The `init_repl_state()` function assembles the REPL's dependency graph in order:

1. Receive WebID from `--webid` flag (set by server per authenticated user session)
2. Run onboarding if `is_first_run` — replicant creation and model selection for first-time OAuth sign-ins. Sets `is_first_run`.
3. Resolve effective model: onboarding selection > persisted `ReplSettings` > CLI `--model` arg > hardcoded default (`deepseek-v4-pro`)
4. Load persisted `ReplSettings` from per-user config (`~/.config/hkask/settings.json`)
5. Resolve Okapi base URL from server environment (`OKAPI_BASE_URL`) or default
6. Initialize shared `InferencePort` for selected model + wrap in `InferenceLoop`
7. Build `AgentService::build()` — creates CNS, loop system, governed tool, pod manager, MCP runtime
8. Register inference loop on loop system
9. Start built-in MCP servers (10 servers)
10. Build `GovernedTool` membrane wrapped around MCP runtime
11. Register agent energy budget with `CyberneticsLoop`
12. Open per-agent SQLCipher-encrypted memory database (keyed by WebID)
13. Build per-agent memory via `AgentService::build_per_agent_memory()` (episodic/semantic ports + ConsolidationService)
14. Populate tool prompt section from MCP runtime discovery
15. Load persona constraints and process manifest for initial agent

**Note:** The WebID arrives from the server via `--webid` (derived from OAuth identity). Onboarding uses the server's configured inference providers (cloud-only). All state (config, memory, history) is scoped to the authenticated WebID.

## 4. Input Loop

### 4.1 Readline Configuration

```
- History: $XDG_DATA_HOME/hkask/kask_history.txt
- Ignore duplicates: true
- Ignore space-prefixed lines: true
- Completion type: List (inline)
- Prompt format: ℏKask [agent_name]>  (default)
```

### 4.2 Slash Command Detection

Inputs starting with `/` are intercepted before inference. The dispatch logic in `commands::handle_slash_command()`:

1. Strip leading `/`
2. Split into `cmd arg1 arg2` (max 3 parts)
3. Match against `SLASH_COMMANDS` table
4. On no match → fuzzy search across primary names, aliases, and descriptions → "did you mean:"

### 4.3 Natural Language → Inference

Inputs not starting with `/` and not matching `"quit"` / `"exit"` are treated as natural language prompts. These route to either:
- `turn::single_agent_turn()` — always (ensemble turn pipeline deferred 2026-06-14)

### 4.4 SIGINT / EOF / Error Handling

| Signal | Behavior |
|--------|----------|
| `Ctrl+C` (SIGINT) | Print hint: "(Ctrl+C — type /quit to exit)" |
| `Ctrl+D` (EOF) | Print "Goodbye!", save history, exit |
| Readline error | Print error, save history, exit |

## 5. Slash Command Registry

All 28 slash commands with aliases, categorized as shown in `/help`:

### 5.1 Session Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/help` | `/h`, `/?` | `[COMMAND]` | Show help, or details for a specific command |
| `/quit` | `/q`, `/exit` | | End the session |
| `/clear` | `/cls` | | Clear the screen |
| `/history` | `/hist` | | Show session history (turn count + response previews) |

### 5.2 Agent Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/agent` | `/a` | `[NAME]` | Switch agent (loads persona constraints), or show current |
| `/agents` | `/ls` | | List registered agents (name, kind, capabilities) |
| `/pods` | | | List agent pods (pod ID, state, WebID) |

### 5.3 Model Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/model` | `/m` | `[NAME\|QUERY]` | Switch model, fuzzy search, or show current. Populates model metadata (context window, thinking support, capabilities) |

**Sub-behaviors:**
- `/model` (no args) → show current model name
- `/model list` → list all available models (name, family, params, size)
- `/model qwen3:8b` → exact match → switch to that model, show metadata
- `/model qwen` → fuzzy match → list all models containing "qwen"
- `/model foobar` → no match → store name anyway (Okapi may be unreachable)

### 5.4 Ensemble Commands — Deferred (2026-06-14)

Ensemble multi-agent commands are deferred. The dual-presence pattern (§7) is the active multi-agent path. Ensemble will evolve from dual-presence learnings when N≥3 stable sessions with distinct ACP agents are achieved.

**Original specification (preserved for future reference):**

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| ~~`/into`~~ | ~~`/i`~~ | ~~`[SESSION]`~~ | ~~Enter ensemble session, or leave it (no args = leave)~~ |
| ~~`/ensemble`~~ | ~~`/ens`~~ | ~~`sessions\|create\|join\|send\|invite\|participants`~~ | ~~Multi-agent ensemble operations~~ |
| ~~`/filter`~~ | ~~`/thresh`~~ | ~~`[0.0-1.0]`~~ | ~~Set/show participation threshold (default: 0.75)~~ |
| ~~`/mode`~~ | | ~~`[freeform\|curator_led\|round_robin]`~~ | ~~Set/show ensemble orchestration mode~~ |
| `/ask` | | `<AGENT> <MESSAGE>` | Force a specific agent to respond (bypasses relevance filter) |

**Deferred (2026-06-14):** ensemble removed. Future multi-agent mode will evolve from dual-presence.

~~**Ensemble subcommands:**~~
~~- `/ensemble sessions` — list active ensemble sessions~~
~~- `/ensemble create <id>` — create a new ensemble chat session~~
~~- `/ensemble join <session> <bot> <role>` — register a bot with role (memory_bot, spandrel_bot, okapi_bot, scholar_bot)~~
~~- `/ensemble invite <bot> [role]` — invite agent into current session~~
~~- `/ensemble participants` — show participants in current session~~
~~- `/ensemble send <session> <message>` — send message to a session~~

~~**Orchestration modes:**~~
~~- `freeform` (default) — agents self-select by relevance confidence~~
~~- `curator_led` — Curator picks which agents speak~~
~~- `round_robin` — all agents speak in turn~~

### 5.5 System Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/status` | `/st` | | System status: agent, model, template, gas, CNS health, loop count, turns |
| `/tools` | | | List MCP tools with descriptions (discovered via GovernedTool) |
| `/templates` | `/tpl` | | List registered templates (ID + type) |
| `/sovereignty` | `/sov` | | Show sovereignty status (Magna Carta compliance) |
| `/invoke` | `/inv` | `<server>/<tool> [args]` | Invoke MCP tool through GovernedTool membrane |
| `/bundle` | `/b` | `[SKILLS]\|list\|off\|skills` | Compose, apply, or manage skill bundles |
| `/repl` | | `[SETTING] [VALUE]` | Show or set REPL inference settings |

### 5.6 Governance Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/escalations` | `/esc` | | List pending escalations (ID, bot, confidence, context) |
| `/resolve` | | `<ID>` | Resolve an escalation |
| `/dismiss` | | `<ID>` | Dismiss an escalation |
| `/metacognition` | `/meta` | | Run a metacognition cycle (Curator self-reflection) |

### 5.7 Alignment Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/consolidate` | `/cons` | `[LIMIT] [--floor CONFIDENCE] [--max MAX_TRIPLES]` | Trigger episodic→semantic consolidation |

### 5.8 Onboarding and Feedback Commands

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/start` | `/tour`, `/onboarding` | | Interactive step-by-step guided tour (9 steps, press Enter to advance, type `skip` to exit) |
| `/feedback` | | | Prompt for a free-text usability note; appended with UTC timestamp + replicant name to `~/.local/share/hkask/feedback.md` |

**`/start` detail:** Each step covers one capability domain: Chat, Commands, Models, Status, Tools, Settings, Memory, Done. Always available — not only on first sign-in. `/tour` and `/onboarding` are aliases. The interactive tour is the same flow that runs automatically on first OAuth sign-in; it can be re-run at any time.

**`/feedback` scope:** REPL-only. Not exposed via CLI subcommand or HTTP API. The file is append-only; each entry is a Markdown `##` heading with ISO-8601 UTC timestamp and a blockquote body. Nothing is transmitted anywhere. Feedback is stored per-user under the authenticated WebID's data directory.

## 6. Single-Agent Turn Pipeline

The turn pipeline is now split between the service layer and the CLI:

- **`ChatService::execute_turn()`** (in `hkask-services`) handles: manifest cascade,
  history suffix, inference via `ChatService::chat()`, and persona filter.
- **CLI (`turn::single_agent_turn()`)** handles: gas guard reservation/settlement,
  response display, tool execution through `GovernedTool`,
  token usage display, energy budget warnings, and CNS updates.

### 6.1 Pipeline Overview

```
┌── Service Layer (ChatService::execute_turn) ─────────────────────────┐
│ 1. Manifest Cascade (optional)                                       │
│    └─ Execute agent's process_manifest → enrich prompt with step_* ctx│
│                                                                      │
│ 2. History Injection (suffix pattern, from episodic storage)          │
│    └─ Append recent N turns via OCAP-gated recall_recent_turns()      │
│    └─ Suffix placement preserves KV cache hits for system prompt      │
│                                                                      │
│ 3. Inference via ChatService::chat()                                  │
│    └─ Agent lookup, system prompt, semantic recall, LLM call          │
│    └─ Returns text + token usage + structured tool calls              │
│                                                                      │
│ 4. Persona Filter (strip forbidden patterns)                          │
│    └─ apply_persona_filter()                                         │
├──────────────────────────────────────────────────────────────────────┤
│ CLI Layer (turn::single_agent_turn)                                    │
│                                                                      │
│ 5. Gas Guard (per-iteration)                                          │
│    └─ EnergyGuard::try_reserve() → inference → settle(actual)         │
│                                                                      │
│ 6. Tool Execution (via GovernedTool + OCAP)                           │
│    └─ Parse structured tool calls from TurnResult                     │
│    └─ Execute through GovernedTool membrane                           │
│    └─ If tool calls found → feed results back to execute_turn()       │
│    └─ If no tool calls → final response, exit loop                    │
│                                                                      │
│ 7. Token Usage Display                                                │
│    └─ "N tokens (P prompt + C completion) across M iterations"       │
│                                                                      │
│ 8. Gas Budget Warning                                                │
│    └─ If < 20%: yellow warning. If 0: red exhausted warning.          │
│                                                                      │
│ 9. CNS Update (read-only)                                            │
│    └─ Algedonic alert check, LoopSystem tick                           │
│                                                                      │
│ 10. Episodic Storage (handled by ChatService::chat() automatically)    │
│    └─ Store (user_input, agent_name, response) as episodic triple     │
└──────────────────────────────────────────────────────────────────────┘
```

**Note:** Auto-condense (87.5% threshold) is implemented in `ChatService::execute_turn()` via direct condenser library call. When context exceeds 87.5% of the model window, the oldest half of history is condensed and replaced with a summary.

### 6.2 Tool Call Parsing (Two Priority Levels)

The REPL supports two tool call formats, checked in priority order:

**Priority 1 — Structured Native Function Calling:**
When `InferenceResult.finish_reason == "tool_calls"`, the structured `tool_calls` list is used directly. No text parsing required.

**Priority 2 — Text Directive Fallback:**
```
<<tool:server/tool_name
{"key": "value"}
>>
```

The parser (`tool_augmented::parse_tool_calls()`) is forgiving:
- If a directive is malformed (invalid JSON, missing tool name), it is treated as plain text
- Multiple tool calls in a single response are supported
- Server is optional (parsed from `server/tool_name` syntax or empty string for discovery)

### 6.3 Tool Call Invocation

All tool calls route through `GovernedTool`, which provides:

1. **OCAP Authorization:** A `DelegationToken` is minted from the session's ACP secret
2. **Energy Budget:** Gas is charged for tool execution
3. **CNS Observability:** Tool invocations emit `cns.tool.*` spans

The full invocation chain: `tool_augmented::invoke_tool_call()` → `GovernedTool::invoke()` → `RawMcpToolPort` → `McpRuntime`.

### 6.4 Tool Results → Followup

When tool calls are found:
```
[text portion of response]

The following tool calls were executed:

✓ tool_name → {result JSON}
✗ tool_name → ERROR: {message}

Based on these results, provide your response.
```

This followup prompt is fed back into the next loop iteration.

### 6.5 Streaming Behavior

- **Iteration 1:** Uses `chat_with_agent_streaming_with_params()` — tokens are printed incrementally to stdout, prefixed with `"{agent_name}: "`
- **Iteration 2+:** Uses `chat_with_agent_with_params()` — non-streaming to avoid redundant output during tool loop followups

## 7. Ensemble (Multi-Agent) Turn Pipeline — Deferred (2026-06-14)

Ensemble multi-agent turn pipeline is deferred. The dual-presence pattern (see `docs/specifications/dual-presence-pattern.md`) is the active multi-agent path. Ensemble will evolve from dual-presence learnings.

**Original specification (preserved for future reference):**

~~When `active_session` is set, user messages route through `turn::ensemble_turn()`:~~

~~1. Calls `ensemble_improv_turn()` — agents self-select by relevance confidence (freeform mode) or follow the configured mode~~
~~2. For each agent that chose to speak, tool-augmented processing is applied~~
~~3. Responses display with confidence score: `AgentName (conf. 0.85): response text`~~
~~4. Agents that were silent display a dim footnote: `AgentName: silent (0.42 — below threshold)`~~
~~5. If a Curator synthesis is produced, it is displayed in gold: `Curator: synthesis`~~
~~6. All responses (including silent judgments) are recorded in session history~~

**Reactivation criterion:** When dual-presence has produced N≥3 stable sessions with distinct ACP agents.

## 8. REPL Settings (`/repl` command)

Per Magna Carta P3 (Generative Space), every inference parameter is user-exposed. The `/repl` command surfaces and mutates all settings.

### 8.1 Settings Table

| Setting | Flag | Type | Default | Valid Range | Description |
|---------|------|------|---------|-------------|-------------|
| `tool_loop_limit` | `loops` | u64 | 21 | 1+ | Max tool-call loop iterations per turn |
| `context_turns` | `context` | u64 | 3 | 0+ (0 = no history) | Past turns appended as context |
| `temperature` | `temp` | f32 | 0.7 | 0.0–2.0 | LLM sampling temperature |
| `top_p` | `top_p` | f32 | 0.9 | 0.0–1.0 | Nucleus sampling threshold |
| `top_k` | `top_k` | u32 | 40 | 1+ | Top-k token filter |
| `min_p` | `min_p` | f32 | 0.0 (disabled) | 0.0–1.0 | Min-p threshold |
| `typical_p` | `typical_p` | f32 | 0.0 (disabled) | 0.0–1.0 | Typical-p (locally typical sampling) |
| `max_tokens` | `max_tokens` | u32 | 512 | 1+ | Maximum completion tokens |
| `seed` | `seed` | Option<u32> | None (random) | u32 or "off" | Deterministic seed |
| `gas_heuristic` | `gas_heuristic` | u64 | 500 | 1+ | Per-turn gas reservation estimate |
| `gas_cap` | `gas_cap` | u64 | 10,000 | 1+ | Total session energy budget |
| `auto_condense` | `auto_condense` | bool | true | on/off | Auto-condense at 87.5% of context window |

### 8.2 Model Metadata (Read-Only)

Populated automatically when switching models via `/model`:

| Field | Source | Description |
|-------|--------|-------------|
| `context_length` | Okapi `/api/show` | Model's context window in tokens |
| `supports_thinking` | Okapi `/api/show` | Whether model supports `<thinking>` tags |
| `capabilities` | Okapi `/api/show` | Model capability flags (e.g., vision, tools) |

### 8.3 Persistence

Settings are persisted to `~/.config/hkask/settings.json` whenever a valid setting is changed. This ensures CLI and API surfaces share the same configuration. The file is JSON-serialized from `ReplSettings` (via serde).

### 8.4 Usage Examples

```
/repl                     # Show all settings
/repl temp 0.3            # Set temperature to 0.3
/repl context 5           # Keep 5 turns of history
/repl seed 42             # Deterministic output
/repl seed off            # Random seed
/repl auto_condense off    # Manual condensation only
/repl reset               # Reset all to defaults
```

## 9. Memory Infrastructure

### 9.1 Memory Types

| Memory | Scope | Storage | Purpose |
|--------|-------|---------|---------|
| Episodic | Private, agent-scoped | SQLCipher `hkask-memory-{agent}.db` | Record of conversations and interactions |
| Semantic | Public, shared | SQLCipher `hkask-memory-{agent}.db` | Consolidated knowledge triples |

### 9.2 Memory Flow Per Turn

1. **Before inference:** Semantic recall retrieves relevant public triples
2. **During inference:** Retrieved triples are incorporated into the prompt context
3. **After inference:** The exchange is stored as an episodic triple (subject: agent, predicate: `responded_to`, object: input hash, context: full response)

### 9.3 Consolidation (`/consolidate`)

User-triggered episodic→semantic consolidation via `ConsolidationService`:

```
/consolidate                           # Show status (candidates, triple counts, low-confidence)
/consolidate run                       # Execute with defaults (limit 100)
/consolidate 50                        # Limit 50 triples
/consolidate run --floor 0.5           # Confidence floor 0.5
/consolidate run --max 500             # Max 500 semantic triples
/consolidate run -f 0.33 -m 200 -l 50 # All flags combined
```

The service shares the same underlying DB connection as the storage ports, so consolidation operates on the agent's actual triples.

### 9.4 Fallback

If the per-agent DB cannot be opened (e.g., wrong passphrase), the REPL falls back to an in-memory database with a warning message. Consolidation works on the in-memory DB.

## 10. Gas Governance (EnergyGuard)

### 10.1 Hold-Settle Pattern

Every inference turn and tool invocation follows a two-phase gas accounting pattern:

1. **Hold (Reserve):** Before starting work, reserve a heuristic estimate of gas
2. **Settle:** After work completes, reconcile with the actual token cost

This is encapsulated in `EnergyGuard`, an RAII guard that:
- Checks `can_proceed()` before reserving
- Stores the heuristic amount
- Settles actual cost after inference
- Syncs `InferenceLoop` from the L6 CyberneticsLoop budget
- Debug-asserts on drop if not settled (prevents leaked reservations)

### 10.2 Energy Budget Configuration

Each agent gets an energy budget registered with the `CyberneticsLoop` at REPL init:

```
Budget:    gas_cap (default 10,000)
Replenish: gas_cap / 10 per replenishment cycle
Alert:     80% usage → yellow CNS alert
Hard limit: true (blocks operations when exhausted)
```

### 10.3 User-Visible Gas Display

```
/status → Gas: ■ 7500/10000 (75%)
After turn → [Gas budget low: 1500/10000 (15%)]  (yellow warning)
After turn → [Gas budget exhausted — some operations may be throttled]  (red)
```

## 11. Model Management

### 11.1 Model Discovery

Models are discovered through Okapi (the inference gateway, built on llama.cpp). The `/model` command queries Okapi's model registry.

### 11.2 Model Switching

Switching models via `/model <name>`:
1. Searches Okapi for the model name (exact or fuzzy match)
2. Updates `state.current_model`
3. Fetches model metadata (context window, thinking support, capabilities)
4. Populates `state.repl_settings.model_meta` for auto-condense threshold calculation
5. The `InferencePort` is NOT recreated — Okapi handles model routing internally

### 11.3 Model Metadata

```rust
struct ModelMeta {
    context_length: u32,      // Model's maximum context window
    supports_thinking: bool,  // Whether model supports <thinking> tags
    capabilities: Vec<String>, // e.g., ["vision", "tools", "json_mode"]
}
```

This metadata feeds into:
- Auto-condense threshold (87.5% of `context_length`)
- Future: thinking mode toggle, JSON mode selection

## 12. Tool-Augmented Inference

### 12.1 Tool Discovery

At REPL init, `GovernedTool::discover_tools()` queries the MCP runtime for all registered tools. Results are formatted into a system prompt section:

```
## Tool Calls
You have access to MCP tools. When you need to invoke a tool, use:

<<tool:server/tool_name
{"key": "value"}
>>

**memory:**
- memory_store — Store memories
- memory_recall — Recall memories
...

You may include multiple tool calls in a single response.
```

### 12.2 Built-in MCP Servers (11)

Started automatically at REPL boot via `builtin_servers::start_builtin_servers()`:

| Server ID | Binary | Purpose |
|-----------|--------|---------|
| `memory` | `hkask-mcp-memory` | Semantic + episodic memory operations |
| `condenser` | `hkask-mcp-condenser` | Context condensation, reranking, compression |
| `spec` | `hkask-mcp-spec` | Specification authoring, curation, validation |
| `research` | `hkask-mcp-research` | Web search, extraction, and feed-based research |
| `companies` | `hkask-mcp-companies` | Company financial data (FMP + EODHD dual-provider) |
| `communication` | `hkask-mcp-communication` | Thin MCP wrapper over core communication crate |
| `media` | `hkask-mcp-media` | Media generation (image, video, audio, 3D) |
| `docproc` | `hkask-mcp-docproc` | Unified document processing (format conversion, OCR, chunking, parsing) |
| `replica` | `hkask-mcp-replica` | Style embedding and prose composition |
| `training` | `hkask-mcp-training` | Model training data ingestion |
| `kanban` | `hkask-mcp-kanban` | Kanban board coordination |
| `filesystem` | `hkask-mcp-filesystem` | Filesystem read/write/search + shell command execution for agent code interaction |

Servers that fail to start are logged and skipped — their tools simply won't be available.

The `filesystem` server auto-starts at REPL boot as the essential sensory/actuation
interface for agents. All other servers require explicit consent via `/mcp start`.

### 12.3 Direct Tool Invocation (`/invoke`)

Users can bypass agent-mediated tool calls and invoke tools directly:

```
/invoke web_search '{"query": "Rust async patterns"}'
/invoke condenser/condenser_compress '{"output": "..."}'
/invoke memory_store
```

All invocations route through `GovernedTool` with OCAP token minting and CNS observability.

## 14. CNS Integration

After each turn, `cns_display::update_cns_and_display()` executes:

1. **Prompt Variety Sensing:**
   - Depth bucket (shallow/medium/deep) → `cns.inference.prompt_depth`
   - Structure (question/imperative/declarative/conditional) → `cns.inference.prompt_structure`
   - Topic keywords → `cns.inference.prompt_domain`

2. **Algedonic Alert Check:**
   - Queries `CnsRuntime::critical_alerts()`
   - Displays alerts with deficit/threshold values
   - Deficits > threshold/2 (50) → escalate to Curator
   - Deficits > threshold (100) → escalate to human

3. **LoopSystem Tick:**
   - Runs `LoopSystem::tick()` — sense→compare→compute→act cycle
   - `CyberneticsLoop` reads CNS variety + energy budgets → produces regulatory actions
   - Regulatory actions are logged via tracing (visible with `RUST_LOG=cns.cybernetics=debug`)

## 15. Welcome Banner

The REPL displays an animated Kask amphora logo on startup:

```
  __              ___________    __
 /  \            /  \  .:::.  /  \
|    |          |    |           |    |
 \__/           |    |    KASK   |    |
                |    |           |    |
                 \__/~~~~~~~~~~~\__/
  shadow           hKask v0.28.0

     A Minimal Viable Container for Agents
```

The eyes animate through center → right → center → left gaze positions over ~1.4 seconds.

**Info row (returning user):**
```
Agent: {name}  Model: {model}  Template: {template}
/help for commands  <TAB> autocomplete  /quit exit
```

**First Steps guide (first sign-in only):** When `ReplState.is_first_run` is `true` (set on first OAuth sign-in when a new WebID and default replicant are provisioned), the compact one-liner is replaced with an expanded guide:

```
  ━━ First Steps ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Getting started:
  • Just type to chat — your replicant is ready
  • /help    — see all available commands
  • /model   — switch models anytime
  • /tools   — discover available MCP tools
  • /status  — check system health and energy
  • /repl    — customize inference settings

  Try: "What can you help me with?"
  Type /start for a guided tour.
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Returning users see the compact one-liner only. `is_first_run` is `false` for all returning sessions.

## 16. Autocomplete and Help System

### 16.1 Tab Completion (`KaskHelper`)

- Slash commands: Tab-completes from the `SLASH_COMMANDS` table (primary names + aliases)
- Dimmed hint text shows the remainder of the matched command
- Highlights completed slash commands in cyan

### 16.2 Help System

**`/help` (no args):** Categorized menu:
```
Session    — help, quit, clear, history
Agent      — agent, agents, pods
Model      — model
System     — status, tools, templates, sovereignty
Governance — escalations, resolve, dismiss, metacognition
Onboarding — start, feedback
```

**`/help <command>`:** Detailed page with usage examples, subcommands, and tips.

**Fuzzy fallback:** Unknown commands trigger fuzzy matching across primary names, aliases, and descriptions. Suggests up to 5 matches.

## 17. Session History

Session history is no longer stored in-memory. Instead, all history access routes
through OCAP-gated episodic storage:

- **Storage:** Each chat exchange is persisted as an episodic triple via `ChatService::store_episodic()` after inference
- **Recall:** History is retrieved via `ChatService::recall_recent_turns()`, which queries
  `EpisodicStoragePort` with a `DelegationToken` bearing `Read` on `Manifest`
- **Display:** `/history` calls `recall_recent_turns()` with `usize::MAX` to retrieve all turns
- **Context:** The per-turn pipeline appends recent turns as a suffix after the current input
  (preserving system prompt KV-cache hits)

This replaces the previous `SessionHistory` / `Turn` in-memory ring buffer.
The correct fix was deletion, not patching: `SessionHistory` duplicated data
already persisted in `EpisodicStoragePort`.

## 18. Auto-Condense

Auto-condense triggers at 87.5% of the model's context window during `ChatService::execute_turn()`. After appending recent conversation history as a suffix via `recall_recent_turns()`, the pipeline checks if the approximate token count exceeds 87.5% of `context_window`. When triggered, it fetches raw episodes via `recall_raw_episodes()`, splits them in half, and calls `InferencePort::generate_with_model()` directly (bypassing the MCP tool for efficiency) to condense the oldest half into a structured summary. The history suffix is replaced with `[Condensed history]` + `[Recent conversation]` blocks. Graceful degradation: if the condenser call fails or returns empty, the full uncondensed context is used with no error surfaced to the user.

Enabled by default (`auto_condense: on`). Toggle via `/repl auto_condense off` or the API settings endpoint.

## 19. Key Design Decisions

### 19.1 History as Suffix (Not Prefix)

System prompt + tool section form a stable prefix that stays cacheable across turns (KV-cache hits). History changes each turn and must be placed after the cache breakpoint. This is a deliberate optimization for Okapi/llama.cpp KV-cache reuse.

### 19.2 InferencePort Lifetime

The `InferencePort` is created once at REPL init, not per-turn. This enables connection reuse (HTTP keep-alive) and allows Okapi to manage model loading/unloading internally.

### 19.3 Shared Infrastructure via AgentService

The REPL routes all infrastructure through `AgentService::build()`, which creates CNS, loop system, governed tool, pod manager, and MCP runtime as a single assembly. The REPL adds surface-specific concerns (inference port, per-agent memory, onboarding) on top.

### 19.4 GovernedTool as Singular Boundary

Every tool invocation — whether from agent tool-use loops, direct `/invoke` commands, or auto-condense — routes through a single `GovernedTool` instance. This is intentional: it means OCAP authorization, energy budgets, and CNS observability are enforced at a single choke point with no bypass paths.

### 19.5 Per-Agent Memory Isolation

Each agent gets its own SQLCipher-encrypted database (`hkask-memory-{agent}.db`). No cross-agent memory access. The `WebID` is deterministically derived from the agent name, ensuring consistent identity across sessions.

## 20. Future Features (Zed Parity Roadmap)

The following features are drawn from Zed's REPL implementation (`crates/repl/`) and represent the leading edge of AI REPL capabilities. Each is assessed for hKask's headless terminal context.

### 20.1 Code Execution via Jupyter Kernel Protocol

**Status:** Planned

Jupyter kernel integration enables agents to write code, execute it, inspect results, and self-correct — closing the agentic loop. The Jupyter wire protocol over ZMQ sockets provides structured output (typed messages for stdout, stderr, errors, display data, input requests) rather than fragile PTY/ANSI parsing.

**Key sub-features:**
- Kernel lifecycle state machine (Starting → Running(idle|busy) → ShuttingDown → Shutdown)
- Multi-source kernel discovery (Jupyter kernelspec, Python venv/conda/poetry/uv/pyenv)
- 3-socket concurrent message loop (iopub, shell, stdin)
- Energy-budget metering per kernel execution
- CNS spans: `cns.repl.kernel.{started,busy,idle,dead,errored}`

### 20.2 Code Cell Detection (`runnable_ranges()`)

Detect what code to execute from cursor position in the input buffer:
- Markdown fenced code blocks with supported language tags
- Jupytext markers (`# %%` with any language's comment prefix)
- Contiguous non-blank code block around/after cursor
- Blank-line skip (if cursor is on blank line, skip forward)
- Trailing blank trim

**Value for hKask:** Agents often generate code blocks. Cell detection means "just run what makes sense" — users don't need to manually select code.

### 20.3 Output Rendering with MIME Ranking

When a kernel returns a MIME bundle (one result with multiple formats), rank by display quality for the terminal medium:

| MIME Type | Terminal Behavior |
|-----------|-------------------|
| `text/plain` | Print directly |
| `text/markdown` | Render with ANSI formatting (bold, code fences) |
| `application/json` | Pretty-print with optional syntax coloring |
| `text/html` | Strip tags, extract text |
| `image/png`, `image/jpeg` | Save to `$XDG_DATA_HOME/hkask/kernel-output/`, print path |
| `DataTable` | Render as ASCII table |

### 20.4 Stream Output Merging

Consecutive `StreamContent(stdout)` messages append to a single output block rather than creating separate entries. Prevents visual fragmentation for code like `for i in range(100): print(i)`.

### 20.5 ClearOutput Protocol

Jupyter's `ClearOutput` message with `wait` flag: `wait: false` clears immediately, `wait: true` defers until next output arrives (prevents flicker). Useful for progress indicators and live-updating displays.

### 20.6 Execution View Keyed by `parent_message_id`

Each execution tracks a view identified by the originating request's `msg_id`. Incoming messages are routed to the correct view. Enables concurrent agent-triggered tool calls without output interleaving.

### 20.7 Input Request (`input()`) Support

When kernel sends `InputRequest`, REPL enters a sub-prompt (using existing `rustyline`), collects input, and sends `InputReply` via the stdin ZMQ socket. Password masking supported. If kernel goes idle before input arrives, pending input is discarded.

### 20.8 Kernel Session Persistence

Kernel sessions survive across chat turns — variables, imports, and state persist. Slash commands for kernel management:
```
/kernel restart           # Reset kernel state
/kernel switch python3.12 # Change kernel
/kernel status            # Show kernel info and state
```

### 20.9 Agent Output Loop (Self-Correcting Agents)

Agent writes code → executes → sees error traceback → rewrites → re-executes → verifies. The output feedback loop is structured: error tracebacks are formatted for LLM consumption, enabling self-correction without human intervention.

### 20.10 Additional Leading-Edge Features

| Feature | Source | Description |
|---------|--------|-------------|
| Thinking mode toggle | Zed, Claude | `/thinking on/off` to show/hide model reasoning chains |
| JSON mode enforcement | OpenAI | `/json on` to enforce structured JSON output |
| Prompt diffing | Aider | Show what changed in the system prompt between turns |
| Multi-model responses | ChatGPT, Claude | `/compare` to run same prompt against multiple models |
| Session export/import | All leading REPLs | Save/load conversation to file (Markdown, JSON, or hKask format) |
| Undo last turn | Claude | `/undo` to remove last exchange from context |
| Context window visualization | Zed, Claude | Visual gauge showing context window fill percentage |
| Tool approval mode | Claude | `/tools ask` to require user confirmation before each tool call |
| Custom slash commands | Aider | User-defined slash commands via config file |
| MCP server hot-reload | — | Start/stop MCP servers without restarting REPL |

## 21. Constraint Compliance

| Constraint | Compliance |
|-----------|------------|
| Headless only (P6) | ✓ Terminal-based (browser terminal renders PTY); no GUI, web app, or dashboard |
| No external MCP deps (P1.2) | ✓ All 10 servers are `hkask-mcp-*` crates |
| No `todo!()`, `unimplemented!()` (P7) | ✓ No stubs in REPL code |
| No deprecated code (P7) | ✓ No `#[deprecated]` annotations |
| No monitoring stacks (P6) | ✓ CNS provides programmatic observability; no Prometheus/Grafana |
| Test depth matches module depth (C8) | ✓ Handlers are shallow (integration-tested via CLI); turn.rs is deep (gas, tool-loop) |
| User sovereignty (P1) | ✓ OCAP tokens, SQLCipher per-agent DBs, `/sovereignty` verification |
| Generative Space (P3) | ✓ `/repl` exposes all parameters; no hidden settings |

## 22. References

- [`PRINCIPLES.md`](../../architecture/core/PRINCIPLES.md) — Architecture principles (P1–P11)
- [`magna-carta.md`](../../architecture/core/magna-carta.md) — User sovereignty charter
- [`hKask-architecture-master.md`](../../architecture/hKask-architecture-master.md) — Architecture index
- [Zed REPL crate](https://github.com/zed-industries/zed/tree/main/crates/repl) — Reference implementation for code execution patterns
- [Jupyter Messaging Protocol](https://jupyter-client.readthedocs.io/en/stable/messaging.html) — Wire protocol specification
- [`Cargo.toml`](../../../crates/hkask-cli/Cargo.toml) — Dependencies and feature flags
