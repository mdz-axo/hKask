//! hKask MCP Skill — exposes registered skills as callable MCP tools.
//!
//! Each skill in the registry becomes available through the `skill_execute` tool.
//! When invoked, the server:
//! 1. Looks up the skill's template in the registry
//! 2. Renders the Jinja2 template with the provided context variables
//! 3. Runs inference on the rendered prompt via the centralized inference router
//! 4. Returns the inference result
//!
//! ## Architectural note: direct domain crate access
//!
//! This MCP server directly uses `hkask-templates`, `hkask-ports`, and `hkask-inference`
//! rather than routing through `hkask-services-skill`. This is intentional: MCP servers
//! are alternate consumer surfaces (alongside the CLI and Web servers) that may use
//! domain crates directly per hKask's hexagonal architecture. `hkask-services-skill`
//! provides skill management (discovery, publishing, hashing, auditing, bundle composition)
//! but does not expose a `SkillService` with execution methods; skill execution — template
//! rendering + inference — is an alternate concern that lives in the MCP surface layer.
//! If `SkillService` gains execution methods in the future, this server should be migrated
//! to use them.

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::server::{CapabilityTier, McpToolError, execute_tool};
use hkask_ports::InferencePort;
use hkask_ports::RegistryIndex;
use hkask_templates::Registry;
use hkask_types::WebID;
use hkask_types::template::LLMParameters;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Name → description for available skills (loaded at startup).
type SkillIndex = HashMap<String, SkillToolDef>;

/// Metadata for a skill available as a tool.
#[derive(Clone)]
pub struct SkillToolDef {
    pub description: String,
    pub template_content: String,
}

/// Skill execution MCP server.
pub struct SkillServer {
    pub webid: WebID,
    pub replicant: String,
    pub daemon: Option<hkask_mcp::DaemonClient>,
    pub inference_port: Arc<dyn InferencePort>,
    pub skills: SkillIndex,
    pub capability_tier: CapabilityTier,
}

impl SkillServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        inference_port: Arc<dyn InferencePort>,
        capability_tier: CapabilityTier,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            inference_port,
            skills: HashMap::new(),
            capability_tier,
        }
    }

    /// Load skills from the bootstrapped template registry.
    pub fn load_skills(&mut self) {
        let registry = Registry::bootstrap();

        for entry in registry.list(None) {
            let template_content = match std::fs::read_to_string(&entry.source_path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let tool_id = registry_entry_to_tool_id(&entry.id);

            self.skills.insert(
                tool_id,
                SkillToolDef {
                    description: format!("[{}] {}", entry.template_type, entry.description),
                    template_content,
                },
            );
        }
    }
}

/// Convert a registry entry ID to a safe MCP tool identifier.
fn registry_entry_to_tool_id(id: &str) -> String {
    id.replace('/', ".").replace('_', "-")
}

/// Render a Jinja2 template with the given context map.
fn render_skill_template(
    template: &str,
    context: &HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);

    for (key, value) in context {
        let val = match value {
            serde_json::Value::String(s) => minijinja::Value::from(s.clone()),
            other => minijinja::Value::from(other.to_string()),
        };
        env.add_global(key, val);
    }

    env.render_str(template, minijinja::value::Value::UNDEFINED)
        .map_err(|e| format!("Template render error: {}", e))
}

// ── MCP Tools ─────────────────────────────────────────────────────────────────

/// Request parameters for `skill_execute`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SkillExecuteRequest {
    /// Skill identifier (e.g., "coding-guidelines.guidelines-assess")
    #[schemars(description = "Skill identifier (e.g., coding-guidelines.guidelines-assess)")]
    skill_id: String,
    /// Context variables to pass to the template as a JSON object
    #[schemars(description = "Context variables as JSON object to pass to the template")]
    context: serde_json::Value,
}

#[tool_router(server_handler)]
impl SkillServer {
    #[tool(description = "Liveness and profile info")]
    pub async fn skill_ping(&self) -> String {
        execute_tool(self, "skill_ping", async {
            Ok(serde_json::json!({
                "status": "ok",
                "version": SERVER_VERSION,
                "mode": if self.capability_tier.embedded { "embedded" } else { "standalone" },
                "skills_loaded": self.skills.len(),
            }))
        })
        .await
    }

    #[tool(description = "List available skill IDs with their descriptions")]
    pub async fn skill_list(&self) -> String {
        execute_tool(self, "skill_list", async {
            let skills: Vec<serde_json::Value> = self
                .skills
                .iter()
                .map(|(id, def)| {
                    serde_json::json!({
                        "id": id,
                        "description": def.description,
                    })
                })
                .collect();
            Ok(serde_json::json!({ "skills": skills }))
        })
        .await
    }

    #[tool(
        description = "Execute a registered skill template with context variables. \
        Renders the skill as a Jinja2 template and runs inference. \
        Use skill_list first to discover available skill IDs."
    )]
    pub async fn skill_execute(
        &self,
        Parameters(SkillExecuteRequest { skill_id, context }): Parameters<SkillExecuteRequest>,
    ) -> String {
        execute_tool(self, "skill_execute", async {
            let def = self.skills.get(&skill_id).ok_or_else(|| {
                let available: Vec<&str> = self.skills.keys().map(|s| s.as_str()).collect();
                McpToolError::new(
                    hkask_types::McpErrorKind::NotFound,
                    format!("Skill '{}' not found. Available: {:?}", skill_id, available),
                )
            })?;

            // Build context map for template rendering
            let mut ctx = HashMap::new();
            if let serde_json::Value::Object(map) = &context {
                for (k, v) in map {
                    ctx.insert(k.clone(), v.clone());
                }
            }

            // Render the Jinja2 template
            let rendered = render_skill_template(&def.template_content, &ctx)
                .map_err(McpToolError::invalid_argument)?;

            // Prepend system prompt with tool-awareness context.
            // The calling agent has MCP tools available (auto-started servers).
            // If the skill references tools that are unavailable, the agent
            // should adapt or report the limitation rather than failing silently.
            let full_prompt = format!(
                "You are executing a skill template. Follow its instructions precisely. \
                 The calling agent has access to MCP tools including filesystem \
                 (fs.read, fs.write, fs.edit, fs.search, shell.exec), memory \
                 (episodic/semantic recall), web search, and context condensation. \
                 If the skill references tools outside this set, adapt the approach \
                 or report the missing capability.\n\n{}",
                rendered
            );

            let params = LLMParameters {
                temperature: 0.3,
                max_tokens: 2048,
                ..Default::default()
            };

            let result = self
                .inference_port
                .generate(&full_prompt, &params, None)
                .await
                .map_err(|e| McpToolError::internal(format!("Inference failed: {}", e)))?;

            Ok(serde_json::Value::String(result.text))
        })
        .await
    }
}

impl hkask_mcp::server::ToolContext for SkillServer {
    fn webid(&self) -> &hkask_types::WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

// ── Server runner ─────────────────────────────────────────────────────────────

pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    let inference_config = InferenceConfig::from_env();
    let inference_router = InferenceRouter::new(inference_config);
    let inference_port: Arc<dyn InferencePort> = Arc::new(inference_router);

    hkask_mcp::run_server(
        "hkask-mcp-skill",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            {
                let webid = ctx.webid;
                let mut server = SkillServer::new(
                    webid,
                    replicant.clone(),
                    daemon_client.clone(),
                    inference_port.clone(),
                    ctx.capability_tier,
                );
                server.load_skills();
                tracing::info!(
                    target: "hkask.mcp.skill",
                    skill_count = server.skills.len(),
                    "Skills loaded from registry"
                );
                Ok::<_, anyhow::Error>(server)
            }
            .map_err(|e| hkask_mcp::McpError::UnexpectedResponse {
                context: "skill server init".into(),
                detail: e.to_string(),
            })
        },
        vec![],
    )
    .await
}
