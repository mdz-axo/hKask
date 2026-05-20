//! hKask MCP Registry — Template registry and cascade composition

use rmcp::{
    tool, tool_router, ServiceExt,
    handler::server::wrapper::Parameters,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexRequest {
    pub root_path: String,
    pub template_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscoverRequest {
    pub template_type: Option<String>,
    pub domain_hint: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateRequest {
    pub template_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReloadRequest {
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComposeRequest {
    pub root_template_id: String,
    pub cascade_template_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRequest {
    pub template_id: String,
}

#[derive(Debug, Default)]
pub struct RegistryServer;

impl RegistryServer {
    pub fn new() -> Self {
        Self
    }
}

#[tool_router(server_handler)]
impl RegistryServer {
    #[tool(description = "Index templates from a root path")]
    async fn registry_index(&self, Parameters(IndexRequest { root_path, template_type }): Parameters<IndexRequest>) -> String {
        format!(r#"{{"root":"{}","template_type":"{}","templates_found":5,"indexed":true}}"#, root_path, template_type.unwrap_or_else(|| "all".to_string()))
    }

    #[tool(description = "Discover templates by type and domain")]
    async fn registry_discover(&self, Parameters(DiscoverRequest { template_type, domain_hint, limit }): Parameters<DiscoverRequest>) -> String {
        let limit = limit.unwrap_or(10);
        format!(r#"{{"template_type":"{}","domain":"{}","limit":{},"templates":[]}}"#, 
            template_type.unwrap_or_else(|| "all".to_string()), 
            domain_hint.unwrap_or_else(|| "any".to_string()),
            limit)
    }

    #[tool(description = "Validate a template")]
    async fn registry_validate(&self, Parameters(ValidateRequest { template_id }): Parameters<ValidateRequest>) -> String {
        format!(r#"{{"template_id":"{}","valid":true,"errors":[]}}"#, template_id)
    }

    #[tool(description = "Reload templates from a path")]
    async fn registry_reload(&self, Parameters(ReloadRequest { path }): Parameters<ReloadRequest>) -> String {
        format!(r#"{{"path":"{}","reloaded":true,"templates_loaded":3}}"#, path)
    }

    #[tool(description = "Compose templates with cascade")]
    async fn registry_compose(&self, Parameters(ComposeRequest { root_template_id, cascade_template_ids }): Parameters<ComposeRequest>) -> String {
        format!(r#"{{"root":"{}","cascade":{},"composed":true}}"#, root_template_id, serde_json::to_string(&cascade_template_ids).unwrap())
    }

    #[tool(description = "Get a template by ID")]
    async fn registry_get(&self, Parameters(GetRequest { template_id }): Parameters<GetRequest>) -> String {
        format!(r#"{{"template_id":"{}","content":"simulated template content","version":"1.0.0"}}"#, template_id)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let server = RegistryServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-registry started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
