//! Agent registration and bot listing command handlers

use crate::commands::config::{init_registry, registry_yaml_path};
use crate::errors::AgentError;
use std::sync::Arc;

pub struct AgentReceipt {
    pub webid: String,
    pub token_hash: String,
    pub registered_at: String,
}

/// List registered agents, optionally filtered by kind
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

/// Get status/details for a specific agent by name
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

/// Register a new agent with ACP
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

    let agent_kind = hkask_types::AgentKind::parse(agent_type).ok_or_else(|| {
        AgentError::RegistrationFailed(format!(
            "Unknown agent type '{}'. Must be 'Bot' or 'Replicant'.",
            agent_type
        ))
    })?;

    let definition = hkask_types::AgentDefinition {
        name: webid_str.to_string(),
        agent_kind,
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

/// Unregister an agent by name
pub async fn agent_unregister(name: &str) -> Result<(), AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    store
        .remove(name)
        .map_err(|e| AgentError::UnregistrationFailed(e.to_string()))?;

    Ok(())
}
