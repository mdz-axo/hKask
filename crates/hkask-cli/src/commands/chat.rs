//! Chat command handlers — inference and memory
//!
//! The chat path delegates to `ChatService` for the full pipeline:
//! agent lookup → prompt composition → semantic recall → inference →
//! episodic storage. The REPL provides pre-resolved ports; standalone
//! calls use ServiceContext's shared infrastructure.

use std::sync::Arc;

use hkask_services::{ChatRequest, ChatService, ResolvedSecrets, ServiceContext};
use hkask_types::ports::InferencePort;

/// Response from a chat inference call.
///
/// Re-exported from `hkask_services::ChatResponse` for surface convenience.
pub type ChatResponse = hkask_services::ChatResponse;

/// Token usage breakdown for gas accounting.
///
/// Re-exported from `hkask_services::TokenUsage` for surface convenience.
pub type TokenUsage = hkask_services::TokenUsage;

/// Send a chat message to an agent and return the response.
///
/// When `secrets` is provided (from onboarding), builds a ServiceContext
/// from them. Otherwise, builds from environment.
///
/// When `inference_port` is provided, the shared port is reused across calls.
/// When `None`, a new port is created via InferenceService.
///
/// Memory integration:
/// - Before inference, recall semantic triples relevant to the user's input
/// - After inference, store an episodic triple recording the exchange
/// - Episodic memory is private (agent-scoped), semantic memory is public
///
/// When `episodic_storage` and `semantic_storage` are provided, they are used
/// for memory operations (enabling persistence across REPL sessions).
/// When `None`, a fresh in-memory adapter is created per call (ephemeral).
#[allow(clippy::too_many_arguments)]
pub async fn chat_with_agent(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
    inference_port: Option<Arc<dyn InferencePort>>,
    secrets: Option<&ResolvedSecrets>,
    episodic_storage: Option<Arc<dyn hkask_agents::ports::EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn hkask_agents::ports::SemanticStoragePort>>,
    _agent_webid: Option<hkask_types::WebID>,
    system_prompt_suffix: Option<&str>,
    tool_section: Option<&str>,
) -> ChatResponse {
    let name = agent_name.unwrap_or("Curator");

    // Build ServiceContext from secrets or environment
    let ctx = match secrets {
        Some(s) => {
            let mcp_secret = hkask_keystore::resolve_mcp_secret()
                .map(|s| String::from_utf8_lossy(&s).to_string())
                .unwrap_or_else(|_| "hkask-mcp-default".to_string());
            let config = hkask_services::ServiceConfig::from_secrets(
                s.acp_secret.clone(),
                s.db_passphrase.clone(),
                mcp_secret,
                name.to_string(),
            );
            match ServiceContext::build(config).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    return ChatResponse {
                        text: format!("ServiceContext error: {}", e),
                        usage: None,
                        finish_reason: "error".to_string(),
                        tool_calls: vec![],
                    };
                }
            }
        }
        None => {
            let config = match hkask_services::ServiceConfig::from_env() {
                Ok(c) => c,
                Err(e) => {
                    return ChatResponse {
                        text: format!("Config error: {}", e),
                        usage: None,
                        finish_reason: "error".to_string(),
                        tool_calls: vec![],
                    };
                }
            };
            match ServiceContext::build(config).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    return ChatResponse {
                        text: format!("ServiceContext error: {}", e),
                        usage: None,
                        finish_reason: "error".to_string(),
                        tool_calls: vec![],
                    };
                }
            }
        }
    };

    let req = ChatRequest {
        input: input.to_string(),
        agent_name: Some(name.to_string()),
        model_override: model_override.map(|s| s.to_string()),
        system_prompt_suffix: system_prompt_suffix.map(|s| s.to_string()),
        tool_section: tool_section.map(|s| s.to_string()),
        inference_port_override: inference_port,
        episodic_storage_override: episodic_storage,
        semantic_storage_override: semantic_storage,
        auth_context: None, // CLI uses legacy system-level token from config secrets
    };

    match ChatService::chat(&ctx, req).await {
        Ok(resp) => resp,
        Err(e) => ChatResponse {
            text: format!("Chat error: {}", e),
            usage: None,
            finish_reason: "error".to_string(),
            tool_calls: vec![],
        },
    }
}

/// CLI handler for `kask chat` subcommand
#[allow(clippy::too_many_arguments)]
pub fn run_chat(
    rt: &tokio::runtime::Runtime,
    registry: &hkask_templates::SqliteRegistry,
    runtime: &hkask_mcp::runtime::McpRuntime,
    handle: &tokio::runtime::Handle,
    template: Option<String>,
    input: Option<std::path::PathBuf>,
    agent: String,
    model: Option<String>,
) {
    if let Some(input_path) = input {
        let onboarding_outcome = match rt.block_on(crate::onboarding::run_onboarding()) {
            Ok(outcome) => outcome,
            Err(e) => {
                eprintln!("Cannot chat: {}", e);
                eprintln!("Run `kask chat` first to complete onboarding interactively.");
                std::process::exit(1);
            }
        };
        let content = super::helpers::or_exit(
            std::fs::read_to_string(&input_path),
            "Failed to read input file",
        );
        let chat_response = rt.block_on(chat_with_agent(
            content.trim(),
            Some(&agent),
            model.as_deref(),
            None,
            onboarding_outcome.resolved_secrets.as_ref(),
            None,
            None,
            None,
            None,
            None,
        ));
        println!("{}: {}", agent, chat_response.text);
        if let Some(ref usage) = chat_response.usage {
            eprintln!(
                "  {} tokens ({} prompt + {} completion)",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            );
        }
    } else {
        crate::repl::run(
            registry,
            runtime,
            template.as_deref(),
            &agent,
            model.as_deref(),
            handle.clone(),
        );
    }
}
