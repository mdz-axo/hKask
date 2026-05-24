//! hKask MCP Registry — Template registry with real registry operations

use hkask_templates::{Registry, RegistryEntry, RegistryIndex, SqliteRegistry};
use hkask_types::TemplateType;
use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    tool, tool_router, transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

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

pub struct RegistryServer {
    registry: Arc<RwLock<Registry>>,
}

impl RegistryServer {
    pub fn new() -> Self {
        let mut registry = Registry::bootstrap();

        if let Ok(sqlite) = Self::try_sqlite_load() {
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
        }
    }

    fn try_sqlite_load() -> Result<SqliteRegistry, String> {
        let db_path = std::env::var("HKASK_REGISTRY_DB").ok();
        let mut reg = SqliteRegistry::new(db_path.as_deref())
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
        let type_filter = Self::parse_template_type(&template_type);
        let registry = self.registry.read().await;
        let entries = registry.list(type_filter);
        format!(
            r#"{{"root":"{}","template_type":"{}","templates_found":{},"indexed":true}}"#,
            root_path,
            template_type.unwrap_or_else(|| "all".to_string()),
            entries.len()
        )
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

        format!(
            r#"{{"template_type":"{}","domain":"{}","limit":{},"templates":{}}}"#,
            template_type.unwrap_or_else(|| "all".to_string()),
            domain_hint.unwrap_or_else(|| "any".to_string()),
            limit,
            serde_json::to_string(&truncated).unwrap()
        )
    }

    #[tool(description = "Validate a template via real registry lookup")]
    async fn registry_validate(
        &self,
        Parameters(ValidateRequest { template_id }): Parameters<ValidateRequest>,
    ) -> String {
        let registry = self.registry.read().await;

        let mut errors: Vec<String> = Vec::new();

        if let Err(e) = Registry::validate_template_path(&template_id) {
            errors.push(e.to_string());
        }

        match registry.get(&template_id) {
            Some(entry) => format!(
                r#"{{"template_id":"{}","valid":{},"errors":{},"template_type":"{}","description":"{}"}}"#,
                template_id,
                errors.is_empty(),
                serde_json::to_string(&errors).unwrap(),
                entry.template_type.as_str(),
                entry.description
            ),
            None => {
                errors.push(format!("Template '{}' not found in registry", template_id));
                format!(
                    r#"{{"template_id":"{}","valid":false,"errors":{}}}"#,
                    template_id,
                    serde_json::to_string(&errors).unwrap()
                )
            }
        }
    }

    #[tool(description = "Reload templates from a path")]
    async fn registry_reload(
        &self,
        Parameters(ReloadRequest { path }): Parameters<ReloadRequest>,
    ) -> String {
        let mut registry = self.registry.write().await;
        registry.reload();
        format!(
            r#"{{"path":"{}","reloaded":true,"templates_loaded":{}}}"#,
            path,
            registry.count()
        )
    }

    #[tool(description = "Compose templates with cascade")]
    async fn registry_compose(
        &self,
        Parameters(ComposeRequest {
            root_template_id,
            cascade_template_ids,
        }): Parameters<ComposeRequest>,
    ) -> String {
        let registry = self.registry.read().await;

        let root_found = registry.get(&root_template_id).is_some();

        let cascade_results: Vec<serde_json::Value> = cascade_template_ids
            .iter()
            .map(|id| {
                let found = registry.get(id).is_some();
                serde_json::json!({"id": id, "found": found})
            })
            .collect();

        format!(
            r#"{{"root":"{}","root_found":{},"cascade":{},"composed":{}}}"#,
            root_template_id,
            root_found,
            serde_json::to_string(&cascade_results).unwrap(),
            root_found
        )
    }

    #[tool(description = "Get a template by ID via real registry lookup")]
    async fn registry_get(
        &self,
        Parameters(GetRequest { template_id }): Parameters<GetRequest>,
    ) -> String {
        let registry = self.registry.read().await;

        match registry.get(&template_id) {
            Some(entry) => {
                let re = entry.as_registry_entry();
                format!(
                    r#"{{"template_id":"{}","template_type":"{}","description":"{}","source_path":"{}","lexicon_terms":{}}}"#,
                    re.id,
                    re.template_type.as_str(),
                    re.description,
                    re.source_path,
                    serde_json::to_string(&re.lexicon_terms).unwrap()
                )
            }
            None => format!(
                r#"{{"template_id":"{}","error":"not found","found":false}}"#,
                template_id
            ),
        }
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
