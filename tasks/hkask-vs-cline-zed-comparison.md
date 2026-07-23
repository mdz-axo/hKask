# hKask vs Cline vs Zed — Turn Process Element-by-Element Comparison

**Date:** 2026-07-22  
**Sources:** hKask codebase (v0.31.0), Cline `src/core/task/index.ts` + `sdk/packages/agents/src/agent-runtime.ts`, Zed `crates/agent/src/thread.rs` + `crates/agent_ui/src/agent_panel.rs`

---

## Classification Legend

| Label | Meaning |
|---|---|
| **ESSENTIAL** | Justified by hKask's unique requirements: Magna Carta principles (P1–P4, P12), OCAP capability gating, cybernetic regulation, skill manifests, multi-provider inference, sovereign memory |
| **JUSTIFIED IDIOMATIC RUST** | Good Rust practice that differs from Cline/Zed (TypeScript/Swift) but is correct for hKask's language and architecture |
| **NOISE** | Unnecessary complexity that doesn't serve the agent loop, may cause bugs, or follows bad practices |

---

## Architecture Overview

### hKask
```
CLI/TUI entry → ReplState → run_turn_with_state → run_turn_loop (sync, block_on)
  → TurnDeps { executor, gas, tools, threads, on_reg_update }
  → TurnExecutor.execute_turn(TurnInput) → ChatService::execute_turn
    → ChatService::prepare_chat (system prompt, memory recall, message array)
    → ChatService::chat (inference, Regulation spans, episodic storage)
  → extract_tool_calls → ToolPort.invoke (OCAP token) → format_tool_results
  → loop until no tool calls or max_loops
```

### Cline
```
Task entry → initiateTaskLoop (while !abort)
  → recursivelyMakeClineRequests(userContent)
    → API stream → parse tool blocks → ToolExecutor.handleCompleteBlock
    → pushToolResult → accumulate in ApiConversationHistory
    → loop until no tools or attempt_completion
  → if no tools: nudge "noToolsUsed", consecutiveMistakeCount++
```

### Zed
```
AgentPanel → Thread entity → running_turn: Option<RunningTurn>
  → stream completion → parse tool_use blocks → AgentTool.run(input, event_stream)
  → append tool_result to user_message → re-request model
  → ThreadEvent stream to UI (AgentText, ToolCall, ToolCallUpdate, Stop)
  → context compaction when approaching token limit
```

---

## Step-by-Step Comparison

### 1. Turn Initiation

**hKask:**
- CLI: `hkask-cli/src/commands/tui.rs` `run_tui()` → `hkask_repl::run()` or `hkask_repl::run_tui()`
- REPL: `run_turn_with_state()` builds `TurnConfig` from `ReplSettings`, constructs `TurnDeps` from `ReplState`, calls `run_turn_loop()` (`crates/hkask-repl/src/turn.rs:131`)
- TUI: `ReplBridge::start_inference(input)` spawns a background tokio task, returns `InferenceRequestId`; TUI polls `poll_inference()` each frame (`crates/hkask-repl/src/tui/repl_bridge.rs:147-149`)
- The turn loop is a **synchronous function** (`run_turn_loop`) that uses `rt.block_on()` to call async inference — the sync/async boundary is at the loop level

**Cline:**
- `initiateTaskLoop(userContent)` — a `while (!abort)` loop calling `recursivelyMakeClineRequests(nextUserContent)` (`src/core/task/index.ts`)
- Fully async TypeScript; the loop awaits each recursion
- First iteration includes file details; subsequent iterations do not

**Zed:**
- `Thread` entity holds `running_turn: Option<RunningTurn>` — a GPUI `Task` that drives the turn (`crates/agent/src/thread.rs`)
- `ThreadView::start_turn()` increments `turn_generation`, starts a timer task
- UI sends a message → `AcpThread` processes → model request begins
- Fully async via GPUI's task system

**Classification: JUSTIFIED IDIOMATIC RUST**

hKask's sync `run_turn_loop` with `block_on` is unusual but justified: the ratatui TUI event loop is synchronous and runs on the main thread; the turn loop runs on a background tokio task where `block_on` is safe. Cline and Zed are async-native (TypeScript async/await, GPUI Task). The `InferenceRequestId` polling pattern in the TUI bridge is a clean Rust idiom for non-blocking UI integration. No noise here.

---

### 2. Pre-Inference Preparation (System Prompt, Memory Recall, Model Resolution)

**hKask:** (`crates/hkask-services-chat/src/chat/service.rs:126-276` `prepare_chat`)
- System prompt: `"You are {name} in the hKask system.\n\n"` + tool_section + api_spec
- Model resolution: `req.model_override` → `ctx.config().default_model`
- Inference port resolution: override → shared port → fresh `InferenceService::resolve_port()` (multi-provider)
- WebID derivation: `WebID::from_persona_with_namespace(name, "userpod")` — cryptographic identity
- Capability token: `CapabilityChecker::grant_registry(Execute, principal_webid, agent_webid)` — OCAP
- **Sovereignty gate (P2):** `has_memory_consent()` for Semantic + Episodic → recall only if consented
- Memory recall: `recall_semantic()` + `recall_episodic()` → merged into "## Relevant Memory" section
- Builds BOTH `full_prompt` (flattened string) AND `messages` (typed `Vec<ChatMessage>`)

**Cline:**
- System prompt composed from environment details, workspace structure, custom instructions
- No memory recall — Cline is stateless in the SDK; conversation history in memory only
- No consent gates, no capability tokens
- Model resolution from provider config

**Zed:**
- `SystemPromptTemplate` + `Templates` system — compose from project context, agent profile, instructions
- No semantic/episodic memory recall — context comes from @mentions (files, symbols, diagnostics, git diff)
- Model resolution via `LanguageModelRegistry`
- No consent gates, no capability tokens

**Classification: ESSENTIAL**

The WebID derivation, OCAP capability tokens, sovereignty-gated memory recall, and multi-provider inference port resolution are all directly justified by hKask's Magna Carta principles (P1 User Sovereignty, P2 Affirmative Consent, P12 Identity) and cybernetic regulation. Neither Cline nor Zed has sovereign memory or capability-gated recall. This is hKask's differentiating core.

---

### 3. Message Array Construction

**hKask:** (`service.rs:252-264`)
- `[system(system_prompt + memory_context), ...thread_messages, user(input)]`
- `thread_messages` from `ThreadRegistry::thread_history_messages()` — preserves `role` tags ("user"/"assistant") as `ChatMessage` (`crates/hkask-repl/src/threads.rs:256-284`)
- When no thread messages: `[system, user]`

**Cline:**
- `ApiConversationHistory` — an accumulating array of user/assistant messages
- Tool results pushed as user messages with structured content blocks
- Native tool calls use provider's function-calling format

**Zed:**
- `messages: Vec<Arc<Message>>` on `Thread` — `UserMessage` and `AgentMessage` variants
- `AgentMessage::to_request()` converts to `LanguageModelRequestMessage` with proper `Role::Assistant`/`Role::User`
- Tool results appended to a user message as `MessageContent::ToolResult`

**Classification: JUSTIFIED IDIOMATIC RUST**

The typed `ChatMessage` array with role preservation is correct and matches what Cline and Zed do. The `thread_history_messages` function that filters to "user"/"assistant" roles and maps to `ChatMessage` is clean. However, see step 7 for a critical bug in how this interacts with the loop.

---

### 4. Inference Call

**hKask:** (`service.rs:326-364`)
- `tokio::time::timeout(120s, inference_port.generate_with_messages(messages, params, model, tools))`
- Regulation spans persisted: `reg.chat.request` (before) and `reg.chat.response` (after, with parent linkage)
- Error mapping: timeout → `ServiceUnavailable` (retryable), other → `BadRequest` (not retryable)
- Fusion bypass: when fusion is active, chat model bypasses fusion (cost-safety guard, enforced regardless of caller override)

**Cline:**
- `this.api.createMessage()` → streaming API call
- No timeout wrapper (relies on provider timeout)
- No regulation/observability spans
- Token cost tracking via `calculateApiCost()`

**Zed:**
- `language_model::stream_completion()` → streaming
- Token usage tracked per-request and cumulatively on `Thread`
- `CompletionError::MaxTokens` → `StopReason::MaxTokens`
- No regulation spans; telemetry via `TelemetrySnapshot`

**Classification: ESSENTIAL**

The 120s timeout, Regulation span persistence (`reg.chat.request`/`response` with parent linkage), and fusion bypass enforcement are all essential to hKask's cybernetic regulation framework. The error classification (retryable vs not) enables the gas governor to make informed settle/release decisions. Neither Cline nor Zed has this.

---

### 5. Response Parsing (Tool Call Extraction)

**hKask:** (`crates/hkask-repl/src/turn.rs:972-986`)
- `extract_tool_calls(response_text, structured_tool_calls)` — **uses only structured tool calls** from native function calling
- When `structured_tool_calls` is empty, returns empty `tool_calls` — does NOT parse text
- `ParsedResponse { text, tool_calls }` — text is always the full response text
- Nudge on iteration 1 if `has_tools` and no tool calls: tells model to emit `<<tool:server/name {json} >>` text directive

**Cline:**
- Two modes: `useNativeToolCalls` (native function calling) or text-based XML parsing (`<tool_use>` blocks)
- `ToolUse` blocks extracted from streaming chunks via `handlePartialBlock` / `handleCompleteBlock`
- Handles both partial (UI streaming) and complete (execution) blocks

**Zed:**
- Native function calling via `language_model::MessageContent::ToolUse`
- `AgentMessageContent::ToolUse(tool_use)` parsed from completion stream
- Tool use only included in request if `tool_results.contains_key(&tool_use.id)` — prevents orphaned tool calls

**Classification: NOISE — mixed-signal parser causes a bug**

The `extract_tool_calls` function ignores text-based tool directives entirely — it only uses `structured_tool_calls` from the inference port. Yet the nudge message on iteration 1 (`turn.rs:232-236`) tells the model to emit `<<tool:server/name {json} >>` text directives. **The parser does not parse what the nudge tells the model to produce.** If a model follows the nudge and emits `<<tool:...>>` text, those tool calls are silently dropped (the text goes into `parsed.text` as final response, potentially breaking the loop).

This is a **bug**: the nudge and parser are inconsistent. Either the nudge should reference native function calling (which is what the parser actually uses), or `extract_tool_calls` should parse `<<tool:...>>` directives as a fallback. Cline handles both modes explicitly with `useNativeToolCalls`; Zed is native-only with no contradictory nudge.

---

### 6. Tool Execution

**hKask:** (`turn.rs:260-276`)
- For each tool call: mint `DelegationToken` with `DelegationResource::Tool`, `DelegationAction::Execute`, `principal_webid`, `agent_webid`, `derive_signing_key(a2a_secret)`
- `deps.tools.invoke(server, tool, args, &token)` — `ToolPort` trait
- Results collected as `Vec<(ToolCall, Result<Value>)>`
- Sequential execution (for loop, `block_on` per call)
- No tool approval/policy gate (OCAP token IS the authorization)

**Cline:**
- `ToolExecutor` class with `ToolExecutorCoordinator` dispatching to registered handlers
- `AutoApprove` policy: `shouldAutoApproveTool()` checks settings
- `PreToolUse` hooks (before execution, can skip/block), `PostToolUse` hooks (after, observe-only)
- `requestToolApproval()` for tools requiring manual approval
- Repeated tool call detection (soft warning / hard escalation)
- Partial block handling for UI streaming

**Zed:**
- `AgentTool` trait: `run(input, event_stream, cx) -> Task<Result<Output>>`
- `ToolPermissionDecision` / `ToolCallAuthorization` — permission system
- `decide_permission_from_settings()` — settings-based auto-approval
- `ThreadSandboxGrants` — per-thread approved sandbox permissions
- Tools in `BTreeMap<SharedString, Arc<dyn AnyAgentTool>>`
- `ToolCallEventStream` for streaming progress updates

**Classification: ESSENTIAL**

The `DelegationToken` with cryptographic signing (`derive_signing_key(a2a_secret)`) and principal/agent WebID binding is hKask's OCAP capability model (P12 Identity, P1 User Sovereignty). Every tool invocation carries a capability token proving the agent is authorized by the principal. Neither Cline (settings-based auto-approve) nor Zed (permission decisions + sandbox grants) has cryptographic capability tokens. This is essential and differentiating.

**Note:** Sequential execution (no parallel tool calls) is a limitation but not noise — it's a deliberate simplicity choice consistent with hKask's single-agent turn model. Cline supports parallel execution via `config.toolExecution === "parallel"`.

---

### 7. Loop Iteration (Growing Array vs. Rebuild)

**hKask:** (`turn.rs:297-298`)
- `current_input = response` — **the model's response text becomes the input for the next iteration**
- `tool_results = Some(format_tool_results(&tool_results_vec))`
- Next iteration: `TurnInput { input: &current_input, tool_results, thread_history, thread_messages }`
- `thread_history` and `thread_messages` are re-fetched each iteration from `ThreadRegistry` (unless `is_seeded()`, but `mark_seeded()` only happens after the loop)
- Thread is only appended after the loop completes (`append_turn` at line 311)
- So during iterations 2+, `thread_messages` contains **only previous turns from before this turn started** — NOT the current turn's iterations

**In `execute_turn`** (`service.rs:702-733`):
- `input_with_context = format!("{}\n\n{}", base_input, thread_history)` — base_input is `current_input` (the previous response)
- `effective_input = format!("{}\n\nThe following tool calls were executed:\n\n{}\n\nBased on these results, provide your response.", input_with_context, tool_results)`

**In `prepare_chat`** (`service.rs:231-239`):
- `full_prompt = format!("{}\n\n## Relevant Memory\n{}\n\nUser: {}", system_prompt, memory_context, req.input)`
- **The model's own response from the previous iteration is labeled as "User:" in the next iteration's prompt**

**Cline:**
- `ApiConversationHistory` accumulates all messages — user, assistant (with tool calls), tool results
- Each iteration appends to the growing array; the model sees the full conversation with proper role tags
- Tool results pushed as user messages with structured content

**Zed:**
- `messages: Vec<Arc<Message>>` accumulates `UserMessage` and `AgentMessage`
- Tool results appended to user message as `MessageContent::ToolResult`
- `AgentMessage::to_request()` converts to proper `Role::Assistant` messages
- The model always sees proper conversation structure with correct role tags

**Classification: NOISE — causes a role-inversion bug**

This is the most significant structural issue in hKask's turn loop. On iteration 2+, the model sees:

```
System: You are {agent}...
## Relevant Memory
{memory}

[Thread: previous turns from earlier conversations]

User: {the model's OWN response from iteration 1}

The following tool calls were executed:
{tool results}

Based on these results, provide your response.
```

The model's own output is labeled as "User:" — a role inversion. The `thread_messages` array (which has correct role tags) only contains turns from *before* this turn started, so it doesn't help. The current turn's iterations are not accumulated in the message array — they're flattened into a single "User:" string with tool results appended.

**Bug caused:** The model may become confused about who said what, potentially addressing its own output as if a user said it. This can cause:
- The model responding to itself ("You said X, but actually...")
- Loss of assistant/user turn structure within a single tool-use loop
- Inconsistent behavior vs. the multi-turn `thread_messages` path (which correctly preserves roles)

Cline and Zed both accumulate messages with proper roles across iterations. hKask should either:
1. Accumulate `ChatMessage`s within the loop (append assistant response + tool result as proper messages), OR
2. At minimum, label the continuation input as "Assistant:" not "User:" and structure it as a continuation, not a new user turn

The `thread_messages` infrastructure already exists and does the right thing — it's just not used for intra-turn accumulation.

---

### 8. Context Management (Condensation, Compaction, Context Window)

**hKask:** (`service.rs:707-722`, `crates/hkask-services-chat/src/chat/condenser.rs`, `threads.rs:193-214`)
- **Auto-condense:** if `auto_condense` and `approx_token_count(input) > context_window * 0.875` → `condense_history()`
- Two-phase: CPU pre-compress (`CondenserEngine::Profile::Heavy`) → LLM summarize via `generate_with_model()`
- Saliency window: most recent N exchanges preserved verbatim; older summarized
- **Thread storage cap:** `MAX_THREAD_TURNS` — prunes oldest with visible `[Context Compacted]` marker
- Both pressure threshold (87.5%) and saliency window are configurable

**Cline:**
- `ContextManager` class — manages context window
- Sliding window approach with file context tracking
- `clineIgnoreController` for excluding paths
- No LLM-based summarization in the SDK (relies on truncation)

**Zed:**
- `ContextCompaction` / `ContextCompactionUpdate` events — explicit compaction flow
- `pending_compaction_telemetry` tracking
- Summarization model (`summarization_model`) separate from chat model
- `CompactionInfo` stored as a `Message::Compaction` variant in the message history
- Token usage ratio tracking (`TokenUsageRatio`)

**Classification: ESSENTIAL (with one NOISE element)**

**ESSENTIAL:** The two-phase condensation (CPU pre-compress → LLM summarize), configurable pressure threshold, and saliency window are sophisticated context management that serves hKask's long-running agent sessions with episodic memory. The visible compaction marker in thread storage (preserving traceability) is a nice touch that Zed's `Message::Compaction` variant also does.

**NOISE:** Both `thread_history` (flattened string) AND `thread_messages` (typed array) exist in `TurnRequest` (`types.rs:176-181`). The string is prepended to input in `execute_turn` (line 702-705), and the typed array is used in `prepare_chat` (line 252-264). Having both is redundant — the typed array supersedes the string. The string path causes the role-inversion bug in step 7 (it flattens history into "User: ..." text). **The `thread_history` string field should be removed**; only `thread_messages` should be used, as it preserves role tags. This is dead-weight complexity that causes bugs.

---

### 9. State Persistence

**hKask:** (`threads.rs:169-218`, `service.rs:386-425`)
- `ThreadRegistry::append_turn()` → `write_thread_file()` → JSON file per thread in `threads/` directory
- Only appended on success (when `final_response` is present, `turn.rs:310-312`)
- `mark_seeded()` only if `!inference_error` (`turn.rs:313-315`)
- Episodic storage via `MemoryService::store_episodic()` — sovereignty-gated (P2 consent)
- `write_active_file()` tracks active thread

**Cline:**
- SDK `Agent` is stateless — no disk persistence; `snapshot()` gives in-memory state
- `ClineCore` harness handles session storage
- `restore(messages)` for external persistence

**Zed:**
- `ThreadMetadataStore` — SQLite-backed (`sqlez`) for thread titles, timestamps, worktree paths
- `DraftPromptStore` — persists unsent messages
- `DbThread` / `DbLanguageModel` — database entities
- Messages stored as `Vec<Arc<Message>>` (serializable via `Serialize/Deserialize`)

**Classification: JUSTIFIED IDIOMATIC RUST**

JSON file-per-thread is a simple, debuggable persistence model appropriate for hKask's single-user userpod architecture. SQLite (Zed) would be overkill for a local REPL; TypeScript in-memory (Cline SDK) doesn't match hKask's sovereign-memory requirement. The sovereignty-gated episodic storage (only store if consent given) is ESSENTIAL — it's P2 enforcement. The conditional append/mark_seeded logic (only on success) is correct error-awareness.

---

### 10. Error Handling

**hKask:** (`turn.rs:190-198, 277-293`)
- **Inference error:** `sink.status("Inference error: {e}")`, `gas_guard.release()`, `inference_error = true`, `break` — does NOT append turn, does NOT mark seeded
- **Tool error:** logged to sink (`✗ {tool} — {err}`), result is `Err`, included in `tool_results_vec` → fed back to model as "ERROR: {err}" in `format_tool_results`
- **Gas exhausted:** returns `TurnOutcome { budget_exhausted: true }` immediately
- **Max iterations:** warns and breaks with current response
- Timeout (120s) mapped to `ServiceUnavailable` (retryable)

**Cline:**
- **Inference error:** throws, caught by outer task handler
- **Tool error:** `formatResponse.toolError()` → pushed as tool result, fed back to model
- **Abort:** `taskState.abort` flag checked at multiple points in `ToolExecutor.handleCompleteBlock`
- `consecutiveMistakeCount` tracking with `maxConsecutiveMistakes` limit
- Repeated tool call detection (soft/hard escalation)

**Zed:**
- `CompletionError::MaxTokens` → `StopReason::MaxTokens`
- Tool errors returned as `Result<Output>` → wrapped in tool-result message
- `ToolCallAuthorization` for permission-denied tools
- Thread error state tracked in `ThreadView.thread_error`

**Classification: JUSTIFIED IDIOMATIC RUST (with one NOISE element)**

**JUSTIFIED:** The gas-aware error handling (release on error, don't settle) is essential to the cybernetic regulation model. The conditional `mark_seeded` (only on success) prevents seeding thread context from a failed turn. Tool errors fed back to the model (not thrown) is the correct agent-loop pattern — same as Cline and Zed.

**NOISE:** The inference error mapping at `service.rs:359-363` maps ALL non-timeout errors to `BadRequest` with `retryable: false`. A network error, provider 503, or rate limit would all be classified as non-retryable `BadRequest`. Cline distinguishes error types for retry logic. hKask should classify provider errors (5xx, rate limit) as `ServiceUnavailable` (retryable) and only map genuine 400-level errors to `BadRequest`. This causes unnecessary turn failure on transient errors.

---

### 11. Gas/Energy/Regulation (hKask Only)

**hKask:** (`turn.rs:156-168, 209-213`, `deps.rs:72-81, 192-254`)
- `GasGovernor::try_reserve(heuristic)` → `Option<GasReservation>` — if None, turn blocked
- After inference: `gas_guard.settle(actual_cost)` where `actual_cost = total_usage.gas_cost()` (1:1 token→gas)
- `gas_status()` returns (remaining, cap) for display
- `EnergyGuard` wraps `CyberneticsLoop` + `InferenceLoop` + WebID
- Regulation spans: `reg.chat.turn` started/completed, `reg.chat.request`/`response`, `reg.memory.episodic_stored`
- `on_reg_update` closure called at end of loop (refreshes TUI regulation display)
- Low gas warning at <20%, exhausted warning at 0%

**Cline:**
- No gas/energy model
- `consecutiveMistakeCount` with `maxConsecutiveMistakes` — a crude loop-limit
- Token cost tracking via `calculateApiCost()` for display only

**Zed:**
- `cumulative_token_usage` and `current_request_token_usage` — tracking only, no enforcement
- `TokenUsageRatio` for display
- No energy/gas governor, no cybernetic regulation

**Classification: ESSENTIAL**

This is hKask's defining feature — the cybernetic regulation loop. Gas reservation before inference (with heuristic), settlement after (with actual cost), and hard blocking when exhausted form a complete feedback loop (sense→orient→decide→act). The Regulation span persistence provides observability for the cybernetic loop. Neither Cline nor Zed has energy governance. This is P5 (simplicity through constraint) and P8 (evidence-backed) in action.

---

### 12. Tool Call Format (Native vs. Text-Based)

**hKask:**
- **Native function calling:** `StructuredToolCall { server, tool, args }` from `inference_port.generate_with_messages()` with `tools: Option<Vec<ChatToolDefinition>>` (`types.rs:79-84`)
- `finish_reason == "tool_calls"` signals structured tool calls
- **Text-based:** `<<tool:server/name {json} >>` directive format (referenced in nudge at `turn.rs:234`, but NOT parsed by `extract_tool_calls`)
- `ChatToolDefinition` is OpenAI-compatible

**Cline:**
- Two modes: `useNativeToolCalls` (boolean) — native function calling OR text-based XML parsing
- Text mode: parses `<tool_use>` XML blocks from response text
- Both modes fully supported with proper parsing

**Zed:**
- Native function calling only via `language_model::MessageContent::ToolUse`
- `AgentTool::Input` deserialized from JSON via `serde` + `JsonSchema`
- No text-based parsing

**Classification: NOISE — dead code path**

The `<<tool:...>>` text-based directive format is referenced in the nudge message but never parsed. This is dead complexity — a code path that exists in documentation/prompts but has no implementation. Either:
1. Remove all references to `<<tool:...>>` from the nudge and use native function calling exclusively (like Zed), OR
2. Implement a text-based fallback parser (like Cline's dual mode)

Currently it's the worst of both: the model is told to use a format that the system can't parse. See step 5 for the bug this causes.

---

### 13. Streaming

**hKask:** (`service.rs:459-577`, `crates/hkask-cli/src/commands/chat.rs:36-105`)
- **`chat_stream()`:** `async_stream` yielding `ChatStreamEvent::Token { text_delta, model }`, `Done { finish_reason, usage, memory_stored }`, `Error { message }`
- **CLI path (`finish_stream`):** prints `text_delta` directly to stdout, flushes per chunk, collects `full_text`, stores episodic at end
- **TUI path:** `ReplBridge::streaming_text(request_id)` polled each frame for partial text
- **Turn loop path (`run_turn_loop`):** uses `execute_turn` → `chat()` (NON-streaming) — `block_on` gets the full response, no streaming within the tool-use loop

**Cline:**
- Full streaming throughout: `api.createMessage()` streams chunks
- `presentAssistantMessage()` with `TaskPresentationScheduler` — cadence-based flush for local vs remote workspaces
- `handlePartialBlock` for UI updates during streaming, `handleCompleteBlock` for execution
- Streaming is integral to the loop, not a separate path

**Zed:**
- Full streaming via `stream_completion()` → event stream
- `ThreadEvent::AgentText(String)` for text deltas
- `ToolCallEventStream` for tool progress streaming
- `AgentText` and `AgentThinking` streamed separately
- Streaming is integral to the GPUI entity update model

**Classification: NOISE — streaming gap in the tool-use loop**

The turn loop (`run_turn_loop`) does NOT stream — it calls `execute_turn` → `chat()` which uses `generate_with_messages` (non-streaming, blocks until full response). The user sees nothing until the full response is returned. Only the CLI one-shot path (`chat_with_agent_streaming` → `finish_stream`) and the WSS handler (`chat_stream`) stream.

**Bug caused:** In the TUI/REPL, when the model is in a tool-use loop (iterations 2+), the user sees no streaming text — just a spinner until the full response arrives. For long responses or multiple tool iterations, this is a poor UX. Cline and Zed stream throughout the entire loop. The `chat_stream` function exists and works — it's just not wired into `run_turn_loop`.

This is a design gap, not a fundamental architecture problem. The `TurnSink` trait already has `agent_text` for incremental output; wiring `chat_stream` into the loop would require making `run_turn_loop` async (or using `block_on` on a stream consumer).

---

### 14. TUI vs CLI (Display Layer Connection)

**hKask:** (`tui/repl_bridge.rs`, `turn.rs:27-74`)
- **`TurnSink` trait:** `agent_text(agent, text)`, `tool_log(msg)`, `status(msg)` — abstracts output
- **`StdoutSink`:** prints directly to stdout (CLI path)
- **`CaptureSink`:** captures into `response_text` and `tool_output` strings (TUI path)
- **`ReplBridge` trait:** `start_inference(input) → InferenceRequestId`, `poll_inference(id) → InferenceState`, `streaming_text(id) → String`
- **`SystemBridge`:** read-only monitoring (gas, regulation, context pressure, pod counts)
- **`SettingsBridge`:** model/settings mutation (`set_model`, `list_models`, `set_setting`)
- **`SessionBridge`:** agent switching, history display
- TUI cannot depend on `hkask-cli` (dependency direction); traits bridge the gap
- Both TUI and CLI share `run_turn_loop` core via `TurnSink`

**Cline:**
- VS Code extension — WebView UI communicates via message passing
- `say()` and `ask()` callbacks for UI interaction
- No shared trait abstraction — ToolExecutor receives callbacks directly
- UI and core are tightly coupled via TypeScript's structural typing

**Zed:**
- `AgentPanel` → `ConversationView` → `ThreadView` — GPUI entity hierarchy
- `ThreadEvent` enum emitted by `Thread`, consumed by `ThreadView` via `cx.subscribe_in`
- `AgentPanel` serializes state to `KeyValueStore` for persistence
- UI and core share the `Thread` entity — no trait boundary, direct entity access
- `ThreadEnvironment` trait for terminal/subagent creation

**Classification: ESSENTIAL (trait design) + JUSTIFIED IDIOMATIC RUST**

**ESSENTIAL:** The four-trait split (`SystemBridge`, `ReplBridge`, `SettingsBridge`, `SessionBridge`) with ≤7 methods each is deep-module discipline (P7) — the TUI gets exactly the capabilities it needs, no more. The `TurnSink` abstraction enabling shared CLI/TUI core is clean separation. The `InferenceRequestId` polling pattern prevents the TUI event loop from blocking.

**JUSTIFIED:** Cline and Zed have tighter UI/core coupling (TypeScript structural typing, GPUI entity access). hKask's trait boundaries are correct Rust — the TUI crate cannot depend on `hkask-cli`, so trait interfaces are mandatory. The `CaptureSink` pattern (collect into strings, return `TurnCapture`) is a pragmatic bridge between sync `run_turn_loop` and async TUI rendering.

---

### 15. Dependency Injection (Traits, Mocks, Testability)

**hKask:** (`deps.rs`, `turn.rs:465-902`)
- **`TurnDeps`:** `executor: &dyn TurnExecutor`, `gas: &dyn GasGovernor`, `tools: &dyn ToolPort`, `threads: &mut dyn ThreadMemory`, `on_reg_update: &dyn Fn()`
- **`TurnInput`:** primitives only (strings, numbers, `Option`) — no `Arc<dyn Port>` types
- **`TurnConfig`:** loop configuration (max_loops, gas_heuristic, saliency_window, WebIDs, a2a_secret)
- **Production adapters:** `ReplTurnExecutor`, `ReplGasGovernor`, `ReplThreadMemory` — wrap real infrastructure
- **Test mocks:** `MockExecutor` (predetermined responses), `MockGas` (configurable reserve/cap), `MockTools` (returning results), `MockThreads` (seeded state tracking), `MockSink` (line capture)
- 13 test functions covering: compaction thresholds, status emission, gas warnings, loop display, error handling, seeding, max iterations

**Cline:**
- `ToolExecutor` receives ~25+ constructor parameters (commandExecutor, callbacks, managers, etc.)
- No trait/interface boundary — concrete class with direct dependencies
- Testing requires mocking the entire VS Code extension context
- SDK `Agent` class takes `config` object with tools, hooks, policies — more testable than core

**Zed:**
- `ThreadEnvironment` trait for terminal/subagent creation — one trait boundary
- `AgentTool` trait for tools — type-safe via associated types
- GPUI `Entity<T>` system — testing via `cx.new()` in test support mode
- `#[cfg(any(test, feature = "test-support"))]` methods for tool removal
- No mock executor or mock gas — tests use real `Thread` with test-support features

**Classification: ESSENTIAL**

The `TurnInput` design (primitives only, no port types) is brilliant — it keeps the test layer completely free of `Arc<dyn InferencePort>` and `Arc<dyn EpisodicStoragePort>`. Tests construct `TurnInput` from strings and numbers, mock the four traits, and verify loop behavior. The `TurnDeps` struct with 4 traits + 1 closure is minimal (≤7 fields, P7). The mock implementations are complete and enable testing the loop's gas, error, seeding, and iteration logic without any real infrastructure.

Cline's 25+ parameter `ToolExecutor` constructor is the anti-pattern hKask avoids. Zed's test-support feature flag is a different approach (test with real entities) that works for GPUI but doesn't match hKask's trait-based DI. hKask's approach is the most testable of the three.

---

## Summary Table

| Step | hKask | Cline | Zed | Classification |
|---|---|---|---|---|
| 1. Turn initiation | Sync loop + `block_on` | Async loop | GPUI Task | **JUSTIFIED** |
| 2. Pre-inference | WebID + OCAP + consent-gated memory | Stateless | @mentions context | **ESSENTIAL** |
| 3. Message array | `[system, ...thread_msgs, user]` | Accumulating history | Accumulating `Vec<Arc<Message>>` | **JUSTIFIED** |
| 4. Inference call | Timeout + Regulation spans + fusion bypass | Provider streaming | `stream_completion` | **ESSENTIAL** |
| 5. Response parsing | Structured only, nudge contradicts | Native or text XML | Native only | **NOISE** (nudge/parser mismatch) |
| 6. Tool execution | OCAP `DelegationToken` + `ToolPort` | AutoApprove + hooks | Permission + sandbox | **ESSENTIAL** |
| 7. Loop iteration | `current_input = response` (role inversion) | Accumulate with roles | Accumulate with roles | **NOISE** (role-inversion bug) |
| 8. Context mgmt | Two-phase condense + thread cap | Truncation | Compaction events | **ESSENTIAL** + **NOISE** (dual `thread_history`/`thread_messages`) |
| 9. State persistence | JSON files + sovereignty-gated episodic | Stateless SDK | SQLite + serializable msgs | **JUSTIFIED** |
| 10. Error handling | Gas-aware + conditional seed | Mistake count + abort | CompletionError → StopReason | **JUSTIFIED** + **NOISE** (over-broad `BadRequest`) |
| 11. Gas/regulation | `GasGovernor` + `EnergyGuard` + spans | None | None | **ESSENTIAL** |
| 12. Tool call format | Native + dead `<<tool:>>` text path | Native or text (both work) | Native only | **NOISE** (dead text path) |
| 13. Streaming | `chat_stream` exists but NOT in turn loop | Full streaming | Full streaming | **NOISE** (streaming gap in loop) |
| 14. TUI vs CLI | 4 traits ≤7 methods + `TurnSink` | VS Code WebView | GPUI entities | **ESSENTIAL** + **JUSTIFIED** |
| 15. Dependency injection | `TurnInput` primitives + 4 mockable traits | 25+ params, no boundary | `ThreadEnvironment` + test-support | **ESSENTIAL** |

---

## NOISE Summary — Actionable Issues

### N1: Nudge/Parser Mismatch (Steps 5, 12)
**File:** `crates/hkask-repl/src/turn.rs:232-236`  
**Bug:** The nudge tells the model to emit `<<tool:server/name {json} >>` text directives, but `extract_tool_calls` (line 972) only uses `structured_tool_calls` from native function calling. Text-based tool directives are silently dropped.  
**Fix:** Either remove the `<<tool:...>>` nudge and rely on native function calling exclusively, or implement a text-based fallback parser in `extract_tool_calls`.

### N2: Role Inversion in Loop Iteration (Step 7)
**File:** `crates/hkask-repl/src/turn.rs:297`, `crates/hkask-services-chat/src/chat/service.rs:231-239`  
**Bug:** `current_input = response` makes the model's own output the "input" for the next iteration, which `prepare_chat` labels as "User: {model_response}". The model sees its own output as a user message.  
**Fix:** Accumulate `ChatMessage`s within the loop (append `ChatMessage::assistant(response)` + `ChatMessage::user(tool_results)` to a growing array), and pass that array to `prepare_chat` instead of reusing the thread's old `thread_messages`. Alternatively, label continuation as "Assistant:" and structure as a continuation prompt.

### N3: Dual `thread_history` / `thread_messages` (Step 8)
**File:** `crates/hkask-services-chat/src/chat/types.rs:176-181`, `service.rs:702-705`  
**Bug:** Both a flattened string (`thread_history`) and typed array (`thread_messages`) exist in `TurnRequest`. The string is prepended to input (causing role flattening); the array is used for proper message construction. Having both is redundant and the string path causes the role-inversion in N2.  
**Fix:** Remove `thread_history: Option<String>` from `TurnRequest` and `TurnInput`. Use only `thread_messages: Option<Vec<ChatMessage>>` everywhere.

### N4: Over-Broad Error Classification (Step 10)
**File:** `crates/hkask-services-chat/src/chat/service.rs:359-363`  
**Bug:** All non-timeout inference errors are mapped to `BadRequest` with `retryable: false`. Transient errors (503, rate limit, network) are incorrectly classified as non-retryable.  
**Fix:** Inspect the error type/message and classify 5xx/rate-limit/network errors as `ServiceUnavailable` (retryable). Only map genuine 400-level errors to `BadRequest`.

### N5: Streaming Gap in Turn Loop (Step 13)
**File:** `crates/hkask-repl/src/turn.rs:189`  
**Bug:** `run_turn_loop` uses `execute_turn` → `chat()` (non-streaming). The user sees no incremental output during tool-use loops. `chat_stream()` exists but is not wired in.  
**Fix:** Either wire `chat_stream()` into the turn loop (requires consuming the stream via `block_on` on a `StreamExt::next` loop), or accept non-streaming for tool iterations and document the limitation. This is lower priority than N1–N4 since the loop typically has short responses between tool calls.

---

## What hKask Gets Right (Don't Touch)

1. **OCAP capability tokens for tool execution** — cryptographic delegation is unmatched by Cline/Zed
2. **Sovereignty-gated memory** — consent-gated recall and storage (P2) is a Magna Carta principle
3. **Gas governor with reserve/settle/release** — complete cybernetic feedback loop
4. **Regulation span persistence** — observability for the cybernetic loop (P8 evidence-backed)
5. **`TurnInput` primitives-only design** — best testability of the three systems
6. **Four-trait TUI bridge (≤7 methods each)** — deep-module discipline (P7)
7. **Two-phase context condensation** — CPU pre-compress + LLM summarize is sophisticated
8. **Multi-provider inference port resolution** — fusion bypass as cost-safety guard
9. **Conditional thread seeding** — only on success, prevents contamination from failed turns
10. **`TurnSink` abstraction** — clean CLI/TUI sharing via a 3-method trait