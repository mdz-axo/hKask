//! Chat command handlers — inference and memory
//!
//! The chat path delegates to `ChatService` for the full pipeline:
//! agent lookup → prompt composition → semantic recall → inference →
//! episodic storage. The REPL provides pre-resolved ports; standalone
//! calls use AgentService's shared infrastructure.
//!
//! Streaming variant (`chat_with_agent_streaming`) calls
//! `ChatService::prepare_chat()` for prompt composition and memory,
//! then streams inference output via `generate_stream_with_model()`
//! so the CLI can print tokens incrementally.

use std::sync::Arc;

use hkask_services::{AgentService, ChatRequest, ChatService, ResolvedSecrets};
use hkask_types::LLMParameters;
use hkask_types::ports::{InferencePort, InferenceUsage};

/// Build AgentService from secrets or environment.
/// Single construction path for all chat variants.
///
/// # REQ: P7-converge — AgentService construction is single-source
async fn build_chat_context(
    name: &str,
    secrets: Option<&ResolvedSecrets>,
) -> Result<AgentService, ChatResponse> {
    let config = match secrets {
        Some(s) => {
            let mcp_secret = hkask_keystore::resolve_mcp_secret()
                .map(|s| String::from_utf8_lossy(&s).to_string())
                .unwrap_or_else(|_| "hkask-mcp-default".to_string());
            hkask_services::ServiceConfig::from_secrets(
                s.acp_secret.clone(),
                s.db_passphrase.clone(),
                mcp_secret,
                name.to_string(),
            )
        }
        None => match hkask_services::ServiceConfig::from_env() {
            Ok(c) => c,
            Err(e) => {
                return Err(ChatResponse {
                    text: format!("Config error: {}", e),
                    usage: None,
                    finish_reason: "error".to_string(),
                    tool_calls: vec![],
                });
            }
        },
    };
    AgentService::build(config).await.map_err(|e| ChatResponse {
        text: format!("AgentService error: {}", e),
        usage: None,
        finish_reason: "error".to_string(),
        tool_calls: vec![],
    })
}

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
/// When `secrets` is provided (from onboarding), builds a AgentService
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

    let ctx = match build_chat_context(name, secrets).await {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
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
        params_override: None,
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

/// Variant of `chat_with_agent` that accepts explicit LLMParameters.
/// Sets `params_override` on the ChatRequest, which ChatService already respects.
#[allow(clippy::too_many_arguments)]
pub async fn chat_with_agent_with_params(
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
    params: &LLMParameters,
) -> ChatResponse {
    let name = agent_name.unwrap_or("Curator");

    let ctx = match build_chat_context(name, secrets).await {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
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
        auth_context: None,
        params_override: Some(params.clone()),
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

/// Send a chat message to an agent and print tokens as they arrive.
///
/// This is the streaming variant of `chat_with_agent()`. It uses
/// `ChatService::prepare_chat()` for prompt composition and memory recall,
/// then streams inference output via `generate_stream_with_model()` so
/// the CLI can print `text_delta` chunks incrementally.
///
/// Returns a `ChatResponse`-like struct with the full assembled text,
/// usage stats, and any tool calls from the final chunk.
#[allow(clippy::too_many_arguments)]
pub async fn chat_with_agent_streaming(
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

    // Build AgentService from secrets or environment
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
            match AgentService::build(config).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    return ChatResponse {
                        text: format!("AgentService error: {}", e),
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
            match AgentService::build(config).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    return ChatResponse {
                        text: format!("AgentService error: {}", e),
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
        auth_context: None,
        params_override: None,
    };

    // Prepare the chat turn (prompt composition, semantic recall, etc.)
    let prepared = match ChatService::prepare_chat(&ctx, &req).await {
        Ok(p) => p,
        Err(e) => {
            return ChatResponse {
                text: format!("Chat prepare error: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    // Stream inference
    let params = LLMParameters {
        temperature: 0.7,
        top_p: 0.9,
        top_k: 40,
        min_p: 0.0,
        typical_p: 0.0,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 512,
        seed: None,
    };

    let stream = prepared.inference_port.generate_stream_with_model(
        &prepared.prompt,
        &params,
        Some(&prepared.model),
    );

    // Consume the stream, printing text deltas as they arrive
    let mut full_text = String::new();
    let mut final_usage: Option<InferenceUsage> = None;
    let mut final_finish_reason = String::from("stop");
    let mut final_tool_calls: Vec<hkask_types::ports::StructuredToolCall> = vec![];

    use futures_util::StreamExt;
    let mut stream = Box::pin(stream);
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if !chunk.text_delta.is_empty() {
                    print!("{}", chunk.text_delta);
                    // Flush stdout to ensure incremental display
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
                full_text.push_str(&chunk.text_delta);
                if let Some(usage) = chunk.usage {
                    final_usage = Some(usage);
                }
                if let Some(reason) = chunk.finish_reason {
                    final_finish_reason = reason;
                }
                if !chunk.tool_calls.is_empty() {
                    final_tool_calls = chunk.tool_calls;
                }
            }
            Err(e) => {
                return ChatResponse {
                    text: format!("Stream error: {}", e),
                    usage: None,
                    finish_reason: "error".to_string(),
                    tool_calls: vec![],
                };
            }
        }
    }
    println!(); // Newline after streaming output

    // Store the exchange as episodic triple
    ChatService::store_episodic(
        &prepared.episodic_port,
        input,
        &full_text,
        prepared.agent_webid,
        &prepared.capability_token,
        &prepared.agent_name,
    );

    ChatResponse {
        text: full_text,
        usage: final_usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
        finish_reason: final_finish_reason,
        tool_calls: final_tool_calls,
    }
}

/// Variant of `chat_with_agent_streaming` that accepts explicit LLMParameters.
/// The streaming path requires the params before prepare_chat since the
/// inference call happens after prepare. We pass params_override through
/// the ChatRequest so PrepareChat receives it, then use directly for streaming.
#[allow(clippy::too_many_arguments)]
pub async fn chat_with_agent_streaming_with_params(
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
    params: &LLMParameters,
) -> ChatResponse {
    let name = agent_name.unwrap_or("Curator");

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
            match AgentService::build(config).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    return ChatResponse {
                        text: format!("AgentService error: {}", e),
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
            match AgentService::build(config).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    return ChatResponse {
                        text: format!("AgentService error: {}", e),
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
        auth_context: None,
        params_override: Some(params.clone()),
    };

    // Prepare the chat turn (prompt composition, semantic recall, etc.)
    let prepared = match ChatService::prepare_chat(&ctx, &req).await {
        Ok(p) => p,
        Err(e) => {
            return ChatResponse {
                text: format!("Chat prepare error: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    // Stream inference using caller-provided params
    let stream = prepared.inference_port.generate_stream_with_model(
        &prepared.prompt,
        params,
        Some(&prepared.model),
    );

    let mut full_text = String::new();
    let mut final_usage: Option<InferenceUsage> = None;
    let mut final_finish_reason = String::from("stop");
    let mut final_tool_calls: Vec<hkask_types::ports::StructuredToolCall> = vec![];

    use futures_util::StreamExt;
    let mut stream = Box::pin(stream);
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if !chunk.text_delta.is_empty() {
                    print!("{}", chunk.text_delta);
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
                full_text.push_str(&chunk.text_delta);
                if let Some(usage) = chunk.usage {
                    final_usage = Some(usage);
                }
                if let Some(reason) = chunk.finish_reason {
                    final_finish_reason = reason;
                }
                if !chunk.tool_calls.is_empty() {
                    final_tool_calls = chunk.tool_calls;
                }
            }
            Err(e) => {
                return ChatResponse {
                    text: format!("Stream error: {}", e),
                    usage: None,
                    finish_reason: "error".to_string(),
                    tool_calls: vec![],
                };
            }
        }
    }
    println!(); // Newline after streaming output

    // Store the exchange as episodic triple
    ChatService::store_episodic(
        &prepared.episodic_port,
        input,
        &full_text,
        prepared.agent_webid,
        &prepared.capability_token,
        &prepared.agent_name,
    );

    ChatResponse {
        text: full_text,
        usage: final_usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
        finish_reason: final_finish_reason,
        tool_calls: final_tool_calls,
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
        print!("{}: ", agent);
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let chat_response = rt.block_on(chat_with_agent_streaming(
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
        // Streaming already printed the response text incrementally.
        // Print the agent label and token usage.
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
