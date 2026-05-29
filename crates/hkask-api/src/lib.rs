//! hKask API — HTTP API with OpenAPI
//!
//! **Endpoints:**
//! - `GET /api/templates` — List templates
//! - `GET /api/templates/:id` — Get template
//! - `POST /api/templates` — Register template
//! - `GET /api/templates/search/:term` — Search templates by lexicon
//! - `GET /api/bots/:id/capabilities` — List bot capabilities
//! - `POST /api/bots/:id/grant` — Grant capability
//! - `GET /api/mcp/servers` — List MCP servers
//! - `GET /api/mcp/tools` — List tools
//! - `GET /api/mcp/tools/:name` — Get tool definition
//! - `POST /api/mcp/invoke` — Invoke an MCP tool
//! - `GET /api/cns/health` — CNS health status
//! - `GET /api/cns/alerts` — Algedonic alerts
//! - `GET /api/cns/variety` — CNS variety counters
//! - `GET /api/pods` — List pods
//! - `POST /api/pods` — Create pod
//! - `POST /api/pods/:id/activate` — Activate pod
//! - `POST /api/pods/:id/deactivate` — Deactivate pod
//! - `GET /api/pods/:id/status` — Get pod status
//! - `POST /api/chat` — Curator chat
//! - `GET /api/sovereignty/status` — User sovereignty status
//! - `POST /api/sovereignty/consent/grant` — Grant explicit consent
//! - `POST /api/sovereignty/consent/revoke` — Revoke explicit consent
//! - `GET /api/sovereignty/killzone` — Kill zone status
//! - `GET /api/sovereignty/access/check` — Check data access status
//! - `POST /api/llm/infer` — SOAP inference endpoint for Russell

use hkask_agents::acp::AcpRuntime;
use hkask_agents::adapters::cns_emitter::CnsEmitterAdapter;
use hkask_agents::adapters::git_cas::GitCasAdapter;
use hkask_agents::adapters::mcp_runtime::McpRuntimeAdapter;
use hkask_agents::adapters::memory_storage::MemoryStorageAdapter;
use hkask_agents::consent::ConsentManager;
use hkask_agents::curator::escalation::EscalationQueue;
use hkask_agents::pod::PodManager;
use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};
use hkask_cns::spans::SpanEmitter;
use hkask_storage::SovereigntyBoundaryStore;
use hkask_templates::SqliteRegistry;
use hkask_types::{CapabilityChecker, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod services;

// Re-export route types for OpenAPI schema generation
pub use routes::{ModelEntry, ModelListResponse, ModelSearchQuery};

use openapi::ApiDoc;

/// API state
#[derive(Clone)]
pub struct ApiState {
    /// Template registry
    pub registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    /// MCP runtime
    pub mcp_runtime: Arc<hkask_mcp::runtime::McpRuntime>,
    /// MCP dispatcher for OCAP-protected tool invocation
    pub mcp_dispatcher: Arc<hkask_mcp::dispatch::McpDispatcher>,
    /// Pod manager
    pub pod_manager: Arc<PodManager>,
    /// Capability checker for OCAP verification
    pub capability_checker: Arc<CapabilityChecker>,
    /// System WebID for signing capabilities
    pub system_webid: WebID,
    /// CNS span emitter for audit trail
    pub cns_emitter: Arc<SpanEmitter>,
    /// Rate limiter for API endpoints
    pub rate_limiter: Arc<RateLimiter>,
    /// Ensemble inferencer (optional - for Russell SOAP inference)
    pub ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::OkapiHttpClient>>,
    /// Spec store for DDMVSS specifications
    pub spec_store: Option<Arc<dyn hkask_types::SpecStore + Send + Sync>>,
    /// Consent manager for user sovereignty
    pub consent_manager: Arc<ConsentManager>,
    /// Escalation queue for Curator escalations
    pub escalation_queue: Arc<EscalationQueue>,
    /// Git CAS adapter for template archival
    pub git_cas: Arc<dyn hkask_agents::ports::GitCASPort>,
    /// Standing ensemble sessions (keyed by session ID)
    pub standing_sessions: Arc<
        tokio::sync::RwLock<
            HashMap<String, Arc<tokio::sync::RwLock<hkask_ensemble::StandingSession>>>,
        >,
    >,
}

impl ApiState {
    pub fn new(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        pod_manager: PodManager,
        capability_secret: &[u8],
        system_webid: WebID,
        ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::OkapiHttpClient>>,
    ) -> Self {
        let observer_webid = WebID::new();
        let rate_limiter = RateLimiter::new(RateLimitConfig {
            max_tokens: 100,
            refill_interval: std::time::Duration::from_millis(600),
        });
        let consent_manager = Arc::new(ConsentManager::new(SovereigntyBoundaryStore::new(
            hkask_storage::Database::in_memory()
                .expect("in-memory db")
                .conn_arc(),
        )));
        let escalation_queue = Arc::new(
            EscalationQueue::new(
                hkask_storage::Database::in_memory()
                    .expect("in-memory db")
                    .conn_arc(),
            )
            .expect("escalation queue init"),
        );
        let git_cas: Arc<dyn hkask_agents::ports::GitCASPort> = Arc::new(GitCasAdapter::from_path(
            PathBuf::from("/tmp/hkask-templates"),
        ));
        let dispatcher_runtime = hkask_mcp::runtime::McpRuntime::new();
        let mcp_dispatcher = Arc::new(hkask_mcp::dispatch::McpDispatcher::new(
            dispatcher_runtime,
            capability_secret,
            hkask_types::cns::RetryConfig::default(),
        ));
        Self {
            registry: Arc::new(tokio::sync::Mutex::new(registry)),
            mcp_runtime: Arc::new(mcp_runtime),
            mcp_dispatcher,
            pod_manager: Arc::new(pod_manager),
            capability_checker: Arc::new(CapabilityChecker::new(capability_secret)),
            system_webid,
            cns_emitter: Arc::new(SpanEmitter::new(observer_webid)),
            rate_limiter: Arc::new(rate_limiter),
            ensemble_inferencer,
            spec_store: None,
            consent_manager,
            escalation_queue,
            git_cas,
            standing_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create ApiState with default adapters.
    ///
    /// The `acp_secret` is the HMAC secret for ACP token signing. It should be
    /// derived from the master key (via `hkask_keystore::master_key::derive_all_internal_secrets`)
    /// or resolved from the environment/keychain (via `hkask_keystore::resolve`).
    ///
    /// The API server is headless and cannot run interactive onboarding — the caller
    /// is responsible for providing a valid ACP secret. Run `kask chat` interactively
    /// first to complete onboarding and store secrets.
    pub fn with_defaults(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        capability_secret: &[u8],
        acp_secret: &[u8],
        system_webid: WebID,
    ) -> Self {
        let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
        let acp_runtime = Arc::new(AcpRuntime::new(acp_secret, None));
        let observer_webid = WebID::new();
        let cns_emitter_adapter = CnsEmitterAdapter::new(observer_webid);
        let mcp_runtime_adapter = McpRuntimeAdapter::new();
        let memory_storage = MemoryStorageAdapter::in_memory().expect("in-memory adapter creation");
        let pod_manager = PodManager::new(
            Arc::new(git_cas),
            acp_runtime,
            Arc::new(cns_emitter_adapter),
            Arc::new(mcp_runtime_adapter),
            Arc::new(memory_storage),
        );
        Self::new(
            registry,
            mcp_runtime,
            pod_manager,
            capability_secret,
            system_webid,
            None,
        )
    }

    /// Create ApiState with consent manager
    pub fn with_consent_manager(mut self, consent_manager: ConsentManager) -> Self {
        self.consent_manager = Arc::new(consent_manager);
        self
    }

    /// Create ApiState with ensemble inferencer
    pub fn with_ensemble_inferencer(
        registry: SqliteRegistry,
        mcp_runtime: hkask_mcp::runtime::McpRuntime,
        pod_manager: PodManager,
        capability_secret: &[u8],
        system_webid: WebID,
        okapi_base_url: &str,
    ) -> Self {
        let inferencer = Arc::new(hkask_ensemble::adapters::OkapiHttpClient::new(
            okapi_base_url,
        ));
        Self::new(
            registry,
            mcp_runtime,
            pod_manager,
            capability_secret,
            system_webid,
            Some(inferencer),
        )
    }

    /// Set the spec store for DDMVSS specifications
    pub fn with_spec_store(mut self, store: Arc<dyn hkask_types::SpecStore + Send + Sync>) -> Self {
        self.spec_store = Some(store);
        self
    }
}

/// Template response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateResponse {
    pub id: String,
    pub template_type: String,
    pub description: String,
    pub source_path: String,
    pub lexicon_terms: Vec<String>,
}

/// Capability request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GrantCapabilityRequest {
    pub capability: String,
}

/// CNS health response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsHealthResponse {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

/// Chat request sent to the Curator or a specified agent.
///
/// The `model` field allows switching the LLM at request time. When omitted,
/// the server default (qwen3:8b) is used. Use `GET /api/models` to discover
/// available models, and `GET /api/models/search?q=...` for fuzzy matching.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatRequest {
    /// User input message
    pub input: String,
    /// Optional template ID to contextualize the prompt
    pub template_id: Option<String>,
    /// Model override for inference (e.g., "qwen3:8b"). If unset, uses the server default.
    #[serde(default)]
    pub model: Option<String>,
}

/// Chat response from the Curator or agent.
///
/// The `model` field echoes which LLM was used, confirming model switching.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatResponse {
    /// Generated response text
    pub output: String,
    /// Template ID that was applied (or "auto-select")
    pub template_id: String,
    /// Model identifier used for inference
    pub model: String,
}

/// CNS variety counter response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VarietyCounterResponse {
    pub variety: u64,
    pub total: u64,
    pub entropy: f64,
}

/// CNS variety response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CnsVarietyResponse {
    pub domains: Vec<String>,
    pub total_deficit: u64,
    pub counters: HashMap<String, VarietyCounterResponse>,
}

/// SOAP inference request from Russell
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoapInferRequest {
    /// Subjective: operator note or context
    pub subjective: Option<String>,
    /// Objective: telemetry data
    pub objective: ObjectiveData,
    /// Assessment: left empty for LLM to fill
    pub assessment: String,
    /// Plan: left empty for LLM to fill
    pub plan: String,
}

/// Authenticated SOAP inference request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoapInferAuthRequest {
    pub request: SoapInferRequest,
    pub capability_token: String,
}

/// SOAP inference response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoapInferResponse {
    /// LLM response text
    pub response: String,
    /// Model used
    pub model: String,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// ACTION: proposals (if any)
    pub actions: Vec<String>,
}

/// SOAP inference error types
#[derive(Debug, thiserror::Error)]
pub enum SoapInferError {
    #[error("Capability verification failed")]
    OcapDenied,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Inference backend error: {0}")]
    InferenceError(String),

    #[error("Invalid capability token: {0}")]
    InvalidToken(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Inference timeout")]
    Timeout,
}

/// Validation error details
#[derive(Debug, Serialize, Deserialize)]
pub enum ValidationErrorType {
    TooManyEvents,
    SubjectiveTooLong,
    MessageTooLong,
    InvalidCharacters,
}

/// ACP registration request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcpRegisterRequest {
    /// Agent WebID (UUID string)
    pub webid: String,
    /// Agent type: "Bot" or "Replicant"
    pub agent_type: String,
    /// Capabilities to grant (e.g., ["tool:execute", "template:render"])
    pub capabilities: Vec<String>,
}

/// ACP registration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcpRegisterResponse {
    /// Granted capability token (HMAC-signed)
    pub token: String,
    /// Registration timestamp (Unix epoch seconds)
    pub registered_at: i64,
    /// Agent WebID
    pub webid: String,
}

/// CNS span types for SOAP inference (type-safe emissions)
pub enum InferenceSpan {
    Start {
        timestamp: String,
        events_count: usize,
        severity_total: u64,
    },
    ValidationError {
        error_type: String,
    },
    OcapDenied {
        reason: String,
    },
    RateLimitExceeded {
        endpoint: String,
    },
    PersonaError {
        error: String,
    },
    InferenceError {
        error: String,
    },
    Timeout {
        timeout_secs: u64,
    },
    Outcome {
        latency_ms: u64,
        actions_count: usize,
        success: bool,
    },
    Execute {
        model: String,
        prompt_length: usize,
        response_length: usize,
    },
}

impl InferenceSpan {
    pub fn span_name(&self) -> &'static str {
        match self {
            InferenceSpan::Start { .. } => "cns.tool.inference.start",
            InferenceSpan::ValidationError { .. } => "cns.tool.inference.validation_error",
            InferenceSpan::OcapDenied { .. } => "cns.tool.inference.ocap_denied",
            InferenceSpan::RateLimitExceeded { .. } => "cns.tool.rate_limit.exceeded",
            InferenceSpan::PersonaError { .. } => "cns.tool.inference.persona_error",
            InferenceSpan::InferenceError { .. } => "cns.tool.inference.error",
            InferenceSpan::Timeout { .. } => "cns.tool.inference.timeout",
            InferenceSpan::Outcome { .. } => "cns.tool.inference.outcome",
            InferenceSpan::Execute { .. } => "cns.tool.inference.execute",
        }
    }

    pub fn observation(&self) -> serde_json::Value {
        match self {
            InferenceSpan::Start {
                timestamp,
                events_count,
                severity_total,
            } => serde_json::json!({
                "timestamp": timestamp,
                "events_count": events_count,
                "severity_total": severity_total,
            }),
            InferenceSpan::ValidationError { error_type } => serde_json::json!({
                "error_type": error_type,
            }),
            InferenceSpan::OcapDenied { reason } => serde_json::json!({
                "reason": reason,
            }),
            InferenceSpan::RateLimitExceeded { endpoint } => serde_json::json!({
                "endpoint": endpoint,
            }),
            InferenceSpan::PersonaError { error } => serde_json::json!({
                "error": error,
            }),
            InferenceSpan::InferenceError { error } => serde_json::json!({
                "error": error,
            }),
            InferenceSpan::Timeout { timeout_secs } => serde_json::json!({
                "timeout_secs": timeout_secs,
            }),
            InferenceSpan::Outcome {
                latency_ms,
                actions_count,
                success,
            } => serde_json::json!({
                "latency_ms": latency_ms,
                "actions_count": actions_count,
                "success": success,
            }),
            InferenceSpan::Execute {
                model,
                prompt_length,
                response_length,
            } => serde_json::json!({
                "model": model,
                "prompt_length": prompt_length,
                "response_length": response_length,
            }),
        }
    }

    pub fn emit(&self, emitter: &crate::SpanEmitter) {
        emitter.emit_tool(self.span_name(), self.observation());
    }
}

/// SOAP inference configuration
#[derive(Clone, Debug)]
pub struct SoapInferenceConfig {
    /// Capability secret for token verification (loaded from keystore)
    pub capability_secret: [u8; 32],
    /// Maximum number of events per request
    pub max_events: usize,
    /// Maximum subjective text length
    pub max_subjective_len: usize,
    /// Maximum event message length
    pub max_message_len: usize,
    /// Inference timeout in seconds
    pub timeout_secs: u64,
    /// Model to use for inference
    pub model: String,
    /// Inference temperature (0.0-1.0)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Path to Jack persona file (loaded at runtime)
    pub jack_persona_path: String,
}

impl SoapInferenceConfig {
    /// Load configuration from environment variables.
    ///
    /// Returns an error if the capability key cannot be resolved from
    /// HKASK_MASTER_KEY, HKASK_CAPABILITY_KEY, or the OS keychain.
    pub fn from_env() -> Result<Self, String> {
        let capability_secret = hkask_keystore::resolve(&hkask_types::SecretRef::derived(
            hkask_types::derivation_contexts::MASTER_KEY_ENV,
            hkask_types::derivation_contexts::CAPABILITY_KEY,
        ))
        .or_else(|_| hkask_keystore::resolve(&hkask_types::SecretRef::env("HKASK_CAPABILITY_KEY")))
        .or_else(|_| {
            hkask_keystore::resolve(&hkask_types::SecretRef::Keychain(
                "capability-key".to_string(),
            ))
        })
        .map(|s| {
            let mut arr = [0u8; 32];
            let bytes: &[u8] = &s;
            let len = bytes.len().min(32);
            arr[..len].copy_from_slice(&bytes[..len]);
            arr
        })
        .map_err(|e| {
            format!(
                "Capability key not available: {}. Run `kask chat` to complete onboarding, \
                 or set HKASK_MASTER_KEY or HKASK_CAPABILITY_KEY.",
                e
            )
        })?;

        let mut config = Self {
            capability_secret,
            max_events: 100,
            max_subjective_len: 4096,
            max_message_len: 1024,
            timeout_secs: 30,
            model: "qwen3:8b".to_string(),
            temperature: 0.2,
            max_tokens: 2048,
            jack_persona_path: "hkask-templates/personas/jack-nurse.md".to_string(),
        };

        if let Ok(val) = std::env::var("HKASK_SOAP_MODEL") {
            config.model = val;
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_TEMPERATURE") {
            config.temperature = val.parse().unwrap_or(config.temperature);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_MAX_TOKENS") {
            config.max_tokens = val.parse().unwrap_or(config.max_tokens);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_TIMEOUT_SECS") {
            config.timeout_secs = val.parse().unwrap_or(config.timeout_secs);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_PERSONA_PATH") {
            config.jack_persona_path = val;
        }

        Ok(config)
    }

    /// Load Jack persona from file at runtime
    pub fn load_jack_persona(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(&self.jack_persona_path)
    }
}

/// Telemetry data from Russell
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ObjectiveData {
    /// Severity counts from recent events
    pub severity_counts: SeverityCounts,
    /// Recent journal events
    pub recent_events: Vec<EventRecord>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct SeverityCounts {
    pub crit: u64,
    pub alert: u64,
    pub warn: u64,
    pub info: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EventRecord {
    pub probe: String,
    pub severity: String,
    pub message: String,
    pub ts: String,
}

/// Tool response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ToolResponse {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_id: String,
}

/// Error response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
}

/// Create pod request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}

/// Create pod response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodResponse {
    pub pod_id: String,
}

/// Pod status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PodStatusResponse {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub template: String,
    pub created_at: i64,
}

/// List pods response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPodsResponse {
    pub pods: Vec<PodStatusResponse>,
}

/// Spec capture request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureRequest {
    pub description: String,
    pub category: String,
    pub domain_anchor: String,
    pub criteria: Vec<String>,
}

/// Spec capture response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub domain_anchor: String,
}

/// Spec list response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecListResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

/// Spec validate request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecValidateRequest {
    pub threshold: f64,
}

/// Spec validate response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecValidateResponse {
    pub valid: bool,
    pub coherence_score: f64,
    pub threshold: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Spec cultivate response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCultivateResponse {
    pub coherence_score: f64,
    pub spec_count: usize,
    pub categories_covered: Vec<String>,
    pub categories_missing: Vec<String>,
}

/// Create API router with OpenAPI documentation and authentication
pub fn create_router(state: ApiState) -> Result<OpenApiRouter, String> {
    let auth_service = std::sync::Arc::new(
        middleware::AuthService::new()
            .map_err(|e| format!("Failed to initialize auth service: {}", e))?,
    );

    Ok(OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::templates_router().into())
        .merge(routes::bots_router().into())
        .merge(routes::pods_router().into())
        .merge(routes::mcp_router().into())
        .merge(routes::cns_router().into())
        .merge(routes::sovereignty_router().into())
        .merge(routes::chat_router().into())
        .merge(routes::models_router().into())
        .merge(routes::ensemble_router().into())
        .merge(routes::soap_infer_router().into())
        .merge(routes::acp_router().into())
        .merge(routes::spec_router().into())
        .merge(routes::curator_router().into())
        .merge(routes::git_router().into())
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            middleware::auth_middleware,
        ))
        .with_state(state))
}

/// Build OpenAPI spec
pub fn create_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
