//! hKask MCP Registry — Template registry index, discovery, and validation

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::parameters::Parameters;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Template entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TemplateEntry {
    pub id: String,
    pub template_type: String,
    pub name: String,
    pub path: String,
    pub git_sha: Option<String>,
    pub owner_webid: String,
    pub created_at: String,
    pub lexicon_terms: Vec<String>,
    pub contract: serde_json::Value,
}

/// Index request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IndexRequest {
    pub root_path: Option<String>,
    pub template_type: Option<String>,
}

/// Discover request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiscoverRequest {
    pub template_type: String,
    domain_hint: Option<String>,
    limit: Option<usize>,
}

/// Validate request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateRequest {
    pub template_id: String,
    contract: Option<serde_json::Value>,
}

/// Compose request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ComposeRequest {
    pub root_template_id: String,
    pub cascade_template_ids: Vec<String>,
}

/// Registry server implementation
pub struct RegistryServer {
    tool_router: ToolRouter<RegistryServer>,
    registry_root: PathBuf,
    templates: std::sync::Arc<tokio::sync::RwLock<Vec<TemplateEntry>>>,
}

impl RegistryServer {
    pub fn new() -> Self {
        let registry_root = std::env::var("HKASK_REGISTRY_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".hkask-registry"));

        Self {
            tool_router: Self::tool_router(),
            registry_root,
            templates: std::sync::Arc::new(tokio::sync::RwLock::new(vec![
                // Seed with example templates
                TemplateEntry {
                    id: "tpl-dispatch-001".to_string(),
                    template_type: "Process".to_string(),
                    name: "dispatch".to_string(),
                    path: "process/dispatch.yaml".to_string(),
                    git_sha: Some("abc123".to_string()),
                    owner_webid: "system".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    lexicon_terms: vec!["select".to_string(), "populate".to_string(), "execute".to_string()],
                    contract: serde_json::json!({
                        "input": { "raw_prompt": "string" },
                        "output": { "selected_template": "string", "rationale": "string" }
                    }),
                },
                TemplateEntry {
                    id: "tpl-selector-001".to_string(),
                    template_type: "Prompt".to_string(),
                    name: "selector".to_string(),
                    path: "prompt/selector.j2".to_string(),
                    git_sha: Some("def456".to_string()),
                    owner_webid: "system".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    lexicon_terms: vec!["WordAct".to_string()],
                    contract: serde_json::json!({
                        "input": { "registry_index": "array", "raw_prompt": "string" },
                        "output": { "selected_template_id": "string", "confidence": "number" }
                    }),
                },
            ])),
        }
    }
}

#[tool_router]
impl RegistryServer {
    #[tool(description = "List available templates in the registry index")]
    async fn registry_index(&self, Parameters(req): Parameters<IndexRequest>) -> String {
        let templates = self.templates.read().await;
        
        let filtered: Vec<&TemplateEntry> = templates
            .iter()
            .filter(|t| req.template_type.as_ref().map_or(true, |tt| t.template_type == *tt))
            .collect();

        serde_json::json!({
            "root_path": self.registry_root.display().to_string(),
            "templates": filtered,
            "count": filtered.len(),
            "template_type_filter": req.template_type
        }).to_string()
    }

    #[tool(description = "Discover templates by type with optional domain hint")]
    async fn registry_discover(&self, Parameters(req): Parameters<DiscoverRequest>) -> String {
        let templates = self.templates.read().await;
        let limit = req.limit.unwrap_or(10);
        
        let mut matches: Vec<&TemplateEntry> = templates
            .iter()
            .filter(|t| t.template_type == req.template_type)
            .collect();

        // If domain hint provided, boost matches (in production, use LLM scoring)
        if let Some(_domain) = req.domain_hint {
            // Domain-aware ranking would go here
            matches.sort_by(|a, b| a.name.cmp(&b.name));
        }

        matches.truncate(limit);

        serde_json::json!({
            "template_type": req.template_type,
            "domain_hint": req.domain_hint,
            "matches": matches,
            "count": matches.len()
        }).to_string()
    }

    #[tool(description = "Validate a template contract and lexicon terms")]
    async fn registry_validate(&self, Parameters(req): Parameters<ValidateRequest>) -> String {
        let templates = self.templates.read().await;
        
        match templates.iter().find(|t| t.id == req.template_id) {
            Some(template) => {
                let mut errors = Vec::new();
                
                // Validate contract structure
                if let Some(contract) = &req.contract {
                    if !contract.is_object() {
                        errors.push("contract must be a JSON object".to_string());
                    }
                }

                // Validate lexicon terms exist (in production, check hLexicon)
                for term in &template.lexicon_terms {
                    if term.is_empty() {
                        errors.push(format!("empty lexicon term"));
                    }
                }

                if errors.is_empty() {
                    serde_json::json!({
                        "valid": true,
                        "template_id": req.template_id,
                        "template_type": template.template_type,
                        "lexicon_terms": template.lexicon_terms.len()
                    }).to_string()
                } else {
                    serde_json::json!({
                        "valid": false,
                        "template_id": req.template_id,
                        "errors": errors
                    }).to_string()
                }
            }
            None => serde_json::json!({
                "valid": false,
                "error": "template not found",
                "template_id": req.template_id
            }).to_string()
        }
    }

    #[tool(description = "Signal hot-reload of templates (explicit refresh)")]
    async fn registry_reload(&self, path: Option<String>) -> String {
        let reload_path = path.unwrap_or_else(|| self.registry_root.display().to_string());
        
        tracing::info!(path = %reload_path, "template reload signaled");
        serde_json::json!({
            "success": true,
            "path": reload_path,
            "message": "Reload signal received. In production, this would trigger filesystem watch or Git polling.",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }).to_string()
    }

    #[tool(description = "Compose cross-template cascades")]
    async fn registry_compose(&self, Parameters(req): Parameters<ComposeRequest>) -> String {
        let templates = self.templates.read().await;
        
        // Validate all templates exist
        let mut cascade = Vec::new();
        for template_id in &req.cascade_template_ids {
            match templates.iter().find(|t| t.id == *template_id) {
                Some(t) => cascade.push(t.clone()),
                None => {
                    return serde_json::json!({
                        "success": false,
                        "error": format!("template not found: {}", template_id)
                    }).to_string();
                }
            }
        }

        // Validate root template
        let root = match templates.iter().find(|t| t.id == req.root_template_id) {
            Some(t) => t.clone(),
            None => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("root template not found: {}", req.root_template_id)
                }).to_string();
            }
        };

        tracing::info!(root = %root.id, cascade_len = cascade.len(), "composed template cascade");
        serde_json::json!({
            "success": true,
            "root_template": root,
            "cascade": cascade,
            "matroshka_depth": cascade.len() + 1,
            "message": "Cascade composed. Matroshka limit: 7"
        }).to_string()
    }

    #[tool(description = "Get a specific template by ID")]
    async fn registry_get(&self, template_id: String) -> String {
        let templates = self.templates.read().await;
        
        match templates.iter().find(|t| t.id == template_id) {
            Some(t) => serde_json::to_string_pretty(&t).unwrap_or_else(|_| "error".to_string()),
            None => serde_json::json!({
                "error": "template not found",
                "template_id": template_id
            }).to_string()
        }
    }
}

#[tool_handler]
impl ServerHandler for RegistryServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = RegistryServer::new();
    let service = server.serve_stdio();
    tracing::info!("hkask-mcp-registry MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
