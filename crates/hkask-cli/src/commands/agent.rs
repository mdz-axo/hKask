//! Agent registration and bot listing — delegates to AgentService.
//!
//! All domain operations (ACP, store) come from AgentService.
//! No direct Database::open(), AcpRuntime::new(), or AgentRegistryStore::new().

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
    let agents = ctx
        .agent_registry_store()
        .list()
        .map_err(ServiceError::from)?;
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
        .map_err(ServiceError::from)
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
    let (_, acp) = ctx.identity();
    let token = acp.register_agent(webid, kind, capabilities).await?;
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
        .map_err(ServiceError::from)?;
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
        .map_err(ServiceError::from)
}

pub fn run_bot(rt: &tokio::runtime::Runtime, action: BotAction) {
    use crate::commands;
    match action {
        BotAction::List { kind } => {
            let agents = block_on!(
                rt,
                commands::bot_list(kind.as_deref()),
                "Failed to list agents"
            );
            if agents.is_empty() {
                println!("No agents registered.");
            } else {
                println!(
                    "{:<25} {:<12} {:<40} SOURCE",
                    "NAME", "KIND", "CAPABILITIES"
                );
                println!("{}", "-".repeat(100));
                for agent in &agents {
                    println!(
                        "{:<25} {:<12} {:<40} {}",
                        agent.definition.name,
                        agent.definition.agent_kind,
                        agent.definition.capabilities.len(),
                        agent.source_yaml
                    );
                }
                println!("\nTotal: {} agents", agents.len());
            }
        }
        BotAction::Status { name } => {
            let agent = block_on!(
                rt,
                commands::bot_status(&name),
                "Failed to get agent status"
            );
            let def = &agent.definition;
            println!("Agent: {}", def.name);
            println!("  Kind: {}", def.agent_kind);
            if let Some(c) = &def.charter {
                println!("  Charter: {}", c.description);
                println!("  Archetype: {}", c.archetype);
            }
            println!("  Capabilities:");
            for cap in &def.capabilities {
                println!("    - {}", cap);
            }
            if !def.rights.is_empty() {
                println!("  Rights:");
                for r in def.rights_flat() {
                    println!("    - {}", r);
                }
            }
            if !def.responsibilities.is_empty() {
                println!("  Responsibilities:");
                for r in def.responsibilities_flat() {
                    println!("    - {}", r);
                }
            }
            if let Some(p) = &def.persona {
                println!("  Persona:");
                println!("    Tone: {}", p.tone);
                println!("    Verbosity: {}", p.verbosity);
                if !p.forbidden.is_empty() {
                    println!("    Forbidden: {}", p.forbidden.join(", "));
                }
            }
            println!("  Registered: {}", agent.registered_at);
            println!("  Source: {}", agent.source_yaml);
        }
    }
}

pub fn run_agent(rt: &tokio::runtime::Runtime, action: crate::cli::AgentAction) {
    use crate::commands;
    match action {
        crate::cli::AgentAction::Register {
            webid,
            agent_type,
            capabilities,
        } => {
            let caps: Vec<String> = capabilities
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            let receipt = block_on!(
                rt,
                commands::agent_register(&webid, &agent_type, caps),
                "Registration failed"
            );
            println!("Agent registered:");
            println!("  WebID: {}", receipt.webid);
            println!("  Token: {}...", &receipt.token_hash[..16]);
            println!("  Registered at: {}", receipt.registered_at);
        }
        crate::cli::AgentAction::Unregister { name } => {
            block_on!(rt, commands::agent_unregister(&name), "Unregister failed");
            println!("Agent unregistered: {}", name);
        }
        crate::cli::AgentAction::List => {
            let agents = block_on!(rt, commands::bot_list(None), "Failed to list agents");
            if agents.is_empty() {
                println!("No agents registered.");
            } else {
                println!("{:<25} {:<12} {:<40}", "NAME", "KIND", "CAPABILITIES");
                println!("{}", "-".repeat(80));
                for agent in &agents {
                    println!(
                        "{:<25} {:<12} {:<40}",
                        agent.definition.name,
                        agent.definition.agent_kind,
                        agent.definition.capabilities.join(", ")
                    );
                }
            }
        }
        crate::cli::AgentAction::Capabilities { name } => {
            let agent = block_on!(
                rt,
                commands::bot_status(&name),
                "Failed to get capabilities"
            );
            println!("Capabilities for {}:", agent.definition.name);
            for cap in &agent.definition.capabilities {
                println!("  - {}", cap);
            }
        }
    }
}
