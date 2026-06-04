//! MCP tools for replicant chat — exposes a hKask replicant as an MCP tool
//!
//! Two tools exposed via MCP protocol:
//! - `replicant:chat` — Send a message to a replicant and receive a response
//! - `replicant:status` — Check replicant registration and identity
//!
//! # Architecture
//!
//! This server bridges Zed's MCP context server model with hKask's pod-mediated
//! chat flow. When Zed's Agent Panel calls `replicant:chat`, this server:
//!
//! 1. Resolves the replicant persona name → WebID
//! 2. Creates a pod via `PodManagerBuilder` (same as `kask chat`)
//! 3. Sends the user's message through pod-mediated inference via `InferencePort`
//! 4. Returns the LLM response as the tool result
//!
//! The replicant persona is configured via `HKASK_AGENT_PERSONA` env var.
//! The model is configured via `HKASK_DEFAULT_MODEL` env var or per-request override.

use hkask_agents::pod::{AgentPersona, PodContext, PodManagerBuilder};
use hkask_mcp::server::{McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_templates::{OkapiConfig, OkapiInference};
use hkask_types::ports::InferencePort;
use hkask_types::{LLMParameters, McpErrorKind, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatRequest {
    /// The message to send to the replicant
    pub message: String,
    /// Model override (optional — uses the server default if empty)
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusRequest {
    /// Replicant persona name (optional — uses the server default if empty)
    #[serde(default)]
    pub persona: String,
}

pub struct ReplicantServer {
    /// Calling agent's WebID (from run_stdio_server resolution)
    webid: WebID,
    /// Replicant persona name
    persona: String,

    /// Default model for inference
    default_model: String,
}

impl ReplicantServer {
    pub fn new(webid: WebID, persona: &str, default_model: &str) -> anyhow::Result<Self> {
        Ok(Self {
            webid,
            persona: persona.to_string(),
            default_model: default_model.to_string(),
        })
    }

    /// Build an inference port for the given model using Okapi config from env.
    fn build_inference_port(&self, model: &str) -> Result<Arc<dyn InferencePort>, String> {
        let base_url = std::env::var("OKAPI_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
        let config = OkapiConfig {
            base_url,
            ..OkapiConfig::default()
        };
        OkapiInference::new(model, config)
            .map(|i| Arc::new(i) as Arc<dyn InferencePort>)
            .map_err(|e| format!("Okapi init error: {}", e))
    }
}

#[tool_router(server_handler)]
impl ReplicantServer {
    #[tool(
        description = "Send a message to a hKask replicant agent and receive a response. The replicant persona is configured via HKASK_AGENT_PERSONA (default: 'Curator'). Optionally override the model per request."
    )]
    async fn replicant_chat(
        &self,
        Parameters(ChatRequest { message, model }): Parameters<ChatRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("replicant:chat", &self.webid);

        if let Err(e) = validate_identifier("message", &message, 8192) {
            return span.error(e.kind, e.to_json_string());
        }

        let model = if model.is_empty() {
            self.default_model.clone()
        } else {
            if let Err(e) = validate_identifier("model", &model, 128) {
                return span.error(e.kind, e.to_json_string());
            }
            model
        };

        tracing::info!(
            target: "hkask.mcp.replicant",
            persona = %self.persona,
            model = %model,
            message_len = message.len(),
            "Replicant chat request"
        );

        // Build inference port
        let inference_port = match self.build_inference_port(&model) {
            Ok(port) => port,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": e,
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Build pod manager — ACP runtime and capability checker are auto-resolved
        // by PodManagerBuilder::build() (same pattern as CLI chat_with_agent).
        let pod_manager = PodManagerBuilder::new()
            .inference_port(inference_port)
            .with_in_memory_storage()
            .build();

        // Construct persona YAML for the replicant
        let persona_yaml = format!(
            r#"
agent:
  name: {}
  type: Replicant
  version: "0.1.0"
charter:
  description: "Chat session with {} via MCP"
  editor: mcp-server
capabilities:
  - "tool:inference:call"
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#,
            self.persona, self.persona
        );

        let persona = match AgentPersona::from_yaml(&persona_yaml) {
            Ok(p) => p,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Persona parse error: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Create and activate the pod
        let pod_id = match pod_manager
            .create_pod(
                "replicant-chat-template",
                &persona,
                Some(self.persona.clone()),
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Pod creation error: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        if let Err(e) = pod_manager.activate_pod(&pod_id).await {
            return span.error(
                McpErrorKind::Internal,
                McpToolOutput::new(serde_json::json!({
                    "error": format!("Pod activation error: {}", e),
                    "persona": self.persona,
                }))
                .to_json_string(),
            );
        }

        let pod_context = match PodContext::from_manager(&pod_manager, &pod_id).await {
            Ok(ctx) => ctx,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Pod context error: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Construct the full prompt (system + user message)
        let system_prompt = format!(
            "You are {}, a Replicant in the hKask system.\n\n",
            self.persona
        );
        let full_prompt = format!("{}\nUser: {}", system_prompt, message);

        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
        };

        let pod_inference_port = match pod_context.inference_port() {
            Ok(port) => port,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Inference port unavailable: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Generate response using the pod's inference port with model override
        match pod_inference_port
            .generate_with_model(&full_prompt, &params, Some(&model))
            .await
        {
            Ok(result) => span.ok(McpToolOutput::new(serde_json::json!({
                "text": result.text,
                "model": result.model,
                "persona": self.persona,
                "usage": {
                    "prompt_tokens": result.usage.prompt_tokens,
                    "completion_tokens": result.usage.completion_tokens,
                    "total_tokens": result.usage.total_tokens,
                },
                "finish_reason": result.finish_reason,
            }))
            .to_json_string()),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolOutput::new(serde_json::json!({
                    "error": format!("Inference error: {}", e),
                    "persona": self.persona,
                    "model": model,
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Check the registration status and identity of the hKask replicant configured for this MCP server."
    )]
    async fn replicant_status(
        &self,
        Parameters(StatusRequest { persona }): Parameters<StatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("replicant:status", &self.webid);

        if !persona.is_empty() {
            if let Err(e) = validate_identifier("persona", &persona, 128) {
                return span.error(e.kind, e.to_json_string());
            }
        }

        let resolved_persona = if persona.is_empty() {
            &self.persona
        } else {
            &persona
        };

        let resolved_webid = WebID::from_persona(resolved_persona.as_bytes());

        span.ok(McpToolOutput::new(serde_json::json!({
            "persona": resolved_persona,
            "webid": resolved_webid.redacted_display().to_string(),
            "agent_type": "Replicant",
            "default_model": self.default_model,
            "server_webid": self.webid.redacted_display().to_string(),
            "okapi_base_url": std::env::var("OKAPI_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string()),
        }))
        .to_json_string())
    }
}
