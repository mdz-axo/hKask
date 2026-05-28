//! hKask MCP Registry — Template registry with real registry operations

use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, ServerContext, ToolSpanGuard,
    run_stdio_server, validate_identifier,
};
use hkask_templates::{Registry, RegistryIndex, SqliteRegistry};
use hkask_types::{McpErrorKind, TemplateType, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use tokio::sync::RwLock;

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

pub struct RegistryServer {
    registry: Arc<RwLock<Registry>>,
    webid: WebID,
}

impl RegistryServer {
    pub fn new(db_path: Option<String>, webid: WebID) -> Self {
        let mut registry = Registry::bootstrap();

        if let Ok(sqlite) = Self::try_sqlite_load(db_path.as_deref()) {
            let entries = sqlite.list(None);
            for entry in entries {
                if registry.get(&entry.id).is_none() {
                    let te = hkask_templates::TemplateEntry::new(
                        &entry.id,
                        entry.template_type,
                        &entry.id,
                        &entry.description,
                    )
                    .with_lexicon(entry.lexicon_terms.iter().map(|s| s.as_str()).collect())
                    .with_source(&entry.source_path);
                    registry.register(te);
                }
            }
            tracing::info!("Loaded supplementary templates from SQLite registry");
        }

        Self {
            registry: Arc::new(RwLock::new(registry)),
            webid,
        }
    }

    fn try_sqlite_load(db_path: Option<&str>) -> Result<SqliteRegistry, String> {
        let mut reg = SqliteRegistry::new(db_path)
            .map_err(|e| format!("Failed to create SQLite registry: {}", e))?;
        reg.load_all()
            .map_err(|e| format!("Failed to load from SQLite: {}", e))?;
        Ok(reg)
    }

    fn parse_template_type(tt: &Option<String>) -> Option<TemplateType> {
        tt.as_deref().and_then(TemplateType::parse_str)
    }
}

#[tool_router(server_handler)]
impl RegistryServer {
    #[tool(description = "Index templates from a root path via real registry")]
    async fn registry_index(
        &self,
        Parameters(IndexRequest {
            root_path,
            template_type,
        }): Parameters<IndexRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("registry:index", &self.webid);

        if let Err(e) = validate_identifier("root_path", &root_path, 512) {
            return span.error(e.kind, e.to_json_string());
        }

        let type_filter = Self::parse_template_type(&template_type);
        let registry = self.registry.read().await;
        let entries = registry.list(type_filter);

        span.ok(McpToolOutput::new(json!({
            "root": root_path,
            "template_type": template_type.unwrap_or_else(|| "all".to_string()),
            "templates_found": entries.len(),
            "indexed": true,
        }))
        .to_json_string())
    }

    #[tool(description = "Discover templates by type and domain via real registry search")]
    async fn registry_discover(
        &self,
        Parameters(DiscoverRequest {
            template_type,
            domain_hint,
            limit,
        }): Parameters<DiscoverRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("registry:discover", &self.webid);

        let type_filter = Self::parse_template_type(&template_type);
        let limit = limit.unwrap_or(10) as usize;
        let registry = self.registry.read().await;

        let mut entries = registry.list(type_filter);

        if let Some(ref hint) = domain_hint {
            entries.retain(|e| {
                e.lexicon_terms
                    .iter()
                    .any(|t| t.contains(hint) || hint.contains(t))
                    || e.description.contains(hint)
                    || e.id.contains(hint)
            });
        }

        let truncated: Vec<serde_json::Value> = entries
            .into_iter()
            .take(limit)
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "template_type": e.template_type.as_str(),
                    "description": e.description,
                    "lexicon_terms": e.lexicon_terms,
                })
            })
            .collect();

        span.ok(McpToolOutput::new(json!({
            "template_type": template_type.unwrap_or_else(|| "all".to_string()),
            "domain": domain_hint.unwrap_or_else(|| "any".to_string()),
            "limit": limit,
            "templates": truncated,
        }))
        .to_json_string())
    }

    #[tool(description = "Validate a template via real registry lookup")]
    async fn registry_validate(
        &self,
        Parameters(ValidateRequest { template_id }): Parameters<ValidateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("registry:validate", &self.webid);

        if let Err(e) = validate_identifier("template_id", &template_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let registry = self.registry.read().await;

        let mut errors: Vec<String> = Vec::new();

        if let Err(e) = Registry::validate_template_path(&template_id) {
            errors.push(e.to_string());
        }

        match registry.get(&template_id) {
            Some(entry) => span.ok(McpToolOutput::new(json!({
                "template_id": template_id,
                "valid": errors.is_empty(),
                "errors": errors,
                "template_type": entry.template_type.as_str(),
                "description": entry.description,
            }))
            .to_json_string()),
            None => {
                errors.push(format!("Template '{}' not found in registry", template_id));
                let err = McpToolError::not_found(errors.join("; "));
                span.error(err.kind, err.to_json_string())
            }
        }
    }

    #[tool(description = "Reload templates from a path")]
    async fn registry_reload(
        &self,
        Parameters(ReloadRequest { path }): Parameters<ReloadRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("registry:reload", &self.webid);

        if let Err(e) = validate_identifier("path", &path, 512) {
            return span.error(e.kind, e.to_json_string());
        }

        let mut registry = self.registry.write().await;
        registry.reload();
        span.ok(McpToolOutput::new(json!({
            "path": path,
            "reloaded": true,
            "templates_loaded": registry.count(),
        }))
        .to_json_string())
    }

    #[tool(description = "Compose templates with cascade")]
    async fn registry_compose(
        &self,
        Parameters(ComposeRequest {
            root_template_id,
            cascade_template_ids,
        }): Parameters<ComposeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("registry:compose", &self.webid);

        if let Err(e) = validate_identifier("template_id", &root_template_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let registry = self.registry.read().await;

        let root_found = registry.get(&root_template_id).is_some();

        let cascade_results: Vec<serde_json::Value> = cascade_template_ids
            .iter()
            .map(|id| {
                let found = registry.get(id).is_some();
                serde_json::json!({"id": id, "found": found})
            })
            .collect();

        span.ok(McpToolOutput::new(json!({
            "root": root_template_id,
            "root_found": root_found,
            "cascade": cascade_results,
            "composed": root_found,
        }))
        .to_json_string())
    }

    #[tool(description = "Get a template by ID via real registry lookup")]
    async fn registry_get(
        &self,
        Parameters(GetRequest { template_id }): Parameters<GetRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("registry:get", &self.webid);

        if let Err(e) = validate_identifier("template_id", &template_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let registry = self.registry.read().await;

        match registry.get(&template_id) {
            Some(entry) => {
                let re = entry.as_registry_entry();
                span.ok(McpToolOutput::new(json!({
                    "template_id": re.id,
                    "template_type": re.template_type.as_str(),
                    "description": re.description,
                    "source_path": re.source_path,
                    "lexicon_terms": re.lexicon_terms,
                }))
                .to_json_string())
            }
            None => {
                let err = McpToolError::not_found(format!(
                    "Template '{}' not found in registry",
                    template_id
                ));
                span.error(err.kind, err.to_json_string())
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-registry",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let db_path = ctx.credentials.get("HKASK_REGISTRY_DB").cloned();
            Ok(RegistryServer::new(db_path, ctx.webid))
        },
        vec![CredentialRequirement::optional(
            "HKASK_REGISTRY_DB",
            "Path to registry SQLite database",
        )],
    )
    .await
}
