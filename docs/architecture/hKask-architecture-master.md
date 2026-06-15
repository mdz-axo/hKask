---
title: "hKask Architecture Master"
audience: [architects, developers, agents]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Architecture Master

**Purpose:** Index to the authoritative architecture documents.

**Project:** hKask (ℏKask - "A Minimal Viable Container for Agents") v0.27.0
**Binary:** `kask`  
**Crate prefix:** `hkask-`

---

## Document Hierarchy

```
core/magna-carta.md  ←  Foundation (4 inviolable principles)
       ↓
core/PRINCIPLES.md  ←  9 principles (P1-P9), constraint forces
       ↓
   core/MDS.md      ←  Minimal Domain Specification (5 categories, 5 tools)
       ↓
loop-architecture.md  ←  4-loop decomposition, RateLimiting→EnergyBudget
```

### Canonical Specifications

| Document | Purpose |
|----------|--------|
| [`core/magna-carta.md`](core/magna-carta.md) | User sovereignty charter — catch-and-release, affirmative consent, OCAP verification |
| [`core/PRINCIPLES.md`](core/PRINCIPLES.md) | 12 architecture principles (P1-P12), 5 anchors, anti-patterns |
| [`core/MDS.md`](core/MDS.md) | Minimal Domain Specification — 5 categories, 5 tools, completeness predicate |
| [`loop-architecture.md`](loop-architecture.md) | 4-loop architecture — RateLimiting→EnergyBudget subsumption, crate↔loop mapping |
| [`mandates/P12-replicant-host-mandate.md`](mandates/P12-replicant-host-mandate.md) | Replicant Host Mandate — every interaction has an author, no unsupervised agency |
| [`energy-gas-payments-api-keys.md`](energy-gas-payments-api-keys.md) | Energy, Gas, Payments & API Key System — economic layer, rJoules, wallets, key lifecycle |

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

### Voice Interaction (Talk + Listen)

The REPL supports bidirectional voice interaction through the media MCP server (`hkask-mcp-media`):

| Command | Behavior |
|---------|----------|
| `/talk on` | Enable speech output — after each agent response, a speech summarizer condenses the output into 1-3 spoken sentences via LLM, then plays through ffplay |
| `/talk off` | Disable speech output |
| `/talk voice [DESC]` | Set or show the TTS voice profile (calls `voice_design` on media server, maps to ElevenLabs presets) |
| `/listen start [SECONDS]` | Record audio from microphone (default 30s), transcribe with word-level timestamps via `transcribe_bundle`, save as `TranscriptBundle` JSON |
| `/listen stop` | Show info about the last recording |
| `/listen view [FILE]` | Open TUI transcript viewer with word-level highlighting synced to audio playback (Richmond Gold #B79163) |

**Architecture:** `/talk` calls the speech summarizer (inference port) → `generate_speech` (MCP media) → ffplay. `/listen` calls `audio_capture` → `transcribe_bundle` (MCP media). Both use `GovernedTool` for OCAP-gated MCP invocation. The `TranscriptViewer` renders `TranscriptBundle` JSON using ratatui + ffplay subprocess.

---

## Service Layer

**Crate:** `hkask-services` — shared business logic for CLI and API surfaces.

**Canonical specification:** [`MDS-agent-service.md`](../specifications/specs/MDS-agent-service.md) — full domain spec, accessor methods, depth test results, and service boundary definitions.

### Summary

`AgentService` is the canonical service layer owning all shared infrastructure. All 26 fields are **private** and exposed through **individual named accessor methods** (replacing the earlier grouped-tuple pattern). `AgentService::build(config)` assembles all shared infrastructure once at startup. Both surfaces compose it and add only presentation-specific fields:

- `ReplState` = `AgentService` + REPL fields (prompt history, input state)
- `ApiState` = `AgentService` + HTTP fields (router, OpenAPI spec)

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

### Key Constraints

1. **MCP servers should not depend on `hkask-services` for orchestration** — P1 Prohibition (out-of-process isolation). Exceptions: servers that are direct surfaces for a service (CLI/API/MCP tri-surface pattern). `hkask-mcp-replica` is a tri-surface for `ComposeService` + `EmbedService`, not an orchestrator.
2. **Domain crates do NOT depend on `hkask-services`** — dependency direction is strictly surface → service → domain.

---

## Daemon & Replicant Server Mode

**Crates:** `hkask-mcp` (daemon transport), `hkask-services` (daemon handler), `hkask-agents` (AgentMode)

### Summary

Replicants can operate in **server mode**, presenting as MCP servers to IDEs (Zed, VSCode) and other hKask agents. The daemon — a Unix domain socket at `~/.config/hkask/daemon.sock` — mediates authentication, role assignment, capability verification, and dual memory encoding between out-of-process MCP binaries and the in-process agent stack.

### Architecture

```mermaid
graph TD
    subgraph "hKask System (Background Service)"
        AS["AgentService::build()"]
        PM["PodManager"]
        US["UserStore"]
        IP["InferencePort"]
        DH["ServiceDaemonHandler"]
        DL["DaemonListener<br/>(~/.config/hkask/daemon.sock)"]
        AS --> PM
        AS --> US
        AS --> IP
        AS --> DH
        DH --> DL
    end

    subgraph "MCP Binary (Out-of-Process)"
        BIN["hkask-mcp-research"]
        BIN -->|"1. auth_query"| DL
        BIN -->|"2. assignment_query"| DL
        BIN -->|"3. capability_query"| DL
        BIN -->|"4. store_experience"| DL
    end

    subgraph "Callers"
        IDE["Zed IDE"]
        HK["hKask Agent"]
    end

    IDE -->|"stdio MCP"| BIN
    HK -->|"GovernedTool"| BIN

    DH -->|"check_auth"| US
    DH -->|"check_assignment"| PM
    DH -->|"check_capability"| PM
    DH -->|"store_experience → dual encoding"| PM
    DH -->|"every 10: generate_narrative"| IP
```

### Startup Flow

1. `kask login <replicant>` — authenticate (creates session in UserStore)
2. `kask pod assign <replicant> <role>` — assign MCP role (P4 Gate 2: sovereignty/consent)
3. `kask pod mode <replicant> server -r <role>` — enter server mode (P4 Gate 1: OCAP)
4. IDE spawns MCP binary with `HKASK_REPLICANT=<replicant>`
5. Binary connects to daemon → auth → assignment → capability → serve

### Memory Flow

- Tool calls → `record_experience()` (fire-and-forget from MCP binary)
- Daemon `store_experience` → dual encoding: episodic (first-person, private) + semantic (third-person, public)
- Every 10 experiences → `generate_narrative()` → inference analyzes session log → stores observations as episodic "narrative"/"thought"
- Existing consolidation pipeline extracts semantic knowledge from both streams

### Agent Modes

| Mode | Behavior | Mutual Exclusion |
|------|----------|-----------------|
| **Chat** | Conversational loop, calls tools via GovernedTool | Cannot coexist with Server (initially) |
| **Server** | Presents as MCP server(s), handles incoming tool calls, records episodic memories | Cannot coexist with Chat (initially) |

Concurrent chat+server mode planned for future release (3-6 months).

### Key Constraints

1. **P4 Dual Gate:** Every MCP server startup requires both capability verification (OCAP token) and assignment verification (sovereignty/consent).
2. **P2 Affirmative Consent:** Passphrase entry via `kask login` creates session. Daemon checks session existence — no passphrase stored with MCP binary.
3. **Out-of-process isolation:** MCP binaries communicate with hKask only through the daemon socket. No direct access to PodManager, memory, or inference.
4. **Mode mutual exclusion (initial):** An agent can be in Chat mode OR Server mode, not both.

---

## Deployment

hKask is designed for two deployment targets: **local workstation** (Ollama) and **cloud server** (remote inference providers). The common deployment target is a headless Ubuntu cloud server accessed via SSH.

### Deployment Architecture

```mermaid
graph TD
    subgraph "Cloud Server (Ubuntu, headless)"
        KASK["kask binary"]
        KEYCHAIN["OS Keychain<br/>(secret-service / flat file)"]
        DB["SQLCipher<br/>(~/.config/hkask/)"]
        SOCK["Daemon Socket<br/>(~/.config/hkask/daemon.sock)"]
        KASK --> KEYCHAIN
        KASK --> DB
        KASK --> SOCK
    end

    subgraph "External Inference"
        DI["DeepInfra<br/>(primary)"]
        FW["Fireworks.ai<br/>(fallback)"]
        FA["fal.ai<br/>(vision/OCR)"]
    end

    subgraph "User Access"
        SSH["SSH Terminal"]
        IDE["Zed / VSCode<br/>(via MCP)"]
    end

    KASK -->|"HTTPS + API Key"| DI
    KASK -->|"HTTPS + API Key"| FW
    KASK -->|"HTTPS + API Key"| FA
    SSH -->|"kask chat"| KASK
    IDE -->|"MCP over SSH tunnel"| SOCK
```

### Common Deployment: Cloud Server

The intended production deployment is an Ubuntu cloud server without local GPU inference:

1. **No Ollama dependency.** The inference router (`hkask-inference`) routes all requests to cloud providers (DeepInfra, Fireworks, fal.ai). Ollama is optional and disabled by default on cloud deployments.
2. **API keys in OS keychain.** Provider API keys are stored in the OS keychain (Linux Secret Service or flat-file fallback), not in environment variables or plaintext files. Loaded once via `kask keystore load --shred`, which securely deletes the source file after explicit user consent.
3. **Encrypted database at rest.** All persistent state (agent registry, memory, sessions) uses SQLCipher with a passphrase-derived key.
4. **SSH-first interaction.** The primary interface is `kask chat` over SSH. MCP servers (for IDE integration) connect via SSH-tunneled Unix socket.

### Provider Configuration

API keys resolve through a 2-tier chain at startup:

| Tier | Source | Security | Persistence |
|------|--------|----------|-------------|
| 1 | OS Keychain (`secret-service` or `~/.local/share/keyrings/`) | Encrypted at rest by OS | Survives reboot |
| 2 | Environment variable | Plaintext in process memory only | Session-only |

Provider selection is controlled by `HKASK_DEFAULT_PROVIDER` (stored in keychain or env var):

| Value | Provider | Use Case |
|-------|----------|----------|
| `DI` | DeepInfra | Primary cloud provider — wide model catalog, per-token pricing, free tier |
| `FW` | Fireworks.ai | Fast serverless inference, fallback |
| `FA` | fal.ai | Specialized vision/OCR/media models |
| `OM` | Ollama | Local inference only (default, typically disabled on cloud) |

### Setup Flow

```bash
# One-time setup on cloud server
cp providers.env.example providers.env   # Fill in DI_API_KEY=sk-...
kask keystore load --path providers.env --shred  # Loads into keychain, shreds file
kask chat                                   # First run: onboarding creates replicant
```

The onboarding flow (`kask chat` first run) prompts for provider configuration if no keys are detected, offering three paths: load from `providers.env`, enter API key directly (masked input), or skip (Ollama-only).

### Security Properties

| Property | Mechanism |
|----------|-----------|
| No plaintext secrets on disk | Keys live in OS keychain; source file shredded after load |
| No secrets in environment | `InferenceConfig` reads from keychain at startup, not `std::env` |
| Affirmative consent before deletion | `--shred` requires explicit `y` response to warning prompt |
| Graceful degradation | Missing keys → backend unavailable (logged), not crash |
| Backward compatible | Existing `export`/`.env` setups continue working (env var tier) |

### Local Development

For local development with Ollama:

```bash
# Default: Ollama on localhost:11434
kask chat
# Or use cloud provider alongside Ollama:
export DI_API_KEY=sk-...
kask chat -m DI/meta-llama/Llama-3.3-70B-Instruct
```

Local and cloud deployments share the same binary, same config, same code path. The only difference is which provider keys are configured.

---

## Reference Artifacts

Detailed lookup tables and diagrams in `reference/`:

| Artifact | Purpose |
|----------|---------|

| [`reference/hKask-hLexicon.md`](reference/hKask-hLexicon.md) | Full 87-term vocabulary catalog |
| [`reference/ports-inventory.md`](reference/ports-inventory.md) | Hexagonal port trait signatures |
| [`reference/utoipa-implementation.md`](reference/utoipa-implementation.md) | OpenAPI generation guide |
| [`reference/template-header-standard.md`](reference/template-header-standard.md) | Template metadata format |
| [`reference/hKask-Curator-persona.md`](reference/hKask-Curator-persona.md) | Curator persona specification |
| [`reference/okapi-integration.md`](reference/okapi-integration.md) | Inference Router API contract (Ollama, Fireworks, DeepInfra) |


---

## Decision Records

| ADR | Topic |
|-----|-------|
| [`ADRs/ADR-030-skill-bundler.md`](ADRs/ADR-030-skill-bundler.md) | Skill bundler — meta-skill composition |
| [`ADRs/ADR-031-consolidation-authorization.md`](ADRs/ADR-031-consolidation-authorization.md) | Consolidation authorization via master passphrase derivation |
| [`ADRs/ADR-032-mcp-gateway-membrane.md`](ADRs/ADR-032-mcp-gateway-membrane.md) | MCP gateway membrane policy — Tier 1 (governed) vs Tier 2 (passthrough) |
| [`ADRs/ADR-033-dampener-override-cooldown.md`](ADRs/ADR-033-dampener-override-cooldown.md) | Dampener override cooldown — per-issuer vs global |
| [`ADRs/ADR-034-academic-author-pipeline.md`](ADRs/ADR-034-academic-author-pipeline.md) | Academic author pipeline — corpus_type discriminator, pre-processing, enumeration, disambiguation |
| [`ADRs/ADR-035-replicant-server-mode.md`](ADRs/ADR-035-replicant-server-mode.md) | Replicant server mode — AgentMode (Chat/Server), daemon socket transport, dual memory encoding, narrative generation |
| [`ADRs/ADR-036-ocr-pipeline.md`](ADRs/ADR-036-ocr-pipeline.md) | OCR pipeline — sealed backend hierarchy (Tesseract, PaddleOCR, FalAI), deterministic routing |
| [`ADRs/ADR-037-wallet-payments.md`](ADRs/ADR-037-wallet-payments.md) | Wallet payment mechanism — rJoule internal currency, multi-chain bridge architecture |
| [`ADRs/ADR-038-media-server.md`](ADRs/ADR-038-media-server.md) | Media MCP server — 28 tools across 6 categories, fal.ai primary backend |

**Archived (retroactive, 2026-06-15):** ADR-024, ADR-025, ADR-026, ADR-027. Recoverable via git history.

---

## Specifications

| Document | Purpose |
|----------|---------|
| [`../specifications/specs/REQUIREMENTS.md`](../specifications/specs/REQUIREMENTS.md) | 22 implemented + 5 deferred goal specs |
| [`../specifications/specs/TRACEABILITY_MATRIX.md`](../specifications/specs/TRACEABILITY_MATRIX.md) | Bidirectional code→test traceability |


---

*Verification commands:* `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`. See [`MDS_SCAFFOLD.md`](../specifications/specs/MDS_SCAFFOLD.md) §6 for the full verification gate table.

---

## Document Structure

```
docs/architecture/
├── hKask-architecture-master.md           # THIS FILE (index)
├── loop-architecture.md                   # Framework (4-loop authority model)
├── energy-gas-payments-api-keys.md        # Framework (gas, payments, API key system)
├── matrix-integration-architecture.md     # Specification (Matrix transport, Conduit sidecar)
├── core/
│   ├── magna-carta.md                     # Foundation (4 inviolable principles)
│   ├── PRINCIPLES.md                      # Framework (P1-P12)
│   └── MDS.md                             # Framework (5 categories, 5 tools)
├── mandates/
│   └── P12-replicant-host-mandate.md      # Framework (replicant host mandate)
├── ADRs/
│   ├── _TEMPLATE.md                       # ADR template
│   ├── ADR-030-skill-bundler.md           # Decision record (Draft)
│   ├── ADR-031-consolidation-authorization.md # Decision record
│   ├── ADR-032-mcp-gateway-membrane.md    # Decision record (Draft)
│   ├── ADR-033-dampener-override-cooldown.md # Decision record (Draft)
│   ├── ADR-034-academic-author-pipeline.md # Decision record (Active)
│   ├── ADR-035-replicant-server-mode.md   # Decision record (Active)
│   ├── ADR-036-ocr-pipeline.md            # Decision record (Draft)
│   ├── ADR-037-wallet-payments.md         # Decision record (Draft)
│   └── ADR-038-media-server.md            # Decision record (Draft)
└── reference/
    ├── hKask-hLexicon.md                  # Vocabulary catalog
    ├── ports-inventory.md                 # Port reference
    ├── utoipa-implementation.md           # API guide
    ├── template-header-standard.md        # Format reference
    ├── hKask-Curator-persona.md           # Persona spec
    └── okapi-integration.md               # Inference Router API contract
```

**Total:** 22 active architecture documents (3 core + 1 mandate + 4 root + 9 ADRs + 1 template + 6 reference artifacts).

**Related folders:** `docs/research/` (lazy-universe-research.md, training-decomposition-traces.md), `docs/specifications/` (wallet-specification.md, MDS_SCAFFOLD.md, etc.)

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
