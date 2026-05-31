//! Chat command handlers — inference, pod-mediated chat, and Russell bridge

use hkask_agents::pod::{AgentPersona, PodContext, PodManagerBuilder};
use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
use hkask_types::LLMParameters;
use std::sync::Arc;

use crate::commands::config::{init_registry, registry_yaml_path};

/// Send a chat message to an agent and return the response.
///
/// Routes through Russell adapter for Russell requests, otherwise uses
/// standard pod-mediated inference with Okapi.
pub async fn chat_with_agent(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
) -> String {
    let name = agent_name.unwrap_or("Curator");

    // Load agent registry
    let (acp, store) = match init_registry().await {
        Ok(r) => r,
        Err(e) => return format!("Registry init error: {}", e),
    };

    let loader = hkask_agents::BotRegistryLoader::new(
        registry_yaml_path(),
        acp.clone(),
        store,
        Arc::new(hkask_agents::adapters::FilesystemRegistrySource::new()),
    );

    let agents = match loader.boot().await {
        Ok(a) => a,
        Err(e) => return format!("Registry load error: {}", e),
    };

    let agent = agents.iter().find(|a| a.definition.name == name);

    // R11: Wire Russell Direct Chat
    if name == "russell" || name == "Russell" {
        return chat_via_russell(input, agent).await;
    }

    // Standard chat flow for non-Russell agents
    let system_prompt = match agent {
        Some(registered) => registered.definition.compose_system_prompt(),
        None => format!("You are {}, an assistant in the hKask system.\n\n", name),
    };

    let config = OkapiConfig::local_dev();
    let agent_kind = match agent {
        Some(registered) => &registered.definition.agent_kind,
        None => {
            return "Agent not registered — run `kask agent register` first.".to_string();
        }
    };
    let default_model = match agent_kind {
        hkask_types::AgentKind::Bot => "deepseek-v4-flash",
        hkask_types::AgentKind::Replicant => "deepseek-v4-pro",
    };
    let model = model_override.unwrap_or(default_model);
    let inference = match OkapiInference::new(model, config) {
        Ok(i) => Arc::new(i) as Arc<dyn InferencePort>,
        Err(e) => return format!("Okapi init error: {}", e),
    };

    let pod_manager = PodManagerBuilder::new()
        .acp_runtime(acp)
        .inference_port(inference.clone())
        .with_in_memory_storage()
        .build();

    let persona_yaml = format!(
        r#"
agent:
  name: {}
  type: {}
  version: "0.1.0"
charter:
  description: "Chat session with {}"
  editor: cli
capabilities:
  - "tool:inference:call"
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#,
        name,
        if name == "Curator" {
            "Replicant"
        } else {
            "Bot"
        },
        name
    );

    let persona = match AgentPersona::from_yaml(&persona_yaml) {
        Ok(p) => p,
        Err(e) => return format!("Persona parse error: {}", e),
    };

    let pod_id = match pod_manager
        .create_pod("chat-template", &persona, Some(name.to_string()))
        .await
    {
        Ok(id) => id,
        Err(e) => return format!("Pod creation error: {}", e),
    };

    if let Err(e) = pod_manager.activate_pod(&pod_id).await {
        return format!("Pod activation error: {}", e);
    }

    let pod_context = match PodContext::from_manager(&pod_manager, &pod_id).await {
        Ok(ctx) => ctx,
        Err(e) => return format!("Pod context error: {}", e),
    };

    let full_prompt = format!("{}\n\nUser: {}", system_prompt, input);

    let params = LLMParameters {
        temperature: 0.7,
        top_p: 0.9,
        top_k: 40,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 512,
        seed: None,
    };

    let inference_port = match pod_context.inference_port() {
        Ok(port) => port,
        Err(e) => return format!("Inference port unavailable: {}", e),
    };

    match inference_port.generate(&full_prompt, &params).await {
        Ok(result) => result.text,
        Err(e) => format!("Inference error: {}", e),
    }
}

/// Chat via Russell ACP bridge (R11: Russell Direct Chat)
async fn chat_via_russell(input: &str, agent: Option<&hkask_types::RegisteredAgent>) -> String {
    use hkask_agents::acp::A2AMessage;
    use hkask_agents::adapters::RussellAcpAdapter;
    use hkask_agents::ports::AcpPort;
    use hkask_types::WebID;

    if agent.is_none() {
        return "Russell is not registered. Use `kask agent register` to register Russell first."
            .to_string();
    }

    let russell_binary =
        std::env::var("HKASK_RUSSELL_BINARY").unwrap_or_else(|_| "russell-acp-server".to_string());

    let russell_adapter = match RussellAcpAdapter::new(russell_binary) {
        Ok(adapter) => adapter,
        Err(e) => return format!("Failed to initialize Russell bridge: {}", e),
    };

    let webid = WebID::from_persona_with_namespace(b"russell-chat-session", "russell");

    if let Err(e) = russell_adapter
        .register_agent(webid, "Replicant", vec![])
        .await
    {
        return format!("Failed to create Russell session: {}", e);
    }

    let message = A2AMessage::TemplateDispatch {
        from: webid,
        to: Some(webid),
        template_id: "russell:direct-chat".to_string(),
        input: serde_json::json!({
            "message": input,
        }),
        correlation_id: uuid::Uuid::new_v4().to_string(),
    };

    match russell_adapter.send_message(message).await {
        Ok(response) => response,
        Err(e) => format!("Russell error: {}", e),
    }
}
