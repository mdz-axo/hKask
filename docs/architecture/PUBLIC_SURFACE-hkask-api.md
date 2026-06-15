# Public Surface Justification — hkask-api

**Crate:** `hkask-api`  
**Public items in lib.rs:** 16  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-api` is the **HTTP API surface** — Axum routes for all hKask domains. Its surface is large because it exposes REST endpoints for every domain:

1. **Route modules** — Each domain (settings, pods, goals, wallet, chat, templates, etc.) has its own route module with 1–3 public functions (router constructor + handlers).
2. **ApiError** — The unified HTTP error type with 7 variants mapping to HTTP status codes.
3. **ServiceErrorResponse** — Newtype bridge for `ServiceError` → Axum `IntoResponse`.
4. **OpenAPI** — utoipa documentation generation for all endpoints.

## Mitigations

- **Per-domain routes:** Each route file (settings.rs, pods.rs, wallet.rs, etc.) has ≤3 public items.
- **Shared state:** `ApiState` consolidates all service dependencies into one extractor.
- **Error mapping is exhaustive:** Every `ServiceError` variant maps to a specific HTTP status.

## Deletion Test

Delete `hkask-api` and the REST API surface, OpenAPI documentation, and HTTP error mapping reappear in every MCP server that needs HTTP exposure. The crate earns its existence.
