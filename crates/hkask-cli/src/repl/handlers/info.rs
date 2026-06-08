//! REPL info handlers — /history, /pods, /templates, /tools, /metacognition, /sovereignty

use hkask_types::ports::ToolPort;

pub(crate) fn handle_history(state: &super::super::ReplState) {
    if state.session_history.turns.is_empty() {
        println!("  No turns in this session yet.");
    } else {
        println!(
            "  Session history ({} turns):",
            state.session_history.turns.len()
        );
        for (i, (agent, response)) in state.session_history.turns.iter().enumerate() {
            let preview = if response.len() > 80 {
                format!("{}…", &response[..80])
            } else {
                response.clone()
            };
            println!("  {:>3}. {}: {}", i + 1, agent, preview);
        }
    }
    println!();
}

pub(crate) fn handle_pods(rt: &tokio::runtime::Handle) {
    match rt.block_on(crate::commands::list_pods()) {
        Ok(pods) => {
            if pods.is_empty() {
                println!("  No pods registered.");
            } else {
                println!("  \x1b[1mAgent pods ({}):\x1b[0m", pods.len());
                for pod in &pods {
                    println!("  \x1b[36m{}\x1b[0m ({})", pod.pod_id, pod.state);
                    println!("    WebID: {}", pod.webid);
                    if let Some(name) = &pod.name {
                        println!("    Name:  {}", name);
                    }
                }
            }
        }
        Err(e) => println!("  \x1b[31mPod listing unavailable:\x1b[0m {}", e),
    }
    println!();
}

pub(crate) fn handle_templates(rt: &tokio::runtime::Handle) {
    let entries = rt.block_on(async { crate::commands::list_templates_local() });
    if entries.is_empty() {
        println!("  No templates registered.");
    } else {
        println!("  \x1b[1mTemplates ({}):\x1b[0m", entries.len());
        for entry in &entries {
            println!(
                "  \x1b[36m{}\x1b[0m ({})",
                entry.id,
                entry.template_type.as_str()
            );
        }
    }
    println!();
}

pub(crate) fn handle_tools(state: &mut super::super::ReplState, rt: &tokio::runtime::Handle) {
    let tools = rt.block_on(state.governed_tool.discover_tools());
    if tools.is_empty() {
        println!("  No MCP tools available. Start MCP servers to register tools.");
    } else {
        println!("  \x1b[1mMCP Tools ({}):\x1b[0m", tools.len());
        for tool_name in &tools {
            if let Some(info) = rt.block_on(state.governed_tool.get_tool_info(tool_name)) {
                println!("  \x1b[36m{}\x1b[0m — {}", info.name, info.description);
            } else {
                println!("  \x1b[36m{}\x1b[0m", tool_name);
            }
        }
        println!("  \x1b[2mAll tool calls route through GovernedTool (OCAP + gas)\x1b[0m");
    }
    println!();
}

pub(crate) fn handle_metacognition(rt: &tokio::runtime::Handle) {
    rt.block_on(async {
        match crate::commands::curator_metacognition().await {
            Ok(summary) => println!("  {}", summary),
            Err(e) => println!("  Error: {}", e),
        }
    });
    println!();
}

pub(crate) fn handle_sovereignty() {
    crate::commands::sovereignty::run(crate::cli::SovereigntyAction::Status);
}
