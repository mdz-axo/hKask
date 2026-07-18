---
title: "ADR-046: tool_calls Deserialization Fix"
audience: [developers]
last_updated: 2026-07-18
version: "0.31.0"
status: "Accepted"
domain: "Core"
mds_categories: [domain, composition]
---

# ADR-046: tool_calls Deserialization Fix

**Date:** 2026-07-18
**Status:** Accepted
**Supersedes:** None
**Superseded by:** None

## Context

The hKask REPL tool-use loop (`crates/hkask-repl/src/turn.rs:148-266`)
terminates when `extract_tool_calls` returns an empty `tool_calls` vector.
The loop calls `ChatService::execute_turn` → `InferenceRouter::generate_with_model`
→ `openai_compatible_generate` → `chat_response_to_result` to deserialize the
provider's JSON response into an `InferenceResult`.

The `InferenceResult` carries a `tool_calls: Vec<StructuredToolCall>` field.
When the model returns `finish_reason=tool_calls`, the provider includes a
`tool_calls` array in the response JSON. The deserialization must extract
this array and map it to `StructuredToolCall` entries.

**The bug:** `ChatChoice` had a `tool_calls` field, but
`ChatResponseMessage` did not. The OpenAI Chat Completions API spec puts
`tool_calls` at `choices[0].message.tool_calls`, not `choices[0].tool_calls`.
When an OpenAI-compliant provider (KiloCode, DeepInfra, Together AI,
OpenRouter) returned `tool_calls` on the `message` object, serde silently
ignored it (unknown field on `ChatResponseMessage`) and
`ChatChoice.tool_calls` deserialized as `None`.

This caused `chat_response_to_result` to produce an `InferenceResult` with
`finish_reason=tool_calls` but `tool_calls=Vec::new()`. The turn loop saw
zero tool calls and terminated after one iteration, producing a prose
preamble instead of executing tools.

## Decision

Move the `tool_calls` field from `ChatChoice` to `ChatResponseMessage`
(non-streaming responses) and from `StreamChoice` to `StreamDelta`
(streaming responses), matching the OpenAI Chat Completions API spec.

### Non-streaming (ChatChoice → ChatResponseMessage)

```rust
// Before (wrong):
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
    pub tool_calls: Option<Vec<RawToolCall>>,  // ← not per spec
}

// After (correct):
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
}

pub struct ChatResponseMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<RawToolCall>>,  // ← per OpenAI spec
}
```

### Streaming (StreamChoice → StreamDelta)

```rust
// Before (wrong):
pub struct StreamChoice {
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
    pub tool_calls: Option<Vec<RawToolCall>>,  // ← not per spec
}

// After (correct):
pub struct StreamChoice {
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

pub struct StreamDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<RawToolCall>>,  // ← per OpenAI spec
}
```

### Callers updated

- `chat_response_to_result`: reads `choice.message.tool_calls` instead of
  `choice.tool_calls`.
- `parse_sse_stream`: reads `choice.delta.tool_calls` instead of
  `choice.tool_calls`.

## Consequences

### Positive

- **Tool-use loop now works with all OpenAI-compliant providers.** The
  model's tool calls are correctly deserialized and passed to the turn loop.
- **Spec compliance.** The struct layout now matches the OpenAI Chat
  Completions API specification.
- **No breaking changes for callers.** `chat_response_to_result` and
  `parse_sse_stream` are the only consumers of these structs; both were
  updated.

### Negative

- **Providers that put `tool_calls` on `choice` (non-spec) will no longer
  work.** This is correct behavior — they were never spec-compliant. If
  such a provider exists, it should be reported as a provider bug.

### Neutral

- The `token_probs` field remains on `ChatChoice` (it is a choice-level
  metadata field, not a message-level field). This is correct per the
  OpenAI spec.

## Verification

1. `cargo test -p hkask-inference` — all tests pass.
2. Functional test: `kask chat` with a tool-dependent prompt ("Use the
   codegraph_structure tool to show me the project structure") produced
   2 iterations with a successful `codegraph_structure` tool call.
3. Inference log confirmed: `finish_reason=tool_calls` with
   `tool_calls_count > 0` (previously `tool_calls_count=0`).

## Cross-references

- [REPL Bootstrap Gap Post-Mortem](../../status/repl-bootstrap-gap-2026-07-18.md)
- [OpenAI Chat Completions API Spec](https://platform.openai.com/docs/api-reference/chat/create)
- `crates/hkask-inference/src/chat_protocol.rs` — the fix
- `crates/hkask-repl/src/turn.rs:148-266` — the turn loop that consumes tool calls
