---
title: "hKask Interface & Composition Specification"
audience: [architects, developers, agents]
last_updated: 2026-05-28
version: "2.1.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [interface, composition]
---

# hKask Interface & Composition Specification

**Purpose:** Authoritative specification for service surfaces, equivalence model, hexagonal ports, unified registry, template cascade, and composition rules. Single source of truth for DDMVSS categories **Interface** and **Composition**.

**Related:** [`domain-and-capability.md`](domain-and-capability.md), [`trust-security-observability.md`](trust-security-observability.md), [`persistence-and-lifecycle.md`](persistence-and-lifecycle.md)

**Verification:** `cargo check --workspace && cargo test -p hkask-templates && cargo test -p hkask-mcp`

---

## 1. Interface Equivalence: MCP ≡ CLI ≡ API

**Focusing assumption:** Three surfaces, one functional core. Every capability is exercisable through MCP, CLI, and API with identical semantics.[^cockburn-hex]

```mermaid
graph LR
    subgraph Surfaces["Interface Surfaces"]
        MCP_S["MCP Server<br/>15 servers, rmcp"]
        CLI_S["CLI Binary<br/>kask, clap v4"]
        API_S["HTTP API<br/>axum, utoipa"]
    end

    subgraph Core["Functional Core"]
        AGENTS["hkask-agents<br/>AgentPod, PodManager"]
        TEMPLATES["hkask-templates<br/>SqliteRegistry"]
        MCP_RT["hkask-mcp<br/>McpRuntime"]
    end

    MCP_S --> MCP_RT
    CLI_S --> AGENTS
    API_S --> AGENTS
    MCP_RT --> AGENTS
    AGENTS --> TEMPLATES
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-001
verified_date: 2026-05-28
verified_against: crates/hkask-cli/src/cli/mod.rs:33; crates/hkask-api/src/lib.rs:636; crates/hkask-mcp/src/runtime.rs:59
status: VERIFIED
-->

[^cockburn-hex]: Cockburn, A. (2005). *Hexagonal Architecture*. http://alistair.cockburn.us/Hexagonal+architecture. Ports-and-adapters pattern.

### 1.1 MCP Server Surface

**Protocol:** rmcp (Rust MCP) — JSON-RPC 2.0 over stdio, in-process, or HTTP transport

**Runtime:** `McpRuntime` (`crates/hkask-mcp/src/runtime.rs:59`)

**Transport options:**

| Transport | Implementation | Use Case | Security |
|-----------|---------------|----------|----------|
| In-process | `InProcessMcpTransport` (`transport.rs:32`) | Co-located servers | No network |
| Stdio | `StdioMcpTransport` (`transport.rs:102`) | Child process servers | Process isolation |
| HTTP | `HttpMcpTransport` (`transport.rs:155`) | Remote servers | HTTPS + OCAP tokens |

**Security:** `SecurityGateway` (`security.rs:51`) enforces OCAP before dispatch. `McpSupervisor` (`supervisor.rs:100`) manages health with configurable restart policies.

### 1.2 CLI Surface

**Binary:** `kask` (built from `hkask-cli`, 3,741 LOC)

**15 subcommand groups** (`crates/hkask-cli/src/cli/mod.rs:33`):

| Subcommand | Purpose |
|-----------|---------|
| `kask chat` | Curator chat interface with `/model` switching and `-m` flag |
| `kask template` | Template management (list, register, get, search, render) |
| `kask bot` | Bot capability management |
| `kask pod` | Agent pod lifecycle (create, activate, deactivate, status) |
| `kask mcp` | MCP server/tool management |
| `kask cns` | CNS monitoring (health, variety, alerts) |
| `kask sovereignty` | User sovereignty (Magna Carta enforcement) |
| `kask registry` | Registry management |
| `kask git` | Git archival |
| `kask ensemble` | Multi-agent ensemble |
| `kask spec` | DDMVSS specifications (capture, decompose, curate, validate) |
| `kask docs` | Documentation generation |
| `kask agent` | ACP agent registration |
| `kask curator` | Curator governance and metacognition |
| `kask replicant` | Replicant identity management |
| `kask keystore` | OS keychain secret management |

### 1.3 HTTP API Surface

**Framework:** axum v0.8 with utoipa v5.5 OpenAPI documentation

**12 route groups** (`crates/hkask-api/src/lib.rs:636-642`):

| Route Group | Purpose |
|------------|----------|
| `templates_router` | Template CRUD and rendering |
| `bots_router` | Bot capability management |
| `pods_router` | Agent pod lifecycle |
| `mcp_router` | MCP server/tool operations |
| `cns_router` | CNS health and monitoring |
| `sovereignty_router` | User sovereignty enforcement |
| `chat_router` | Chat interface (supports `model` field) |
| `models_router` | Okapi model catalog (list, search) |
| `ensemble_router` | Multi-agent ensemble |
| `soap_infer_router` | SOAP inference (Okapi bridge) |
| `acp_router` | ACP agent registration |
| `spec_router` | DDMVSS specification operations |

**OpenAPI:** Generated at `docs/generated/openapi.json`. Implementation details in [`reference/utoipa-implementation.md`](reference/utoipa-implementation.md).

### 1.4 Equivalence Matrix

| Capability | MCP | CLI | API |
|-----------|-----|-----|-----|
| Invoke tool | `tool_call(server, name, args)` | `kask mcp tools` | `POST /api/v1/mcp/tools` |
| Render template | `template_render(id, ctx)` | `kask template render` | `POST /api/v1/templates/{id}/render` |
| Create pod | N/A | `kask pod create` | `POST /api/v1/pods` |
| Grant capability | `ocap_grant(target, cap)` | `kask bot grant` | `POST /api/v1/bots/capabilities` |
| Query CNS | `cns_health()` | `kask cns health` | `GET /api/v1/cns/health` |
| Capture goal | `spec/goal/capture` | `kask spec capture` | `POST /api/v1/specs` |
| List templates | `registry_list(type)` | `kask template list` | `GET /api/v1/templates` |
| Switch model | `inference:models` | `/model <name>` or `-m` flag | `POST /api/chat {model}` |
| List models | `inference:models` | `/model <query>` | `GET /api/models` |
| Search models | `inference:models(filter)` | `/model qwen` | `GET /api/models/search?q=...` |

---

## 2. Hexagonal Port Architecture

hKask uses hexagonal architecture with explicit port traits defining integration boundaries.[^cockburn-hex]

**Full port trait signatures:** [`reference/ports-inventory.md`](reference/ports-inventory.md)

### 2.1 Driving Ports (Input)

| Port | Trait | Crate | Purpose |
|------|-------|-------|---------|
| MCP Runtime | `McpTransport` | `hkask-mcp` | MCP server communication |
| CLI | (clap derive) | `hkask-cli` | Command-line parsing |
| HTTP API | (axum routes) | `hkask-api` | HTTP request handling |
| ACP Transport | `AcpTransport` | `hkask-agents` | Agent Communication Protocol |

### 2.2 Driven Ports (Output)

| Port | Trait | Crate | Purpose |
|------|-------|-------|---------|
| ACP | `AcpPort` (`ports/acp.rs`) | `hkask-agents` | Agent registration, A2A messaging |
| Git CAS | `GitCASPort` (`ports/git_cas.rs`) | `hkask-agents` | Content-addressed template storage |
| Memory Storage | `MemoryStoragePort` (`ports/memory_storage.rs`) | `hkask-agents` | Episodic/semantic persistence |
| MCP Runtime | `MCPRuntimePort` (`ports/mcp_runtime.rs`) | `hkask-agents` | Tool dispatch |
| CNS Emit | `NuEventSink` (`event.rs`) | `hkask-types` | Cybernetic event emission |
| CNS Emit | `CnsEmit` (`spans.rs`) | `hkask-cns` | Structured span emission |
| Sovereignty | `SovereigntyChecker` (concrete) | `hkask-agents` | User sovereignty enforcement |
| Inference | `InferencePort` (`inference_port.rs`) | `hkask-templates` | LLM inference (Okapi) |
| Spec Store | `SpecStore` (`spec.rs`) | `hkask-types` | Specification persistence |
| Spec Observer | `SpecObserver` (`spec.rs`) | `hkask-types` | Spec CNS spans |
| Spec Curator | `SpecCurator` (`spec.rs`) | `hkask-types` | Curation evaluation |
| Audit | `AuditLogPort` (`audit.rs`) | `hkask-types` | Audit trail persistence |
| MCP Dispatch | `McpPort` (`ports.rs`) | `hkask-templates` | MCP tool invocation |
| Metacognition | `MetacognitionStoreAdapter` (concrete) | `hkask-agents` | Health snapshot persistence |
| Standing Session | `StandingSessionPort` (`ports/standing_session.rs`) | `hkask-agents` | Session state persistence |
| ACP Transport | `AcpTransport` (`ports/acp_transport.rs`) | `hkask-agents` | Wire-level transport |
| Ensemble Inference | `InferenceClient` (`ports.rs`) | `hkask-ensemble` | Inference with retry/circuit breaker |
| Ensemble Metrics | `MetricsSource` (`ports.rs`) | `hkask-ensemble` | SSE metrics streaming |
| Capability Query | `CapabilityQueryPort` (`ocap_enforcement.rs`) | `hkask-ensemble` | OCAP capability lookups |
| Registry | `RegistryIndex` (`ports.rs`) | `hkask-templates` | Template discovery |

### 2.3 Port Composition Patterns

**Adapter chaining:** Adapters wrap other adapters for cross-cutting concerns (e.g., `SecurityGateway` wraps `McpRuntime` to add OCAP verification).

**Port aggregation:** `PodManager` aggregates multiple ports into a single facade (`pod/manager.rs:30`).

**Optional CNS:** CNS emitter is `Option<Arc<dyn CnsEmit>>` to avoid breaking tests.

### 2.4 Async Purity

All ports use `#[async_trait]`. No `block_in_place` or `block_on` in library code.

---

## 3. Unified Registry

### 3.1 Architectural Invariant

**Rust is the loom. YAML/Jinja2 is the thread.** The loom doesn't change when you weave a different pattern.[^evans-ddd]

| Layer | Technology | Mutability |
|-------|-----------|------------|
| Hard (Kernel) | Rust | Fixed, stable |
| Soft (Material) | YAML, Jinja2, MD | Mutable, evolving |
| Testing | Rust (tests) | Verification edge |

**Rust owns:** Parsing YAML steps, rendering Jinja2 via minijinja, enforcing matroshka depth, validating hLexicon terms, routing MCP/LLM calls.

**Rust does NOT own:** Which templates exist, what they say, how selection logic is phrased, what steps a manifest contains.

[^evans-ddd]: Evans, E. (2003). *Domain-Driven Design*. Addison-Wesley. Code vs. content separation.

### 3.2 Registry Schema

**Single registry** with `template_type` discriminator — not three separate registries.[^fowler-poeaa]

**Implementation:** `SqliteRegistry` (`crates/hkask-templates/src/`)

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique template identifier |
| `template_type` | `TemplateType` | Prompt, Process, Cognition, Specification |
| `lexicon_terms` | `Vec<String>` | hLexicon terms used |
| `description` | `String` | Human-readable description |
| `source_path` | `String` | Filesystem path to source |
| `required_capabilities` | `Vec<String>` | Capabilities needed |

[^fowler-poeaa]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture*. Addison-Wesley. Registry pattern (pp. 490–494).

### 3.3 Template Types

| Type | Domain | Description | hLexicon Grounding |
|------|--------|-------------|-------------------|
| **Prompt** | WordAct | LLM prompt templates — "Say" | Rendering, expression terms |
| **Process** | FlowDef | Workflow templates — "Do" | Sequence, composition terms |
| **Cognition** | KnowAct | Reasoning templates — "Think" | Analysis, evaluation terms |
| **Specification** | FlowDef | DDMVSS spec templates — "Define" | Spec-curation terms (9) |

### 3.4 Cascade Rules

Template cascade follows the matroshka (nesting doll) pattern:

1. **Cascade depth ≤ 7** — prevents infinite recursion
2. **Manifest steps execute sequentially** — ordered ordinal execution
3. **Capability attenuation follows composition** — each level may attenuate
4. **Dependency graph validated** — `DependencyGraph` (`dependency.rs:21`) ensures acyclic

---

## 4. Template Rendering Pipeline

```mermaid
graph LR
    SELECT["Select<br/>template"] --> RESOLVE["Resolve<br/>dependencies"]
    RESOLVE --> ASSEMBLE["Assemble<br/>context"]
    ASSEMBLE --> VALIDATE["Validate<br/>capabilities"]
    VALIDATE --> RENDER["Render<br/>Jinja2"]
    RENDER --> AUDIT["Audit<br/>trail"]
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-002
verified_date: 2026-05-25
verified_against: crates/hkask-templates/src/renderer.rs:16; context_assembly.rs:126; capability_validator.rs:21; audit.rs:87
status: VERIFIED
-->

| Stage | Implementation | Purpose |
|-------|---------------|---------|
| Select | `SqliteRegistry.get()` | Retrieve by ID and type |
| Resolve | `DependencyGraph` + `SqliteRegistry` (`dependency.rs:21`) | Dependency graph, cascade |
| Assemble | `ContextAssembler` (`context_assembly.rs:126`) | Build context from fragments |
| Validate | `CapabilityAwareValidator` (`capability_validator.rs:21`) | Verify capabilities |
| Render | `TemplateRendererImpl` (`renderer.rs:16`) | Jinja2 via minijinja |
| Audit | `AuditTrail` (`audit.rs:87`) | Record execution + timing |

### 4.1 Manifest Step Grammar

Manifests define executable workflow steps following the Interpreter pattern:[^gamma-interpreter]

```yaml
steps:
  - ordinal: 1
    action: select          # Render selector template, call fast model
    template_ref: registry/templates/selector.j2
    model_tier: fast_local
    mcp: hkask-mcp-inference
  - ordinal: 2
    action: populate        # Bind input into selected template
    template_ref: "{{ selected_template_id }}"
  - ordinal: 3
    action: execute         # Submit to model/tool per contract
    target: from_template_contract
```

**~50 lines of Rust logic** interprets this grammar. Never changes when templates are added.

[^gamma-interpreter]: Gamma, E., et al. (1994). *Design Patterns*. Addison-Wesley. Interpreter pattern (pp. 243–255).

### 4.2 Inference Integration

LLM inference delegated to Okapi via `InferencePort`:

```rust
// crates/hkask-templates/src/inference_port.rs:116
pub trait InferencePort: Send + Sync {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError>;
    async fn models(&self) -> Result<Vec<ModelInfo>, InferenceError>;
}
```

**Implementation:** `OkapiInference` (`inference_port.rs:156`) — HTTP client to Okapi GGUF inference server.

**Resilience:** Circuit-breaker via `CircuitState` enum (`resilience.rs:12`) — Closed, Open, HalfOpen.

---

## 5. Multi-Agent Composition

### 5.1 Agent Pod Composition

See [`domain-and-capability.md`](domain-and-capability.md) §4 for pod lifecycle and composition.

### 5.2 Ensemble Coordination

`hkask-ensemble` (4,698 LOC) provides multi-agent chat:
- Multiple agents in shared conversation
- Each agent has own pod, capabilities, template context
- Ensemble coordination manages turn-taking and context sharing

### 5.3 ACP (Agent Communication Protocol)

Bidirectional federation via JSON-RPC 2.0 over stdio:
- `AcpPort` trait (`ports/acp.rs:18`) — bridge interface
- `AcpTransport` trait (`ports/acp_transport.rs:86`) — wire protocol
- `AcpWireMessage` / `AcpWireResponse` — message format

---

## References

[^cockburn-hex]: Cockburn, A. (2005). *Hexagonal Architecture*.
[^evans-ddd]: Evans, E. (2003). *Domain-Driven Design*. Addison-Wesley.
[^fowler-poeaa]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture*. Addison-Wesley.
[^gamma-interpreter]: Gamma, E., et al. (1994). *Design Patterns*. Addison-Wesley.
[^lewis-rag]: Lewis, P., et al. (2020). Retrieval-augmented generation for knowledge-intensive NLP tasks. *NeurIPS*, 33, 9459–9474.
[^vander-aalst-wf]: van der Aalst, W. M. P., et al. (2003). Workflow patterns. *Distributed and Parallel Databases*, 14(1), 5–51.
