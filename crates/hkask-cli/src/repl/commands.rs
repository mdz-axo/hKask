use super::ReplState;
use super::display::{print_command_help, print_help};
use super::handlers::{handle_ensemble, handle_into, handle_model};

pub(super) struct SlashCommand {
    pub primary: &'static str,
    pub aliases: &'static [&'static str],
    pub args: &'static str,
    pub about: &'static str,
}

impl SlashCommand {
    pub(super) fn matches(&self, input: &str) -> bool {
        input == self.primary || self.aliases.contains(&input)
    }
}

pub(super) const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        primary: "help",
        aliases: &["h", "?"],
        args: "[COMMAND]",
        about: "Show help, or details for a specific command",
    },
    SlashCommand {
        primary: "quit",
        aliases: &["q", "exit"],
        args: "",
        about: "End the session",
    },
    SlashCommand {
        primary: "clear",
        aliases: &["cls"],
        args: "",
        about: "Clear the screen",
    },
    SlashCommand {
        primary: "status",
        aliases: &["st"],
        args: "",
        about: "System status (CNS, agent, pod count)",
    },
    SlashCommand {
        primary: "agent",
        aliases: &["a"],
        args: "[NAME]",
        about: "Switch agent, or show current",
    },
    SlashCommand {
        primary: "agents",
        aliases: &["ls"],
        args: "",
        about: "List registered agents",
    },
    SlashCommand {
        primary: "pods",
        aliases: &[],
        args: "",
        about: "List agent pods",
    },
    SlashCommand {
        primary: "templates",
        aliases: &["tpl"],
        args: "",
        about: "List registered templates",
    },
    SlashCommand {
        primary: "tools",
        aliases: &[],
        args: "",
        about: "List MCP tools",
    },
    SlashCommand {
        primary: "ensemble",
        aliases: &["ens"],
        args: "sessions|create|join|send|invite|participants",
        about: "Multi-agent ensemble operations",
    },
    SlashCommand {
        primary: "into",
        aliases: &["i"],
        args: "[SESSION]",
        about: "Enter ensemble session, or leave it",
    },
    SlashCommand {
        primary: "filter",
        aliases: &["thresh"],
        args: "[0.0-1.0]",
        about: "Set/show participation threshold",
    },
    SlashCommand {
        primary: "mode",
        aliases: &[],
        args: "[freeform|curator_led|round_robin]",
        about: "Set/show ensemble orchestration mode",
    },
    SlashCommand {
        primary: "ask",
        aliases: &[],
        args: "<AGENT> <MESSAGE>",
        about: "Force a specific agent to respond",
    },
    SlashCommand {
        primary: "model",
        aliases: &["m"],
        args: "[NAME|QUERY]",
        about: "Switch model, fuzzy search, or show current",
    },
    SlashCommand {
        primary: "escalations",
        aliases: &["esc"],
        args: "",
        about: "List pending escalations",
    },
    SlashCommand {
        primary: "resolve",
        aliases: &[],
        args: "<ID>",
        about: "Resolve an escalation",
    },
    SlashCommand {
        primary: "dismiss",
        aliases: &[],
        args: "<ID>",
        about: "Dismiss an escalation",
    },
    SlashCommand {
        primary: "metacognition",
        aliases: &["meta"],
        args: "",
        about: "Run a metacognition cycle",
    },
    SlashCommand {
        primary: "sovereignty",
        aliases: &["sov"],
        args: "",
        about: "Show sovereignty status",
    },
    SlashCommand {
        primary: "history",
        aliases: &["hist"],
        args: "",
        about: "Show session history",
    },
];

pub(super) fn find_command(input: &str) -> Option<&'static SlashCommand> {
    SLASH_COMMANDS.iter().find(|c| c.matches(input))
}

pub(super) fn fuzzy_match_command(input: &str) -> Vec<&'static SlashCommand> {
    let lower = input.to_lowercase();
    SLASH_COMMANDS
        .iter()
        .filter(|c| {
            c.primary.contains(&lower)
                || c.aliases.iter().any(|a| a.contains(&lower))
                || c.about.to_lowercase().contains(&lower)
        })
        .collect()
}

pub(super) fn handle_slash_command(
    input: &str,
    template_id: Option<&str>,
    rt: &tokio::runtime::Handle,
    state: &mut ReplState,
) -> bool {
    let without_slash = &input[1..];
    let parts: Vec<&str> = without_slash.splitn(3, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let arg1 = parts.get(1).map(|s| s.trim()).unwrap_or("");
    let arg2 = parts.get(2).map(|s| s.trim()).unwrap_or("");

    match cmd.as_str() {
        "help" | "h" | "?" => {
            if arg1.is_empty() {
                print_help();
            } else {
                print_command_help(arg1);
            }
        }
        "quit" | "q" | "exit" => {
            println!("Goodbye!");
            return true;
        }
        "clear" | "cls" => {
            print!("\x1b[2J\x1b[H");
        }
        "history" | "hist" => {
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
        "status" | "st" => {
            let agent_display = state.current_agent.clone();
            let tpl = template_id.unwrap_or("auto-select");
            println!("  Agent:      \x1b[1m{}\x1b[0m", agent_display);
            println!("  Model:      \x1b[1m{}\x1b[0m", state.current_model);
            println!("  Template:   {}", tpl);
            println!("  CNS:        \x1b[32mHEALTHY\x1b[0m (no alerts)");
            println!("  Turns:      {}", state.session_history.turns.len());
            match &state.active_session {
                Some(session) => {
                    let config = rt
                        .block_on(async { crate::commands::ensemble_improv_config(session).await });
                    match config {
                        Ok(cfg) => {
                            println!(
                                "  Ensemble:   \x1b[33m{}\x1b[0m (mode: {}, threshold: {:.2})",
                                session,
                                cfg.mode.as_str(),
                                cfg.participation_threshold
                            );
                        }
                        Err(e) => {
                            println!(
                                "  Ensemble:   \x1b[33m{}\x1b[0m (config error: {})",
                                session, e
                            );
                        }
                    }
                }
                None => {
                    println!("  Ensemble:   single-agent");
                }
            }
            println!();
        }
        "agent" | "a" => {
            if arg1.is_empty() {
                println!("  Current agent: \x1b[1m{}\x1b[0m", state.current_agent);
                println!(
                    "  Use \x1b[36m/agent <NAME>\x1b[0m to switch, \x1b[36m/agents\x1b[0m to list"
                );
            } else {
                state.current_agent = arg1.to_string();
                println!("  Switched to agent: \x1b[1m{}\x1b[0m", state.current_agent);
            }
            println!();
        }
        "agents" | "ls" => {
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
        "escalations" | "esc" => {
            rt.block_on(async {
                match crate::commands::curator_escalations().await {
                    Ok(escalations) => {
                        if escalations.is_empty() {
                            println!("  No pending escalations.");
                        } else {
                            println!("  {:<20} {:<15} {:<10} CONTEXT", "ID", "BOT", "CONFIDENCE");
                            println!("  {}", "-".repeat(70));
                            for esc in &escalations {
                                println!(
                                    "  {:<20} {:<15} {:<10.2} {}",
                                    &esc.id[..std::cmp::min(20, esc.id.len())],
                                    esc.bot_id.0.to_string().split('-').next().unwrap_or("?"),
                                    esc.confidence,
                                    &esc.error_context
                                        [..std::cmp::min(40, esc.error_context.len())],
                                );
                            }
                            println!("\n  Total: {} pending", escalations.len());
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            });
            println!();
        }
        "resolve" => {
            if arg1.is_empty() {
                println!("  Usage: /resolve <ID>");
            } else {
                rt.block_on(async {
                    match crate::commands::curator_resolve(arg1).await {
                        Ok(()) => println!("  Escalation \x1b[32m{}\x1b[0m resolved.", arg1),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            println!();
        }
        "dismiss" => {
            if arg1.is_empty() {
                println!("  Usage: /dismiss <ID>");
            } else {
                rt.block_on(async {
                    match crate::commands::curator_dismiss(arg1).await {
                        Ok(()) => println!("  Escalation \x1b[33m{}\x1b[0m dismissed.", arg1),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            println!();
        }
        "metacognition" | "meta" => {
            rt.block_on(async {
                match crate::commands::curator_metacognition().await {
                    Ok(summary) => println!("  {}", summary),
                    Err(e) => println!("  Error: {}", e),
                }
            });
            println!();
        }
        "sovereignty" | "sov" => {
            let sov_state = hkask_types::UserSovereigntyState::new();
            println!("  Sovereignty Status:");
            println!("    Consent:    {}", sov_state.explicit_consent);
            println!("    Compromised: {}", sov_state.is_compromised());
            println!(
                "    Kill zone:  {}",
                sov_state.kill_zone_state.kill_zone_active
            );
            println!();
        }
        "pods" => {
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
        "templates" | "tpl" => {
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
        "tools" => {
            println!("  MCP tools: (use \x1b[36mkask mcp list-tools\x1b[0m for details)");
            println!();
        }
        "ensemble" | "ens" => {
            handle_ensemble(arg1, arg2, &mut state.active_session, rt);
        }
        "into" | "i" => {
            handle_into(arg1, &mut state.active_session, rt);
        }
        "filter" | "thresh" => {
            handle_filter(arg1, &state.active_session, rt);
        }
        "mode" => {
            handle_mode(arg1, &state.active_session, rt);
        }
        "ask" => {
            handle_ask(arg1, arg2, rt, state);
        }
        "model" | "m" => {
            handle_model(arg1, rt, state);
        }
        _ => {
            let fuzzy = fuzzy_match_command(&cmd);
            if fuzzy.is_empty() {
                println!("  Unknown command: \x1b[31m/{}\x1b[0m", cmd);
            } else {
                println!("  Unknown command: \x1b[31m/{}\x1b[0m — did you mean:", cmd);
                for c in &fuzzy {
                    println!("    \x1b[36m/{}\x1b[0m — {}", c.primary, c.about);
                }
            }
            println!("  Type \x1b[36m/help\x1b[0m for available commands.");
            println!();
        }
    }
    false
}

// ── Inline sub-handlers (small enough to stay here) ────────────────────────

pub(super) fn handle_filter(
    arg: &str,
    active_session: &Option<String>,
    rt: &tokio::runtime::Handle,
) {
    let session_id = match active_session {
        Some(s) => s.clone(),
        None => {
            println!(
                "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
            );
            println!();
            return;
        }
    };
    if arg.is_empty() {
        let config =
            rt.block_on(async { crate::commands::ensemble_improv_config(&session_id).await });
        match config {
            Ok(cfg) => {
                println!(
                    "  Participation threshold: \x1b[1m{:.2}\x1b[0m",
                    cfg.participation_threshold
                );
                println!("  (0.0 = all speak, 1.0 = nobody speaks, 0.75 = default)");
            }
            Err(e) => println!("  Error: {}", e),
        }
    } else {
        match arg.parse::<f64>() {
            Ok(threshold) => {
                rt.block_on(async {
                    match crate::commands::ensemble_improv_set_threshold(&session_id, threshold)
                        .await
                    {
                        Ok(()) => {
                            let clamped = threshold.clamp(0.0, 1.0);
                            println!(
                                "  Participation threshold set to \x1b[1m{:.2}\x1b[0m",
                                clamped
                            );
                            if clamped < 0.5 {
                                println!("  \x1b[2m(low — most agents will speak)\x1b[0m");
                            } else if clamped > 0.9 {
                                println!("  \x1b[2m(high — very selective)\x1b[0m");
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            Err(_) => {
                println!(
                    "  Invalid threshold: \x1b[31m{}\x1b[0m. Must be 0.0-1.0",
                    arg
                );
            }
        }
    }
    println!();
}

pub(super) fn handle_mode(arg: &str, active_session: &Option<String>, rt: &tokio::runtime::Handle) {
    let session_id = match active_session {
        Some(s) => s.clone(),
        None => {
            println!(
                "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
            );
            println!();
            return;
        }
    };
    if arg.is_empty() {
        let config =
            rt.block_on(async { crate::commands::ensemble_improv_config(&session_id).await });
        match config {
            Ok(cfg) => {
                println!("  Ensemble mode: \x1b[1m{}\x1b[0m", cfg.mode.as_str());
                println!("  Options: freeform, curator_led, round_robin");
            }
            Err(e) => println!("  Error: {}", e),
        }
    } else {
        match hkask_ensemble::ImprovMode::parse_mode(arg.trim()) {
            Some(mode) => {
                rt.block_on(async {
                    match crate::commands::ensemble_improv_set_mode(&session_id, mode.clone()).await
                    {
                        Ok(()) => {
                            println!("  Ensemble mode set to \x1b[1m{}\x1b[0m", mode.as_str());
                            match mode {
                                hkask_ensemble::ImprovMode::Freeform => {
                                    println!("  \x1b[2m(agents self-select by relevance)\x1b[0m");
                                }
                                hkask_ensemble::ImprovMode::CuratorLed => {
                                    println!("  \x1b[2m(Curator picks who speaks)\x1b[0m");
                                }
                                hkask_ensemble::ImprovMode::RoundRobin => {
                                    println!("  \x1b[2m(all agents speak in turn)\x1b[0m");
                                }
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            None => {
                println!("  Unknown mode: \x1b[31m{}\x1b[0m", arg);
                println!("  Options: freeform, curator_led, round_robin");
            }
        }
    }
    println!();
}

pub(super) fn handle_ask(
    arg1: &str,
    arg2: &str,
    rt: &tokio::runtime::Handle,
    state: &mut super::ReplState,
) {
    if arg1.is_empty() || arg2.is_empty() {
        println!("  Usage: \x1b[36m/ask <agent> <message>\x1b[0m");
        return;
    }

    match &state.active_session {
        Some(session) => {
            let response = rt.block_on(crate::commands::chat_with_agent(
                arg2,
                Some(arg1),
                None,
                Some(state.inference_port.clone()),
            ));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, response);

            let manager_session = session.clone();
            rt.block_on(async {
                let _ = crate::commands::ensemble_chat_send(
                    manager_session,
                    format!("[direct to {}] {}", arg1, arg2),
                )
                .await;
            });
        }
        None => {
            let response = rt.block_on(crate::commands::chat_with_agent(
                arg2,
                Some(arg1),
                None,
                Some(state.inference_port.clone()),
            ));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, response);
        }
    }
}
