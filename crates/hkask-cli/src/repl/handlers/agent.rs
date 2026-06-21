//! REPL /agent and /agents handlers — agent switching and listing

pub(crate) fn handle_agent(
    arg1: &str,
    state: &mut super::super::ReplState,
    rt: &tokio::runtime::Handle,
) {
    if arg1.is_empty() {
        println!("  Current agent: \x1b[1m{}\x1b[0m", state.current_agent);
        println!("  Use \x1b[36m/agent <NAME>\x1b[0m to switch, \x1b[36m/agents\x1b[0m to list");
    } else {
        state.current_agent = arg1.to_string();
        // Persona constraints require rich AgentDefinition from hkask-agents.
        // TODO: convert from store type when rich persona fields are needed.
        state.persona_constraints = None;
        println!("  Switched to agent: \x1b[1m{}\x1b[0m", state.current_agent);
    }
    println!();
}

pub(crate) fn handle_agents(rt: &tokio::runtime::Handle) {
    rt.block_on(async {
        match crate::commands::bot_list(None).await {
            Ok(agents) => {
                if agents.is_empty() {
                    println!("  No agents registered.");
                } else {
                    println!("  \x1b[1mAgents ({}):\x1b[0m", agents.len());
                    println!("  {:<25} {:<12} CAPABILITIES", "NAME", "KIND");
                    println!("  {}", "-".repeat(70));
                    for agent in &agents {
                        println!(
                            "  \x1b[36m{:<25}\x1b[0m {:<12} {}",
                            agent.definition.name,
                            agent.definition.agent_kind,
                            agent.definition.capabilities.join(", "),
                        );
                    }
                }
            }
            Err(e) => println!("  Error listing agents: {}", e),
        }
    });
    println!();
}
