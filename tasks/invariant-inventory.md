# hKask Invariant Inventory — Phase 0 T0.3

Proves the test harness is green on the UNMODIFIED tree before any refactor
slice begins. Every item below MUST remain true after consolidation.

---

## 1. MCP Server Tools (16 servers, 238 tools)

Each server uses `rmcp` `#[tool_router]` + `#[tool(...)]` + `Parameters<T>`
seam. Tool-behavior contract tests verified by `check-mcp-tool-tests.sh`.

| Server | Tools | Cross-deps | Status |
|--------|-------|------------|--------|
| hkask-mcp-companies | 41 | 5 | ✅ |
| hkask-mcp-media | 38 | 10 | ✅ |
| hkask-mcp-memory | 20 | 11 | ✅ |
| hkask-mcp-scenarios | 18 | 3 | ✅ |
| hkask-mcp-kata-kanban | 18 | 10 | ✅ |
| hkask-mcp-docproc | 18 | 11 | ✅ |
| hkask-mcp-research | 17 | 3 | ✅ |
| hkask-mcp-curator | 11 | 7 | ✅ |
| hkask-mcp-communication | 11 | 4 | ✅ |
| hkask-mcp-replica | 10 | 14 | ✅ |
| hkask-mcp-training | 8 | 11 | ✅ |
| hkask-mcp-condenser | 8 | 9 | ✅ |
| hkask-mcp-codegraph | 8 | 2 | ✅ |
| hkask-mcp-filesystem | 7 | 3 | ✅ |
| hkask-mcp-skill | 3 | 6 | ✅ |
| hkask-mcp-regulation | 2 | 6 | ✅ |

**Total: 238 tools. All must survive consolidation with identical `Parameters<T>` contracts.**

## 2. Skill Registry Manifests (98 manifests)

Located in `registry/manifests/*.yaml`. Manifest format uses `manifest:`
block with `id`, `category`, `name`, `description`, `functional_role`,
`version`, `visibility` fields. Skills are loaded via registry loader into
`ReplState.manifest_state`.

Skill directories in `.agents/skills/` (50 directories) contain `SKILL.md`
companion files. The registry manifest YAML is the canonical source of truth.

**Key skill manifests with `reg.*` spans** (verified by `check-reg-canonical.sh`):
All `reg.*` references in Rust code and `.j2` templates are canonical
(registered in `CANONICAL_NAMESPACES` in `crates/hkask-types/src/event.rs`).

**All 98 manifests must survive consolidation with their `reg.*` namespaces intact.**

## 3. Inference Provider Routes (8 providers)

Trait: `InferencePort` (5 methods) in `crates/hkask-ports/src/inference_port.rs`
Implementation: `InferenceRouter` in `crates/hkask-inference/src/inference_router/mod.rs`

| Prefix | Provider | Backend File | Chat | Vision | Local |
|--------|----------|-------------|------|--------|-------|
| DI/ | DeepInfra | deepinfra_backend.rs | ✅ | ✅ | ❌ |
| FA/ | fal.ai | fal_backend.rs | ✅ | ✅ | ❌ |
| TG/ | Together AI | together_backend.rs | ✅ | ✅ | ❌ |
| OR/ | OpenRouter | openrouter_backend.rs | ✅ | ✅ | ❌ |
| KC/ | KiloCode | kilocode_backend.rs | ✅ | ✅ | ❌ |
| OM/ | Ollama | ollama_backend.rs | ✅ | ✅ | ✅ |
| CL/ | Cline | cline_backend.rs | ✅ | ✅ | ❌ |
| RP/ | RunPod | runpod_backend.rs | ❌ | ✅ | ❌ |

Dispatch: `chat_backend()` / `vision_backend()` match-fns on 2-letter prefix.
Embedding: `EmbeddingRouter` (DeepInfra, OpenRouter).
Fusion: `fusion_orchestrator.rs` — multi-model deliberation.

**All 8 provider routes must survive consolidation with identical dispatch behavior.**

## 4. Port Traits (16 traits in hkask-ports)

| Trait | File | Purpose |
|-------|------|---------|
| InferencePort | inference_port.rs | LLM generation (5 methods) |
| ToolPort | tool.rs | Tool dispatch |
| CircuitBreakerPort | regulation.rs | Circuit breaker |
| LedgerObserver | regulation.rs | Ledger observation |
| LedgerStoragePort | regulation.rs | Ledger storage |
| EscalationPort | escalation.rs | Escalation |
| WalletBudgetPort | wallet_budget_port.rs | Wallet budget |
| StepExecutor | pipeline_runner.rs | Pipeline step execution |
| ConsentPort | consent_port.rs | Consent management |
| EmbeddingPort | embedding_port.rs | Embedding generation |
| SkillRegistryIndex | registry.rs | Skill registry indexing |
| RegistryIndex | registry.rs | Registry indexing |
| FederationTransport | federation.rs | Federation transport |
| FederationSyncPort | federation.rs | Federation sync |
| FederationDispatch | federation.rs | Federation dispatch |
| GitCASPort | git_cas/port.rs | Git content-addressable storage |

## 5. CI Gate Verification (baseline green)

| Gate | Script | Result |
|------|--------|--------|
| MCP tool-behavior tests | check-mcp-tool-tests.sh | ✅ 0 violations, 0 allowlisted |
| Reg canonical namespaces | check-reg-canonical.sh | ✅ All canonical |
| No Result<_, String> | check-string-errors.sh | ✅ None found |
| Workspace compiles | cargo check --workspace | ✅ 4.30s |

## 6. REPL/Chat Architecture

- `ReplState` struct: 128 lines of fields (agent_webid, current_model, current_agent, active_session, resolved_secrets, tool_definitions, manifest_state, service_context, repl_settings, is_first_run, talk_config, improv_mode, kanban_service, degraded_servers, thread_registry, host)
- `TuiReplBridge` implements 4 traits: ReplBridge, SystemBridge, SettingsBridge, SessionBridge
- Thread lifecycle: `threads.rs` (thread registry, creation, switching)
- Turn execution: `turn.rs` (single-turn inference dispatch)
- Command dispatch: `commands.rs` (slash commands, settings, model switching)
- Built-in MCP servers: `builtin_servers.rs` (in-process MCP server registration)
- Services-chat crate: separate chat service layer (11 cross-deps)

## 7. Cross-Crate Edge Summary

- **Total cross-crate edges:** 397
- **Target reduction:** ≥15% → need to reduce by ≥60 edges to ≤337
- **Highest fan-out:** hkask-cli (29), hkask-services-context (22), hkask-api (20), hkask-repl (19)
- **Highest fan-in:** hkask-types (54), hkask-ports (32), hkask-storage (31), hkask-database (30)
- **Consolidation target:** reduce edges WITHOUT removing reachability between any formerly-connected pair