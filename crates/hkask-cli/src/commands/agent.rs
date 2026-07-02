//! Agent registration and listing — delegates to AgentService.
//!
//! All domain operations come from AgentService. Each command builds its
//! context once at the entry point and uses it directly.

use std::path::PathBuf;

use hkask_agents::a2a::AgentReceipt;
use hkask_ports::git_cas::GixCasAdapter;
use hkask_services_context::AgentService;
use hkask_storage::RegisteredAgent;
use hkask_types::agent_paths;

use crate::block_on;
use crate::cli::BotAction;
use hex;

/// Revert an agent to a specific commit.
pub(crate) fn revert_agent(
    ctx: &AgentService,
    name: &str,
    commit: &str,
    reason: &str,
) -> Result<String, String> {
    let adapter =
        GixCasAdapter::from_env().map_err(|e| format!("Failed to init CAS adapter: {e}"))?;
    let commit_hash: hkask_ports::git_cas::CommitHash = commit
        .parse()
        .map_err(|e: hkask_ports::git_cas::ParseHashError| format!("Invalid hash: {e}"))?;
    let sanitized = agent_paths::sanitize_name(name);
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let agent_dir = base.join("hkask").join("agents").join(sanitized);
    let file = "agent.yaml".to_string();
    let content = adapter
        .get_content(&commit_hash, &file)
        .map_err(|e| format!("CAS read failed: {e}"))?;
    std::fs::create_dir_all(&agent_dir).map_err(|e| format!("mkdir: {e}"))?;
    std::fs::write(agent_dir.join(&file), &content).map_err(|e| format!("Write failed: {e}"))?;
    Ok(format!(
        "Reverted {} to commit {} ({})",
        name,
        &commit[..8],
        reason
    ))
}

/// List and display agents.
pub(crate) fn list_agents(ctx: &AgentService, kind_filter: Option<&str>) {
    let agents = match ctx.storage().agents.list() {
        Ok(all) => match kind_filter {
            Some(kind_str) => {
                if let Ok(kind) = kind_str.parse() {
                    all.into_iter()
                        .filter(|a| a.definition.agent_kind == kind)
                        .collect()
                } else {
                    eprintln!("Unknown agent kind: {}", kind_str);
                    return;
                }
            }
            None => all,
        },
        Err(e) => {
            eprintln!("Failed to list agents: {}", e);
            return;
        }
    };
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

/// Show detailed status for one agent.
pub(crate) fn show_agent_status(ctx: &AgentService, name: &str) {
    let agent = match ctx.storage().agents.get(name) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Agent not found: {}", e);
            return;
        }
    };
    let def = &agent.definition;
    println!("Agent: {}", def.name);
    println!("  Kind: {}", def.agent_kind);
    if let Some(c) = &def.charter {
        println!("  Charter: {}", c.description);
    }
    println!("  Capabilities:");
    for cap in &def.capabilities {
        println!("    - {}", cap);
    }
    if !def.rights.is_empty() {
        println!("  Rights:");
        for r in &def.rights {
            println!("    - {}", r.to_display_string());
        }
    }
    if !def.responsibilities.is_empty() {
        println!("  Responsibilities:");
        for r in &def.responsibilities {
            println!("    - {}", r.to_display_string());
        }
    }
    println!("  Registered: {}", agent.registered_at);
    println!("  Source: {}", agent.source_yaml);
}

/// List agents (compact format for `kask agent list`).
pub(crate) fn list_agents_compact(ctx: &AgentService) {
    match ctx.storage().agents.list() {
        Ok(agents) => {
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
        Err(e) => eprintln!("Failed to list agents: {}", e),
    }
}

/// Show capabilities for one agent.
pub(crate) fn show_agent_capabilities(ctx: &AgentService, name: &str) {
    match ctx.storage().agents.get(name) {
        Ok(agent) => {
            println!("Capabilities for {}:", agent.definition.name);
            for cap in &agent.definition.capabilities {
                println!("  - {}", cap);
            }
        }
        Err(e) => eprintln!("Agent not found: {}", e),
    }
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; action is a BotAction variant (List or Status)
/// post: for List — prints table of all agents; for Status — prints detailed agent info
pub fn run_bot(rt: &tokio::runtime::Runtime, action: BotAction) {
    tracing::info!(target: "cns.cli", operation = "bot", action = ?action, "CNS");
    let ctx = super::helpers::build_agent_service();
    match action {
        BotAction::List { kind } => list_agents(&ctx, kind.as_deref()),
        BotAction::Status { name } => show_agent_status(&ctx, &name),
    }
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; action is an AgentAction variant
pub fn run_agent(rt: &tokio::runtime::Runtime, action: crate::cli::AgentAction) {
    tracing::info!(target: "cns.cli", operation = "agent", action = ?action, "CNS");
    let ctx = super::helpers::build_agent_service();
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
                register_agent(&ctx, &webid, &agent_type, caps),
                "Registration failed"
            );
            println!("Agent registered:");
            println!("  WebID: {}", receipt.webid);
            println!("  Token: {}...", &receipt.token_hash[..16]);
            println!("  Registered at: {}", receipt.registered_at);
        }
        crate::cli::AgentAction::Unregister { name } => match ctx.storage().agents.remove(&name) {
            Ok(()) => println!("Agent unregistered: {}", name),
            Err(e) => eprintln!("Failed to unregister: {}", e),
        },
        crate::cli::AgentAction::List => list_agents_compact(&ctx),
        crate::cli::AgentAction::Capabilities { name } => show_agent_capabilities(&ctx, &name),
        crate::cli::AgentAction::Revert {
            name,
            commit,
            reason,
        } => match revert_agent(&ctx, &name, &commit, &reason) {
            Ok(msg) => println!("{}", msg),
            Err(e) => eprintln!("Revert failed: {}", e),
        },
    }
}

// ── Async helpers (kept minimal — only for genuinely async operations) ────

/// Register an agent — needs async because A2A registration is async.
async fn register_agent(
    ctx: &AgentService,
    webid_str: &str,
    agent_type: &str,
    capabilities: Vec<String>,
) -> Result<AgentReceipt, String> {
    let webid: hkask_types::WebID = webid_str
        .parse()
        .map_err(|e| format!("Invalid WebID: {e}"))?;
    let kind = hkask_agents::AgentKind::parse(agent_type)
        .ok_or_else(|| format!("Unknown agent kind: {}", agent_type))?;
    let (_, a2a) = ctx.identity();
    a2a.register_agent(webid, kind, capabilities)
        .await
        .map_err(|e| format!("A2A registration failed: {e}"))
}
