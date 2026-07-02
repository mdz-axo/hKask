---
title: "Class Diagram — Service Layer Decomposition"
audience: [architects, developers]
last_updated: 2026-06-30
version: "0.31.0"
status: "Active"
domain: "Composition"
mds_categories: ["composition", "domain"]
diataxis: "class-diagram"
source: "hkask-services-core through hkask-services-wallet/src/lib.rs, hkask-ports/src/lib.rs"
---

# Service Layer Class Diagram

The hKask service layer comprises **11 subcrates** decomposed from the original monolithic `hkask-services-core` crate, following the Strangler Fig pattern (archived ADR-040). Every subcrate depends on `hkask-services-core` as its universal foundation. `hkask-services-context` provides `AgentService`, the canonical DI container that assembles all shared infrastructure (CNS, governance, storage, infra). Domain services (chat, curator, compose, skill, kata-kanban, corpus, wallet, onboarding, runtime) are thin orchestrators that delegate to domain crates via `AgentService` or port traits.

```mermaid
classDiagram
    direction TB

    %% ── Ports (Hexagonal Interfaces) ──────────────────────────────────────
    namespace ports {
        class InferencePort {
            <<interface>>
            +infer(prompt, params) InferenceResult
            +infer_stream(prompt, params) Stream
        }
        class ToolPort {
            <<interface>>
            +name() String
            +description() String
            +execute(input) Result
        }
        class CircuitBreakerPort {
            <<interface>>
            +check() Outcome
            +record_success()
            +record_failure()
        }
        class CnsObserver {
            <<interface>>
            +on_event(event) bool
        }
        class RegistryIndex {
            <<interface>>
            +list_entries() Vec~RegistryEntry~
            +get(name) Option~RegistryEntry~
        }
        class SkillRegistryIndex {
            <<interface>>
            +list_skills() Vec~Skill~
            +get_skill(name) Option~Skill~
        }
        class FederationDispatch {
            <<interface>>
            +register_peer(replica, domain, matrix_domain, matrix_id)
            +invite(peer) Result
            +accept(peer) Result
            +reject(peer) Result
            +pause(peer, reason) Result
            +resume(peer) Result
            +revoke(peer, reason) Result
            +leave(reason) Result
            +linked_peers() Vec~ReplicaId~
            +link_state(peer) Option~String~
        }
        class CnsStoragePort {
            <<interface>>
            +query_algedonic(since, limit) Result~Vec~NuEvent~~
        }
        class FederationTransport {
            <<interface>>
            +send(peer, message) Result
            +recv() Result
            +simulate_partition(peer)
            +heal_partition(peer)
        }
        class FederationSyncPort {
            <<interface>>
            +query_public_since(cursor, limit) Result~Vec~FederatedTriple~~
            +cursor_for(source) u64
            +advance_cursor(source, cursor)
        }
    }

    %% ── Foundation ────────────────────────────────────────────────────────
    namespace services_foundation {
        class ServiceError {
            +enum ServiceError
            GoalNotFound, Escalation, EscalationNotFound
            Metacognition, Inference
        }
        class ServiceConfig {
            +default_model: String
            +inference_config: InferenceConfig
            +db_path: PathBuf
            +load() ServiceConfig
        }
        class HkaskSettings {
            +enabled_features: Vec~String~
            +save()
        }
        class InferenceContext {
            +shared_port: Arc~dyn InferencePort~
            +default_model: String
        }
    }

    %% ── Context (DI Container) ────────────────────────────────────────────
    namespace services_context {
        class AgentService {
            -infra: InfraContext
            -governance: GovernanceContext
            -cns: CnsContext
            -storage: StorageContext
            -system_webid: WebID
            +build(config) AgentService
            +config() &ServiceConfig
            +webid() &WebID
            +governance() &GovernanceContext
            +infra() &InfraContext
            +cns() &CnsContext
            +storage() &StorageContext
            +identity() (&WebID, &A2ARuntime)
            +curator_ready() Result
            +build_per_agent_memory(db) PerAgentMemory
        }
        class PerAgentMemory {
            +episodic_storage: Arc~dyn EpisodicStoragePort~
            +semantic_storage: Arc~dyn SemanticStoragePort~
            +consolidation_service: ConsolidationService
        }
    }

    %% ── Chat Service ──────────────────────────────────────────────────────
    namespace services_chat {
        class ChatService {
            +chat(ctx, request) ChatTurnResponse
            +prepare_chat(ctx, bot_id) PreparedChat
        }
        class MemoryService {
            +has_memory_consent(ctx, owner, category) bool
            +recall_semantic(port, input, token) Option~String~
            +recall_episodic(port, input, token) Vec~RecalledEpisode~
            +store_episode(port, episode)
            +paired_recall(episodic, semantic) Vec~RecalledEpisode~
        }
        class ChatTurnRequest {
            +prompt: String
            +bot_id: String
            +model_override: Option~String~
        }
        class ChatTurnResponse {
            +response: String
            +tool_calls: Vec~StructuredToolCall~
            +token_usage: TokenUsage
        }
    }

    %% ── Compose Service ───────────────────────────────────────────────────
    namespace services_compose {
        class ComposeService {
            +compose(request) ComposeResult
        }
        class ComposeRequest {
            +prompt: String
            +db_path: PathBuf
            +cognition: CognitionConfig
            +inference_ctx: InferenceContext
        }
        class ComposeResult {
            +generated_prose: String
            +exemplar_count: u32
            +validation: Option~CentroidValidation~
        }
    }

    %% ── Curator Service ───────────────────────────────────────────────────
    namespace services_curator {
        class CuratorService {
            +list_escalations(ctx) Vec~EscalationResponse~
            +resolve(ctx, id, resolved_by) Result
            +dismiss(ctx, id, dismissed_by) Result
            +metacognition(ctx) Result~String~
        }
        class EscalationResponse {
            +id: String
            +template_id: String
            +status: String
            +confidence: f64
        }
    }

    %% ── Kata-Kanban Service ───────────────────────────────────────────────
    namespace services_kata_kanban {
        class KataEngine {
            -inference: Arc~dyn InferencePort~
            -registry: Arc~dyn RegistryIndex~
            -history: KataHistory
            +from_env() KataEngine
            +load_manifest(path) KataManifest
            +run_bundle(bundle) KataResult
            +execute(state) KataResult
        }
        class KanbanService {
            +create_board(owner, name) Board
            +add_task(board_id, spec) Task
            +move_task(task_id, status) Task
            +verify_task(task_id, criterion) Verification
            +dejam(board_id) Vec~UnjamItem~
        }
        class Board {
            +board_id: BoardId
            +name: String
            +owner: WebID
            +columns: Vec~ColumnDef~
        }
        class Task {
            +task_id: TaskId
            +title: String
            +status: TaskStatus
            +priority: Priority
            +owner: WebID
        }
    }

    %% ── Runtime Services ──────────────────────────────────────────────────
    namespace services_runtime {
        class ServiceDaemonHandler {
            -pod_manager: Arc~ActivePods~
            -user_store: Arc~Mutex~UserStore~~
            +handle_assign(request) Result
            +handle_capability(query) Result
        }
        class ProviderIntelligence {
            +fetch_state(provider) ProviderState
            +usage_status(provider) UsageStatus
        }
    }

    %% ── Skill Service ─────────────────────────────────────────────────────
    namespace services_skill {
        class SkillAuditor {
            -registry: &dyn RegistryIndex
            -skill_index: &dyn SkillRegistryIndex
            +audit_all() SkillAuditReport
        }
        class SkillAuditReport {
            +health_score: SkillHealthScore
            +defects: Vec~Defect~
        }
        class BundleService {
            +compose(skill_ids) BundleComposeResult
            +evolve(bundle, delta) BundleComposeResult
        }
    }

    %% ── Wallet Service ────────────────────────────────────────────────────
    namespace services_wallet {
        class WalletService {
            -manager: Arc~WalletManager~
            -issuer: Arc~ApiKeyIssuer~
            +build(config, store, sink) WalletService
            +balance() WalletBalance
            +deposit_address() DepositAddress
            +withdraw(amount, to) TxHash
            +create_api_key(caps) ApiKeyMaterial
        }
    }

    %% ── Onboarding Service ────────────────────────────────────────────────
    namespace services_onboarding {
        class OnboardingService {
            +resolve_secrets() ResolvedSecrets
            +register_matrix(config) MatrixRegistrationResult
            +sign_in(user) SignInOutcome
        }
    }

    %% ── Corpus Service ────────────────────────────────────────────────────
    namespace services_corpus {
        class DiscoveryService {
            +discover(request) DiscoverResult
        }
        class EmbedService {
            +embed(config) EmbedResult
            +embed_progress() EmbedProgress
        }
        class DiscoverResult {
            +works: Vec~DiscoveredWork~
        }
        class EmbedResult {
            +phase: EmbedPhase
            +entities: Vec~Entity~
        }
    }

    %% ── Surfaces (Consumers) ──────────────────────────────────────────────
    namespace surfaces {
        class CLI {
            <<binary>>
            +main()
        }
        class API {
            <<server>>
            +routes()
            +openapi_spec()
        }
    }

    %% ═══ DEPENDENCY RELATIONSHIPS ═════════════════════════════════════════

    %% Foundation: every service crate depends on core
    services_chat::ChatService ..> services_foundation::ServiceError : uses
    services_chat::ChatService ..> services_foundation::InferenceContext : uses
    services_compose::ComposeService ..> services_foundation::ServiceError : uses
    services_compose::ComposeRequest o-- services_foundation::InferenceContext : composes
    services_curator::CuratorService ..> services_foundation::ServiceError : uses
    services_kata_kanban::KataEngine ..> services_foundation::ServiceError : uses
    services_runtime::ServiceDaemonHandler ..> services_foundation::ServiceError : uses
    services_skill::SkillAuditor ..> services_foundation::ServiceError : uses
    services_skill::BundleService ..> services_foundation::ServiceError : uses
    services_wallet::WalletService ..> services_foundation::ServiceError : uses
    services_onboarding::OnboardingService ..> services_foundation::ServiceError : uses
    services_corpus::DiscoveryService ..> services_foundation::ServiceError : uses

    %% Context (DI container): services parameterized on AgentService
    services_chat::ChatService ..> services_context::AgentService : takes &AgentService
    services_chat::MemoryService ..> services_context::AgentService : takes &AgentService
    services_curator::CuratorService ..> services_context::AgentService : takes &AgentService
    services_skill::BundleService ..> services_context::AgentService : takes &AgentService
    services_context::AgentService ..> services_context::PerAgentMemory : build_per_agent_memory()
    services_context::AgentService o-- services_foundation::ServiceConfig : config

    %% Context embeds wallet and daemon in InfraContext
    services_context::AgentService o-- services_wallet::WalletService : wallet (optional)
    services_context::AgentService o-- services_runtime::ServiceDaemonHandler : daemon

    %% Port dependency: core, context, and services use port traits
    services_foundation::InferenceContext o-- "1" ports::InferencePort : shared_port
    services_kata_kanban::KataEngine o-- "1" ports::InferencePort : inference
    services_kata_kanban::KataEngine o-- "1" ports::RegistryIndex : registry
    services_skill::SkillAuditor o-- "1" ports::RegistryIndex : registry
    services_skill::SkillAuditor o-- "1" ports::SkillRegistryIndex : skill_index
    services_skill::BundleService ..> ports::InferencePort : uses
    services_skill::BundleService ..> ports::SkillRegistryIndex : uses

    %% Surfaces depend on service subcrates
    surfaces::CLI ..> services_context::AgentService : uses
    surfaces::CLI ..> services_chat::ChatService : uses
    surfaces::CLI ..> services_curator::CuratorService : uses
    surfaces::CLI ..> services_compose::ComposeService : uses
    surfaces::CLI ..> services_wallet::WalletService : uses
    surfaces::CLI ..> services_skill::SkillAuditor : uses
    surfaces::CLI ..> services_skill::BundleService : uses
    surfaces::CLI ..> services_kata_kanban::KataEngine : uses
    surfaces::CLI ..> services_kata_kanban::KanbanService : uses
    surfaces::CLI ..> services_onboarding::OnboardingService : uses
    surfaces::CLI ..> services_corpus::DiscoveryService : uses
    surfaces::CLI ..> services_corpus::EmbedService : uses
    surfaces::CLI ..> services_runtime::ProviderIntelligence : uses
    surfaces::CLI ..> services_foundation::ServiceConfig : uses

    surfaces::API ..> services_context::AgentService : uses
    surfaces::API ..> services_chat::ChatService : uses
    surfaces::API ..> services_curator::CuratorService : uses
    surfaces::API ..> services_wallet::WalletService : uses
    surfaces::API ..> services_skill::SkillAuditor : uses
    surfaces::API ..> services_foundation::ServiceConfig : uses

    %% Runtime depends on context
    services_runtime::ProviderIntelligence ..> services_foundation::ServiceError : uses
    services_corpus::EmbedService ..> services_runtime::ServiceDaemonHandler : uses
```

---

## DIAGRAM_ALIGNMENT

| Field | Value |
|-------|-------|
| **ID** | `DIAG-IC-008` |
| **Verified Date** | 2026-06-30 |
| **Verified Against** | `crates/hkask-services-core through hkask-services-wallet/src/lib.rs`, `crates/hkask-ports/src/lib.rs`, `crates/hkask-services-core through hkask-services-wallet/Cargo.toml`, `crates/hkask-cli/Cargo.toml`, `crates/hkask-api/Cargo.toml` |
| **Status** | `VERIFIED` |

### Verification checklist

- [x] 11 subcrates enumerated (core, chat, compose, context, curator, kata-kanban, runtime, skill, wallet, onboarding, corpus)
- [x] Key public structs per subcrate matched to `lib.rs` re-exports
- [x] Port traits (`<<interface>>`) from `hkask-ports/src/lib.rs` verified
- [x] CLI deps: 11 `hkask-services-core through hkask-services-wallet` crates in `hkask-cli/Cargo.toml` lines 36–46
- [x] API deps: 6 `hkask-services-core through hkask-services-wallet` crates in `hkask-api/Cargo.toml` lines 19–25
- [x] Core foundation: every service subcrate depends on `hkask-services-core`
- [x] Context DI: ChatService, MemoryService, CuratorService, BundleService take `&AgentService`
- [x] Embedded: WalletService, ServiceDaemonHandler live in `InfraContext`
- [x] Port interfaces (10 total): InferencePort, ToolPort, CircuitBreakerPort, CnsObserver, RegistryIndex, SkillRegistryIndex, FederationDispatch, CnsStoragePort, FederationTransport, FederationSyncPort

---

## Cross-Reference

- **MDS.md § AgentService Specification** ([`docs/architecture/core/MDS.md`](../architecture/core/MDS.md#agentservice-specification)) — defines the 25 accessor methods on `AgentService`, the bounded context, and the service layer contract table listing all 11 subcrates with their contract prefixes and counts.
- **MDS.md § 1.4 Service Layer Subsystems** — domain ontology table mapping each subcrate to its domain, contract prefix, and decomposition status.
- **PRINCIPLES.md** — P5 (Essentialism) governs the service layer: thin orchestration, delegates to domain crates, ≤7 public functions per module.
- **ADR-040** — Strangler Fig decomposition of the monolithic `hkask-services-core` into 11 subcrates (2026-06-27).
