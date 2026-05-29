use super::display::{print_command_help, print_help};
use super::helper::SessionHistory;

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
    current_agent: &mut String,
    current_model: &mut String,
    session_history: &mut SessionHistory,
    template_id: Option<&str>,
    active_session: &mut Option<String>,
    rt: &tokio::runtime::Handle,
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
            if session_history.turns.is_empty() {
                println!("  No turns in this session yet.");
            } else {
                println!("  Session history ({} turns):", session_history.turns.len());
                for (i, (agent, response)) in session_history.turns.iter().enumerate() {
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
            let agent_display = current_agent.clone();
            let tpl = template_id.unwrap_or("auto-select");
            println!("  Agent:      \x1b[1m{}\x1b[0m", agent_display);
            println!(
                "  Model:      \x1b[1m{}\x1b[0m",
                if current_model.is_empty() {
                    "default"
                } else {
                    current_model
                }
            );
            println!("  Template:   {}", tpl);
            println!("  CNS:        \x1b[32mHEALTHY\x1b[0m (no alerts)");
            println!("  Turns:      {}", session_history.turns.len());
            match &active_session {
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
                println!("  Current agent: \x1b[1m{}\x1b[0m", current_agent);
                println!(
                    "  Use \x1b[36m/agent <NAME>\x1b[0m to switch, \x1b[36m/agents\x1b[0m to list"
                );
            } else {
                *current_agent = arg1.to_string();
                println!("  Switched to agent: \x1b[1m{}\x1b[0m", current_agent);
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
            let state = hkask_types::UserSovereigntyState::new();
            println!("  Sovereignty Status:");
            println!("    Consent:    {}", state.explicit_consent);
            println!("    Compromised: {}", state.is_compromised());
            println!("    Kill zone:  {}", state.detector.kill_zone_active);
            println!();
        }
        "pods" => {
            let pods = rt.block_on(crate::commands::list_pods());
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
            handle_ensemble(arg1, arg2, active_session, rt);
        }
        "into" | "i" => {
            handle_into(arg1, active_session, rt);
        }
        "filter" | "thresh" => {
            handle_filter(arg1, active_session, rt);
        }
        "mode" => {
            handle_mode(arg1, active_session, rt);
        }
        "ask" => {
            handle_ask(arg1, arg2, active_session, rt);
        }
        "model" | "m" => {
            handle_model(arg1, current_model, rt);
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

pub(super) fn handle_ensemble(
    subcmd: &str,
    rest: &str,
    active_session: &mut Option<String>,
    rt: &tokio::runtime::Handle,
) {
    match subcmd {
        "sessions" | "list" | "" => {
            rt.block_on(async {
                match crate::commands::ensemble_chat_list().await {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            println!("  No active ensemble sessions.");
                            println!("  Use \x1b[36m/ensemble create <id>\x1b[0m to start one.");
                        } else {
                            println!("  \x1b[1mEnsemble sessions:\x1b[0m");
                            for s in &sessions {
                                let active = match &active_session {
                                    Some(a) if a == s => " \x1b[1;33m← active\x1b[0m",
                                    _ => "",
                                };
                                println!("    \x1b[36m•\x1b[0m {}{}", s, active);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            });
        }
        "create" => {
            if rest.is_empty() {
                println!("  Usage: \x1b[36m/ensemble create <session-id>\x1b[0m");
            } else {
                let session = rest.split_whitespace().next().unwrap_or(rest);
                rt.block_on(async {
                    match crate::commands::ensemble_chat_create(session.to_string()).await {
                        Ok(msg) => println!("  \x1b[32m✓\x1b[0m {}", msg),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
        }
        "join" | "register" => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 3 {
                println!("  Usage: \x1b[36m/ensemble join <session> <bot> <role>\x1b[0m");
                println!("  Roles: memory_bot, spandrel_bot, okapi_bot, scholar_bot");
            } else {
                rt.block_on(async {
                    match crate::commands::ensemble_chat_register(
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                    )
                    .await
                    {
                        Ok(msg) => println!("  \x1b[32m✓\x1b[0m {}", msg),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
        }
        "invite" => match &active_session {
            Some(session) => {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.is_empty() {
                    println!("  Usage: \x1b[36m/ensemble invite <bot> [role]\x1b[0m");
                    println!(
                        "  Roles: memory_bot, spandrel_bot, okapi_bot, scholar_bot (default: custom)"
                    );
                } else {
                    let bot = parts[0];
                    let role = parts
                        .get(1)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "custom".to_string());
                    rt.block_on(async {
                        match crate::commands::ensemble_chat_register(
                            session.clone(),
                            bot.to_string(),
                            role,
                        )
                        .await
                        {
                            Ok(msg) => println!("  \x1b[32m✓\x1b[0m {}", msg),
                            Err(e) => println!("  Error: {}", e),
                        }
                    });
                }
            }
            None => {
                println!(
                    "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
                );
            }
        },
        "participants" | "who" => match &active_session {
            Some(session) => {
                rt.block_on(async {
                    match crate::commands::ensemble_participants(session).await {
                        Ok(participants) => {
                            if participants.is_empty() {
                                println!("  No participants in session.");
                            } else {
                                println!("  \x1b[1mParticipants ({}):\x1b[0m", participants.len());
                                for (name, role, caps) in &participants {
                                    println!(
                                        "    \x1b[36m{}\x1b[0m ({}) caps: {}",
                                        name, role, caps
                                    );
                                }
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
            None => {
                println!(
                    "  \x1b[31mNo active session.\x1b[0m Use \x1b[36m/into <session>\x1b[0m first."
                );
            }
        },
        "send" | "say" => {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() < 2 {
                println!("  Usage: \x1b[36m/ensemble send <session> <message>\x1b[0m");
            } else {
                rt.block_on(async {
                    match crate::commands::ensemble_chat_send(
                        parts[0].to_string(),
                        parts[1].to_string(),
                    )
                    .await
                    {
                        Ok(_) => println!("  \x1b[32m✓\x1b[0m Message sent to {}", parts[0]),
                        Err(e) => println!("  Error: {}", e),
                    }
                });
            }
        }
        other => {
            println!("  Unknown ensemble subcommand: \x1b[31m{}\x1b[0m", other);
            println!("  Use: sessions, create, join, invite, participants, send");
            println!("  Type \x1b[36m/help ensemble\x1b[0m for details.");
        }
    }
    println!();
}

pub(super) fn handle_into(
    arg: &str,
    active_session: &mut Option<String>,
    rt: &tokio::runtime::Handle,
) {
    if arg.is_empty() {
        match active_session {
            Some(_) => {
                let leaving = active_session.take().unwrap();
                println!(
                    "  Left ensemble session \x1b[33m{}\x1b[0m. Back to single-agent mode.",
                    leaving
                );
            }
            None => {
                println!("  Not in an ensemble session.");
                println!("  Use \x1b[36m/into <session-id>\x1b[0m to enter one.");
                println!("  Use \x1b[36m/ensemble create <id>\x1b[0m to create one first.");
            }
        }
    } else {
        let session = arg.trim().to_string();
        let exists = rt.block_on(async {
            match crate::commands::ensemble_chat_list().await {
                Ok(sessions) => sessions.contains(&session),
                Err(_) => false,
            }
        });

        if exists {
            *active_session = Some(session.clone());
            let config_result =
                rt.block_on(async { crate::commands::ensemble_improv_config(&session).await });
            match config_result {
                Ok(config) => {
                    println!("  Entered ensemble session \x1b[33m{}\x1b[0m", session);
                    println!(
                        "  Mode: \x1b[1m{}\x1b[0m  Threshold: \x1b[1m{:.2}\x1b[0m",
                        config.mode.as_str(),
                        config.participation_threshold
                    );
                    println!("  Messages now go to the ensemble. \x1b[2m/into\x1b[0m to leave.");
                }
                Err(e) => {
                    println!(
                        "  Entered ensemble session \x1b[33m{}\x1b[0m (config error: {})",
                        session, e
                    );
                }
            }
        } else {
            println!(
                "  Session \x1b[31m{}\x1b[0m not found. Create it first with \x1b[36m/ensemble create {}\x1b[0m",
                session, session
            );
        }
    }
    println!();
}

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
    active_session: &Option<String>,
    rt: &tokio::runtime::Handle,
) {
    if arg1.is_empty() || arg2.is_empty() {
        println!("  Usage: \x1b[36m/ask <agent> <message>\x1b[0m");
        return;
    }

    match active_session {
        Some(session) => {
            let response = rt.block_on(crate::commands::chat_with_agent(arg2, Some(arg1), None));
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
            let response = rt.block_on(crate::commands::chat_with_agent(arg2, Some(arg1), None));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, response);
        }
    }
}

fn handle_model(arg1: &str, current_model: &mut String, rt: &tokio::runtime::Handle) {
    use hkask_templates::{OkapiConfig, search_okapi_models};

    if arg1.is_empty() {
        // Show current model
        if current_model.is_empty() {
            println!("  Current model: \x1b[2mdefault\x1b[0m (Okapi default)");
        } else {
            println!("  Current model: \x1b[1m{}\x1b[0m", current_model);
        }
        println!(
            "  Use \x1b[36m/model <name>\x1b[0m to switch, \x1b[36m/model <query>\x1b[0m to search"
        );
    } else {
        let config = OkapiConfig::local_dev();
        let matches = rt.block_on(search_okapi_models(&config, arg1));

        if matches.is_empty() {
            // No matches — Okapi may be unreachable or no matching models
            // Set the model anyway (user may know a valid model name)
            *current_model = arg1.to_string();
            println!("  Model set to: \x1b[1m{}\x1b[0m", current_model);
            println!("  \x1b[2m(Okapi unreachable — model name stored for next inference)\x1b[0m");
        } else if matches.len() == 1 {
            // Exact single match — switch to it
            *current_model = matches[0].name.clone();
            println!("  Model set to: \x1b[1m{}\x1b[0m", current_model);
            if let Some(ref details) = matches[0].details {
                if let Some(ref fam) = details.family {
                    println!("  Family: {}", fam);
                }
                if let Some(ref params) = details.parameter_size {
                    println!("  Parameters: {}", params);
                }
                if let Some(ref quant) = details.quantization_level {
                    println!("  Quantization: {}", quant);
                }
            }
        } else {
            // Multiple matches — show fuzzy search results
            println!(
                "  \x1b[1mModels matching '\x1b[36m{}\x1b[0m\x1b[1m' ({}):\x1b[0m",
                arg1,
                matches.len()
            );
            println!("  {:<30} {:<12} {:<15} SIZE", "NAME", "FAMILY", "PARAMS");
            println!("  {}", "-".repeat(70));
            for m in &matches {
                let family = m
                    .details
                    .as_ref()
                    .and_then(|d| d.family.as_deref())
                    .unwrap_or("-");
                let params = m
                    .details
                    .as_ref()
                    .and_then(|d| d.parameter_size.as_deref())
                    .unwrap_or("-");
                let size_str = m
                    .size
                    .map(|s| format!("{:.1} GB", s as f64 / 1_073_741_824.0))
                    .unwrap_or_else(|| "-".to_string());
                println!(
                    "  \x1b[36m{:<30}\x1b[0m {:<12} {:<15} {}",
                    m.name, family, params, size_str
                );
            }
            println!();
            println!("  Use \x1b[36m/model <name>\x1b[0m to switch to a specific model");
        }
    }
    println!();
}
