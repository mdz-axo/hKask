# Open Questions — hKask Service Layer Extraction

## F1 — Streaming responses

Current API returns single `ChatResponse`. Neither surface supports SSE chunked
inference. The service layer needs a `ChatStream` result type that both surfaces
adapt.

## F2 — Session lifecycle across surfaces

REPL `active_session` and API `standing_sessions` should migrate to
`ServiceContext`, but durability semantics need specification.

## F3 — Unified authentication context

API `AuthContext`, CLI `ResolvedSecrets`, MCP env vars — need single
`AuthContext` abstraction.

## F4 — MCP server service access

MCP servers are separate processes and can't share `ServiceContext` by reference.
They need shared primitives (`hkask-storage`, `hkask-keystore`,
`InferenceService` factory pattern).

## F5 — Test seam depth (C8)

Every service operation testable in isolation with mock `ServiceContext`.
Update test inventory.

## F6 — REPL vs API state boundary

Document which fields stay in surface vs service.

## F7 — ServiceConfig vs environment variables

`HKASK_DB_PATH`, `OKAPI_BASE_URL`, etc. should be resolved once in
`ServiceConfig`.

## F8 — GovernedTool membrane boundary

REPL creates `GovernedTool` with shared `CyberneticsLoop`; API creates in
`build_governed_mcp_tool()`; CLI commands create "disconnected" one. Service
layer should own canonical construction, but one-shot CLI commands need design.