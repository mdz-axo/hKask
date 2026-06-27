//! Agent registration and bot listing — delegates to AgentService.
//!
//! All domain operations (A2A, store) come from AgentService.
//! No direct Database::open(), A2ARuntime::new(), or AgentRegistryStore::new().

use std::path::PathBuf;
use std::str::FromStr;

use crate::block_on;
use crate::cli::BotAction;
use hex;
use hkask_mcp::GixCasAdapter;
use hkask_services::ServiceError;
use hkask_storage::{AgentDefinition, RegisteredAgent};
use hkask_types::{AgentKind, WebID, agent_paths};

#[derive(Debug)]
pub struct AgentReceipt {
    pub webid: String,
    pub token_hash: String,
    pub registered_at: String,
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  kind_filter is None or a valid AgentKind string; service context must be buildable
/// post: returns all registered agents, optionally filtered by kind; empty vec if none match
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

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  name is a non-empty agent name string; agent must exist in registry
/// post: returns the RegisteredAgent for the given name or ServiceError if not found
pub async fn bot_status(name: &str) -> Result<RegisteredAgent, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    ctx.agent_registry_store()
        .get(name)
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  webid_str is a valid WebID; agent_type is a valid AgentKind; capabilities is a list of capability strings
/// post: registers the agent via A2A, stores in registry, returns AgentReceipt with webid, token_hash, and timestamp
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
        .register_agent(webid, kind, capabilities.clone())
        .await
        .map_err(|e| ServiceError::A2A {
            message: e.to_string(),
        })?;
    // Build the self-contained agent definition YAML.
    let source_yaml = format!(
        "# Agent definition for {name} — registered via CLI.\n\
         agent:\n  name: \"{name}\"\n  type: {kind}\n\n\
         capabilities:\n{cap_lines}\n",
        name = webid_str,
        kind = agent_type,
        cap_lines = capabilities
            .iter()
            .map(|c| format!("  - {c}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // Persist to agents/{name}/agent.yaml for discovery and REPL loading.
    let yaml_path = agent_paths::agent_definition_yaml(webid_str);
    if let Some(parent) = yaml_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&yaml_path, &source_yaml);

    let def = AgentDefinition {
        name: webid_str.to_string(),
        agent_kind: kind,
        charter: None,
        capabilities,
        rights: vec![],
        responsibilities: vec![],
    };
    let reg = RegisteredAgent {
        definition: def,
        token_hash: hex::encode(token.signature_bytes()),
        registered_at: hkask_types::time::now_rfc3339(),
        source_yaml,
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

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  name is a non-empty agent name string; agent must exist in registry
/// post: removes the agent from the registry; returns Ok(()) or ServiceError if not found
pub async fn agent_unregister(name: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    ctx.agent_registry_store()
        .remove(name)
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; action is a BotAction variant (List or Status)
/// post: for List — prints table of all agents (or "No agents registered"); for Status — prints detailed agent info
pub fn run_bot(rt: &tokio::runtime::Runtime, action: BotAction) {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "bot", action = ?action, "CNS");
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
                println!("  Charter: {}", c.purpose);
            }
            println!("  Capabilities:");
            for cap in &def.capabilities {
                println!("    - {}", cap);
            }
            if !def.rights.is_empty() {
                println!("  Rights:");
                for r in &def.rights {
                    println!("    - {}", r.name);
                }
            }
            if !def.responsibilities.is_empty() {
                println!("  Responsibilities:");
                for r in &def.responsibilities {
                    println!("    - {}", r.name);
                }
            }
            println!("  Registered: {}", agent.registered_at);
            println!("  Source: {}", agent.source_yaml);
        }
    }
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; action is an AgentAction variant (Register, Unregister, List, Capabilities)
/// post: dispatches to the appropriate handler; prints results to stdout; exits on fatal errors
pub fn run_agent(rt: &tokio::runtime::Runtime, action: crate::cli::AgentAction) {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "agent", action = ?action, "CNS");
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
        crate::cli::AgentAction::Revert {
            name,
            commit,
            reason,
        } => {
            let adapter = GixCasAdapter::from_env().unwrap_or_else(|e| {
                eprintln!("Failed to initialize CAS adapter: {}", e);
                std::process::exit(1);
            });
            let commit_hash: hkask_ports::git_cas::CommitHash =
                commit
                    .parse()
                    .unwrap_or_else(|e: hkask_ports::git_cas::ParseHashError| {
                        eprintln!("Invalid commit hash '{}': {}", commit, e);
                        std::process::exit(1);
                    });
            let sanitized = agent_paths::sanitize_name(&name);
            let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            let pod_dir = base.join("hkask").join("agents").join(&sanitized);
            let pod_db_path = pod_dir.join("pod.db");
            if !pod_db_path.exists() {
                eprintln!(
                    "Pod database not found at {}. Is the pod activated?",
                    pod_db_path.display()
                );
                std::process::exit(1);
            }

            // Safety snapshot before revert
            let safety_commit = block_on!(
                rt,
                adapter.snapshot_pod_dir(
                    &pod_dir,
                    &format!("safety: pre-revert {} ({})", name, reason)
                ),
                "Safety snapshot failed"
            );
            println!("Safety snapshot: {}", safety_commit);

            // Restore pod.db from target commit
            block_on!(
                rt,
                adapter.restore_file_from_commit(&pod_dir, &commit_hash, "pod.db", &pod_db_path),
                "Restore failed"
            );
            println!("Agent '{}' reverted to commit {}.", name, commit_hash);
            println!(
                "\nTo undo this revert, restore from safety snapshot {}.",
                safety_commit
            );
            println!(
                "\n⚠️  Pod '{}' is still running with pre-revert state.",
                name
            );
            println!("   Restart the pod to apply the restored database:");
            println!(
                "   kask pod deactivate {} && kask pod activate {}",
                name, name
            );
        }
        crate::cli::AgentAction::SpawnAgent {
            source,
            new_name,
            commit,
        } => {
            let adapter = GixCasAdapter::from_env().unwrap_or_else(|e| {
                eprintln!("Failed to initialize CAS adapter: {}", e);
                std::process::exit(1);
            });
            let commit_hash: hkask_ports::git_cas::CommitHash =
                commit
                    .parse()
                    .unwrap_or_else(|e: hkask_ports::git_cas::ParseHashError| {
                        eprintln!("Invalid commit hash '{}': {}", commit, e);
                        std::process::exit(1);
                    });
            let source_sanitized = agent_paths::sanitize_name(&source);
            let target_sanitized = agent_paths::sanitize_name(&new_name);
            let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            let source_dir = base.join("hkask").join("agents").join(&source_sanitized);
            let target_dir = base.join("hkask").join("agents").join(&target_sanitized);
            std::fs::create_dir_all(&target_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create agent directory: {}", e);
                std::process::exit(1);
            });
            let new_db_path = target_dir.join("pod.db");

            // Restore pod.db from source agent's commit into the new agent dir
            block_on!(
                rt,
                adapter.restore_file_from_commit(&source_dir, &commit_hash, "pod.db", &new_db_path),
                "Spawn agent restore failed"
            );
            println!("Agent spawned from '{}' as '{}'.", source, new_name);
            println!("  Source commit: {}", commit_hash);
            println!("  Database:      {}", new_db_path.display());
            println!("\nActivate with: kask pod activate {}", new_name);
        }
    }
}
