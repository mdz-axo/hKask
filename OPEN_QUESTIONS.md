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
Update test inventory. Must address before Task 7b.

## F6 — REPL vs API state boundary

Document which fields stay in surface vs service.

## F7 — ServiceConfig vs environment variables

`HKASK_DB_PATH`, `OKAPI_BASE_URL`, etc. should be resolved once in
`ServiceConfig`. Currently 3 places read HKASK_DB_PATH.

## F8 — GovernedTool membrane boundary

REPL creates `GovernedTool` with shared `CyberneticsLoop`; API creates in
`build_governed_mcp_tool()`; CLI commands create "disconnected" one. Service
layer should own canonical construction, but one-shot CLI commands need design.

## F9 — Production memory persistence (HIGH)

Episodic/semantic memory stores use `in_memory_db()` even when
`config.in_memory: false`. This means production deployments lose all memories
on restart. Need `memory_db_path`/`memory_passphrase` fields in `ServiceConfig`
to persist memory to file-backed databases. This is a P1 User Sovereignty
concern — the user configured persistent storage and got ephemeral.

## F10 — ServiceContext god-object trajectory

`ServiceContext` has 19 public `Arc` fields. Guard against further growth with
sub-structs (`CnsInfra`, `MemoryInfra`, `McpInfra`) and `#[non_exhaustive]`
before adding domain service modules.

## F11 — InvalidPassphrase vs LoginFailed security

`InvalidPassphrase(String)` leaks whether the passphrase was wrong vs user not
found. `LoginFailed(String)` is deliberately opaque. Should unify for security
or document the distinction.

## F12 — ValidationError(String) too generic

`ValidationError(String)` is extremely broad. Consider `ConfigValidation(String)`
or domain-specific variants.

## F13 — CapabilityChecker secret inconsistency

`ServiceContext::build()` creates three `CapabilityChecker` instances with two
different secrets: `mcp_secret` for the field, `acp_secret` for `PodManager`
and `FullMcpAdapter`. Verify which secret should be used where before Task 7b.

## F14 — Dual error mapping in API (planned cleanup)

`hkask-api/src/error.rs` has 14 direct `From<DomainError>` impls PLUS the
`From<ServiceError>` adapter. Each domain error is mapped to `ApiError` twice.
This is intentional during strangler fig migration. Delete direct paths in
Task 7f.