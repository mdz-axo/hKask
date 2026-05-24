---
title: "hKask Subsystem Entity Relationship Diagrams"
audience: [data architects, developers, agents]
last_updated: 2026-05-24
togaf_phase: "C — Data"
version: "1.0.0"
status: "Active"
domain: "Data"
---

# hKask Subsystem Entity Relationship Diagrams

**Purpose:** Mermaid ERDs for all 11 core crates, grounded in actual Rust source types. Supplements [`hKask-erd.md`](hKask-erd.md) (conceptual model) and [`registry-erd.md`](registry-erd.md) (high-temp tables).

**Related:** [`hKask-erd.md`](hKask-erd.md), [`application-architecture.md`](application-architecture.md), [`data-architecture.md`](data-architecture.md)

---

## 1. hkask-types — Foundation Types

The type foundation: 76 public types across 16 modules. All ID types are newtype wrappers around `uuid::Uuid`.[^rust-newtype]

```mermaid
erDiagram
    WebID ||--o{ NuEvent : "observes"
    WebID ||--o{ CapabilityToken : "delegates"
    WebID ||--o{ Goal : "owns"
    WebID ||--o{ Bot : "identifies"
    WebID ||--o{ Replicant : "identifies"

    EventID ||--o{ NuEvent : "identifies"
    GoalID ||--o{ Goal : "identifies"
    GoalID ||--o{ GoalCriterion : "has"
    GoalID ||--o{ GoalArtifact : "produces"
    GoalID ||--o{ GoalCapabilityToken : "authorizes"

    SpecId ||--o{ Spec : "identifies"
    Spec ||--o{ GoalSpec : "contains"
    GoalSpec ||--o{ GoalSpec : "decomposes_into"
    GoalSpec ||--o{ Criterion : "requires"
    Spec ||--o{ SpecCurationRecord : "curated_by"

    TemplateID ||--o{ TemplateInvocation : "invokes"
    TemplateID ||--o{ CurationRecord : "evaluated_in"
    BotID ||--o{ TemplateInvocation : "invoked_by"

    NuEvent {
        EventID id PK
        DateTime timestamp
        WebID observer_webid FK
        Span span
        Phase phase
        json observation
        json regulation
        json outcome
        u8 recursion_depth
        EventID parent_event FK
        string visibility
    }

    Span {
        string Prompt
        string Tool
        string AgentPod
        string Connector
        string Pipeline
        string Energy
        string Review
        string Sovereignty
        string Goal
        string Spec
    }

    Phase {
        string Observe
        string Regulate
        string Outcome
    }

    Goal {
        GoalID id PK
        WebID webid FK
        string text
        GoalState state
        Visibility visibility
        DateTime created_at
        DateTime completed_at
        GoalID parent_goal_id FK
        u8 depth
    }

    GoalCriterion {
        string id
        GoalID goal_id FK
        string criterion_type
        string description
        bool satisfied
    }

    GoalArtifact {
        string id
        GoalID goal_id FK
        string artifact_ref
        string artifact_type
        DateTime created_at
    }

    CapabilityToken {
        string id PK
        CapabilityResource resource
        string resource_id
        CapabilityAction action
        WebID delegated_from FK
        WebID delegated_to FK
        string signature
        i64 expires_at
        u8 attenuation_level
        u8 max_attenuation
        string context_nonce
    }

    Spec {
        SpecId id PK
        string name
        SpecCategory category
        DomainAnchor domain_anchor
        GoalSpec goals
        WebID signed_by FK
        DateTime created_at
    }

    TemplateInvocation {
        TemplateID id PK
        TemplateID template_id FK
        BotID bot_id FK
        f32 temperature
        LLMParameters parameters
        json input
        json outputs
        usize selected_index
        TemplateOutcome outcome
        DateTime timestamp
    }

    LexiconTerm {
        string term PK
        Domain domain
        string definition
        string academic_citation
    }

    DataSovereigntyBoundary {
        SovereigntyId id PK
        set sovereign_data
        set shared_data
        set public_data
        AcquisitionResistance resistance
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-001
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/id.rs; crates/hkask-types/src/event.rs; crates/hkask-types/src/goal.rs; crates/hkask-types/src/capability.rs; crates/hkask-types/src/spec.rs; crates/hkask-types/src/template.rs; crates/hkask-types/src/lexicon.rs; crates/hkask-types/src/sovereignty.rs
status: VERIFIED
-->

---

## 2. hkask-agents — Pod Lifecycle & ACP

Agent pods, ACP runtime, OCAP delegation, and sovereignty enforcement. 36 public structs, 12 enums, 9 traits.[^cockburn-hexagonal]

```mermaid
erDiagram
    PodManager ||--o{ AgentPod : "manages"
    AgentPod ||--|| AgentPersona : "has"
    AgentPod ||--|| TemplateCrate : "loaded_from"
    AgentPod ||--|| CapabilityToken : "authorized_by"
    AgentPod }o--|| PodLifecycleState : "in_state"

    AgentPersona ||--|| AgentIdentity : "identifies"
    AgentPersona ||--|| AgentCharter : "chartered_by"
    AgentPersona ||--o{ AccessRight : "grants"

    TemplateCrate ||--o{ TemplateFile : "contains"

    AcpRuntime ||--o{ AcpAgent : "registers"
    AcpRuntime ||--o{ A2AMessage : "routes"
    AcpRuntime ||--o{ AuditLogEntry : "logs"
    AcpRuntime ||--|| RootAuthority : "rooted_in"

    Bot ||--|| BotCapabilities : "has"
    Replicant ||--|| ReplicantCapabilities : "has"

    SovereigntyChecker ||--|| UserSovereigntyState : "enforces"
    ConsentManager ||--o{ ConsentRecord : "tracks"
    EscalationQueue ||--o{ EscalationEntry : "queues"

    OCAP ||--o{ AttenuationHistory : "tracks"
    AttenuationHistory ||--o{ AttenuationEntry : "chain"

    AgentPod {
        PodID id PK
        WebID webid FK
        AgentType agent_type
        AgentPersona persona
        TemplateCrate template_crate
        CapabilityToken capability_token
        PodLifecycleState state
        i64 created_at
        u8 max_attenuation
    }

    PodLifecycleState {
        string Populated
        string Registered
        string Activated
        string Deactivated
    }

    AgentType {
        string Bot
        string Replicant
    }

    AcpAgent {
        WebID webid PK
        string agent_type
        array capabilities
        i64 registered_at
        bool active
    }

    A2AMessage {
        string message_type
        WebID from FK
        WebID to FK
        string template_id
        json input
        string correlation_id
    }

    AuditLogEntry {
        string id PK
        i64 timestamp
        WebID from FK
        WebID to FK
        string message_type
        string correlation_id
        string event_type
        json metadata
    }

    EscalationEntry {
        string id PK
        TemplateID template_id FK
        BotID bot_id FK
        string output
        f64 confidence
        u32 retry_count
        string error_context
        DateTime created_at
        EscalationStatus status
        DateTime resolved_at
    }

    ConsentRecord {
        string webid FK
        set granted_categories
        i64 granted_at
        i64 revoked_at
        bool active
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-002
verified_date: 2026-05-24
verified_against: crates/hkask-agents/src/pod.rs; crates/hkask-agents/src/acp.rs; crates/hkask-agents/src/bot.rs; crates/hkask-agents/src/replicant.rs; crates/hkask-agents/src/consent.rs; crates/hkask-agents/src/curator/escalation.rs
status: VERIFIED
-->

---

## 3. hkask-ensemble — Multi-Agent Chat & Deliberation

Chat coordination, deliberation sessions, confidence routing, macaroon capabilities, and Okapi integration. 42 structs, 19 enums, 6 traits.[^beer-vsm]

```mermaid
erDiagram
    EnsembleChatManager ||--o{ EnsembleChat : "manages"
    EnsembleChat ||--o{ ChatParticipant : "includes"
    EnsembleChat ||--o{ ChatMessage : "contains"

    DeliberationCoordinator ||--o{ DeliberationSession : "coordinates"
    DeliberationSession ||--o{ AgentResponse : "collects"
    DeliberationSession ||--o{ ChatParticipant : "involves"

    WebIDCapabilityRegistry ||--o{ WebIDCapabilityEntry : "registers"
    WebIDCapabilityEntry ||--o{ OkapiCapability : "holds"
    OkapiCapability ||--|| Macaroon : "secured_by"
    Macaroon ||--o{ Caveat : "constrained_by"

    OcapEnforcer }o--|| WebIDCapabilityRegistry : "queries"

    ConfidenceRouter }o--|| InferenceClient : "routes_via"
    ResilientOkapiClient }o--|| CircuitBreaker : "protected_by"
    CnsIntegration ||--|| SpanEmitter : "emits_via"

    ChatMessage {
        WebID from FK
        string content
        DateTime timestamp
        string template_id
    }

    ChatParticipant {
        WebID webid PK
        ParticipantRole role
        string pod_id
    }

    DeliberationSession {
        string session_id PK
        array participants
        map responses
        DeliberationStatus status
    }

    AgentResponse {
        WebID agent_webid FK
        string content
        f64 confidence
        string template_used
        u64 processing_time_ms
    }

    Macaroon {
        string location
        string identifier
        array caveats
        bytes32 signature
    }

    OkapiCapability {
        CapabilityId id PK
        WebID issuer FK
        WebID holder FK
        Macaroon macaroon
        TemplateID template_id FK
        DateTime expires_at
        Visibility visibility
    }

    WebIDCapabilityEntry {
        WebID webid PK
        array capabilities
        DateTime created_at
        DateTime last_used_at
        bool active
    }

    CircuitBreaker {
        CircuitState state
        u32 failure_count
        u32 success_count
        Instant last_failure_time
        CircuitBreakerConfig config
        string name
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-003
verified_date: 2026-05-24
verified_against: crates/hkask-ensemble/src/chat.rs; crates/hkask-ensemble/src/deliberation.rs; crates/hkask-ensemble/src/macaroon.rs; crates/hkask-ensemble/src/capability.rs; crates/hkask-ensemble/src/webid_registry.rs; crates/hkask-ensemble/src/resilience.rs
status: VERIFIED
-->

---

## 4. hkask-memory — Episodic & Semantic Pipelines

Memory taxonomy grounded in Tulving's episodic/semantic distinction.[^tulving] 7 structs, 3 enums, 1 trait.

```mermaid
erDiagram
    EpisodicMemory }o--|| TripleStore : "stores_in"
    SemanticMemory }o--|| TripleStore : "stores_in"
    SemanticMemory }o--|| EmbeddingStore : "indexes_in"
    GoalMemory ||--o{ GoalSemanticMemory : "records_semantic"
    GoalMemory ||--o{ GoalEpisodicMemory : "records_episodic"

    EpisodicMemory {
        TripleStore triple_store
    }

    SemanticMemory {
        TripleStore triple_store
        EmbeddingStore embedding_store
    }

    GoalSemanticMemory {
        GoalID goal_id FK
        WebID webid FK
        string goal_text
        string completion_state
        usize artifact_count
        string created_at
        string completed_at
    }

    GoalEpisodicMemory {
        GoalID goal_id FK
        WebID webid FK
        string experience
        string outcome_summary
        array lessons_learned
        string timestamp
    }

    GoalMemory {
        WebID agent_webid FK
        map semantic_store
        map episodic_store
    }

    DedupResult {
        array triples
        usize original_count
        usize duplicates_removed
    }

    BayesianOps {
        fn combine
        fn retract
        fn join
        fn decay
        fn weighted_average
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-004
verified_date: 2026-05-24
verified_against: crates/hkask-memory/src/episodic.rs; crates/hkask-memory/src/semantic.rs; crates/hkask-memory/src/goal_memory.rs; crates/hkask-memory/src/bayesian.rs; crates/hkask-memory/src/recall_dedup.rs
status: VERIFIED
-->

---

## 5. hkask-mcp — MCP Runtime & Dispatch

Tool registration, capability-gated dispatch, security gateway, and archival service. 12 structs, 1 enum.[^mcp-spec]

```mermaid
erDiagram
    McpRuntime ||--o{ McpServer : "registers"
    McpServer ||--o{ McpTool : "provides"

    McpDispatcher }o--|| McpRuntime : "discovers_via"
    McpDispatcher }o--|| CapabilityChecker : "validates_via"
    McpDispatcher }o--|| RateLimiter : "throttles_via"

    SecurityGateway ||--|| SecurityPolicy : "enforces"
    SecurityGateway ||--o{ AuditEntry : "logs"
    SecurityGateway }o--|| CapabilityChecker : "checks_via"

    ArchivalService }o--|| AdapterContainer : "uses"
    ArchivalService }o--|| SovereigntyChecker : "enforces"
    ArchivalService }o--|| SpanEmitter : "observes_via"

    AdapterContainer }o--o| GitCasAdapter : "wraps"

    McpRuntime {
        map servers
        map tool_registry
    }

    McpServer {
        string id PK
        string name
        array tools
        bool connected
    }

    McpTool {
        string name PK
        string description
        json input_schema
        string server_id FK
    }

    McpDispatcher {
        McpRuntime runtime
        CapabilityChecker capability_checker
        RateLimiter rate_limiter
        map bot_capabilities
        McpMcpRetryConfig retry_config
    }

    SecurityPolicy {
        usize max_input_size
        set allowed_tools
        set denied_tools
        bool require_capabilities
        bool enable_rate_limiting
    }

    AuditEntry {
        DateTime timestamp
        WebID bot_id FK
        string tool_name
        AuditAction action
        bool success
        string error_message
    }

    ArchivalService {
        AdapterContainer adapter_container
        SovereigntyChecker sovereignty_checker
        SpanEmitter span_emitter
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-005
verified_date: 2026-05-24
verified_against: crates/hkask-mcp/src/runtime.rs; crates/hkask-mcp/src/dispatch.rs; crates/hkask-mcp/src/security.rs; crates/hkask-mcp/src/adapter_container.rs; crates/hkask-mcp/src/archival_service.rs
status: VERIFIED
-->

---

## 6. hkask-api — HTTP API & OpenAPI

28+ request/response models served through axum with utoipa OpenAPI generation. 39 structs, 3 enums.[^utoipa]

```mermaid
erDiagram
    ApiState }o--|| SqliteRegistry : "templates"
    ApiState }o--|| McpRuntime : "tools"
    ApiState }o--|| PodManager : "pods"
    ApiState }o--|| CapabilityChecker : "capabilities"
    ApiState }o--|| SpanEmitter : "observability"
    ApiState }o--|| RateLimiter : "throttling"
    ApiState }o--o| OkapiHttpClient : "inference"
    ApiState }o--o| SpecStore : "specs"

    ChatRequest ||--|| ChatResponse : "request_response"
    SoapInferAuthRequest ||--|| SoapInferResponse : "request_response"
    CreatePodRequest ||--|| CreatePodResponse : "request_response"
    SpecCaptureRequest ||--|| SpecCaptureResponse : "request_response"
    AcpRegisterRequest ||--|| AcpRegisterResponse : "request_response"

    SoapInferAuthRequest ||--|| SoapInferRequest : "wraps"
    SoapInferRequest ||--|| ObjectiveData : "contains"
    ObjectiveData ||--|| SeverityCounts : "counts"
    ObjectiveData ||--o{ EventRecord : "events"

    ApiState {
        Arc registry
        Arc mcp_runtime
        Arc pod_manager
        Arc capability_checker
        WebID system_webid
        Arc cns_emitter
        Arc rate_limiter
        Arc ensemble_inferencer
        Arc spec_store
    }

    ChatRequest {
        string input
        string template_id
    }

    SoapInferRequest {
        string subjective
        ObjectiveData objective
        string assessment
        string plan
    }

    SoapInferResponse {
        string response
        string model
        u64 latency_ms
        array actions
    }

    ObjectiveData {
        SeverityCounts severity_counts
        array recent_events
    }

    SeverityCounts {
        u64 crit
        u64 alert
        u64 warn
        u64 info
    }

    EventRecord {
        string probe
        string severity
        string message
        string ts
    }

    PodStatusResponse {
        string pod_id
        string name
        string state
        string webid
        string agent_type
        string template
        i64 created_at
    }

    SpecCaptureRequest {
        string description
        string category
        string domain_anchor
        array criteria
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-006
verified_date: 2026-05-24
verified_against: crates/hkask-api/src/lib.rs; crates/hkask-api/src/routes.rs; crates/hkask-api/src/openapi.rs
status: VERIFIED
-->

---

## 7. hkask-keystore — Keychain & Encryption

OS keychain integration and AES-256-GCM encryption with Argon2id key derivation. 3 structs, 2 enums.[^nist-sp800-132]

```mermaid
erDiagram
    Keychain }o--|| WebID : "keyed_by"
    Keychain }o--|| KeychainError : "returns"

    KeyRing ||--|| EncryptionService : "provides_key_to"
    EncryptionService }o--|| EncryptionError : "returns"

    SecretRef ||--|| Keychain : "resolves_via"

    Keychain {
        string service_name
    }

    KeyRing {
        bytes32 key
    }

    EncryptionService {
        Aes256Gcm cipher
    }

    KeychainError {
        string Platform
        string NotFound
        string Encryption
    }

    EncryptionError {
        string KeyDerivation
        string Encryption
        string Decryption
        string InvalidPassphrase
    }

    SecretRef {
        string Env
        string Keychain
        u32 Generated
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-007
verified_date: 2026-05-24
verified_against: crates/hkask-keystore/src/keychain.rs; crates/hkask-keystore/src/encryption.rs
status: VERIFIED
-->

---

## 8. Cross-Crate Dependency Graph

```mermaid
graph TB
    subgraph "Foundation"
        TYPES[hkask-types<br/>76 public types]
        KEYSTORE[hkask-keystore<br/>3 structs]
    end

    subgraph "Storage"
        STORAGE[hkask-storage<br/>SQLite + SQLCipher]
    end

    subgraph "Domain Logic"
        MEMORY[hkask-memory<br/>7 structs]
        CNS[hkask-cns<br/>observability]
        TEMPLATES[hkask-templates<br/>registry + cascade]
    end

    subgraph "Agent Layer"
        AGENTS[hkask-agents<br/>36 structs]
        ENSEMBLE[hkask-ensemble<br/>42 structs]
    end

    subgraph "Interface Layer"
        MCP[hkask-mcp<br/>12 structs]
        API[hkask-api<br/>39 structs]
        CLI[hkask-cli<br/>commands]
    end

    TYPES --> STORAGE
    TYPES --> KEYSTORE
    TYPES --> CNS
    TYPES --> MEMORY
    TYPES --> TEMPLATES
    TYPES --> AGENTS
    TYPES --> ENSEMBLE
    TYPES --> MCP
    TYPES --> API

    STORAGE --> MEMORY
    STORAGE --> TEMPLATES
    STORAGE --> AGENTS

    KEYSTORE --> AGENTS
    KEYSTORE --> ENSEMBLE

    CNS --> TEMPLATES
    CNS --> AGENTS
    CNS --> ENSEMBLE
    CNS --> MCP

    MEMORY --> AGENTS

    TEMPLATES --> AGENTS
    TEMPLATES --> MCP

    AGENTS --> ENSEMBLE
    AGENTS --> MCP
    AGENTS --> API

    MCP --> API
    MCP --> CLI

    ENSEMBLE --> API
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-008
verified_date: 2026-05-24
verified_against: Cargo.toml workspace dependencies; crates/*/src/lib.rs import analysis
status: VERIFIED
-->

---

## References

[^rust-newtype]: The Rust Project. (2024). *Rust API Guidelines — Newtype pattern*. <https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype>. The newtype pattern used for all ID types ensures type safety across UUID-based identifiers.

[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture*. <https://alistair.cockburn.us/hexagonal-architecture/>. The port/adapter pattern used throughout hkask-agents (GitCASPort, AcpPort, MemoryStoragePort, MCPRuntimePort, KeystorePort, SovereigntyPort).

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model — the ensemble crate's multi-agent deliberation maps to Beer's System 4 (intelligence) and System 5 (policy).

[^tulving]: Tulving, E. (1972). Episodic and Semantic Memory. In E. Tulving & W. Donaldson (Eds.), *Organization of Memory* (pp. 381–403). Academic Press. The episodic/semantic distinction governs hkask-memory's architecture.

[^mcp-spec]: Anthropic. (2024). *Model Context Protocol Specification*. <https://modelcontextprotocol.io/specification>. The MCP runtime implements tool discovery, invocation, and capability-gated dispatch per this specification.

[^utoipa]: utoipa Contributors. (2024). *utoipa: Compile-time OpenAPI documentation*. <https://crates.io/crates/utoipa>. Used for compile-time OpenAPI spec generation from Rust types.

[^nist-sp800-132]: NIST. (2010). *Recommendation for Password-Based Key Derivation*. NIST Special Publication 800-132. <https://csrc.nist.gov/publications/detail/sp/800-132/final>. Argon2id parameters in hkask-keystore follow NIST guidance for memory-hard KDFs.

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Every ERD grounded in Rust source. Every relationship verified against code.*
