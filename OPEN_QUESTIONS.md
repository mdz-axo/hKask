# Open Questions — hKask Service Layer Extraction

Items F1–F10 from the service layer extraction project. Each entry includes
topic, status, constraint force classification, affected crates, and a
recommended resolution approach.

---

## Resolved

| ID | Topic | Session | Resolution |
|----|-------|---------|------------|
| F5 | Test seam depth (C8) | 24 | `PodManager::new_mock()` uses deterministic test ACP secret; 4 pod tests now pass without env vars |
| F6 | REPL vs API state boundary | 21 | `ServiceContext` bridges both surfaces |
| F7 | ServiceConfig vs environment variables | 22 | `ServiceConfig::from_env()` resolves from both |
| F9 | `serde_json::Value` from `EpisodicStoragePort.recall` | 24 | `RecalledEpisode` typed DTO replaces untyped Values |
| F10 | `serde_json::Value` from `SemanticStoragePort.recall` | 25 | `RecalledSemantic` typed DTO replaces untyped Values; `triple_to_json` deleted |

## Deferred

### F1 — Streaming Responses

- **Force:** Hypothesis — needs verification of client-side consumption patterns
- **Affected crates:** `hkask-services`, `hkask-api`, `hkask-cli`
- **Description:** The service layer currently returns complete results. Streaming
  would require changing `ChatService::chat` to return a stream and both surfaces
  to consume it incrementally. The Okapi inference layer supports streaming, but
  the service layer batches results.
- **Recommendation:** Wait until a concrete use case requires streaming. The current
  batch model is simpler and sufficient for CLI and API.

### F2 — Session Lifecycle Across Surfaces

- **Force:** Guideline — best practice, relaxable with reason
- **Affected crates:** `hkask-cli`, `hkask-api`, `hkask-services`
- **Description:** Sessions are CLI-local currently. The API doesn't share session
  state with the CLI. If an agent operates across both surfaces, session continuity
  breaks.
- **Recommendation:** Evaluate when multi-surface agent sessions become a product
  requirement. `ServiceContext` already provides the shared state container.

### F3 — Unified Authentication Context

- **Force:** Guideline — both surfaces have working auth; unification is convenience
- **Affected crates:** `hkask-api`, `hkask-cli`, `hkask-services`, `hkask-keystore`
- **Description:** API uses HTTP auth headers; CLI uses the keystore. `ServiceContext`
  holds the ACP secret but doesn't unify the auth context into a single abstraction
  that both surfaces delegate to.
- **Recommendation:** Introduce an `AuthContext` value object in `hkask-services` that
  captures the resolved identity and capabilities. Both surfaces construct it from
  their respective sources and pass it to service operations.

### F4 — MCP Server Service Access

- **Force:** Evidence — MCP servers are out-of-process and use domain primitives
- **Affected crates:** `hkask-mcp-*` (all MCP server crates)
- **Description:** MCP servers run out-of-process and don't depend on
  `hkask-services`. They use domain primitives (`TripleStore`, `SemanticMemory`)
  directly. This is correct for the out-of-process model but means MCP servers
  can't use service-layer conveniences like `InferenceService` or `ServiceContext`.
- **Recommendation:** Keep the current model. Out-of-process servers should use
  domain primitives, not services. The service layer is for in-process surfaces
  (CLI, API).

### F8 — GovernedTool Membrane Boundary

- **Force:** Guardrail — inference governance is a measured boundary (CNS)
- **Affected crates:** `hkask-cns`, `hkask-agents`, `hkask-mcp`
- **Description:** `GovernedTool` in `hkask-cns` governs tool invocations (gas
  budget, variety tracking, algedonic spans). The boundary between governed and
  ungoverned tool calls should be explicit, but currently the membrane is optional
  (when `governed_tool` is `None` on `PodContext`, calls bypass governance).
- **Recommendation:** Keep governance optional for now. The CNS design intent is
  observability-first: governance adds value but shouldn't block tool invocation
  when not configured. When governance becomes mandatory, enforce at the type level
  (make `GovernedTool` required in `PodContext`).

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*