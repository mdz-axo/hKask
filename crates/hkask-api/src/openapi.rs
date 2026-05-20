//! OpenAPI specification

use utoipa::OpenApi;

use crate::{
    ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, ErrorResponse, GrantCapabilityRequest, ListPodsResponse, PodStatusResponse,
    TemplateResponse, ToolResponse,
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
    )),
    tags(
        (name = "templates", description = "Template management"),
        (name = "bots", description = "Bot capability management"),
        (name = "pods", description = "Agent pod lifecycle management"),
        (name = "mcp", description = "MCP servers and tools"),
        (name = "cns", description = "CNS monitoring"),
        (name = "chat", description = "Curator chat interface"),
    ),
    info(
        title = "hKask API",
        version = "0.1.0",
        description = "Planck's Constant of Agent Systems - HTTP API",
    ),
    servers(
        (url = "/api", description = "hKask API server"),
    ),
)]
pub struct ApiDoc;
