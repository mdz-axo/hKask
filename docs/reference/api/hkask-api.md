---
title: "hkask-api — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-api` provides the HTTP API server with OpenAPI integration. Built on `axum` with middleware for CNS spans, session cookies, capability tokens, admin role-gating, and API key authentication. OpenAPI documentation is generated via `utoipa`.

## Public Modules

| Module | Purpose |
|---|---|
| `error` | `ApiError` — API error type |
| `middleware` | Auth middleware, session middleware, CNS middleware, admin middleware, and API key auth |
| `openapi` | `ApiDoc` — OpenAPI schema definition |
| `routes` | All route modules comprising the API surface |
| `git_cas` | Git CAS initialization (`init_git_cas`, `GitCasBundle`) |

## Key Types

### `ApiState`

Composes `AgentService` for all shared infrastructure. The `agent_service` field is the single source of truth for domain objects. Surface-specific fields are the only fields that don't come from `AgentService`.

| Field | Type | Purpose |
|---|---|---|
| `agent_service` | `Arc<AgentService>` | Single source of truth for all shared infrastructure |
| `template_adapter` | `Arc<TemplateCrateLoader>` | Template-loading adapter (surface-specific) |
| `git_cas_port` | `Arc<dyn GitCASPort>` | Git CAS port for all CAS operations (surface-specific) |
| `gix_cas` | `Arc<GixCasAdapter>` | Gix CAS adapter for admin operations (surface-specific) |
| `wallet_service` | `Option<Arc<WalletService>>` | Wallet service for rJoule payments and API keys |
| `api_key_auth_service` | `Option<Arc<ApiKeyAuthService>>` | API key authentication for Bearer token verification |

Constructors:
- `ApiState::with_defaults()` — resolves config from env and keychain, builds `AgentService`, constructs `ApiState`. Headless — caller must ensure secrets are available.
- `ApiState::from_service_context(ctx)` — canonical construction from a pre-built `AgentService`. All shared infrastructure comes from `ctx`.
- `with_wallet_service(svc)` — attach a wallet service, returning `Self`.
- `start_loops()` — start all registered loop system cycles.
- `shutdown_loops()` — signal loop system shutdown.

### `ApiError`

API error type re-exported from `error` module. Includes `Internal` variant with a `message` field.

## Route Groups

The API router is assembled via `create_router(state) -> OpenApiRouter`, merging these route groups:

| Router Function | Route Group |
|---|---|
| `auth_router()` | Authentication endpoints |
| `landing_page` | `GET /` |
| `health_check` | `GET /health` |
| `templates_router()` | Template CRUD (`/api/templates`, `/api/templates/:id`, `/api/templates/search/:term`) |
| `terminal_router()` | Terminal interaction |
| `pods_router()` | Pod lifecycle (`/api/pods`, `/api/pods/:id/activate`, etc.) |
| `mcp_router()` | MCP server/tool management (`/api/mcp/servers`, `/api/mcp/tools`, `/api/mcp/invoke`) |
| `replicant_router()` | Replicant management |
| `cns_router()` | CNS health, alerts, variety (`/api/cns/health`, `/api/cns/alerts`, `/api/cns/variety`) |
| `sovereignty_router()` | Sovereignty status, consent grant/revoke, access check (`/api/sovereignty/*`) |
| `chat_router()` | Curator chat (`POST /api/chat`) |
| `models_router()` | Model listing |
| `a2a_router()` | A2A agent registration |
| `bundles_router()` | Bundle management |
| `curator_router()` | Curator governance |
| `episodic_router()` | Episodic memory storage/query/usage (`/api/episodic/*`) |
| `export_router()` | Sovereignty export |
| `consolidation_router()` | Episodic→semantic consolidation |
| `git_router()` | Git CAS operations |
| `goal_router()` | Goal coordination |
| `settings_router()` | Settings management |
| `wallet_router()` | Wallet operations |
| `routes::admin::*` | Admin endpoints (`/api/v1/admin/invite`, `/api/v1/admin/sessions`, `/api/v1/admin/config`) |

## Request/Response Types

Re-exported from `routes`:

| Type | Route |
|---|---|
| `ApiChatRequest` / `ApiChatResponse` | `POST /api/chat` |
| `CnsHealthResponse` | `GET /api/cns/health` |
| `CnsVarietyResponse` / `VarietyCounterResponse` | `GET /api/cns/variety` |
| `CreatePodRequest` / `CreatePodResponse` | `POST /api/pods` |
| `PodStatusResponse` | `GET /api/pods/:id/status` |
| `ListPodsResponse` | `GET /api/pods` |
| `TemplateResponse` | `GET /api/templates/:id` |
| `ModelEntry` / `ModelListResponse` / `ModelSearchQuery` | Model routes |
| `A2ARegisterRequest` / `A2ARegisterResponse` | A2A registration |
| `WithdrawalFeeEstimateResponse` | Wallet fee estimation |

## Middleware Stack

Applied in order (outermost layer runs first):

1. **CNS span middleware** — captures all requests for observability
2. **Session cookie middleware** — injects `AuthContext` if valid session exists (DEP-020)
3. **Capability token middleware** — requires Bearer token if no `AuthContext`
4. **Admin role-gating middleware** — restricts admin routes to admin roles
5. **API key auth middleware** — allows Bearer token auth on wallet routes (conditional, only if `api_key_auth_service` is available)

## OpenAPI Integration

- `ApiDoc` (`openapi` module) defines the OpenAPI schema via `utoipa::OpenApi`.
- `create_router()` returns an `OpenApiRouter` that serves both routes and OpenAPI specification.
- `create_openapi()` builds an `OpenApi` document with all route paths collected from the router (without state or middleware). Used for spec generation separate from the running server.

## Feature Flags

No feature flags are defined on this crate itself. It is pulled in by the `hkask-cli` `api` feature.
