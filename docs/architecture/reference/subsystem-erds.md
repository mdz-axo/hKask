---
title: "hKask Subsystem Entity Relationship Diagrams"
audience: [data architects, developers, agents]
last_updated: 2026-05-24
version: "1.0.0"
status: "Active"
domain: "Data"
ddmvss_categories: [persistence]
---

# hKask Subsystem Entity Relationship Diagrams

**Purpose:** Mermaid ERDs for all 11 core crates, grounded in actual Rust source types. Supplements [`hKask-erd.md`](hKask-erd.md) (conceptual model) and [`registry-erd.md`](registry-erd.md) (high-temp tables).

**Related:** [`hKask-erd.md`](hKask-erd.md), [`../interface-and-composition.md`](../interface-and-composition.md), [`../persistence-and-lifecycle.md`](../persistence-and-lifecycle.md)

---

## Contents

| Section | Description |
|---------|-------------|
| [§1 hkask-types — Foundation Types](#1-hkask-types--foundation-types) | Core ID types and ν-event foundation |
| [§2 hkask-agents — Pod Lifecycle & ACP](#2-hkask-agents--pod-lifecycle--acp) | Agent pod, WebID, capability tokens |
| [§3 hkask-ensemble — Multi-Agent Chat](#3-hkask-ensemble--multi-agent-chat--deliberation) | Multi-agent deliberation and chat sessions |
| [§4 hkask-memory — Episodic & Semantic](#4-hkask-memory--episodic--semantic-pipelines) | Memory pipelines and triple storage |
| [§5 hkask-mcp — MCP Runtime & Dispatch](#5-hkask-mcp--mcp-runtime--dispatch) | MCP server dispatch and tool invocation |
| [§6 hkask-api — HTTP API & OpenAPI](#6-hkask-api--http-api--openapi) | REST endpoints and OpenAPI spec |
| [§7 hkask-keystore — Keychain & Encryption](#7-hkask-keystore--keychain--encryption) | OS keychain integration and AES-256-GCM |
| [§8 hkask-storage — SQLite & SQLCipher](#8-hkask-storage--sqlite-persistence--sqlcipher) | Bitemporal triples, sqlite-vec, encryption |
| [§9 hkask-cns — Cybernetic Nervous System](#9-hkask-cns--cybernetic-nervous-system) | Spans, variety counters, algedonic alerts |
| [§10 hkask-templates — Registry & Cascade](#10-hkask-templates--registry-cascade--manifest-execution) | Template registry, cascade, and manifest execution |
| [§11 MCP Server Composite ERD](#11-mcp-server-composite-erd) | All 15 MCP servers in composite view |
| [§12 Cross-Crate Dependency Graph](#12-cross-crate-dependency-graph) | Workspace-wide crate dependency relationships |
| [References](#references) | Citations and references |

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

## 8. hkask-storage — SQLite Persistence & SQLCipher

Bitemporal triple store, embedding vectors, blob storage, and sovereignty boundaries. 17 public structs, 10 error enums, 1 trait.[^bitemporal]

```mermaid
erDiagram
    Database ||--o{ TripleStore : "owns"
    Database ||--o{ EmbeddingStore : "owns"
    Database ||--o{ BlobStore : "owns"
    Database ||--o{ NuEventStore : "owns"
    Database ||--o{ AuditLogStore : "owns"
    Database ||--o{ SqliteGoalRepository : "owns"
    Database ||--o{ SqliteSpecStore : "owns"

    TripleStore ||--o{ Triple : "stores"
    EmbeddingStore ||--o{ Embedding : "indexes"
    BlobStore ||--o{ Blob : "stores"
    NuEventStore ||--o{ NuEvent : "records"
    AuditLogStore ||--o{ AuditEntry : "logs"
    SqliteGoalRepository ||--o{ Goal : "manages"
    SovereigntyBoundaryStore ||--o{ SovereigntyBoundaryEntry : "enforces"

    Triple ||--o{ Embedding : "has_vector"
    Triple ||--o{ NuEvent : "produces"
    Triple }o--|| WebID : "owned_by"

    Database {
        Arc_Mutex_Connection conn
        bytes16 salt
    }

    Triple {
        TripleID id PK
        string entity
        string attribute
        json value
        DateTime valid_from
        DateTime valid_to
        float confidence
        WebID perspective FK
        Visibility visibility
        WebID owner_webid FK
    }

    Embedding {
        string id PK
        TripleID entity_ref FK
        array_f32 vector
        int dimensions
        string model
    }

    Blob {
        string id PK
        string content_type
        int size
        string blake3_hash
        bytes data
        Visibility visibility
        WebID owner_webid FK
    }

    AuditEntry {
        string id PK
        DateTime timestamp
        string actor_webid FK
        string action
        string resource
        string outcome
        json details
    }

    SovereigntyBoundaryEntry {
        string id PK
        string webid FK
        array sovereign_categories
        array shared_categories
        array public_categories
        string resistance
        float kill_zone_threshold
        i64 created_at
    }

    GoalJudgeResponse {
        string verdict
        string reason
        float confidence
    }

    SqliteGoalRepository {
        Arc_Connection conn
        bytes capability_secret
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-009
verified_date: 2026-05-24
verified_against: crates/hkask-storage/src/triples.rs; crates/hkask-storage/src/embeddings.rs; crates/hkask-storage/src/blobs.rs; crates/hkask-storage/src/nu_event_store.rs; crates/hkask-storage/src/audit_log.rs; crates/hkask-storage/src/goals.rs; crates/hkask-storage/src/sovereignty.rs; crates/hkask-storage/src/spec_store.rs
status: VERIFIED
-->

---

## 9. hkask-cns — Cybernetic Nervous System

Span emission, algedonic alerts, variety monitoring, energy budgets, sovereignty observation, and composition metrics. 20 public structs, 5 enums, 1 trait.[^beer-vsm]

```mermaid
erDiagram
    CnsRuntime ||--|| AlgedonicManager : "contains"
    CnsRuntime ||--|| VarietyMonitor : "contains"
    SpanEmitter }o--|| NuEventSink : "writes_to"

    AlgedonicManager ||--o{ RuntimeAlert : "emits"
    AlgedonicManager }o--o| EscalationCallback : "escalates_via"

    VarietyMonitor ||--o{ VarietyTracker : "tracks"
    EnergyEmitter ||--|| EnergyAccount : "manages"
    EnergyAccount ||--|| EnergyBudget : "constrained_by"
    EnergyAccount ||--o{ OpportunityCost : "records"

    CompositionObserver ||--|| CompositionObserverState : "holds"
    CompositionObserverState ||--|| CompositionMetrics : "tracks"
    CompositionObserverState ||--o{ VarietyMetrics : "monitors"
    CompositionObserverState ||--|| EnergyAccount : "accounts"

    SovereigntyObserver ||--|| SovereigntyObserverState : "holds"
    SovereigntyObserver ||--|| AlgedonicManager : "delegates_to"
    SovereigntyObserverState ||--o{ SovereigntyEvent : "records"

    GoalVarietyMonitor ||--o{ GoalVarietyCounter : "per_webid"
    RateLimiter ||--o{ CnsTokenBucket : "per_webid"
    ReviewQueue ||--o{ Violation : "queues"

    SpanEmitter {
        WebID observer_webid
        Option_NuEventSink sink
    }

    RuntimeAlert {
        string domain
        u64 deficit
        u64 threshold
        AlertSeverity severity
        bool escalated
        Instant timestamp
    }

    EnergyBudget {
        u64 cap
        u64 remaining
        float cost_per_token
        float alert_threshold
        bool hard_limit
    }

    EnergyAccount {
        string id
        EnergyBudget budget
        u64 total_allocated
        u64 total_consumed
    }

    CompositionMetrics {
        u64 total_attempts
        u64 successful_translations
        u64 failed_translations
        float energy_cost_variance
        u64 template_diversity
        u64 security_violations_blocked
    }

    VarietyMetrics {
        string entity_type
        u64 count
        u64 deficit
        u64 threshold
        bool alert_triggered
    }

    SovereigntyEvent {
        SovereigntyEventType event_type
        Instant timestamp
        WebID webid FK
        SovereigntyId sovereignty_id
        string data_category
    }

    Violation {
        uuid id PK
        WebID agent_id FK
        string violation_type
        string description
        DateTime occurred_at
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-010
verified_date: 2026-05-24
verified_against: crates/hkask-cns/src/spans.rs; crates/hkask-cns/src/algedonic.rs; crates/hkask-cns/src/variety.rs; crates/hkask-cns/src/energy.rs; crates/hkask-cns/src/runtime.rs; crates/hkask-cns/src/rate_limit.rs; crates/hkask-cns/src/review_queue.rs; crates/hkask-cns/src/observers/composition.rs; crates/hkask-cns/src/observers/sovereignty.rs; crates/hkask-cns/src/goal_variety.rs
status: VERIFIED
-->

---

## 10. hkask-templates — Registry, Cascade & Manifest Execution

Unified template registry, Jinja2 rendering, manifest step executor, cascade engine, curator pipeline, context assembly, and Okapi inference. 40+ structs, 10 enums, 9 traits.[^cockburn-hexagonal]

```mermaid
erDiagram
    Registry ||--o{ TemplateEntry : "indexes"
    TemplateEngine ||--|| TemplateRegistry : "wraps"
    TemplateEngine ||--|| Environment : "renders_via"

    ManifestExecutorImpl ||--|| TemplateRendererImpl : "renders_via"
    ManifestExecutorImpl ||--|| McpPort : "dispatches_via"
    ManifestExecutorImpl ||--|| CnsEmit : "observes_via"
    ManifestExecutorImpl ||--o| AppMemoryAdapter : "recalls_via"
    ManifestExecutorImpl ||--o| NoopCsp : "enforces_via"

    ContextAssembler ||--o{ ContextFragment : "assembles"
    ContextFragment }o--|| FragmentSource : "from"

    CuratorPipeline ||--o{ TemplateInvocation : "evaluates"
    CuratorPipeline ||--o{ CurationRecord : "records"
    CuratorPipeline ||--|| VarietyCounter : "checks"
    CuratorPipeline ||--o{ OCAPBoundary : "enforces"

    CascadeEngine ||--|| CascadeConfig : "configured_by"
    CascadeEngine ||--|| SpanEmitter : "emits_via"
    CascadeContext ||--|| CascadeLimits : "bounded_by"

    OkapiInference ||--o| CircuitBreaker : "protected_by"
    OkapiInference ||--o| RateLimiter : "throttled_by"
    OkapiInference ||--|| SpanEmitter : "observes_via"

    ContractValidator ||--|| OkapiCapabilities : "checks_against"
    CapabilityAwareValidator ||--|| OkapiCapabilities : "checks_against"

    DependencyGraph ||--o{ DependencyEdge : "tracks"
    AuditTrail ||--o{ ExecutionAudit : "records"
    ProvenanceManager ||--o{ TemplateProvenance : "tracks"

    ProcessManifest ||--|{ ManifestStep : "defines"
    ManifestStep }o--|| Action : "performs"

    TemplateEntry {
        string id PK
        TemplateType template_type
        string name
        string description
        array lexicon_terms
        string source_path
        int cascade_level
        int matroshka_limit
    }

    ProcessManifest {
        string id PK
        string name
        string description
        array steps
    }

    ManifestStep {
        int ordinal
        Action action
        string description
        string template_ref FK
        string model_tier
        string mcp
        string renderer
    }

    ContextFragment {
        string content
        FragmentSource source
        array_f32 embedding
        u8 priority
    }

    EvaluationResult {
        string invocation_id
        CurationDecision decision
        string rationale
        bool ocap_checked
        i64 variety_impact
    }

    ExecutionAudit {
        uuid id PK
        WebID bot_id FK
        string template_id FK
        string input_hash
        uuid outcome_event_id
        DateTime executed_at
        u64 duration_ms
        bool success
    }

    TemplateProvenance {
        string template_id FK
        string git_sha
        WebID modified_by FK
        DateTime modified_at
        string branch
    }

    InferenceResult {
        string text
        string model
        Usage usage
        string finish_reason
    }

    OkapiConfig {
        string base_url
        string api_key
        u64 timeout_secs
        int pool_max_idle
    }

    DependencyEdge {
        string caller
        string callee
        u8 depth
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-011
verified_date: 2026-05-24
verified_against: crates/hkask-templates/src/registry.rs; crates/hkask-templates/src/engine.rs; crates/hkask-templates/src/ports.rs; crates/hkask-templates/src/manifest.rs; crates/hkask-templates/src/cascade.rs; crates/hkask-templates/src/curator_pipeline.rs; crates/hkask-templates/src/context_assembly.rs; crates/hkask-templates/src/inference_port.rs; crates/hkask-templates/src/okapi_config.rs; crates/hkask-templates/src/resilience.rs; crates/hkask-templates/src/contract_validator.rs; crates/hkask-templates/src/capability_validator.rs; crates/hkask-templates/src/dependency.rs; crates/hkask-templates/src/audit.rs; crates/hkask-templates/src/provenance.rs
status: VERIFIED
-->

---

## 11. MCP Server Composite ERD

All 15 MCP servers share a thin-adapter pattern: each implements one or more port traits from `hkask-mcp` and delegates to an external service or internal crate. This composite ERD shows the shared structure and per-server specializations.[^mcp-spec]

```mermaid
erDiagram
    MCP_SERVER ||--|| MCP_TOOL : "provides"
    MCP_SERVER }o--|| HKASK_MCP_RUNTIME : "registered_in"

    INFERENCE_SERVER ||--|| OKAPI_CONNECTOR : "delegates_to"
    CONDENSER_SERVER ||--|| TEMPLATE_ABSTRACTION : "delegates_to"
    WEB_SERVER ||--|| FIRECRAWL_CONNECTOR : "delegates_to"

    OCAP_SERVER ||--|| CAPABILITY_MANAGER : "delegates_to"
    KEYSTORE_SERVER ||--|| OS_KEYCHAIN : "delegates_to"
    CNS_SERVER ||--|| SPAN_EMITTER : "delegates_to"
    GIT_SERVER ||--|| GIT_CAS : "delegates_to"
    REGISTRY_SERVER ||--|| TEMPLATE_REGISTRY : "delegates_to"
    GML_SERVER ||--|| GML_ENGINE : "delegates_to"
    SPEC_SERVER ||--|| SPEC_CAPTURE : "delegates_to"
    GITHUB_SERVER ||--|| GITHUB_API : "delegates_to"
    FMP_SERVER ||--|| FMP_API : "delegates_to"
    TELNYX_SERVER ||--|| TELNYX_API : "delegates_to"
    FAL_SERVER ||--|| FAL_API : "delegates_to"
    RSS_SERVER ||--|| RSS_PARSER : "delegates_to"

    MCP_SERVER {
        string id PK
        string name
        bool connected
    }

    MCP_TOOL {
        string name PK
        string description
        json input_schema
        string server_id FK
    }

    HKASK_MCP_RUNTIME {
        map servers
        map tool_registry
    }

    INFERENCE_SERVER {
        string id "hkask-mcp-inference"
        string purpose "Okapi LLM inference"
    }

    CONDENSER_SERVER {
        string id "hkask-mcp-condenser"
        string purpose "General-purpose context reranking and condensation"
    }

    WEB_SERVER {
        string id "hkask-mcp-web"
        string purpose "Search, scrape, extract"
    }



    OCAP_SERVER {
        string id "hkask-mcp-ocap"
        string purpose "Capability management"
    }

    KEYSTORE_SERVER {
        string id "hkask-mcp-keystore"
        string purpose "OS keychain operations"
    }

    CNS_SERVER {
        string id "hkask-mcp-cns"
        string purpose "CNS operations"
    }

    GIT_SERVER {
        string id "hkask-mcp-git"
        string purpose "Git CAS operations"
    }

    REGISTRY_SERVER {
        string id "hkask-mcp-registry"
        string purpose "Registry operations"
    }

    GML_SERVER {
        string id "hkask-mcp-gml"
        string purpose "GML allosteric engine"
    }

    SPEC_SERVER {
        string id "hkask-mcp-spec"
        string purpose "DDMVSS spec capture"
    }

    GITHUB_SERVER {
        string id "hkask-mcp-github"
        string purpose "GitHub integration"
    }

    FMP_SERVER {
        string id "hkask-mcp-fmp"
        string purpose "FMP integration"
    }

    TELNYX_SERVER {
        string id "hkask-mcp-telnyx"
        string purpose "Telnyx integration"
    }

    FAL_SERVER {
        string id "hkask-mcp-fal"
        string purpose "FAL integration"
    }

    RSS_SERVER {
        string id "hkask-mcp-rss-reader"
        string purpose "RSS feed reading"
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-SUBSYS-012
verified_date: 2026-05-24
verified_against: mcp-servers/hkask-mcp-inference/src/main.rs; mcp-servers/hkask-mcp-web/src/main.rs; mcp-servers/hkask-mcp-ocap/src/main.rs; Cargo.toml workspace members
status: VERIFIED
-->

---

## 12. Cross-Crate Dependency Graph

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

[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture*. <https://alistair.cockburn.us/hexagonal-architecture/>. The port/adapter pattern used throughout hkask-agents (GitCASPort, AcpPort, MemoryStoragePort, MCPRuntimePort, StandingSessionPort).

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model — the ensemble crate's multi-agent deliberation maps to Beer's System 4 (intelligence) and System 5 (policy).

[^tulving]: Tulving, E. (1972). Episodic and Semantic Memory. In E. Tulving & W. Donaldson (Eds.), *Organization of Memory* (pp. 381–403). Academic Press. The episodic/semantic distinction governs hkask-memory's architecture.

[^mcp-spec]: Anthropic. (2024). *Model Context Protocol Specification*. <https://modelcontextprotocol.io/specification>. The MCP runtime implements tool discovery, invocation, and capability-gated dispatch per this specification.

[^utoipa]: utoipa Contributors. (2024). *utoipa: Compile-time OpenAPI documentation*. <https://crates.io/crates/utoipa>. Used for compile-time OpenAPI spec generation from Rust types.

[^nist-sp800-132]: NIST. (2010). *Recommendation for Password-Based Key Derivation*. NIST Special Publication 800-132. <https://csrc.nist.gov/publications/detail/sp/800-132/final>. Argon2id parameters in hkask-keystore follow NIST guidance for memory-hard KDFs.

[^bitemporal]: Johnston, R., & Weis, T. (2018). *Bitemporal Data: Theory and Practice*. Morgan Kaufmann. The bitemporal triple schema in hkask-storage uses valid-time and transaction-time dimensions for full auditability.

---

*ℏKask — A Minimal Viable Container for Agents — v0.21.0*
*Every ERD grounded in Rust source. Every relationship verified against code.*
