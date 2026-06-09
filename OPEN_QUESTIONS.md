# Open Questions — hKask Service Layer Extraction

Items F1–F10 from the service layer extraction project. Each entry includes
topic, status, constraint force classification, affected crates, and a
recommended resolution approach.

---

## Resolved

| ID | Topic | Session | Resolution |
|----|-------|---------|------------|
| F1 | Streaming responses (inference override + API SSE) | 27 | `OkapiInference::generate_stream()` override sends `stream: true`, parses SSE/NDJSON into `InferenceStreamChunk`. `generate_stream_with_model()` on trait. `POST /api/chat/stream` SSE endpoint with channel bridge. CLI incremental printing remaining. |
| F3 | Unified AuthContext | 27 | `ChatService::chat()` uses `ctx.capability_checker.grant_registry()` for both auth and legacy paths. `mcp_secret`/`acp_secret` split classified as Guardrail (defense in depth). Only `ChatService::chat()` mints `DelegationToken` in services. |
| F5 | Test seam depth (C8) | 24 | `PodManager::new_mock()` uses deterministic test ACP secret; 4 pod tests now pass without env vars |
| F6 | REPL vs API state boundary | 21 | `ServiceContext` bridges both surfaces |
| F7 | ServiceConfig vs environment variables | 22 | `ServiceConfig::from_env()` resolves from both |
| F8 | GovernedTool membrane boundary | 26 | `.with_governed_tool()` wired in `PodManager::new(...)` chain in `ServiceContext::build()`. Governance is optional; when not configured, pod-initiated calls bypass CNS governance. |
| F9 | `serde_json::Value` from `EpisodicStoragePort.recall` | 24 | `RecalledEpisode` typed DTO replaces untyped Values |
| F10 | `serde_json::Value` from `SemanticStoragePort.recall` | 25 | `RecalledSemantic` typed DTO replaces untyped Values; `triple_to_json` deleted |

## Deferred

### F1 (remaining) — CLI Incremental Printing

- **Force:** Guideline — surface-specific delivery, user preference
- **Affected crates:** `hkask-cli`
- **Description:** The CLI (`kask chat` and REPL) waits for the complete result
  before printing. Streaming output should print `text_delta` chunks as they
  arrive from `generate_stream_with_model()`.
- **Recommendation:** Call the inference port directly from the CLI surface
  (option a per depth test: streaming is surface-specific, not a service concern).
  Memory recall and episodic storage happen before/after the streaming inference.

### F2 — Session Lifecycle Across Surfaces

- **Force:** Guideline — best practice, relaxable with reason
- **Affected crates:** `hkask-cli`, `hkask-api`, `hkask-services`
- **Description:** Sessions are CLI-local currently. The API doesn't share session
  state with the CLI. If an agent operates across both surfaces, session continuity
  breaks.
- **Recommendation:** Evaluate when multi-surface agent sessions become a product
  requirement. `ServiceContext` already provides the shared state container.

### F4 — MCP Server Duplication

- **Force:** Guideline — duplication is surface-specific or architecturally required; Prohibition still applies (MCP servers must NOT depend on `hkask-services`)
- **Affected crates:** `hkask-mcp-goal`, `hkask-mcp-replicant`, `hkask-mcp-spec`
- **Description:** Three MCP servers duplicate service-layer operations, but analysis shows
  all three fall under parity tests (option c), not domain-crate extraction (option b):
  - **goal**: Both delegate to `SqliteGoalRepository`; duplication is surface-specific
    validation and error mapping. Already in domain crate.
  - **replicant**: P1 Prohibition — `PodService` and `InferenceService` are explicitly
    excluded from MCP use. Duplication is intentional per architecture (process isolation).
    ACP secret resolution and agent loading are thin wrappers with MCP-specific fallbacks.
  - **spec**: 8 of 11 tools are MCP-only (OCAP verification, Writing Excellence, test
    traceability). The 3 partially-duplicated tools use the same `Spec` types and
    `SpecStore` trait from domain crates.
- **Recommendation:** Add parity integration tests for goal and spec duplicated operations.
  Document the intentional replication for replicant. No domain-crate extraction needed.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*