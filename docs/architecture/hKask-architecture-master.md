---
title: "hKask Architecture Master"
audience: [architects, developers, agents]
last_updated: 2026-06-10
version: "0.27.2"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Architecture Master

**Purpose:** Index to the authoritative architecture documents.

**Project:** hKask (ℏKask - "A Minimal Viable Container for Agents") v0.27.2
**Binary:** `kask`  
**Crate prefix:** `hkask-`

---

## Document Hierarchy

```
magna-carta.md  ←  Foundation (4 inviolable principles)
       ↓
PRINCIPLES.md  ←  9 principles (P1-P9), constraint forces
       ↓
   MDS.md      ←  Minimal Domain Specification (5 categories, 6 tools)
       ↓
loop-architecture.md  ←  4-loop decomposition, RateLimiting→EnergyBudget
```

### Canonical Specifications

| Document | Purpose |
|----------|---------|
| [`magna-carta.md`](magna-carta.md) | User sovereignty charter — catch-and-release, affirmative consent, OCAP verification |
| [`PRINCIPLES.md`](PRINCIPLES.md) | 9 architecture principles (P1-P9), 5 anchors, anti-patterns |
| [`MDS.md`](MDS.md) | Minimal Domain Specification — 5 categories, 6 tools, completeness predicate |
| [`loop-architecture.md`](loop-architecture.md) | 4-loop architecture — RateLimiting→EnergyBudget subsumption, crate↔loop mapping |

### Historical

| Document | Status |
|----------|--------|
| `MDS.md` | Deleted — superseded by MDS.md (9→5 categories, 9→6 tools) |
| `MDS.md §7.1-7.2` | Deleted — covered by MDS.md §7.1-7.2 |
| `MDS.md §7.2` | Deleted — covered by MDS.md §7.2 |
| `MDS.md §7.4` | Deleted — covered by MDS.md §7.4 |
| `MDS.md §7.3` | Deleted — covered by MDS.md §7.3 + PRINCIPLES.md §2.1 |

---

## REPL Architecture

The interactive REPL (`kask chat`) implements four features that govern inference behavior:

### Context Injection

Conversation history is appended as a **suffix** (after the cache breakpoint) so the KV cache prefix — system prompt + template — remains identical across turns. Models that cache KV state across requests (e.g., Ollama with `keep_alive`) see prefix cache hits on every turn, reducing per-turn inference latency. Controlled by `ReplSettings.context_turns` (default 3, 0 = no history).

### Unbounded Tool-Use Loop

The REPL loops tool calls until the model stops requesting them, gated by `ReplSettings.tool_loop_limit` (default 21). Each iteration checks the energy budget via `GovernedTool` before executing. If the limit is hit, the loop breaks and returns the partial response — the system tells the model it can continue by asking.

### Auto-Condense

At 87.5% of the model's context window, old session history is condensed via the condenser library (`hkask-mcp-condenser`). The condenser summarizes older turns into a compact form, freeing context space for new messages. Controlled by `ReplSettings.auto_condense` (default on). When off, the user must condense manually.

### Model Awareness

On model switch (`/model`), the REPL fetches metadata from Ollama's `/api/show` endpoint:
- `context_length` — the model's native context window size (used by auto-condense)
- `supports_thinking` — whether the model supports thinking/reasoning tokens
- `capabilities` — model feature list (vision, tools, etc.)

Populated into `ReplSettings.model_meta` as read-only fields. Unknown until the first model detail fetch succeeds.

### ReplSettings

User-configurable inference parameters exposed via three surfaces:

| Setting | Type | Range | Default | Description |
|---------|------|-------|---------|-------------|
| `tool_loop_limit` | usize | ≥1 | 21 | Max tool-call iterations per turn |
| `context_turns` | usize | ≥0 | 3 | Past turns in context (0 = no history) |
| `temperature` | f32 | 0.0–2.0 | 0.7 | Sampling temperature |
| `top_p` | f32 | 0.0–1.0 | 0.9 | Nucleus sampling |
| `top_k` | u32 | ≥1 | 40 | Top-k filtering |
| `min_p` | f32 | 0.0–1.0 | 0.0 | Min-p threshold (0.0 = disabled) |
| `typical_p` | f32 | 0.0–1.0 | 0.0 | Locally typical sampling (0.0 = disabled) |
| `max_tokens` | u32 | ≥1 | 512 | Max completion tokens per call |
| `seed` | u32 or `off` | — | random | Deterministic seed |
| `gas_heuristic` | u64 | ≥1 | 500 | Per-turn gas reservation |
| `gas_cap` | u64 | ≥1 | 10,000 | Total session energy budget cap |
| `auto_condense` | bool | — | true | Auto-condense at 87.5% of context window |
| `model_meta` | read-only | — | None | Model context_length, thinking, capabilities |

### Magna Carta P3 — Equal Surface Exposure

All ReplSettings fields are equally exposed across:
- **REPL:** `/repl` slash command (show/set individual fields)
- **CLI:** `kask settings show|set|reset` commands
- **API:** `GET /api/settings` and `PUT /api/settings` endpoints

All three surfaces read/write the same `~/.config/hkask/settings.json` file. No settings are hidden, admin-gated, or surface-restricted.

---

## Service Layer

**Crate:** `hkask-services` — shared business logic for CLI and API surfaces.

### AgentService Architecture (v0.27.2)

`AgentService` is the canonical service layer owning all shared infrastructure. Fields are **private** and exposed through **8 group methods** — 7 return tuples of references, 1 returns a dedicated service wrapper:

```rust
agent_service.memory()       // (&Arc<EpisodicStoragePort>, &Arc<SemanticStoragePort>)
agent_service.cns()          // (&Arc<RwLock<CnsRuntime>>, &Arc<RwLock<CyberneticsLoop>>, &Arc<LoopSystem>, &Arc<dyn NuEventSink>)
agent_service.governance()   // (&Arc<CapabilityChecker>, &Arc<McpDispatcher>, &Arc<EscalationQueue>)
agent_service.storage()      // (7 store references: registry, goals, specs, sessions, users, agent_registry, git_cas)
agent_service.coordination() // (&Option<Arc<InferencePort>>, &Arc<McpRuntime>, &Arc<PodManager>, &Arc<RwLock<SessionManager>>)
agent_service.identity()     // (&WebID, &Arc<AcpRuntime>)
agent_service.sovereignty()  // SovereigntyService (wraps ConsentManager — P1/P2 affirmative consent)
agent_service.config()       // &ServiceConfig
```

See [`../specifications/MDS-agent-service.md`](../specifications/MDS-agent-service.md) for full specification.

### Dependency Direction

```mermaid
graph TD
    CLI["hkask-cli"]
    API["hkask-api"]
    SVC["hkask-services (AgentService)"]
    CLI --> SVC
    API --> SVC
    SVC --> AGENTS[hkask-agents]
    SVC --> CNS[hkask-cns]
    SVC --> MEM[hkask-memory]
    SVC --> TEMPLATES[hkask-templates]
    SVC --> TYPES[hkask-types]
    SVC --> STORAGE[hkask-storage]
```

Domain crates **never** depend on `hkask-services`. MCP servers **never** depend on `hkask-services` (P1 Prohibition — out-of-process isolation).

### AgentService Composition

`AgentService::build(config)` assembles all shared infrastructure once at startup. Both surfaces compose it and add only presentation-specific fields:

- `ReplState` = `AgentService` + REPL fields (prompt history, input state)
- `ApiState` = `AgentService` + HTTP fields (router, OpenAPI spec)

`AgentService::build()` replaces four independent assembly paths: `Stores::init`, `build_loop_system`, `build_governed_mcp_tool`, `build_ensemble_session`. Dependency order: DB → stores → CNS → loop system → governed tool → ACP/pods → inference port → memory adapters.

### Surface vs Service Boundary

| Concern | Owner | Examples |
|---------|-------|----------|
| Business logic normalization | `hkask-services` | Multi-step workflows, cross-crate orchestration, error normalization |
| Input validation | Surface | CLI arg parsing, HTTP body schema, path params |
| OCAP gates | Surface | `GovernedTool` membrane, capability checks before service call |
| HTTP status mapping | `hkask-api` | `ServiceError → StatusCode` |
| CLI formatting | `hkask-cli` | Table output, color, progress indicators |
| Field encapsulation | `hkask-services` | All 27 fields private, accessed via 7 domain adapter methods |

### Depth Test Results (Post-Essentialist v0.27.2)

| Module | Public API | Call Sites (CLI+API) | Status |
|--------|-----------|---------------------|--------|
| `AgentService` | 8 methods (7 groups + build) | 2 surfaces | ✅ Pass — encapsulated |
| `ChatService` | 4 functions | 8+ | ✅ Pass — CNS instrumented (P9) |
| `InferenceService` | 3 functions | 11+ | ✅ Pass |
| `ComposeService` | 1 function + 7 types | 2+ | ✅ Deep — 220 lines behind 1 call |
| `EmbedService` | 2 functions + 9 types | 2+ | ✅ Deep — 200 lines behind 2 calls |
| `OnboardingService` | 7 functions + 2 types | 2+ | ✅ Pass — reduced from 8 methods |
| `VerificationService` | 3 functions + 5 types | 2+ | ✅ Pass |
| `skill.rs` | 6 freestanding functions + 2 types | 4+ | ✅ Pass — freestanding, no wrapper struct |
| `consolidation.rs` | 4 freestanding functions | 2+ | ✅ Pass — rate limiter + passphrase verify + consolidate |
| `ArchivalService` | 4 functions + 2 types | 1 surface | ⚠️ Shallow — single-consumer HTTP pass-through |

### Modules Added (v0.27.2)

| Module | Purpose |
|--------|--------|
| `CnsService` (cns.rs) | CNS health, alerts, variety queries wrapping shared `CnsRuntime` — 3 async methods + 3 unit tests |
| `SovereigntyService` (sovereignty.rs) | Consent grant/revoke/check wrapping `ConsentManager` — 4 methods + 1 unit test, resolves P1 Prohibition

### Skipped Domains

| Domain | Reason |
|--------|--------|
| memory | 2 call sites — insufficient depth |
| spec | 4 call sites — insufficient depth |
| goal | CRUD pass-throughs — no business logic to normalize |
| models | Covered by `InferenceService` |

### Key Constraints

1. **MCP servers do NOT depend on `hkask-services`** — P1 Prohibition (out-of-process isolation). Service layer is in-process only.
2. **Domain crates do NOT depend on `hkask-services`** — dependency direction is strictly surface → service → domain.

---

## Reference Artifacts

Detailed lookup tables and diagrams in `reference/`:

| Artifact | Purpose |
|----------|---------|
| [`reference/hKask-erd.md`](reference/hKask-erd.md) | Core entity relationship diagrams |
| [`reference/registry-erd.md`](reference/registry-erd.md) | Registry schema diagrams |
| [`reference/subsystem-erds.md`](reference/subsystem-erds.md) | Per-crate ERDs |
| [`reference/hKask-hLexicon.md`](reference/hKask-hLexicon.md) | Full 87-term vocabulary catalog |
| [`reference/ports-inventory.md`](reference/ports-inventory.md) | Hexagonal port trait signatures |
| [`reference/utoipa-implementation.md`](reference/utoipa-implementation.md) | OpenAPI generation guide |
| [`reference/template-header-standard.md`](reference/template-header-standard.md) | Template metadata format |
| [`reference/hKask-Curator-persona.md`](reference/hKask-Curator-persona.md) | Curator persona specification |
| [`reference/okapi-integration.md`](reference/okapi-integration.md) | Okapi LLM API contract |


---

## Decision Records

| ADR | Topic |
|-----|-------|
| [`ADR-022-comprehensive-security-hardening.md`](ADR-022-comprehensive-security-hardening.md) | ADV-REVIEW-F2 security hardening (T01-T22) |
| [`ADR-024-unified-registry.md`](ADR-024-unified-registry.md) | Unified registry with `template_type` discriminator (retroactive) |
| [`ADR-025-attenuation-depth-limit.md`](ADR-025-attenuation-depth-limit.md) | 7-level attenuation depth limit (retroactive) |
| [`ADR-026-bitemporal-triple-schema.md`](ADR-026-bitemporal-triple-schema.md) | Bitemporal triple schema with valid-time × transaction-time (retroactive) |
| [`ADR-027-argon2-hkdf-master-key.md`](ADR-027-argon2-hkdf-master-key.md) | Argon2id + HKDF-SHA256 master key derivation (retroactive) |
| [`ADR-030-skill-bundler.md`](ADR-030-skill-bundler.md) | Skill bundler — meta-skill composition |
| [`ADR-031-consolidation-authorization.md`](ADR-031-consolidation-authorization.md) | Consolidation authorization via master passphrase derivation |
| [`ADR-032-mcp-gateway-membrane.md`](ADR-032-mcp-gateway-membrane.md) | MCP gateway membrane policy — Tier 1 (governed) vs Tier 2 (passthrough) |
| [`ADR-033-dampener-override-cooldown.md`](ADR-033-dampener-override-cooldown.md) | Dampener override cooldown — per-issuer vs global |

---

## Specifications

| Document | Purpose |
|----------|---------|
| [`../specifications/REQUIREMENTS.md`](../specifications/REQUIREMENTS.md) | 22 implemented + 5 deferred goal specs |
| [`../specifications/TRACEABILITY_MATRIX.md`](../specifications/TRACEABILITY_MATRIX.md) | Bidirectional code→test traceability |


---

*Verification commands:* `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`. See [`MDS_SCAFFOLD.md`](../specifications/MDS_SCAFFOLD.md) §6 for the full verification gate table.

---

## Document Structure

```
docs/architecture/
├── hKask-architecture-master.md           # THIS FILE (index — includes REPL Architecture)
├── MDS.md                              # Framework
├── PRINCIPLES.md                          # Framework (P3 updated with ReplSettings)
├── loop-architecture.md                   # Framework (4-loop authority model)
├── magna-carta.md                         # Framework
├── MDS.md §7.1-7.2               # SPEC (Domain + Capability)
├── MDS.md §7.2           # SPEC (Interface + Composition)
├── MDS.md §7.3        # SPEC (Trust + Observability)
├── MDS.md §7.4           # SPEC (Persistence + Lifecycle)
├── ADR-022-comprehensive-security-hardening.md  # Decision record
├── ADR-024-unified-registry.md            # Decision record
├── ADR-025-attenuation-depth-limit.md     # Decision record
├── ADR-026-bitemporal-triple-schema.md    # Decision record
├── ADR-027-argon2-hkdf-master-key.md      # Decision record
├── ADR-030-skill-bundler.md                # Decision record
├── ADR-031-consolidation-authorization.md  # Decision record
├── ADR-032-mcp-gateway-membrane.md        # Decision record (Draft)
├── ADR-033-dampener-override-cooldown.md   # Decision record (Draft)
└── reference/
    ├── hKask-erd.md                       # Diagram artifact
    ├── registry-erd.md                    # Diagram artifact
    ├── subsystem-erds.md                  # Diagram artifact
    ├── hKask-hLexicon.md                  # Vocabulary catalog
    ├── ports-inventory.md                 # Port reference
    ├── utoipa-implementation.md           # API guide
    ├── template-header-standard.md        # Format reference
    ├── hKask-Curator-persona.md           # Persona spec
    └── okapi-integration.md               # Okapi API contract
```

**Total:** 24 active architecture documents (4 specs + 4 framework + 1 index + 9 active ADRs + 6 active reference artifacts).

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.2*
