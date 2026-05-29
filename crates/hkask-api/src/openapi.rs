//! OpenAPI specification

use utoipa::OpenApi;

use crate::{
    ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, ErrorResponse, GrantCapabilityRequest, ListPodsResponse, ModelEntry,
    ModelListResponse, PodStatusResponse, TemplateResponse, ToolResponse,
};

/// API documentation
#[derive(OpenApi)]
#[openapi(
    components(schemas(
        TemplateResponse,
        GrantCapabilityRequest,
        CnsHealthResponse,
        CnsVarietyResponse,
        ToolResponse,
        ErrorResponse,
        CreatePodRequest,
        CreatePodResponse,
        PodStatusResponse,
        ListPodsResponse,
        ChatRequest,
        ChatResponse,
        ModelEntry,
        ModelListResponse,
    )),
    tags(
        (name = "templates", description = "Template management"),
        (name = "bots", description = "Bot capability management"),
        (name = "pods", description = "Agent pod lifecycle management"),
        (name = "mcp", description = "MCP servers and tools"),
        (name = "cns", description = "CNS monitoring"),
        (name = "chat", description = "Curator chat interface"),
        (name = "models", description = "Okapi model catalog"),
    ),
    info(
        title = "hKask API",
        version = "0.1.0",
        description = "A Minimal Viable Container for Agents - HTTP API"
    ),
    servers(
        (url = "/api", description = "hKask API server"),
    ),
)]
pub struct ApiDoc;
