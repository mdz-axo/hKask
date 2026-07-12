---
title: "WSS Chat Endpoint — WebSocket Streaming Chat for hKask"
audience: [architects, developers]
last_updated: 2026-07-02
version: "0.31.0"
status: "Implemented — Phase 1 complete (2026-07-02). Tools-through-streaming + WSS handler + MCP auto-discovery."
domain: "Communication"
mds_categories: [domain, composition, lifecycle]
anchored_on: [PRINCIPLES.md P1, P2, P3, P4, P5, P9, P12]
reviewed_via: [pragmatic-semantics, essentialist, deep-module, coding-guidelines]
---

# WSS Chat Endpoint — WebSocket Streaming Chat for hKask

**Purpose:** Define the design space and implementation plan for adding a `wss://` (WebSocket Secure) chat endpoint to hKask, enabling persistent, bidirectional, full-duplex streaming chat over TLS.

**Decision:** Three candidate approaches are presented. Approach B (Full-Pipeline WSS) is the recommended path. Approach A is a viable fast-path if latency to ship matters more than completeness. Approach C is deferred as over-engineering for the current scale.

---

## 1. Motivation

hKask currently has two chat surfaces:

| Surface | Transport | Streaming | Bidirectional | Memory Pipeline |
|---------|-----------|-----------|---------------|-----------------|
| TUI (`hkask-tui`) | In-process (ReplBridge) | ✅ Poll-based partial text | ✅ | ✅ Full |
| HTTP API (`POST /api/chat`) | HTTP/1.1 | ❌ Blocking | ❌ | ✅ Full |
| HTTP API (`POST /api/chat/stream`) | HTTP/1.1 + SSE | ✅ Server → Client only | ❌ | ❌ Raw inference only |
| Terminal (`GET /api/v1/terminal/ws`) | WSS | ✅ Bidirectional bytes | ✅ | ✅ (via `kask repl`) |

The terminal WebSocket proves the transport pattern works. The SSE endpoint proves streaming inference works. Neither combines the two with the full `ChatService` pipeline (memory recall, episodic storage, tool execution, manifest cascades).

A WSS chat endpoint would:
- Enable browser-based chat UIs with streaming token output (like the terminal, but for agent conversation)
- Allow persistent multi-turn sessions over a single connection (no HTTP request per turn)
- Support bidirectional control (send prompt, cancel generation, change model mid-session)
- Run the full `ChatService` pipeline with memory integration

---

## 2. Existing Infrastructure (Reusable)

| Infrastructure | What It Provides | Location |
|---------------|-----------------|----------|
| `axum::ws::WebSocketUpgrade` | WebSocket upgrade handshake, typed message frames | `axum` workspace dep (feature `ws`) |
| Session cookie auth (`extract_cookie`) | Validates `hkask_session` cookie before upgrade | `hkask-api/src/routes/terminal.rs:52-55` |
| `ChatService::prepare_chat()` | Agent lookup, prompt composition, semantic recall, memory merge, inference port resolution | `hkask-services-chat/src/chat/service.rs:48-195` |
| `InferencePort::generate_stream_with_model()` | Streaming token-by-token inference from any backend | `hkask-ports/src/inference_port.rs:61-74` |
| `InferenceRouter::dispatch_generate_stream()` | Routes streaming to DeepInfra, Together, OpenRouter, KiloCode | `hkask-inference/src/inference_router/dispatch.rs:79-206` |
| `ChatService::store_episodic()` | Post-turn episodic memory persistence with sovereignty gates | `hkask-services-chat/src/chat/service.rs:479-514` |
| CNS span emission (`CnsSpan::Chat`, `NuEvent`) | Observability for chat turns | `hkask-services-chat/src/chat/service.rs:241-290` |
| Auth middleware (`AuthService`, `AuthContext`) | Bearer token or session-based capability verification | `hkask-api/src/middleware/auth.rs` |
| `ApiState` + `AgentService` | Shared application state wired into axum | `hkask-api` |
| `tokio-tungstenite` / `tungstenite` | Already in `Cargo.lock` as transitive deps of axum | — |

**Key insight from the codebase** (from `ChatService::chat()` doc comment, line 204-205):

> For streaming, use `prepare_chat()` + `generate_stream_with_model()` directly on the inference port.

`prepare_chat()` was intentionally designed to be composable with streaming — it handles all the pre-inference work (agent lookup, memory recall, prompt composition) and returns the resolved inference port, ready for the caller to stream from.

---

## 3. Design Space — Three Approaches

### 3.1 Approach A: Minimal WSS (Raw Inference Stream)

Thin WebSocket wrapper around `prepare_chat()` → `generate_stream_with_model()`. No memory pipeline during stream, episodic storage fires after the stream completes. Basically the same logic as `POST /api/chat/stream` but over a persistent WebSocket instead of SSE.

**Protocol:**

```
Client → Server:  {"type": "prompt", "input": "...", "model": "..."}
Server → Client:  {"type": "token",   "text_delta": "Hel", "model": "..."}
Server → Client:  {"type": "token",   "text_delta": "lo",  "model": "..."}
Server → Client:  {"type": "done",    "finish_reason": "stop", "usage": {...}}
Client → Server:  {"type": "cancel"}   // mid-generation cancel
Client → Server:  {"type": "prompt", "input": "...", "model": "..."}   // next turn
```

**New code:**
- `hkask-api/src/routes/chat_ws.rs` — new route module (~120 lines)
- Protocol message types (Serialize/Deserialize enums) (~40 lines)
- Route registration in `hkask-api/src/routes/mod.rs` (~3 lines)
- `hkask-api/src/routes/chat.rs` — register router (~3 lines)

**Pros:**
- Smallest diff — reuses `prepare_chat()` and `generate_stream_with_model()` exactly as designed
- Proven pattern — the existing `chat_stream` SSE endpoint does the same thing, just swapping SSE for WS frames
- Bidirectional control (cancel, multi-turn) comes "for free" with WebSocket

**Cons:**
- No memory pipeline during stream (same limitation as `POST /api/chat/stream`)
- No tool execution during stream — inference happens, then the response is sent; tool calls are lost in the stream because `generate_stream_with_model()` doesn't expose tools
- Episodic storage is fire-and-forget after the stream (OK for most cases, but no streaming feedback about memory operations)

**Effort:** ~200 lines, 2-3 new files modified.

---

### 3.2 Approach B: Full-Pipeline WSS (Recommended)

Adds a `ChatService::chat_stream()` method that wraps the full pipeline around the streaming inference port. The WebSocket handler calls `prepare_chat()`, streams tokens from `generate_stream_with_model()`, then runs episodic storage + CNS events after the stream completes. This is the WSS equivalent of the non-streaming `ChatService::chat()` but with streaming token delivery.

**New `ChatService` method:**

```rust
/// Execute a streaming chat turn: full pipeline with token-by-token output.
///
/// Calls prepare_chat() for agent lookup + memory recall + prompt composition,
/// then streams tokens from generate_stream_with_model(), then stores the
/// exchange in episodic memory (with sovereignty gate) and emits CNS spans.
///
/// Returns a Stream of ChatStreamEvent — caller decides how to deliver
/// (WebSocket frames, SSE events, TUI updates).
pub async fn chat_stream(
    ctx: &AgentService,
    req: ChatTurnRequest,
) -> Result<impl Stream<Item = ChatStreamEvent>, ServiceError>
```

**`ChatStreamEvent` enum:**

```rust
pub enum ChatStreamEvent {
    Token { text_delta: String, model: String },
    ToolCall { tool_calls: Vec<StructuredToolCall> },
    Done { finish_reason: String, usage: Option<TokenUsage> },
    Error { message: String },
}
```

**Protocol (same as Approach A, but richer Done event):**

```
Server → Client:  {"type": "done", "finish_reason": "stop", "usage": {...},
                    "memory_stored": true, "cns_span_id": "..."}
```

**New code:**
- `hkask-services-chat/src/chat/service.rs` — `chat_stream()` method (~80 lines)
- `hkask-services-chat/src/chat/types.rs` — `ChatStreamEvent` enum (~20 lines)
- `hkask-api/src/routes/chat_ws.rs` — WebSocket handler (~150 lines)
- Protocol message types (~50 lines)
- Route registration (~6 lines)

**Pros:**
- Full pipeline parity with `POST /api/chat` — memory, tools, CNS, episodic storage
- Single source of truth — `prepare_chat()` is still the pre-inference workhorse
- The `ChatStreamEvent` enum is transport-agnostic — the same stream could power SSE, WSS, or a TUI update
- Sovereignty gates (memory consent checks) are enforced in the service layer, not duplicated in the transport

**Cons:**
- Heavier than Approach A (more code, more surface to test)
- Tool execution during streaming is tricky — tool calls appear mid-stream and may need to pause token output while tools execute
- Cancel semantics need care — cancel during tool execution vs. cancel during token streaming have different cleanup requirements

**Effort:** ~350 lines, 4-5 files modified/created.

---

### 3.3 Approach C: Transport Abstraction (Port Trait)

Extract a `ChatStreamPort` trait into `hkask-ports` that abstracts the streaming chat pipeline from the transport. Three implementations: WSS handler, SSE handler, and TUI bridge (replacing the current `ReplBridge` pattern). This follows the hexagonal architecture already used for `InferencePort`, `CnsObserver`, etc.

```
┌──────────────────────────────────────────────────┐
│                  ChatStreamPort                   │
│  stream_turn(input) → Stream<ChatStreamEvent>    │
│  cancel()                                         │
│  model_override(model)                            │
└──────────┬───────────────────┬───────────────────┘
           │                   │
    ┌──────▼──────┐   ┌───────▼────────┐   ┌──────▼──────┐
    │ WssHandler   │   │  SseHandler    │   │ TuiBridge    │
    │ (axum WS)   │   │  (axum SSE)    │   │ (in-process)  │
    └─────────────┘   └────────────────┘   └─────────────┘
```

**New code:**
- `hkask-ports/src/chat_stream.rs` — `ChatStreamPort` trait (~40 lines)
- `hkask-services-chat/src/chat/streaming.rs` — `ChatStreamService` impl (~100 lines)
- `hkask-api/src/routes/chat_ws.rs` — WSS handler as port consumer (~120 lines)
- `hkask-api/src/routes/chat.rs` — refactor SSE to use port (~30 lines delta)
- `hkask-tui/src/repl_bridge.rs` — refactor to use port (~40 lines delta)

**Pros:**
- Clean architecture — transport-agnostic domain logic
- Enables swapping transports without touching the chat pipeline
- Aligns with existing hexagonal pattern (`InferencePort`, `CnsObserver`, `ToolPort`)
- Future-proof — adding a new transport (e.g., Unix domain socket, gRPC stream) is one new handler

**Cons:**
- Over-engineering for the current problem — only 2-3 transports exist and one (SSE) is already implemented
- Significant refactor of existing code (TUI bridge, SSE endpoint)
- Port traits add indirection — `ChatStreamPort` would wrap `ChatService` which wraps `InferencePort` — three layers of abstraction for a single call chain
- The `deep-module` discipline asks: "delete the module — does complexity reappear?" If `ChatStreamPort` is deleted, each transport writes ~15 lines of direct `ChatService` calls. The port trait doesn't reduce complexity; it reorganizes it.

**Effort:** ~500 lines, 6-8 files modified/created.

---

## 4. Recommendation: Approach B (Full-Pipeline WSS)

**Rationale:**

1. **P5 (Essentialism):** Approach B adds exactly what's needed — a streaming variant of `ChatService::chat()` — and nothing more. Approach A leaves memory/tools on the table. Approach C adds a port abstraction layer that fails the deletion test.

2. **Composability:** `prepare_chat()` already exists as the pre-inference workhorse. `ChatStreamEvent` is a natural output type that any transport can consume. The design composes existing pieces rather than reinventing them.

3. **P9 (Homeostatic Self-Regulation):** CNS spans are emitted during the stream lifecycle (start, token chunks, completion, episodic storage), giving observability parity with the non-streaming path.

4. **Incremental delivery:** Approach B can be delivered as two phases:
   - **Phase 1:** `ChatService::chat_stream()` + `ChatStreamEvent` + `chat_ws` route (core path)
   - **Phase 2:** Tool call handling during stream, mid-stream cancel, model hot-swap (hardening)

---

## 5. Protocol Specification

### 5.1 WebSocket endpoint

```
GET /api/v1/chat/ws
```

- Upgrades to WebSocket (same auth model as `/api/v1/terminal/ws`)
- Requires valid `hkask_session` cookie or `Authorization: Bearer <token>` header
- Returns 401 if unauthenticated, 101 on upgrade

### 5.2 Message framing

All messages are JSON-encoded text frames. Binary frames are reserved for future use (e.g., image upload).

**Client → Server:**

| `type` | Fields | Description |
|--------|--------|-------------|
| `prompt` | `input: String`, `model?: String`, `template_id?: String` | Start a new chat turn |
| `cancel` | — | Cancel the current generation |
| `ping` | — | Keepalive (server responds with `pong`) |

**Server → Client:**

| `type` | Fields | Description |
|--------|--------|-------------|
| `token` | `text_delta: String`, `model: String` | Streaming token chunk |
| `tool_call` | `tool_calls: Vec<StructuredToolCall>` | Tool call detected mid-stream |
| `done` | `finish_reason: String`, `usage?: TokenUsage`, `memory_stored: bool` | Turn complete |
| `error` | `message: String`, `code?: String` | Error during generation |
| `pong` | — | Response to client ping |

### 5.3 Session lifecycle

```
Client                    Server
  │                          │
  │── GET /api/v1/chat/ws ──▶│  (upgrade + auth)
  │◀────── 101 Upgrade ──────│
  │                          │
  │── {"type":"prompt",...}─▶│
  │                          │── prepare_chat()
  │◀── {"type":"token",...}──│── stream tokens
  │◀── {"type":"token",...}──│
  │◀── {"type":"done",...}───│── store episodic + CNS
  │                          │
  │── {"type":"prompt",...}─▶│  (next turn, same connection)
  │           ...            │
  │                          │
  │────────── close ────────▶│  (or server timeout)
```

- Multi-turn: the connection stays open across turns. Each `prompt` starts a new generation.
- Concurrency: at most one generation in flight per connection. A `prompt` while generating sends back `{"type":"error","message":"generation in progress"}`.
- Timeout: idle connections close after 5 minutes (configurable). Active generations time out after 120s (same as `ChatService::chat()`).

---

## 6. Implementation Sequence

### Phase 1 — Core Path (Recommended first deliverable)

| Step | What | Crate | Lines |
|------|------|-------|-------|
| 1 | Add `ChatStreamEvent` enum to types | `hkask-services-chat` | ~20 |
| 2 | Add `ChatService::chat_stream()` | `hkask-services-chat` | ~80 |
| 3 | Add protocol message types | `hkask-api` (new `ws_types.rs`) | ~50 |
| 4 | Add `chat_ws` route handler | `hkask-api` (new `routes/chat_ws.rs`) | ~150 |
| 5 | Register route in `chat_router()` | `hkask-api` | ~3 |
| 6 | Integration test (connect, send prompt, verify tokens, verify done) | `hkask-api/tests` | ~80 |

### Phase 2 — Hardening

| Step | What | Notes |
|------|------|-------|
| 7 | Handle tool calls during stream | Pause token output, execute tools, resume with tool results in context |
| 8 | Mid-stream cancel | Drop the inference future, send `{"type":"done","finish_reason":"cancelled"}` |
| 9 | Model hot-swap | `{"type":"config","model":"..."}` changes model for next turn |
| 10 | Connection telemetry | CNS spans for WS connect/disconnect, message counts, latency |

### Phase 3 — Polish (Deferrable)

| Step | What | Notes |
|------|------|-------|
| 11 | Binary frame support | Image/audio upload for multimodal chat |
| 12 | Session resumption | Reconnect and resume an interrupted turn |
| 13 | Rate limiting | Per-user connection and message rate limits |

---

## 7. Existing Infrastructure Reused

| Infrastructure | Used For | Crate |
|---------------|----------|-------|
| `axum::ws::WebSocketUpgrade` + `ws::WebSocket` | WebSocket upgrade and framed I/O | `axum` (workspace dep, feature `ws`) |
| Session cookie extraction (`extract_cookie`) | Auth before upgrade | `hkask-api` |
| `ApiState` + `AgentService` | Shared state (inference port, storage, config) | `hkask-api` |
| `ChatService::prepare_chat()` | Agent lookup, prompt composition, memory recall | `hkask-services-chat` |
| `InferencePort::generate_stream_with_model()` | Streaming token output | `hkask-ports` |
| `InferenceRouter::dispatch_generate_stream()` | Backend-specific streaming dispatch | `hkask-inference` |
| `ChatService::store_episodic()` | Post-turn memory persistence | `hkask-services-chat` |
| `MemoryService::has_memory_consent()` | Sovereignty gate (P2) | `hkask-services-chat` / `hkask-memory` |
| `CnsSpan::Chat` + `NuEvent` | Turn observability | `hkask-types`, `hkask-cns` |
| `LLMParameters` | Inference parameter struct | `hkask-types` |

---

## 8. What Is NOT Being Built

Explicit exclusions — considered and rejected:

- **No new port trait (`ChatStreamPort`).** Deferred to Approach C. The deletion test: if `ChatStreamPort` is removed, the WSS handler calls `ChatService::chat_stream()` directly — ~10 lines of glue code. The trait does not earn its existence at the current scale (2 transports).
- **No SSE refactor.** The existing `POST /api/chat/stream` endpoint continues to work as-is. It is not migrated to use `chat_stream()` in Phase 1 (avoids regression risk on a working endpoint).
- **No TUI bridge refactor.** The `ReplBridge` trait continues to work as-is. It is not migrated to use `ChatStreamEvent` in Phase 1. The TUI's polling model (`poll_inference()`) is fundamentally different from push-based streaming and warrants separate design.
- **No MCP tool streaming over WSS.** The MCP tool surface remains HTTP-only for now. Streaming tool execution over WebSocket is a separate design problem.
- **No client SDK.** The WSS endpoint is documented via OpenAPI/utoipa and intended for direct consumption by browser clients. No hKask-provided JavaScript/TypeScript SDK.
- **No gRPC streaming.** WebSocket is the right transport for browser-first clients. gRPC-web streaming adds complexity (envoy proxy requirement) without benefit over WSS.
- **No connection multiplexing.** One generation per connection. Multi-generation concurrency (e.g., side-channel tool calls while streaming) is out of scope.

---

## 9. CNS Span Additions

| Span | Namespace | Phase | Description |
|------|-----------|-------|-------------|
| `ws_chat_connect` | `Chat` | Act | WebSocket upgraded, session validated |
| `ws_chat_disconnect` | `Chat` | Act | Connection closed (normal or error) |
| `ws_chat_turn_start` | `Chat` | Act | `prompt` message received, `prepare_chat()` begun |
| `ws_chat_token` | `Chat` | Observe | Token chunk emitted (sampled: 1 span per N chunks to avoid noise) |
| `ws_chat_turn_done` | `Chat` | Act | Turn complete, episodic stored |
| `ws_chat_cancel` | `Chat` | Act | Client-requested cancellation |

---

## 10. Open Questions

1. **Tool call handling during stream.** When the model emits a tool call mid-stream, should the server:
   - (a) Pause streaming, execute the tool, resume streaming with tool results in context? (Full agent loop)
   - (b) Emit the tool call as a `tool_call` event and continue streaming? (Client decides)
   - (c) Buffer tool calls and only emit them in the `done` event? (Simplest, but loses interactivity)
   
   *Recommendation: (a) for Phase 2. (c) for Phase 1 — emit tool calls in the `done` event, matching the non-streaming `chat()` behavior.*

2. **Episodic storage timing.** Should the exchange be stored to episodic memory:
   - Before the first token is streamed? (Safer — data is persisted even if the client disconnects mid-stream)
   - After the stream completes? (Current plan — matches `chat()` behavior, but loses data on disconnect)
   
   *Recommendation: Store after stream completes (current plan). A mid-stream disconnect is a partial turn — storing a partial response would be semantically wrong. Add a `memory_stored: false` flag on disconnect to signal the gap.*

3. **Auth model — cookie vs. bearer token.** The terminal WS uses `hkask_session` cookie. The existing `/api/chat` uses `Authorization: Bearer` header. Which should the WSS endpoint use?
   
   *Recommendation: Support both. Check `Authorization` header first (for API clients), fall back to `hkask_session` cookie (for browser clients). This matches the terminal's cookie approach while enabling programmatic access.*

4. **Should the SSE endpoint be migrated to use `chat_stream()`?**
   
   *Recommendation: Yes, but in a follow-up. It reduces duplication and adds memory/tools support to the SSE path. Not in Phase 1 to avoid regression risk.*

---

## 11. Success Criteria

- [ ] `GET /api/v1/chat/ws` accepts WebSocket upgrades with valid auth
- [ ] Rejects unauthenticated connections with 401 (fail-closed per P4)
- [ ] Client can send `{"type":"prompt","input":"..."}` and receive streaming `{"type":"token",...}` events
- [ ] Turn completes with `{"type":"done","finish_reason":"stop","usage":{...},"memory_stored":true}`
- [ ] Memory recall (semantic + episodic) runs before inference (verified via CNS spans)
- [ ] Episodic storage runs after stream completes, with sovereignty gate (P2)
- [ ] CNS spans emitted for connect, turn start, turn done, disconnect
- [ ] Integration test: full turn over WSS, verify token chunks, done event, and episodic storage
- [ ] No regression in existing `POST /api/chat` or `POST /api/chat/stream` endpoints (existing tests pass)
- [ ] OpenAPI schema documents the WSS endpoint (`utoipa` path annotation)

---

## 12. Related Documents

- `docs/plans/deployment-and-backup.md` — Terminal WebSocket implementation (auth model, session cookie pattern)
- `crates/hkask-api/src/routes/chat.rs` — Existing chat and SSE streaming endpoints
- `crates/hkask-api/src/routes/terminal.rs` — WebSocket upgrade pattern to follow
- `crates/hkask-services-chat/src/chat/service.rs` — `prepare_chat()` and `chat()` — pre-inference pipeline
- `crates/hkask-ports/src/inference_port.rs` — `InferencePort` trait with `generate_stream()`
- `crates/hkask-inference/src/inference_router/dispatch.rs` — `dispatch_generate_stream()` backend routing
