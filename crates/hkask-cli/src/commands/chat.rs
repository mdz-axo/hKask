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

use hkask_ports::{InferencePort, InferenceUsage};
use hkask_services_chat::{ChatService, ChatTurnRequest, MemoryService, PreparedChat};
use hkask_services_context::AgentService;
use hkask_services_onboarding::ResolvedSecrets;
use hkask_types::template::LLMParameters;

/// Build AgentService from secrets or environment.
/// Routes through the canonical `helpers::build_agent_service_from_secrets`.
///
/// # REQ: P7-converge — AgentService construction is single-source
/// expect: "I can access all hKask functionality through the kask CLI"
fn build_chat_context(
    name: &str,
    secrets: Option<&ResolvedSecrets>,
) -> Result<AgentService, ChatTurnResponse> {
    let from_secrets = secrets.map(|s| (name, s));
    super::helpers::build_agent_service_from_secrets(from_secrets).map_err(|e| ChatTurnResponse {
        text: format!("AgentService error: {}", e),
        usage: None,
        finish_reason: "error".to_string(),
        tool_calls: vec![],
    })
}

/// Response from a chat inference call.
pub use hkask_services_chat::ChatTurnResponse;

/// Token usage breakdown for gas accounting.
pub use hkask_services_chat::TokenUsage;

/// Send a chat message to an agent and return the response.
///
/// When `secrets` is provided (from onboarding), builds a AgentService
/// from them. Otherwise, builds from environment.
///
/// When `inference_port` is provided, the shared port is reused across calls.
/// When `None`, a new port is created via InferenceService.
///
/// Memory integration:
/// - Before inference, recall semantic h_mems relevant to the user's input
/// - After inference, store an episodic h_mem recording the exchange
/// - Episodic memory is private (agent-scoped), semantic memory is public
///
/// When `episodic_storage` and `semantic_storage` are provided, they are used
/// for memory operations (enabling persistence across REPL sessions).
/// When `None`, a fresh in-memory adapter is created per call (ephemeral).
#[allow(clippy::too_many_arguments)]
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  input is a non-empty string; agent_name defaults to Curator; secrets or env config must be resolvable
/// post: sends chat message through ChatService pipeline; returns ChatTurnResponse with text, usage, and finish_reason
pub async fn chat_with_agent(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
    inference_port: Option<Arc<dyn InferencePort>>,
    secrets: Option<&ResolvedSecrets>,
    episodic_storage: Option<Arc<dyn hkask_agents::ports::EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn hkask_agents::ports::SemanticStoragePort>>,
    _agent_webid: Option<hkask_types::WebID>,
    tool_section: Option<&str>,
) -> ChatTurnResponse {
    let name = agent_name.unwrap_or("Curator");

    let ctx = match build_chat_context(name, secrets) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    let req = ChatTurnRequest {
        input: input.to_string(),
        agent_name: Some(name.to_string()),
        model_override: model_override.map(|s| s.to_string()),
        tool_section: tool_section.map(|s| s.to_string()),
        inference_port_override: inference_port,
        episodic_storage_override: episodic_storage,
        semantic_storage_override: semantic_storage,
        auth_context: None,
        params_override: None,
        api_spec: None,
        tools: None,
    };

    match ChatService::chat(&ctx, req).await {
        Ok(resp) => resp,
        Err(e) => ChatTurnResponse {
            text: format!("Chat error: {}", e),
            usage: None,
            finish_reason: "error".to_string(),
            tool_calls: vec![],
        },
    }
}

/// Variant of `chat_with_agent` that accepts explicit LLMParameters.
/// Sets `params_override` on the ChatTurnRequest, which ChatService already respects.
#[allow(clippy::too_many_arguments)]
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  input is non-empty; params is a valid LLMParameters struct; secrets or env config must be resolvable
/// post: same as chat_with_agent but with explicit LLMParameters override for temperature, top_p, etc.
pub async fn chat_with_agent_with_params(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
    inference_port: Option<Arc<dyn InferencePort>>,
    secrets: Option<&ResolvedSecrets>,
    episodic_storage: Option<Arc<dyn hkask_agents::ports::EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn hkask_agents::ports::SemanticStoragePort>>,
    _agent_webid: Option<hkask_types::WebID>,
    tool_section: Option<&str>,
    params: &LLMParameters,
) -> ChatTurnResponse {
    let name = agent_name.unwrap_or("Curator");

    let ctx = match build_chat_context(name, secrets) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    let req = ChatTurnRequest {
        input: input.to_string(),
        agent_name: Some(name.to_string()),
        model_override: model_override.map(|s| s.to_string()),
        tool_section: tool_section.map(|s| s.to_string()),
        inference_port_override: inference_port,
        episodic_storage_override: episodic_storage,
        semantic_storage_override: semantic_storage,
        auth_context: None,
        params_override: Some(params.clone()),
        api_spec: None,
        tools: None,
    };

    match ChatService::chat(&ctx, req).await {
        Ok(resp) => resp,
        Err(e) => ChatTurnResponse {
            text: format!("Chat error: {}", e),
            usage: None,
            finish_reason: "error".to_string(),
            tool_calls: vec![],
        },
    }
}

/// Stream inference output, store episodic memory, and return assembled response.
async fn finish_stream(
    prepared: &PreparedChat,
    params: &LLMParameters,
    input: &str,
) -> ChatTurnResponse {
    let stream = prepared.inference_port.generate_stream_with_model(
        &prepared.prompt,
        params,
        Some(&prepared.model),
        None,
    );

    let mut full_text = String::new();
    let mut final_usage: Option<InferenceUsage> = None;
    let mut final_finish_reason = String::from("stop");
    let mut final_tool_calls: Vec<hkask_ports::StructuredToolCall> = vec![];

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
                return ChatTurnResponse {
                    text: format!("Stream error: {}", e),
                    usage: None,
                    finish_reason: "error".to_string(),
                    tool_calls: vec![],
                };
            }
        }
    }
    println!();

    MemoryService::store_episodic(
        &prepared.episodic_port,
        input,
        &full_text,
        prepared.agent_webid,
        &prepared.capability_token,
        &prepared.agent_name,
    );

    ChatTurnResponse {
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

/// Send a chat message to an agent and print tokens as they arrive.
///
/// This is the streaming variant of `chat_with_agent()`. It uses
/// `ChatService::prepare_chat()` for prompt composition and memory recall,
/// then streams inference output via `generate_stream_with_model()` so
/// the CLI can print `text_delta` chunks incrementally.
///
/// Returns a `ChatTurnResponse`-like struct with the full assembled text,
/// usage stats, and any tool calls from the final chunk.
#[allow(clippy::too_many_arguments)]
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  input is non-empty; agent_name defaults to Curator; secrets or env config must be resolvable
/// post: streams inference output token-by-token to stdout; stores episodic h_mem; returns assembled ChatTurnResponse
pub async fn chat_with_agent_streaming(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
    inference_port: Option<Arc<dyn InferencePort>>,
    secrets: Option<&ResolvedSecrets>,
    episodic_storage: Option<Arc<dyn hkask_agents::ports::EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn hkask_agents::ports::SemanticStoragePort>>,
    _agent_webid: Option<hkask_types::WebID>,
    tool_section: Option<&str>,
) -> ChatTurnResponse {
    let name = agent_name.unwrap_or("Curator");

    let ctx = match build_chat_context(name, secrets) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    let req = ChatTurnRequest {
        input: input.to_string(),
        agent_name: Some(name.to_string()),
        model_override: model_override.map(|s| s.to_string()),
        tool_section: tool_section.map(|s| s.to_string()),
        inference_port_override: inference_port,
        episodic_storage_override: episodic_storage,
        semantic_storage_override: semantic_storage,
        auth_context: None,
        params_override: None,
        api_spec: None,
        tools: None,
    };

    // Prepare the chat turn (prompt composition, semantic recall, etc.)
    let prepared = match ChatService::prepare_chat(&ctx, &req).await {
        Ok(p) => p,
        Err(e) => {
            return ChatTurnResponse {
                text: format!("Chat prepare error: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    // Stream inference — chat should bypass fusion so the user's chosen
    // model is used directly, while skills route through the fusion group.
    let fusion_active = ctx.config().inference_config.fusion.is_some();
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
        disable_thinking: false,
        adapter: None,
        bypass_fusion: fusion_active,
        fusion_config: None,
    };

    finish_stream(&prepared, &params, input).await
}

/// Variant of `chat_with_agent_streaming` that accepts explicit LLMParameters.
/// The streaming path requires the params before prepare_chat since the
/// inference call happens after prepare. We pass params_override through
/// the ChatTurnRequest so PrepareChat receives it, then use directly for streaming.
#[allow(clippy::too_many_arguments)]
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  input is non-empty; params is a valid LLMParameters struct; secrets or env config must be resolvable
/// post: same as chat_with_agent_streaming but with explicit LLMParameters override
pub async fn chat_with_agent_streaming_with_params(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
    inference_port: Option<Arc<dyn InferencePort>>,
    secrets: Option<&ResolvedSecrets>,
    episodic_storage: Option<Arc<dyn hkask_agents::ports::EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn hkask_agents::ports::SemanticStoragePort>>,
    _agent_webid: Option<hkask_types::WebID>,
    tool_section: Option<&str>,
    params: &LLMParameters,
) -> ChatTurnResponse {
    let name = agent_name.unwrap_or("Curator");

    let ctx = match build_chat_context(name, secrets) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    let req = ChatTurnRequest {
        input: input.to_string(),
        agent_name: Some(name.to_string()),
        model_override: model_override.map(|s| s.to_string()),
        tool_section: tool_section.map(|s| s.to_string()),
        inference_port_override: inference_port,
        episodic_storage_override: episodic_storage,
        semantic_storage_override: semantic_storage,
        auth_context: None,
        params_override: Some(params.clone()),
        api_spec: None,
        tools: None,
    };

    // Prepare the chat turn (prompt composition, semantic recall, etc.)
    let prepared = match ChatService::prepare_chat(&ctx, &req).await {
        Ok(p) => p,
        Err(e) => {
            return ChatTurnResponse {
                text: format!("Chat prepare error: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    finish_stream(&prepared, params, input).await
}

/// CLI entry-point: `kask chat [agent] [-m model]`
#[allow(clippy::too_many_arguments)]
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; registry is a mutable SqliteRegistry; runtime is a valid McpRuntime; agent is non-empty
/// post: either runs onboarding + streaming chat with input file, or launches the interactive REPL
pub fn run_chat(
    rt: &tokio::runtime::Runtime,
    registry: &mut hkask_templates::SqliteRegistry,
    runtime: &hkask_mcp::runtime::McpRuntime,
    handle: &tokio::runtime::Handle,
    template: Option<String>,
    input: Option<std::path::PathBuf>,
    agent: String,
    model: Option<String>,
    tui: bool,
) {
    if let Some(input_path) = input {
        let onboarding_outcome = match rt.block_on(crate::onboarding::run_onboarding()) {
            Ok(outcome) => outcome,
            Err(e) => {
                // Cancelled is a deliberate user action — don't treat it as an error.
                if matches!(e, crate::onboarding::OnboardingError::Cancelled) {
                    std::process::exit(0);
                }
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
        ));
        // Streaming already printed the response text incrementally.
        // Print the agent label and token usage.
        if let Some(ref usage) = chat_response.usage {
            eprintln!(
                "  {} tokens ({} prompt + {} completion)",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            );
        }
    } else if tui
        || std::env::var("HKASK_TUI")
            .map(|v| v == "1")
            .unwrap_or(false)
    {
        #[cfg(feature = "tui")]
        {
            hkask_repl::run_tui(
                registry,
                runtime,
                template.as_deref(),
                &agent,
                model.as_deref(),
                handle.clone(),
                Arc::new(crate::repl_host::CliHost),
            );
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("TUI not built — rebuild with `cargo build --features tui`");
            hkask_repl::run(
                registry,
                runtime,
                template.as_deref(),
                &agent,
                model.as_deref(),
                handle.clone(),
                Arc::new(crate::repl_host::CliHost),
            );
        }
    } else {
        hkask_repl::run(
            registry,
            runtime,
            template.as_deref(),
            &agent,
            model.as_deref(),
            handle.clone(),
            Arc::new(crate::repl_host::CliHost),
        );
    }
}
