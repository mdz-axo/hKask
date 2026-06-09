# Open Questions — hKask Service Layer Extraction

Items F1–F10 from the service layer extraction project. Each entry includes
topic, status, constraint force classification, affected crates, and a
recommended resolution approach.

---

## Resolved

| ID | Topic | Session | Resolution |
|----|-------|---------|------------|
| F1 | Streaming responses (inference override + API SSE) | 27 | `OkapiInference::generate_stream()` override sends `stream: true`, parses SSE/NDJSON into `InferenceStreamChunk`. `generate_stream_with_model()` on trait. `POST /api/chat/stream` SSE endpoint with channel bridge. CLI incremental printing via `ChatService::prepare_chat()` + `generate_stream_with_model()`. |
| F3 | Unified AuthContext | 27 | `ChatService::chat()` uses `ctx.capability_checker.grant_registry()` for both auth and legacy paths. `mcp_secret`/`acp_secret` split classified as Guardrail (defense in depth). Only `ChatService::chat()` mints `DelegationToken` in services. |
| F4 | MCP Server Duplication | 28 | All three servers classified as parity-test candidates (option c). Goal: delegates to same domain crate. Replicant: P1 Prohibition against `PodService`/`InferenceService`. Spec: 8/11 tools are MCP-only. P1 Prohibition documentation added to `agent_loader.rs` and `tools.rs`. |
| F5 | Test seam depth (C8) | 24 | `PodManager::new_mock()` uses deterministic test ACP secret; 4 pod tests now pass without env vars |
| F6 | REPL vs API state boundary | 21 | `ServiceContext` bridges both surfaces |
| F7 | ServiceConfig vs environment variables | 22 | `ServiceConfig::from_env()` resolves from both |
| F8 | GovernedTool membrane boundary | 26 | `.with_governed_tool()` wired in `PodManager::new(...)` chain in `ServiceContext::build()`. Governance is optional; when not configured, pod-initiated calls bypass CNS governance. |
| F9 | `serde_json::Value` from `EpisodicStoragePort.recall` | 24 | `RecalledEpisode` typed DTO replaces untyped Values |
| F10 | `serde_json::Value` from `SemanticStoragePort.recall` | 25 | `RecalledSemantic` typed DTO replaces untyped Values; `triple_to_json` deleted |

## Deferred

### F2 — Session Lifecycle Across Surfaces

- **Force:** Guideline — best practice, relaxable with reason
- **Affected crates:** `hkask-cli`, `hkask-api`, `hkask-services`
- **Description:** Sessions are CLI-local currently. The API doesn't share session
  state with the CLI. If an agent operates across both surfaces, session continuity
  breaks.
- **Recommendation:** Evaluate when multi-surface agent sessions become a product
  requirement. `ServiceContext` already provides the shared state container.

### Ensemble standing_start — Surface-specific (Divergent)

- **Force:** Guideline — best practice, relaxable with reason
- **Affected crates:** `hkask-cli`, `hkask-api`, `hkask-services`
- **Description:** `EnsembleService` explicitly documents standing sessions as
  Divergent (CLI: YAML file bootstrap, API: JSON body + MCP discovery + gas governance).
  The depth test fails — extracting `standing_start` would require a complex parameter
  type capturing divergent surface inputs, adding more interface cost than behavior
  benefit. No stub exists.
- **Recommendation:** Standing sessions remain surface-specific. If CLI and API
  converge on a common config format, re-evaluate.
- **Decision:** #84 (Session 29)

### Sovereignty consent enforcement — Already extracted

- **Force:** Evidence — measured fact
- **Affected crates:** `hkask-services`, `hkask-api`
- **Description:** `SovereigntyService::check_access()` returns `AccessCheck` with all
  data needed for enforcement. The API route's 6-line enforcement block (if no consent
  and not PUBLIC → return 403) is surface-specific HTTP error mapping. Correct
  architecture: service returns data, surface decides presentation.
- **Recommendation:** No extraction needed. The current architecture is correct.
- **Decision:** #85 (Session 29)

### Chat PromptStrategy — Depth test fails

- **Force:** Hypothesis — needs verification
- **Affected crates:** `hkask-services`, `hkask-templates`
- **Description:** `ChatService::prepare_chat()` composes prompts with ~30 lines of
  straightforward string assembly. The existing `PromptStrategy` enum in
  `hkask-templates` is used in API routes for per-input framing. A strategy pattern
  in ChatService would add indirection without reducing complexity.
- **Recommendation:** Re-evaluate if prompt composition grows significantly.
  Current architecture is sufficient.
- **Decision:** #86 (Session 29)

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*