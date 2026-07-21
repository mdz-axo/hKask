//! REPL /agent and /agents handlers — agent switching, listing, registration.

use hkask_types::{AgentKind, WebID};

/// Handle `/agent` — switch agent, or register a new one.
pub fn handle_agent(
    arg1: &str,
    rest: &str,
    state: &mut super::super::ReplState,
    rt: &tokio::runtime::Handle,
) {
    match arg1 {
        "" => {
            println!("  Current agent: \x1b[1m{}\x1b[0m", state.current_agent);
            println!("  Use \x1b[36m/agent <NAME>\x1b[0m to switch");
            println!("  Use \x1b[36m/agent register <webid> <type> <caps>\x1b[0m to register");
            println!();
        }

        "register" => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 2 {
                println!("  \x1b[31mError:\x1b[0m WebID and agent type required");
                println!(
                    "  Usage: \x1b[36m/agent register <webid> <bot|replicant> [cap1,cap2,...]\x1b[0m"
                );
                println!();
                return;
            }
            let webid_str = parts[0];
            let agent_type_str = parts[1];
            let capabilities: Vec<String> = parts
                .get(2)
                .map(|s| s.split(',').map(|c| c.trim().to_string()).collect())
                .unwrap_or_default();

            let webid = match webid_str.parse::<WebID>() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("  \x1b[31m✗\x1b[0m Invalid WebID '{}': {}", webid_str, e);
                    println!();
                    return;
                }
            };

            let agent_type = match AgentKind::parse(agent_type_str) {
                Some(k) => k,
                None => {
                    eprintln!(
                        "  \x1b[31m✗\x1b[0m Invalid agent type: {} (use 'bot' or 'replicant')",
                        agent_type_str
                    );
                    println!();
                    return;
                }
            };

            let a2a = state.service_context.governance().a2a.clone();
            match rt.block_on(a2a.register_agent(webid, capabilities.clone())) {
                Ok(_token) => {
                    println!("  \x1b[32m✓\x1b[0m Registered agent: {}", webid_str);
                    println!("    Type:         {}", agent_type);
                    println!("    Capabilities: {}", capabilities.join(", "));
                    println!();
                }
                Err(e) => {
                    eprintln!("  \x1b[31m✗\x1b[0m Registration failed: {}", e);
                    println!();
                }
            }
        }

        "spawn" => {
            let _name = rest.trim();
            println!("  \x1b[2mAgent spawning is handled via /pod create.\x1b[0m");
            println!("  \x1b[2mUse /pod create <template> <persona.yaml> [name]\x1b[0m");
            println!();
        }

        // Default: switch agent
        name => {
            let msg = switch_agent(state, name);
            println!("  \x1b[1m{}\x1b[0m", msg);
            println!();
        }
    }
}

/// Switch the active agent and load its persona constraints.
/// Returns a confirmation string. Shared by the REPL `/agent` handler and
/// the TUI `SessionBridge` (no parallel logic).
pub(crate) fn switch_agent(state: &mut super::super::ReplState, name: &str) -> String {
    state.current_agent = name.to_string();
    state.persona_constraints = state
        .service_context
        .storage()
        .agents
        .get(name)
        .ok()
        .and_then(|agent| {
            hkask_agents::yaml_parser::parse_agent_from_yaml(&agent.source_yaml)
                .map_err(|e| format!("{e}"))
                .or_else(|_| {
                    let disk_path = hkask_types::agent_paths::agent_definition_yaml(name);
                    std::fs::read_to_string(&disk_path)
                        .map_err(|e| format!("Failed to read agent YAML from disk: {e}"))
                        .and_then(|content| {
                            hkask_agents::yaml_parser::parse_agent_from_yaml(&content)
                                .map_err(|e| format!("{e}"))
                        })
                })
                .ok()
                .and_then(|def| def.persona)
        });
    format!("Switched to agent: {}", state.current_agent)
}

/// Render the registered-agent list as a display string (no printing).
pub(crate) fn list_agents_display(state: &super::super::ReplState) -> String {
    match state.service_context.storage().agents.list() {
        Ok(agents) if agents.is_empty() => "No agents registered.".to_string(),
        Ok(agents) => {
            let mut out = format!("Agents ({})\n", agents.len());
            out.push_str(&format!("{:<25} {:<12} CAPABILITIES\n", "NAME", "KIND"));
            out.push_str(&"-".repeat(70));
            out.push('\n');
            for agent in &agents {
                out.push_str(&format!(
                    "{:<25} {}\n",
                    agent.definition.name,
                    agent.definition.capabilities.join(", ")
                ));
            }
            out
        }
        Err(e) => format!("Error listing agents: {}", e),
    }
}

/// Handle `/agents` — list all registered agents.
pub fn handle_agents(state: &super::super::ReplState) {
    println!("{}", list_agents_display(state));
}
