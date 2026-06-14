//! OpenAPI specification

use utoipa::OpenApi;

use crate::{
    ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, GrantCapabilityRequest, ListPodsResponse, ModelEntry, ModelListResponse,
    ModelSearchQuery, PodStatusResponse, TemplateResponse,
};

use crate::routes::{ArchiveRequest, ArchiveResponse, ResolveShaResponse};
use crate::routes::{CreateGoalRequest, GoalListResponse, GoalResponse, SetGoalStateRequest};
use crate::routes::{
    DismissEscalationRequest, DismissEscalationResponse, EscalationEntryResponse,
    EscalationStatsResponse, ListEscalationsResponse, MetacognitionStatusResponse,
    ResolveEscalationRequest, ResolveEscalationResponse,
};

// Handler-local types needed in schemas
use crate::routes::cns::CnsSubscribeParams;

/// API documentation
#[derive(OpenApi)]
#[openapi(
    components(schemas(
        TemplateResponse,
        GrantCapabilityRequest,
        CnsHealthResponse,
        CnsVarietyResponse,
        CreatePodRequest,
        CreatePodResponse,
        PodStatusResponse,
        ListPodsResponse,
        ChatRequest,
        ChatResponse,
        ModelEntry,
        ModelListResponse,
        ModelSearchQuery,
        // Curator schemas
        ListEscalationsResponse,
        EscalationEntryResponse,
        ResolveEscalationRequest,
        ResolveEscalationResponse,
        DismissEscalationRequest,
        DismissEscalationResponse,
        EscalationStatsResponse,
        MetacognitionStatusResponse,
        // Git schemas
        ArchiveRequest,
        ArchiveResponse,
        ResolveShaResponse,
        // Goal schemas
        CreateGoalRequest,
        SetGoalStateRequest,
        GoalResponse,
        GoalListResponse,
        // CNS subscribe params
        CnsSubscribeParams,
    )),
    tags(
        (name = "templates", description = "Template management"),
        (name = "bots", description = "Bot capability management"),
        (name = "mcp", description = "MCP servers and tools"),
        (name = "cns", description = "CNS monitoring"),
        (name = "chat", description = "Curator chat interface"),
        (name = "models", description = "Multi-provider model catalog (Ollama, Fireworks, DeepInfra)"),
        (name = "curator", description = "Curator escalation and metacognition"),
        (name = "git", description = "Git archival and resolution"),
        (name = "acp", description = "ACP agent registration and management"),
        (name = "goals", description = "Goal coordination substrate (OCAP-gated)"),
        (name = "bundles", description = "Bundle composition, application, and evolution"),
        (name = "episodic", description = "Episodic memory store and query"),
        (name = "sovereignty", description = "Consent and access governance (Magna Carta)"),
        (name = "specs", description = "MDS specification management"),
        (name = "consolidation", description = "Context consolidation and condensation"),
    ),
    info(
        title = "hKask API",
        version = "0.27.0",
        description = "A Minimal Viable Container for Agents - HTTP API"
    ),
    servers(
        (url = "/api", description = "hKask API server"),
    ),
)]
pub struct ApiDoc;
