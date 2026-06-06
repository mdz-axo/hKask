use super::ReplState;
use super::display::{print_command_help, print_help};
use super::handlers::{handle_ensemble, handle_into, handle_model};
use hkask_types::ports::ToolPort;
use std::sync::Arc;

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
        primary: "invoke",
        aliases: &["inv"],
        args: "<server>/<tool> [args]",
        about: "Invoke an MCP tool through GovernedTool",
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
    SlashCommand {
        primary: "hhh",
        aliases: &["alignment", "align"],
        args: "[on|off|status|model]",
        about: "Toggle HHH alignment mode (Helpful, Harmless, Honest)",
    },
    SlashCommand {
        primary: "bundle",
        aliases: &["b"],
        args: "[SKILL1 SKILL2 ...] | list | off | skills",
        about: "Compose, apply, or manage skill bundles",
    },
    SlashCommand {
        primary: "consolidate",
        aliases: &["cons"],
        args: "[LIMIT] [--floor CONFIDENCE] [--max MAX_TRIPLES]",
        about: "Trigger episodic→semantic consolidation with optional semantic cleanup",
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
            let gas_remaining = state.inference_loop.gas_remaining();
            let gas_cap = state.inference_loop.gas_cap();
            let gas_pct = if gas_cap > 0 {
                (gas_remaining as f64 / gas_cap as f64) * 100.0
            } else {
                100.0
            };
            let gas_bar = if gas_pct > 60.0 {
                "\x1b[32m■\x1b[0m" // green
            } else if gas_pct > 20.0 {
                "\x1b[33m■\x1b[0m" // yellow
            } else {
                "\x1b[31m■\x1b[0m" // red
            };
            println!("  Agent:      \x1b[1m{}\x1b[0m", agent_display);
            println!("  Model:      \x1b[1m{}\x1b[0m", state.current_model);
            println!("  Template:   {}", tpl);
            println!(
                "  Gas:        {} {}/{} ({:.0}%)",
                gas_bar, gas_remaining, gas_cap, gas_pct
            );
            // Check CNS health
            let cns_health = rt.block_on(state.cns.read());
            let cns_status = match rt.block_on(async { cns_health.health().await }) {
                health if health.critical_count > 0 => {
                    format!(
                        "\x1b[31m\u{26a0} CRITICAL\x1b[0m ({} critical, {} warnings)",
                        health.critical_count, health.warning_count
                    )
                }
                health if health.warning_count > 0 => {
                    format!(
                        "\x1b[33m\u{26a0} WARNING\x1b[0m ({} warnings)",
                        health.warning_count
                    )
                }
                _ => "\x1b[32mHEALTHY\x1b[0m (no alerts)".to_string(),
            };
            println!("  CNS:        {}", cns_status);
            // Show LoopSystem registered loops
            let loop_count = rt.block_on(state.loop_system.registered_count());
            let loop_ids = rt.block_on(state.loop_system.registered_loop_ids());
            let ids_str = loop_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("  Loops:      {} registered ({})", loop_count, ids_str);
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
                // Load persona constraints from the agent definition
                state.persona_constraints = rt
                    .block_on(crate::commands::bot_status(arg1))
                    .ok()
                    .and_then(|agent| agent.definition.persona);
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
                                    esc.bot_id
                                        .as_uuid()
                                        .to_string()
                                        .split('-')
                                        .next()
                                        .unwrap_or("?"),
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
        "invoke" | "inv" => {
            super::handlers::handle_invoke(arg1, arg2, state, rt);
        }
        "model" | "m" => {
            handle_model(arg1, rt, state);
        }
        "hhh" | "alignment" | "align" => {
            handle_hhh(arg1, state);
        }
        "consolidate" | "cons" => {
            let cons_arg = if arg2.is_empty() {
                arg1.to_string()
            } else {
                format!("{} {}", arg1, arg2)
            };
            super::handlers::handle_consolidate(&cons_arg, state, rt);
        }
        "bundle" | "b" => {
            // /bundle [SKILL1 SKILL2 ...] | list | off | skills
            match arg1 {
                "list" => {
                    println!("  \x1b[1mSkill Bundles\x1b[0m");
                    println!("  (use \x1b[36mkask bundle list\x1b[0m for full details)");
                    println!();
                }
                "off" => {
                    println!("  Bundle deactivated.");
                    println!();
                }
                "skills" => {
                    println!("  \x1b[1mAvailable Skills\x1b[0m");
                    println!("  (use \x1b[36mkask bundle skills\x1b[0m for full details)");
                    println!();
                }
                "" => {
                    println!("  \x1b[1mBundle Commands\x1b[0m");
                    println!(
                        "    \x1b[36m/bundle SKILL1 SKILL2\x1b[0m  Compose a bundle from skills"
                    );
                    println!("    \x1b[36m/bundle list\x1b[0m          List all bundles");
                    println!("    \x1b[36m/bundle off\x1b[0m           Deactivate current bundle");
                    println!("    \x1b[36m/bundle skills\x1b[0m        List available skills");
                    println!();
                }
                skills_arg => {
                    println!("  Composing bundle from: {}", skills_arg);
                    println!(
                        "  (use \x1b[36mkask bundle compose SKILL1 SKILL2\x1b[0m for full composition)"
                    );
                    println!();
                }
            }
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

// ── HHH alignment mode handler ──────────────────────────────────────────────

fn handle_hhh(arg: &str, state: &mut super::ReplState) {
    use hkask_agents::HhhMode;

    match arg.trim() {
        "on" => {
            if state.gate_inference_port.is_none() {
                println!(
                    "  \x1b[31m\u{2717} HHH mode unavailable\x1b[0m — gate model initialization failed."
                );
                println!(
                    "  Run \x1b[36m/hhh model <name>\x1b[0m to configure a different gate model."
                );
            } else {
                state.hhh_mode = HhhMode::Active;
                println!(
                    "  \x1b[32m\u{2713} HHH mode activated\x1b[0m (Helpful, Harmless, Honest)"
                );
                println!(
                    "  Gate model: \x1b[1m{}\x1b[0m, max iterations: {}",
                    state.hhh_config.gate_model, state.hhh_config.max_iterations
                );
            }
        }
        "off" => {
            state.hhh_mode = HhhMode::Inactive;
            println!("  \x1b[33m\u{2717} HHH mode deactivated\x1b[0m");
        }
        "status" | "" => {
            let mode_str = match state.hhh_mode {
                HhhMode::Active => "\x1b[32mACTIVE\x1b[0m",
                HhhMode::Inactive => "\x1b[33mINACTIVE\x1b[0m",
            };
            println!("  HHH Mode:    {}", mode_str);
            println!(
                "  Gate Model:  \x1b[1m{}\x1b[0m",
                state.hhh_config.gate_model
            );
            println!("  Iterations:  {}", state.hhh_config.max_iterations);
            println!("  Threshold:   {}", state.hhh_config.pass_threshold);
            if state.gate_inference_port.is_none() {
                println!(
                    "  \x1b[31m\u{26a0} Gate model unavailable\x1b[0m — use /hhh model <name> to configure"
                );
            }
        }
        arg_str if arg_str.starts_with("model ") => {
            let model_name = arg_str[6..].trim();
            if model_name.is_empty() {
                println!("  Usage: \x1b[36m/hhh model <name>\x1b[0m");
            } else {
                // Recreate the gate inference port with the new model
                match hkask_templates::OkapiInference::new(model_name, state.okapi_config.clone()) {
                    Ok(port) => {
                        state.gate_inference_port = Some(Arc::new(port));
                        state.hhh_config.gate_model = model_name.to_string();
                        println!(
                            "  Gate model set to: \x1b[1m{}\x1b[0m",
                            state.hhh_config.gate_model
                        );
                    }
                    Err(e) => {
                        println!("  \x1b[31mFailed to initialize gate model: {}\x1b[0m", e);
                    }
                }
            }
        }
        _ => {
            println!("  \x1b[1mHHH Alignment Mode\x1b[0m (Helpful, Harmless, Honest)");
            println!();
            println!("  \x1b[36m/hhh on\x1b[0m      Activate HHH mode");
            println!("  \x1b[36m/hhh off\x1b[0m     Deactivate HHH mode");
            println!("  \x1b[36m/hhh status\x1b[0m  Show current HHH settings");
            println!("  \x1b[36m/hhh model\x1b[0m   Change gate model");
        }
    }
    println!();
}

// ── Consolidation handler (delegated to handlers::consolidation) ──────────────
// handle_consolidate is provided by super::handlers::handle_consolidate

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
            let chat_response = rt.block_on(crate::commands::chat_with_agent(
                arg2,
                Some(arg1),
                None,
                Some(state.inference_port.clone()),
                state.resolved_secrets.as_ref(),
                Some(state.episodic_storage.clone()),
                Some(state.semantic_storage.clone()),
                Some(state.agent_webid),
                None, // No HHH suffix for /ask
            ));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, chat_response.text);
            if let Some(ref usage) = chat_response.usage {
                println!(
                    "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                );
            }

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
            let chat_response = rt.block_on(crate::commands::chat_with_agent(
                arg2,
                Some(arg1),
                None,
                Some(state.inference_port.clone()),
                state.resolved_secrets.as_ref(),
                Some(state.episodic_storage.clone()),
                Some(state.semantic_storage.clone()),
                Some(state.agent_webid),
                None, // No HHH suffix for /ask
            ));
            println!("\x1b[1m{}\x1b[0m: {}\n", arg1, chat_response.text);
            if let Some(ref usage) = chat_response.usage {
                println!(
                    "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                );
            }
        }
    }
}
