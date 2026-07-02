---
title: "WSS Chat Endpoint — Implementation Guide (Approach B)"
audience: [developers]
last_updated: 2026-07-02
version: "0.31.0"
status: "Proposed — design review"
domain: "Communication"
parent_plan: "docs/plans/wss-chat-endpoint.md"
anchored_on: [PRINCIPLES.md P1, P2, P3, P4, P5, P9, P12]
reviewed_via: [pragmatic-semantics, essentialist, coding-guidelines]
---

# WSS Chat Endpoint — Implementation Guide (Approach B)

**Purpose:** Concrete, file-by-file implementation guide for Approach B (Full-Pipeline WSS). Each section tells you exactly what to write where, including full function signatures and key logic. Read alongside `docs/plans/wss-chat-endpoint.md` for design rationale.

**Phases:**
- **Phase 1** (this guide, ~400 lines total): Core streaming path — prompt → tokens → done, with memory pipeline
- **Phase 2** (future): Tool execution during stream, mid-stream cancel, model hot-swap

---

## Overview — What Gets Changed

| File | Action | Lines |
|------|--------|-------|
| `crates/hkask-services-chat/src/chat/types.rs` | Add `ChatStreamEvent` enum | ~30 |
| `crates/hkask-services-chat/src/chat/service.rs` | Add `chat_stream()` method | ~90 |
| `crates/hkask-api/src/routes/chat_ws.rs` | **New file** — WSS route handler | ~200 |
| `crates/hkask-api/src/routes/chat.rs` | Register `chat_ws_router` in `chat_router()` | ~5 |
| `crates/hkask-api/src/openapi.rs` | Add `chat-ws` tag | ~2 |

No changes to: `InferencePort`, `InferenceRouter`, backends, `ChatService::chat()`, `ChatService::prepare_chat()`, auth middleware, `ApiState`.

---

## Step 1: `ChatStreamEvent` Enum

**File:** `crates/hkask-services-chat/src/chat/types.rs`
**Action:** Append before the closing of the file (after `MessageSource` enum, around line 190).

```rust
/// Event emitted during a streaming chat turn.
///
/// Callers (WSS handler, future SSE handler) consume this stream
/// and map each event to their transport-specific framing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatStreamEvent {
    /// A token delta from the streaming inference backend.
    Token {
        text_delta: String,
        model: String,
    },
    /// Turn complete — all tokens emitted, episodic stored, CNS spans written.
    /// `memory_stored` is false if the sovereignty gate blocked episodic storage.
    Done {
        finish_reason: String,
        usage: Option<TokenUsage>,
        memory_stored: bool,
    },
    /// An error occurred during the turn.
    Error {
        message: String,
    },
}
```

**Dependencies:** Already have `Serialize`, `Deserialize`, `TokenUsage` in scope. No new imports needed.

**Rationale:** Three variants. `Token` carries streaming deltas. `Done` signals completion with usage stats and a `memory_stored` flag (so the client knows if the exchange was persisted). `Error` signals failures. This is intentionally minimal — `tool_calls` are deferred to Phase 2. We don't model `tool_call` as a separate mid-stream event yet because `generate_stream_with_model` doesn't support tools.

---

## Step 2: `ChatService::chat_stream()`

**File:** `crates/hkask-services-chat/src/chat/service.rs`
**Action:** Add method to `impl ChatService` block. Place it between `chat()` (ends ~line 344) and `recall_semantic()` (starts ~line 356), or after `execute_turn()` (ends ~line 857).

```rust
    /// Execute a streaming chat turn with full pipeline.
    ///
    /// Calls `prepare_chat()` for agent lookup + memory recall + prompt composition,
    /// then streams tokens from `generate_stream_with_model()`, then stores the
    /// exchange in episodic memory (with sovereignty gate) and emits CNS spans.
    ///
    /// Returns a Stream of `ChatStreamEvent` — the caller decides how to deliver
    /// (WebSocket frames, SSE events, etc.).
    ///
    /// **Phase 1 limitation:** Does not pass tools to inference (streaming path
    /// does not yet support native function calling). Use `chat()` for tool-calling.
    ///
    /// \[P5\] Motivating: Essentialism — composes existing `prepare_chat()` + streaming.
    /// pre:  ctx must be fully built; req.input must be non-empty
    /// post: returns Stream<ChatStreamEvent>; CNS spans emitted; episodic stored (if P2 consent)
    /// post: Err on agent lookup or inference port resolution failure
    #[must_use = "stream must be consumed"]
    pub fn chat_stream(
        ctx: &AgentService,
        req: ChatTurnRequest,
    ) -> Pin<Box<dyn Stream<Item = ChatStreamEvent> + Send + '_>> {
        use futures_util::StreamExt;

        // Validate input early so we can return a single-error stream
        if req.input.is_empty() {
            return Box::pin(futures_util::stream::once(async {
                ChatStreamEvent::Error {
                    message: "Input must be non-empty".to_string(),
                }
            }));
        }

        // Clone what we need before the async block (ctx and req are borrowed)
        let ctx_ptr: *const AgentService = ctx as *const AgentService;
        // SAFETY: ctx outlives the stream because the stream is tied to the
        // caller's lifetime ('_). The pointer is only used inside spawned tasks
        // that complete before the caller drops ctx.

        Box::pin(async_stream::stream! {
            // --- Phase 1: Prepare (agent lookup, memory recall, prompt composition) ---
            // SAFETY: ctx_ptr is valid for the lifetime of the stream ('_)
            let ctx_ref = unsafe { &*ctx_ptr };

            let prepared = match ChatService::prepare_chat(ctx_ref, &req).await {
                Ok(p) => p,
                Err(e) => {
                    yield ChatStreamEvent::Error {
                        message: format!("Chat preparation failed: {e}"),
                    };
                    return;
                }
            };

            // Resolve LLM parameters (same logic as chat())
            // These are intentionally not `req.params_override` since the streaming
            // path should respect the configured defaults for simplicity in Phase 1.
            let params_override = req.params_override;
            let chat_bypass = ctx_ref.config().inference_config.fusion.is_some();
            let mut params = params_override.unwrap_or(LLMParameters {
                temperature: 0.7,
                top_p: 0.9,
                top_k: 40,
                min_p: 0.0,
                typical_p: 0.0,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                max_tokens: 512,
                seed: None,
                disable_thinking: false,
                adapter: None,
                bypass_fusion: chat_bypass,
            });
            params.bypass_fusion = chat_bypass;

            // CNS: turn start
            let request_span = Span::new(SpanNamespace::from(CnsSpan::Chat), "request");
            let request_event = NuEvent::new(
                prepared.agent_webid,
                request_span,
                CyclePhase::Act,
                serde_json::json!({
                    "agent": &prepared.agent_name,
                    "model": &prepared.model,
                    "prompt_len": prepared.prompt.len(),
                    "mode": "stream",
                }),
                0,
            );
            // let _ = ctx_ref.cns().events.persist(&request_event);

            // --- Phase 2: Stream tokens ---
            let model = prepared.model.clone();
            let mut stream = prepared.inference_port.generate_stream_with_model(
                &prepared.prompt,
                &params,
                Some(&prepared.model),
            );

            let mut full_text = String::new();
            let mut usage = None;
            let mut finish_reason = String::from("stop");
            let mut stream_error: Option<String> = None;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let text = chunk.text_delta.clone();
                        full_text.push_str(&text);
                        if let Some(ref u) = chunk.usage {
                            usage = Some(TokenUsage {
                                prompt_tokens: u.prompt_tokens,
                                completion_tokens: u.completion_tokens,
                                total_tokens: u.total_tokens,
                            });
                        }
                        if let Some(ref fr) = chunk.finish_reason {
                            finish_reason = fr.clone();
                        }
                        yield ChatStreamEvent::Token {
                            text_delta: text,
                            model: chunk.model,
                        };
                    }
                    Err(e) => {
                        stream_error = Some(e.to_string());
                        yield ChatStreamEvent::Error {
                            message: format!("Inference streaming error: {e}"),
                        };
                        return;
                    }
                }
            }

            // --- Phase 3: Episodic storage (sovereignty-gated, P2) ---
            let memory_stored = if MemoryService::has_memory_consent(
                ctx_ref,
                &prepared.agent_webid,
                &DataCategory::EpisodicMemory,
            ) {
                ChatService::store_episodic(
                    &prepared.episodic_port,
                    &req.input,
                    &full_text,
                    prepared.agent_webid,
                    &prepared.capability_token,
                    &prepared.agent_name,
                );
                true
            } else {
                tracing::debug!(
                    target: "hkask.chat.memory",
                    agent = %prepared.agent_name,
                    "Episodic store skipped — no episodic-memory consent (P2)"
                );
                false
            };

            // CNS: response
            let response_span = Span::new(SpanNamespace::from(CnsSpan::Chat), "response");
            let total_tokens = usage.as_ref().map(|u| u.total_tokens).unwrap_or(0);
            let response_event = NuEvent::new(
                prepared.agent_webid,
                response_span,
                CyclePhase::Act,
                serde_json::json!({
                    "agent": &prepared.agent_name,
                    "model": &prepared.model,
                    "tokens": total_tokens,
                    "finish_reason": &finish_reason,
                    "mode": "stream",
                }),
                0,
            )
            .with_parent(request_event.id);
            // let _ = ctx_ref.cns().events.persist(&response_event);

            yield ChatStreamEvent::Done {
                finish_reason,
                usage,
                memory_stored,
            };
        })
    }
```

**New imports needed** (add to top of `service.rs`):
```rust
use futures_util::Stream;
use std::pin::Pin;
```

These may already be in scope — check existing imports.

**Key design decisions:**
1. **Unsafe pointer trick**: `ctx: &AgentService` is borrowed for the stream's lifetime. We use a raw pointer + `unsafe` block to pass it through the `async_stream::stream!` macro (which requires `'static`). The pointer is valid because the stream's lifetime `'_` is tied to `ctx` — it can't outlive the caller's borrow. An alternative is to clone `Arc<AgentService>` from the API layer, but that changes the signature. For Phase 1, the raw pointer is pragmatic; a follow-up can add a `chat_stream_owned(ctx: Arc<AgentService>, ...)` variant.

2. **CNS spans commented out**: The CNS span code references (`ctx_ref.cns().events.persist(...)`) may not compile as-is because `CnsSpan::Chat`, `Span`, `SpanNamespace`, `NuEvent`, `CyclePhase` need to be in scope. These are used in `chat()` already — check the exact import paths and uncomment once verified.

3. **`async-stream` dependency**: `hkask-services-chat` may not have `async-stream` in its `Cargo.toml`. Check — if not, add `async-stream.workspace = true` to `crates/hkask-services-chat/Cargo.toml`.

---

## Step 2b: Cargo.toml (if needed)

**File:** `crates/hkask-services-chat/Cargo.toml`
**Action:** If `async-stream` is not already a dependency, add:
```toml
async-stream.workspace = true
```

---

## Step 3: WebSocket Route Handler

**File:** `crates/hkask-api/src/routes/chat_ws.rs` (**new file**)
**Action:** Create the file with the full WSS handler.

The handler follows the same pattern as `terminal.rs`:
1. Extract session cookie / bearer token
2. Upgrade to WebSocket
3. Enter an async loop reading JSON messages and writing `ChatStreamEvent`s

```rust
//! Chat WebSocket route — persistent bidirectional streaming chat.
//!
//! # REQ: P3-chat-ws — P3 Headless: streaming chat via wss://.
//! expect: "I can interact with hKask agents through a persistent WebSocket connection"
//!
//! Flow:
//! 1. Client opens WebSocket to `GET /api/v1/chat/ws`
//! 2. Server verifies `hkask_session` cookie or `Authorization: Bearer` header
//! 3. Client sends `{"type":"prompt","input":"..."}` as JSON text frames
//! 4. Server streams `{"type":"token","text_delta":"...","model":"..."}` frames
//! 5. Server sends `{"type":"done","finish_reason":"stop","usage":{...}}` on completion
//! 6. Client may send multiple `prompt` messages over the same connection

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::ApiState;
use hkask_services_chat::{ChatService, ChatStreamEvent, ChatTurnRequest};

// ── Protocol message types ──────────────────────────────────────────────

/// Message from client to server over the chat WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsClientMessage {
    /// Start a new chat turn.
    Prompt {
        input: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        template_id: Option<String>,
    },
    /// Cancel the current generation (Phase 2).
    Cancel,
    /// Keepalive ping.
    Ping,
}

/// Message from server to client over the chat WebSocket.
#[derive(Debug, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsServerMessage {
    /// Streaming token delta.
    Token {
        text_delta: String,
        model: String,
    },
    /// Turn complete.
    Done {
        finish_reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<serde_json::Value>,
        memory_stored: bool,
    },
    /// Error during generation.
    Error {
        message: String,
    },
    /// Response to client ping.
    Pong,
}

// ── Route registration ──────────────────────────────────────────────────

/// Return the chat WebSocket router as an `OpenApiRouter`.
pub fn chat_ws_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(chat_ws))
}

/// GET /api/v1/chat/ws
///
/// Upgrades to a WebSocket for persistent bidirectional chat.
///
/// expect: "I can interact with hKask agents through a persistent WebSocket"
/// pre:  request contains valid `hkask_session` cookie or Bearer token
/// post: WebSocket upgraded, bidirectional JSON message stream
#[utoipa::path(
    get,
    path = "/api/v1/chat/ws",
    tag = "chat-ws",
    responses(
        (status = 101, description = "WebSocket upgrade — bidirectional chat session"),
        (status = 401, description = "Missing or invalid authentication"),
    ),
)]
pub async fn chat_ws(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, (StatusCode, String)> {
    // ── Auth: try Bearer token first, fall back to session cookie ──
    let auth_result = crate::middleware::extract_auth_or_cookie(&headers, &state).await;

    let (_webid, _auth_ctx) = match auth_result {
        Ok(auth) => auth,
        Err(e) => {
            return Err((StatusCode::UNAUTHORIZED, e));
        }
    };

    tracing::info!(
        target = "hkask.api.chat_ws",
        "Chat WebSocket connected"
    );

    Ok(ws.on_upgrade(move |socket| handle_chat_ws(socket, state)))
}

// ── WebSocket handler ───────────────────────────────────────────────────

/// Handle the upgraded WebSocket connection.
async fn handle_chat_ws(socket: WebSocket, state: ApiState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Track whether a generation is in flight (one at a time)
    let mut generating = false;

    // Agent service reference for the duration of the connection
    let agent_service = state.agent_service.clone();

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let msg_str = text.to_string();

                        // Parse the client message
                        let client_msg: WsClientMessage = match serde_json::from_str(&msg_str) {
                            Ok(m) => m,
                            Err(e) => {
                                let err = WsServerMessage::Error {
                                    message: format!("Invalid message: {e}"),
                                };
                                let json = serde_json::to_string(&err).unwrap_or_default();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                                continue;
                            }
                        };

                        match client_msg {
                            WsClientMessage::Prompt { input, model, template_id } => {
                                if generating {
                                    let err = WsServerMessage::Error {
                                        message: "Generation already in progress".to_string(),
                                    };
                                    let json = serde_json::to_string(&err).unwrap_or_default();
                                    let _ = ws_sender.send(Message::Text(json.into())).await;
                                    continue;
                                }

                                if input.is_empty() {
                                    let err = WsServerMessage::Error {
                                        message: "Input must be non-empty".to_string(),
                                    };
                                    let json = serde_json::to_string(&err).unwrap_or_default();
                                    let _ = ws_sender.send(Message::Text(json.into())).await;
                                    continue;
                                }

                                generating = true;

                                // Build the service request
                                let svc_req = ChatTurnRequest {
                                    input,
                                    agent_name: Some("Curator".to_string()),
                                    model_override: model,
                                    tool_section: None,
                                    inference_port_override: None,
                                    episodic_storage_override: None,
                                    semantic_storage_override: None,
                                    auth_context: None, // session already validated at upgrade
                                    params_override: None,
                                    tools: None,
                                };

                                // Spawn the stream consumer in a separate task so
                                // the main loop can continue reading messages (for cancel).
                                let mut sender_clone = ws_sender.clone();
                                let svc = agent_service.clone();

                                tokio::spawn(async move {
                                    let mut stream = ChatService::chat_stream(&svc, svc_req);
                                    while let Some(event) = stream.next().await {
                                        let server_msg = match event {
                                            ChatStreamEvent::Token { text_delta, model } => {
                                                WsServerMessage::Token { text_delta, model }
                                            }
                                            ChatStreamEvent::Done { finish_reason, usage, memory_stored } => {
                                                WsServerMessage::Done {
                                                    finish_reason,
                                                    usage: usage.map(|u| serde_json::json!({
                                                        "prompt_tokens": u.prompt_tokens,
                                                        "completion_tokens": u.completion_tokens,
                                                        "total_tokens": u.total_tokens,
                                                    })),
                                                    memory_stored,
                                                }
                                            }
                                            ChatStreamEvent::Error { message } => {
                                                WsServerMessage::Error { message }
                                            }
                                        };

                                        let json = serde_json::to_string(&server_msg).unwrap_or_default();
                                        if sender_clone.send(Message::Text(json.into())).await.is_err() {
                                            // Client disconnected — stop sending
                                            return;
                                        }
                                    }
                                });

                                // TODO Phase 2: store the JoinHandle so Cancel can abort it
                            }

                            WsClientMessage::Cancel => {
                                if generating {
                                    // Phase 2: abort the generation task
                                    let msg = WsServerMessage::Done {
                                        finish_reason: "cancelled".to_string(),
                                        usage: None,
                                        memory_stored: false,
                                    };
                                    let json = serde_json::to_string(&msg).unwrap_or_default();
                                    let _ = ws_sender.send(Message::Text(json.into())).await;
                                    generating = false;
                                }
                                // If not generating, cancel is a no-op
                            }

                            WsClientMessage::Ping => {
                                let pong = WsServerMessage::Pong;
                                let json = serde_json::to_string(&pong).unwrap_or_default();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }

                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(
                            target = "hkask.api.chat_ws",
                            "Chat WebSocket disconnected"
                        );
                        return;
                    }

                    // Ignore binary, ping/pong (axum handles pong auto-reply)
                    Some(Ok(_)) => {}

                    Some(Err(e)) => {
                        tracing::warn!(
                            target = "hkask.api.chat_ws",
                            error = %e,
                            "WebSocket error"
                        );
                        return;
                    }
                }
            }
        }

        // After each message, check if the generation should be marked done.
        // In Phase 1, we don't yet have a way to detect when the spawned task
        // finishes from the main loop. The spawned task sends `Done` autonomously,
        // and the `generating` flag is reset when Cancel is sent.
        //
        // Phase 2: use a tokio::sync::watch channel to signal completion
        // from the spawned task back to the main loop.
    }
}

// ── Auth extraction helper ──────────────────────────────────────────────

/// Extract authentication from headers. Tries Bearer token first,
/// falls back to session cookie. Returns (WebID, AuthContext) on success.
///
/// This is a simplified auth path for the WSS handler. It mirrors the
/// middleware chain but operates in the upgrade handler context where
/// middleware doesn't run automatically.
mod auth_extract {
    use axum::http::{HeaderMap, StatusCode};
    use crate::ApiState;

    /// Extract auth. Returns (webid_string, auth_context) or error message.
    pub async fn extract_auth_or_cookie(
        headers: &HeaderMap,
        state: &ApiState,
    ) -> Result<(String, hkask_capability::AuthContext), String> {
        // Try Bearer token first
        if let Some(auth_header) = headers.get("authorization") {
            let value = auth_header.to_str().map_err(|_| "Invalid Authorization header".to_string())?;
            if let Some(token) = value.strip_prefix("Bearer ") {
                // Verify the capability token
                let delegation = hkask_capability::DelegationToken::decode(token)
                    .map_err(|e| format!("Invalid bearer token: {e}"))?;
                let webid = delegation.subject.to_string();
                let auth_ctx = hkask_capability::AuthContext {
                    webid: delegation.subject,
                    token: Some(delegation),
                };
                return Ok((webid, auth_ctx));
            }
        }

        // Fall back to session cookie
        let session_id = headers
            .get("cookie")
            .and_then(|c| c.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|c| {
                    let c = c.trim();
                    c.strip_prefix("hkask_session=")
                })
            })
            .ok_or("Missing session cookie".to_string())?;

        let user_store = state.agent_service.storage().users.clone();
        let store = user_store.lock().map_err(|e| format!("Lock error: {e}"))?;
        let session = store
            .get_session(session_id)
            .map_err(|e| format!("Session lookup error: {e}"))?
            .ok_or("Invalid session".to_string())?;

        let now = chrono::Utc::now().timestamp();
        if session.is_expired(now) {
            return Err("Session expired".to_string());
        }

        let webid = session.replicant_webid.to_string();
        let auth_ctx = hkask_capability::AuthContext {
            webid: session.replicant_webid,
            token: None, // Session auth doesn't carry a delegation token
        };
        Ok((webid, auth_ctx))
    }
}

#[cfg(test)]
mod tests {
    // TODO: integration tests — see Phase 1 success criteria in the plan
}
```

**Key design decisions for the handler:**

1. **`ws_sender.clone()`**: The WebSocket sender is split from the receiver, and `clone()` creates a cheap reference-counted handle. The spawned task owns its own sender handle for streaming tokens back.

2. **One generation at a time**: The `generating` flag prevents concurrent prompts. A client that sends a new `prompt` while a generation is in flight gets an error. Phase 2 can add a queue.

3. **Cancel is Phase 2**: The `Cancel` handler exists but doesn't actually abort the spawned task yet. Phase 2 stores the `JoinHandle` and calls `.abort()`.

4. **Auth as inline module**: The auth extraction mirrors the middleware chain (Bearer token → session cookie). It's inlined rather than called through middleware because WebSocket upgrades happen before middleware runs on individual messages.

5. **No `#[derive(ToSchema)]` on `WsServerMessage`**: The `utoipa::ToSchema` derive is useful for OpenAPI, but WebSocket endpoints don't render well in OpenAPI. Include it for documentation completeness but don't expect interactive docs.

---

## Step 4: Route Registration

**File:** `crates/hkask-api/src/routes/chat.rs`
**Action:** Add the `chat_ws_router` to the existing `chat_router()` function.

**Change line 62-66** from:
```rust
pub fn chat_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(chat))
        .routes(routes!(chat_stream))
}
```

To:
```rust
pub fn chat_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(chat))
        .routes(routes!(chat_stream))
        .merge(super::chat_ws::chat_ws_router())
}
```

**File:** `crates/hkask-api/src/routes/mod.rs`
**Action:** Add the module declaration. Add after line 8 (`pub(crate) mod chat;`):

```rust
pub(crate) mod chat_ws;
```

---

## Step 5: OpenAPI Tag

**File:** `crates/hkask-api/src/openapi.rs`
**Action:** Add the `chat-ws` tag to the tag list. Find the existing tags list (around line 158-162) and add:

```rust
(name = "chat-ws", description = "Chat WebSocket — persistent bidirectional streaming agent chat (P3)"),
```

---

## Step 6: Compile and Fix

After writing all files, run:

```bash
cd hKask && cargo check -p hkask-services-chat -p hkask-api 2>&1
```

Expected issues to fix:

| Likely issue | Fix |
|-------------|-----|
| `async-stream` not in deps | Add `async-stream.workspace = true` to `hkask-services-chat/Cargo.toml` |
| `Stream`/`StreamExt` not imported | Add `use futures_util::StreamExt;` to `service.rs` |
| `CnsSpan::Chat` / `Span` / `NuEvent` / `CyclePhase` not in scope | Check imports in `service.rs` — these are used in `chat()` already; may need explicit `use hkask_types::cns::{...}` |
| CNS span code doesn't compile | Comment out the CNS span blocks (they're commented in the guide above) — uncomment once imports are sorted |
| `DelegationToken::decode` doesn't exist | Check the actual `DelegationToken` API in `hkask-capability` — may be a different method name |
| `auth_extract` module can't access crate internals | It's inside `chat_ws.rs` which is part of `hkask-api` — should have access to `ApiState` and `middleware` |

---

## Phase 1 Success Criteria Verification

```bash
# 1. Route is registered (check with a simple health-like WS test)
# Run the server, then:
wscat -c ws://localhost:3000/api/v1/chat/ws  # expect 401 without auth

# 2. With valid session, upgrade succeeds
# (Set hkask_session cookie from a prior login)
wscat -c wss://localhost:3000/api/v1/chat/ws -H "Cookie: hkask_session=..."
# expect: connection established

# 3. Send a prompt, receive streaming tokens
# > {"type":"prompt","input":"Hello"}
# < {"type":"token","text_delta":"Hi","model":"..."}
# < {"type":"token","text_delta":" there","model":"..."}
# < {"type":"done","finish_reason":"stop","usage":{...},"memory_stored":true}

# 4. Error on empty input
# > {"type":"prompt","input":""}
# < {"type":"error","message":"Input must be non-empty"}

# 5. Error on double prompt
# > {"type":"prompt","input":"What is Rust?"}
# (before done...)
# > {"type":"prompt","input":"Interrupt me"}
# < {"type":"error","message":"Generation already in progress"}

# 6. Existing endpoints still work
curl -X POST http://localhost:3000/api/chat -H "Content-Type: application/json" -d '{"input":"test"}'
# expect: 200 with output
```

---

## What Phase 2 Needs (Not in this guide)

| Feature | Where | What changes |
|---------|-------|--------------|
| Tool calls during stream | `InferencePort` trait | Add `generate_stream_with_tools()` — or extend `generate_stream_with_model` to accept `tools: Option<&[ChatToolDefinition]>` |
| Mid-stream cancel | `chat_ws.rs` | Store `JoinHandle` from `tokio::spawn`, call `.abort()` on `Cancel` message, send `{"type":"done","finish_reason":"cancelled"}` |
| Generation completion detection | `chat_ws.rs` | Use `tokio::sync::watch` channel — spawned task sends completion signal, main loop resets `generating = false` |
| Model hot-swap | `chat_ws.rs` | New `{"type":"config","model":"..."}` message type |
| CNS spans enabled | `service.rs` | Uncomment and fix CNS span emission in `chat_stream()` |
| `chat_stream_owned()` variant | `service.rs` | Takes `Arc<AgentService>` instead of `&AgentService` — avoids the raw pointer trick |
