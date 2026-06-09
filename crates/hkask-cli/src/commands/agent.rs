//! Agent registration and bot listing command handlers

use crate::block_on;
use crate::cli::BotAction;
use crate::errors::AgentError;
use hkask_services::{AgentReceipt, AgentService, ServiceContext};

/// List registered agents, optionally filtered by kind
pub async fn bot_list(
    kind_filter: Option<&str>,
) -> Result<Vec<hkask_types::RegisteredAgent>, AgentError> {
    let ctx = ServiceContext::build(hkask_services::ServiceConfig::from_env()?).await?;
    AgentService::list(&ctx, kind_filter)
        .await
        .map_err(Into::into)
}

/// Get status/details for a specific agent by name
pub async fn bot_status(name: &str) -> Result<hkask_types::RegisteredAgent, AgentError> {
    let ctx = ServiceContext::build(hkask_services::ServiceConfig::from_env()?).await?;
    AgentService::status(&ctx, name).await.map_err(Into::into)
}

/// Register a new agent with ACP
pub async fn agent_register(
    webid_str: &str,
    agent_type: &str,
    capabilities: Vec<String>,
) -> Result<AgentReceipt, AgentError> {
    let ctx = ServiceContext::build(hkask_services::ServiceConfig::from_env()?).await?;
    AgentService::register(&ctx, webid_str, agent_type, capabilities)
        .await
        .map_err(Into::into)
}

/// Unregister an agent by name
pub async fn agent_unregister(name: &str) -> Result<(), AgentError> {
    let ctx = ServiceContext::build(hkask_services::ServiceConfig::from_env()?).await?;
    AgentService::unregister(&ctx, name).map_err(Into::into)
}
/// CLI handler for `kask bot` subcommand
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
                        agent.source_yaml,
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
            if let Some(charter) = &def.charter {
                println!("  Charter: {}", charter.description);
                println!("  Archetype: {}", charter.archetype);
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
            if let Some(persona) = &def.persona {
                println!("  Persona:");
                println!("    Tone: {}", persona.tone);
                println!("    Verbosity: {}", persona.verbosity);
                if !persona.forbidden.is_empty() {
                    println!("    Forbidden: {}", persona.forbidden.join(", "));
                }
            }
            println!("  Registered: {}", agent.registered_at);
            println!("  Source: {}", agent.source_yaml);
        }
    }
}

/// CLI handler for `kask agent` subcommand
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
                        agent.definition.capabilities.join(", "),
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
