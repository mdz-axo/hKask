# Zed Architecture Briefs — Phase 0 T0.2

External research stimulus (domain_supplement, confidence ≤0.55). Zed is a
reference, not a spec. hKask core ontology overrides any conflict.

---

## Brief 1: MCP Server Hosting / Tool Dispatch

**Source:** `zed-industries/zed` — `crates/agent/src/agent.rs` (field
`context_server_registry: Entity<ContextServerRegistry>`), DeepWiki §8.4
Native Agent and Thread Management, Zed MCP docs
(https://zed.dev/docs/assistant/model-context-protocol).

**Mechanism:** Zed implements MCP from scratch (not the official Rust SDK —
the SDK didn't exist when Zed started, per discussion #29370). The
`ContextServerRegistry` is a GPUI entity that manages MCP server lifecycle.
Servers are discovered from three sources: (1) extensions (via
`context_server_command` in extension.toml), (2) settings.json (custom
command+args+env), (3) remote servers over streamable HTTP. When a server
adds/removes/modifies tools at runtime, Zed auto-reloads the tool list
without restarting the server. Tools are addressed as
`mcp:<server>:<tool_name>` in the agent's tool registry.

**Key insight:** Zed's MCP dispatch is *thin* — the `ContextServerRegistry`
just owns the child process / HTTP connection, and tool calls are forwarded
through the MCP JSON-RPC protocol. There is no per-tool abstraction layer;
the agent sees a flat namespace of `mcp:<server>:<tool_name>` entries. This
is shallower than hKask's `Parameters<T>` + `#[tool_router]` + `execute_tool`
dispatch, which wraps each tool call in a gas/energy/pipeline guard.

**Transferability to hKask:** Partial. Zed's flat namespace is simpler but
loses hKask's OCAP-gated capability checking. The auto-reload-on-change
mechanism transfers well (hKask currently requires reindex for codegraph
tools). The from-scratch MCP implementation is NOT transferable — hKask
already uses `rmcp = "1"` (official Rust SDK), which Zed is now considering
adopting.

---

## Brief 2: Inference Provider Abstraction

**Source:** `zed-industries/zed` — `crates/agent/src/agent.rs` (field
`models: LanguageModels`), DeepWiki §8.7 Language Model Integration sidebar
topics (Provider Architecture, Cloud LLM Provider, LLM Provider
Implementations), Zed Agent docs (https://zed.dev/docs/ai/zed-agent).

**Mechanism:** Zed uses a `LanguageModel` trait (`dyn LanguageModel`) with
implementations for each provider (Anthropic, OpenAI, Google, Ollama,
OpenRouter, etc.). The `LanguageModelRegistry` is the central registry; the
`LanguageModels` struct on `NativeAgent` caches the model list. Model IDs
follow a `"provider/model_name"` format (e.g., `"anthropic/claude-sonnet-4"`).
There is a separate `Cloud LLM Provider` for Zed's own hosted inference
service. The provider architecture handles: streaming completions, tool
definitions, vision/multimodal input, token counting, and authentication
(API keys stored in settings/keychain).

**Key insight:** Zed's provider abstraction is a single trait with many
implementations — very similar to hKask's `InferencePort` trait implemented by
`InferenceRouter`. The difference: Zed's `LanguageModel` trait is the direct
provider interface (one impl per provider), while hKask's `InferencePort` is
implemented once by `InferenceRouter` which internally dispatches to 8
backends via `chat_backend()` / `vision_backend()` match-fns. hKask's design
is deeper (the router is a deep module with a narrow `InferencePort`
interface) but the backend dispatch is a match-fn, not a trait — each backend
implements `ChatBackend` / `VisionBackend` separately.

**Transferability to hKask:** The `"provider/model_name"` ID format is
already used by hKask (as prefix-based: `DI/`, `OR/`, etc.). Zed's approach
of one trait per provider (vs. hKask's match-fn dispatch) would add a trait
layer but would NOT reduce complexity — it would move the match-fn into a
trait object dispatch. This is a lateral move, not a consolidation. The
interesting transfer is Zed's `LanguageModelRegistry` as a single source of
truth for model metadata — hKask already has this via `RouterModelEntry` and
`model_constants.rs`.

---

## Brief 3: REPL / Chat / Agent-Thread Lifecycle

**Source:** `zed-industries/zed` — `crates/agent/src/agent.rs` (NativeAgent),
`crates/agent/src/thread.rs` (Thread), `crates/agent/src/thread_store.rs`
(ThreadStore), `crates/agent/src/db.rs` (ThreadsDatabase), `crates/acp_thread/`
(ACP protocol wrapper), DeepWiki §8.4 and §8.6.

**Mechanism:** Zed's agent thread lifecycle is:
1. `NativeAgent` (top-level orchestrator, GPUI entity) manages sessions
2. `Session` bridges internal `Thread` logic with `AcpThread` protocol wrapper
3. `Thread` is the core conversation entity: holds `messages: Vec<Message>`,
   `tools: BTreeMap<SharedString, Arc<dyn AnyAgentTool>>`, `model: Option<Arc<dyn LanguageModel>>`
4. `AgentTool` trait defines tool interface (4 built-in tools: read_file, edit_file, terminal, spawn_agent)
5. `ThreadEnvironment` allows tools to interact with app shell (spawn terminals, subagents)
6. `ThreadStore` + `ThreadsDatabase` (SQLite via `sqlez`) persist threads to disk
7. Auto-compaction at 80K token context window, retains ~20K tokens of recent messages
8. `MAX_SUBAGENT_DEPTH = 1` prevents infinite recursion
9. `ActionLog` tracks tool actions for telemetry/undo
10. `ThreadMetadataStore` (separate from `ThreadStore`) manages lightweight UI metadata

**Key insight:** Zed separates full thread content (`ThreadStore` → `ThreadsDatabase`)
from UI metadata (`ThreadMetadataStore` → `ThreadMetadataDb`) — two separate
SQLite stores. hKask's `ReplState` holds thread state in-memory with
`thread_registry` and `manifest_state`, and the REPL's `TuiReplBridge`
implements 4 traits (ReplBridge, SystemBridge, SettingsBridge, SessionBridge)
to bridge between the REPL and the TUI. Zed's `Thread` has a flat tool map
(`BTreeMap<SharedString, Arc<dyn AnyAgentTool>>`), while hKask routes tools
through MCP servers + manifest cascades.

**Transferability to hKask:** The `AgentTool` trait pattern (with a flat
tool map) is simpler than hKask's MCP-server-per-domain approach but would
lose hKask's OCAP capability gating. The auto-compaction mechanism (byte
budget of ~20K tokens) is a concrete improvement hKask could adopt — hKask's
REPL currently relies on the condenser MCP server for this, which is a
separate process. The `ActionLog` pattern (tracking tool actions for undo)
is a valuable addition hKask's REPL lacks. The separation of content store
from metadata store is an architectural simplification that could reduce
edges in hKask's storage layer.