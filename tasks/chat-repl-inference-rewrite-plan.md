# Chat/REPL/Inference Rewrite Plan — Phase 2

## Root Cause Diagnosis (Phase A)

### IS-2: "You responding to yourself" — CONFIRMED ROOT CAUSE

**Defect:** Inference echoes the user's own input back as a model response on
complex multi-turn queries.

**Root cause:** The entire inference pipeline is **string-based, not
message-array-based**. The `InferencePort` trait accepts `prompt: &str` — a
single flat string. The `build_chat_request` function always creates a
2-message array `[system, user]` from that string. Multi-turn conversation
history is flattened into a single `user` role message.

**Evidence chain:**

1. `InferencePort::generate_with_model(&str, ...)` — takes a single flat string
   (`hkask-types/src/ports/inference_port.rs:21`)

2. `build_chat_request(model, prompt: &str, ...)` creates `[system, user]` —
   always exactly 2 messages, never a multi-turn array
   (`hkask-inference/src/chat_protocol.rs:116-159`)

3. `ChatService::prepare_chat` composes `system_prompt + memory + "User: " + input`
   as one flat string (`hkask-services-chat/src/chat/service.rs:231-239`)

4. `ChatService::execute_turn` concatenates `base_input + thread_history +
   tool_results` into one flat string (`service.rs:676-707`)

5. `ThreadRegistry::thread_history` formats turns as
   `"User: ...\nAssistant: ..."` inside a single string
   (`hkask-repl/src/threads.rs:235-238`)

**Mechanism:** When multi-turn history is included, previous assistant
responses are embedded as plain text inside the `user` role message. The model
receives:
```json
{"role": "user", "content": "User: prev question\nAssistant: prev response\n\nUser: new question"}
```
The model sees assistant text labeled as user content → mirrors/echoes it.

### IS-1: "kask chat and kask tui do not work for complex queries"

Same root cause as IS-2. Complex multi-turn queries require proper message
arrays. The string-flattening approach breaks role semantics.

### IS-3: "Three prior passes have not produced working code"

Each prior pass patched the string-based architecture without replacing it.
The string-based `InferencePort` trait is the foundational defect — no amount
of prompt formatting fixes role confusion at the API layer.

## Cline/Zed Pattern Study (Phase B)

### Transferable Pattern 1: Typed Message Array (Cline + Zed)

Both Cline and Zed use a `Vec<Message>` where each `Message` has a typed
`Role` enum (`User`, `Assistant`, `System`). This array is sent directly to
the provider's `/v1/chat/completions` endpoint as the `messages` field.

hKask already has `ChatMessage { role: String, content: String }` in
`chat_protocol.rs` — but it's only used to construct `[system, user]`. The fix
is to build proper multi-turn arrays from thread history.

### Transferable Pattern 2: Tool-Use Loop (Cline)

Cline's agent loop: user message → LLM call → if tool_calls, execute tools,
append tool results as new `user` messages → repeat LLM call → until no more
tool_calls.

hKask's `run_turn_loop` (`hkask-repl/src/turn.rs:131-324`) already implements
this pattern — but it feeds the tool results back as a string appended to the
prompt, not as a proper `user` role message in the array.

### Transferable Pattern 3: Thread as Message Vector (Zed)

Zed's `Thread` holds `messages: Vec<Message>` directly. hKask's
`ChatThread` stores `turns: Vec<TurnEntry>` with proper roles — the storage is
correct, but the serialization to the inference layer flattens roles into text.

## Architecture (Phase C)

### Change: Add `generate_with_messages` to `InferencePort`

```rust
fn generate_with_messages(
    &self,
    messages: &[ChatMessage],
    parameters: &LLMParameters,
    model_override: Option<&str>,
    tools: Option<&[ChatToolDefinition]>,
) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
    // Default: flatten to string (backward compat for condenser/embedding)
    let prompt = messages.iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    self.generate_with_model(&prompt, parameters, model_override, tools)
}
```

Each backend overrides this to pass the message array directly to the API.

### Change: `build_chat_request` accepts `Vec<ChatMessage>`

New function `build_chat_request_messages(model, messages, params, stream, ...)`
that takes a pre-built message array instead of a single prompt string.

### Change: `ChatService` builds message arrays

`prepare_chat` and `execute_turn` construct `Vec<ChatMessage>` from:
- System prompt (role: "system")
- Thread history turns (role: "user"/"assistant" per TurnEntry)
- Current user input (role: "user")
- Tool results (role: "user" with tool result content)

### Change: `ThreadRegistry::thread_history_messages` returns `Vec<ChatMessage>`

New method that returns the thread as a message array instead of a flattened
string. The old `thread_history` method is kept for display purposes.

## Task Sequence

| Task | Size | Files | Acceptance |
|------|------|-------|------------|
| R1: Add `ChatMessage` to `hkask-types` public API | S | `hkask-types/src/lib.rs` | `ChatMessage` re-exported from hkask-types |
| R2: Add `generate_with_messages` to `InferencePort` | M | `inference_port.rs`, `chat_protocol.rs` | Trait method exists with default impl; existing tests pass |
| R3: Implement `generate_with_messages` in `InferenceRouter` | M | `inference_router/inference_port.rs` | Router delegates to backend message-array path |
| R4: Add `build_chat_request_messages` to `chat_protocol` | M | `chat_protocol.rs` | New function builds request from `Vec<ChatMessage>`; unit tests verify role assignments |
| R5: Update each backend to use message array | M | `openai_backend.rs`, `deepinfra_backend.rs`, `together_backend.rs`, `openrouter_backend.rs`, `ollama_backend.rs` | Backends pass messages array to API; existing tests pass |
| R6: Add `thread_history_messages` to `ThreadRegistry` | S | `hkask-repl/src/threads.rs` | Returns `Vec<ChatMessage>` with correct roles; unit tests verify |
| R7: Rewrite `ChatService::prepare_chat` for message arrays | M | `hkask-services-chat/src/chat/service.rs` | Builds `Vec<ChatMessage>` from system + memory + thread history + input; unit test verifies role assignments |
| R8: Rewrite `ChatService::chat` to use `generate_with_messages` | M | `service.rs` | Calls `generate_with_messages` with message array; no role echo in test |
| R9: Rewrite `ChatService::execute_turn` for message arrays | M | `service.rs` | Builds message array including tool results as user messages |
| R10: Update `chat.rs` CLI streaming path | S | `hkask-cli/src/commands/chat.rs` | Uses message-array path for streaming |
| R11: Update REPL turn loop for message arrays | M | `hkask-repl/src/turn.rs` | Tool results fed as user messages in array, not string concatenation |
| R12: Integration test — multi-turn no echo | M | `hkask-services-chat/src/chat/tests.rs` | 5-turn conversation: each response has correct role, no echo of user input |