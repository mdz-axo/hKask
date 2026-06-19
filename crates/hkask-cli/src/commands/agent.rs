//! Agent registration and bot listing — delegates to AgentService.
//!
//! All domain operations (A2A, store) come from AgentService.
//! No direct Database::open(), A2ARuntime::new(), or AgentRegistryStore::new().


use std::str::FromStr;

use crate::block_on;
use crate::cli::BotAction;
use hkask_services::ServiceError;
use hkask_types::{AgentDefinition, AgentKind, RegisteredAgent, WebID};

#[derive(Debug)]
pub struct AgentReceipt {
    pub webid: String,
    pub token_hash: String,
    pub registered_at: String,
}

pub async fn bot_list(kind_filter: Option<&str>) -> Result<Vec<RegisteredAgent>, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    let agents =
        ctx.agent_registry_store()
            .list()
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })?;
    Ok(match kind_filter.and_then(AgentKind::parse) {
        Some(kind) => agents
            .into_iter()
            .filter(|a| a.definition.agent_kind == kind)
            .collect(),
        None => agents,
    })
}

pub async fn bot_status(name: &str) -> Result<RegisteredAgent, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    ctx.agent_registry_store()
        .get(name)
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })
}

pub async fn agent_register(
    webid_str: &str,
    agent_type: &str,
    capabilities: Vec<String>,
) -> Result<AgentReceipt, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    let webid = WebID::from_str(webid_str)?;
    let kind = AgentKind::parse(agent_type).ok_or_else(|| ServiceError::InvalidAgentType {
        source: None,
        message: agent_type.to_string(),
    })?;
    let (_, a2a) = ctx.identity();
    let token = a2a
        .register_agent(webid, kind, capabilities)
        .await
        .map_err(|e| ServiceError::A2A {
            message: e.to_string(),
        })?;
    let def = AgentDefinition {
        name: webid_str.to_string(),
        agent_kind: kind,
        charter: None,
        capabilities: vec![],
        rights: vec![],
        responsibilities: vec![],
        persona: None,
        depends_on: vec![],
        process_manifest: None,
        voice_description: None,
        voice_id: None,
    };
    let reg = RegisteredAgent {
        definition: def,
        token_hash: hex::encode(token.signature_bytes()),
        registered_at: hkask_types::time::now_rfc3339(),
        source_yaml: "cli-register".to_string(),
    };
    ctx.agent_registry_store()
        .insert(&reg)
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })?;
    Ok(AgentReceipt {
        webid: webid_str.to_string(),
        token_hash: hex::encode(token.signature_bytes()),
        registered_at: reg.registered_at,
    })
}

pub async fn agent_unregister(name: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    ctx.agent_registry_store()
        .remove(name)
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })
}

