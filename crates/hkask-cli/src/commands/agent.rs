//! Agent registration, curator, and ensemble standing command handlers

use crate::commands::config::{init_registry, open_registry_db, registry_yaml_path};
use crate::errors::{AgentError, CuratorError, EnsembleError};
use std::sync::Arc;

pub struct AgentReceipt {
    pub webid: String,
    pub token_hash: String,
    pub registered_at: String,
}

pub async fn bot_list(
    kind_filter: Option<&str>,
) -> Result<Vec<hkask_types::RegisteredAgent>, AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let loader = hkask_agents::BotRegistryLoader::new(
        registry_yaml_path(),
        _acp,
        store,
        Arc::new(hkask_agents::adapters::FilesystemRegistrySource::new()),
    );

    let agents = loader
        .boot()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let filtered = if let Some(kind_str) = kind_filter {
        let kind = hkask_types::AgentKind::parse(kind_str)
            .ok_or_else(|| AgentError::InvalidType(kind_str.to_string()))?;
        agents
            .into_iter()
            .filter(|a| a.definition.agent_kind == kind)
            .collect()
    } else {
        agents
    };

    Ok(filtered)
}

pub async fn bot_status(name: &str) -> Result<hkask_types::RegisteredAgent, AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let loader = hkask_agents::BotRegistryLoader::new(
        registry_yaml_path(),
        _acp,
        store,
        Arc::new(hkask_agents::adapters::FilesystemRegistrySource::new()),
    );

    let agents = loader
        .boot()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    agents
        .into_iter()
        .find(|a| a.definition.name == name)
        .ok_or_else(|| AgentError::NotFound(name.to_string()))
}

pub async fn agent_register(
    webid_str: &str,
    agent_type: &str,
    capabilities: Vec<String>,
) -> Result<AgentReceipt, AgentError> {
    let (acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let webid = hkask_types::WebID::from_string(webid_str);

    let token = acp
        .register_agent(webid, agent_type.to_string(), capabilities)
        .await
        .map_err(|e| AgentError::RegistrationFailed(e.to_string()))?;

    let definition = hkask_types::AgentDefinition {
        name: webid_str.to_string(),
        agent_kind: hkask_types::AgentKind::parse(agent_type)
            .unwrap_or(hkask_types::AgentKind::Bot),
        binding_contract: false,
        editor: "cli".to_string(),
        charter: None,
        capabilities: vec![],
        rights: vec![],
        responsibilities: vec![],
        reporting: None,
        standing_session: None,
        persona: None,
        depends_on: vec![],
        readiness_probe: None,
        process_manifest: None,
    };

    let registered = hkask_types::RegisteredAgent {
        definition,
        token_hash: token.signature.clone(),
        registered_at: chrono::Utc::now().to_rfc3339(),
        source_yaml: "cli-register".to_string(),
    };

    store
        .insert(&registered)
        .map_err(|e| AgentError::RegistrationFailed(e.to_string()))?;

    Ok(AgentReceipt {
        webid: webid_str.to_string(),
        token_hash: token.signature,
        registered_at: registered.registered_at,
    })
}

pub async fn agent_unregister(name: &str) -> Result<(), AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    store
        .remove(name)
        .map_err(|e| AgentError::UnregistrationFailed(e.to_string()))?;

    Ok(())
}

pub async fn chat_with_agent(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
) -> String {
    use hkask_agents::pod::{PodContext, PodManagerBuilder};
    use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
    use hkask_types::LLMParameters;
    use std::sync::Arc;

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
    // Check if this is a Russell chat request
    if name == "russell" || name == "Russell" {
        // Check if Russell is registered
        if agent.is_none() {
            return "Russell is not registered. Use `kask agent register` to register Russell first.".to_string();
        }

        // Use RussellAcpAdapter for direct chat
        use hkask_agents::acp::A2AMessage;
        use hkask_agents::adapters::RussellAcpAdapter;
        use hkask_agents::ports::AcpPort;
        use hkask_types::WebID;

        // Get Russell binary path from environment or use default
        let russell_binary = std::env::var("HKASK_RUSSELL_BINARY")
            .unwrap_or_else(|_| "russell-acp-server".to_string());

        // Create Russell adapter (bridge secret derived from master key via HKDF)
        let russell_adapter = match RussellAcpAdapter::new(russell_binary) {
            Ok(adapter) => adapter,
            Err(e) => return format!("Failed to initialize Russell bridge: {}", e),
        };

        // Create a WebID for this chat session
        let webid = WebID::from_persona_with_namespace(b"russell-chat-session", "russell");

        // Register with Russell (creates a session)
        if let Err(e) = russell_adapter
            .register_agent(webid, "Replicant", vec![])
            .await
        {
            return format!("Failed to create Russell session: {}", e);
        }

        // Send the message to Russell
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
    } else {
        // Standard chat flow for non-Russell agents
        let system_prompt = match agent {
            Some(registered) => registered.definition.compose_system_prompt(),
            None => format!("You are {}, an assistant in the hKask system.\n\n", name),
        };

        // Create inference port
        let config = OkapiConfig::local_dev();
        let model = model_override.unwrap_or("qwen3:8b");
        let inference = match OkapiInference::new(model, config) {
            Ok(i) => Arc::new(i) as Arc<dyn InferencePort>,
            Err(e) => return format!("Okapi init error: {}", e),
        };

        let pod_manager = PodManagerBuilder::new()
            .acp_runtime(acp)
            .inference_port(inference.clone())
            .with_in_memory_storage()
            .build();

        // Create or find pod for this agent
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

        let persona = match hkask_agents::pod::AgentPersona::from_yaml(&persona_yaml) {
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

        // Activate pod (registers with ACP, grants MCP access)
        if let Err(e) = pod_manager.activate_pod(&pod_id).await {
            return format!("Pod activation error: {}", e);
        }

        // Create PodContext (R1: all access goes through pod)
        let pod_context = match PodContext::from_manager(&pod_manager, &pod_id).await {
            Ok(ctx) => ctx,
            Err(e) => return format!("Pod context error: {}", e),
        };

        // Emit CNS span for observability
        pod_context.emit_span(
            "cns.prompt.chat",
            "chat_interaction",
            serde_json::json!({
                "agent": name,
                "input_length": input.len(),
            }),
        );

        // Build prompt with system context
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

        // Use inference port from PodContext (R1: pod-mediated inference)
        let inference_port = match pod_context.inference_port() {
            Ok(port) => port,
            Err(e) => return format!("Inference port unavailable: {}", e),
        };

        match inference_port.generate(&full_prompt, &params).await {
            Ok(result) => result.text,
            Err(e) => format!("Inference error: {}", e),
        }
    }
}

pub async fn curator_escalations() -> Result<Vec<hkask_agents::EscalationEntry>, CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .list_pending()
        .map_err(|e| CuratorError::EscalationNotFound(e.to_string()))
}

pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .resolve(id, "cli-administrator")
        .map_err(|e| CuratorError::EscalationResolutionFailed(e.to_string()))
}

pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .dismiss(id, "cli-administrator")
        .map_err(|e| CuratorError::EscalationResolutionFailed(e.to_string()))
}

pub async fn curator_metacognition() -> Result<String, CuratorError> {
    use hkask_agents::adapters::CnsRuntimeAdapter;
    use hkask_agents::curator::{MetacognitionConfig, MetacognitionLoop};
    use hkask_cns::CnsRuntime;

    let conn = open_registry_db()?;
    let queue = Arc::new(
        hkask_agents::EscalationQueue::new(conn)
            .map_err(|e| CuratorError::DatabaseError(e.to_string()))?,
    );

    let cns = Arc::new(CnsRuntimeAdapter::new(Arc::new(CnsRuntime::new())));
    let config = MetacognitionConfig::default();
    let loop_instance = MetacognitionLoop::new(cns, queue, config);

    let snapshot = loop_instance
        .run_cycle()
        .await
        .map_err(|e| CuratorError::MetacognitionFailed(e.to_string()))?;

    Ok(loop_instance.generate_summary(&snapshot))
}

pub fn ensemble_standing_start(
    config_path: &std::path::Path,
) -> Result<hkask_ensemble::StandingSessionStatus, EnsembleError> {
    let session = hkask_ensemble::bootstrap_standing_session(config_path)
        .map_err(|e| EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}

pub fn ensemble_standing_status() -> Result<hkask_ensemble::StandingSessionStatus, EnsembleError> {
    let config_path = std::path::Path::new("registry/manifests/standing-ensemble-session.yaml");
    if !config_path.exists() {
        return Err(EnsembleError::SessionNotFound(
            "Standing session not bootstrapped. Run 'kask ensemble standing-start' first."
                .to_string(),
        ));
    }

    let session = hkask_ensemble::bootstrap_standing_session(config_path)
        .map_err(|e| EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}
