//! Slash command registry and dispatch for the hKask REPL.

use super::ReplState;
use super::display::{print_command_help, print_help};
use super::handlers;

// ── Command table ──────────────────────────────────────────────────────

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

// ── Lookup ─────────────────────────────────────────────────────────────

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

// ── Dispatch ───────────────────────────────────────────────────────────

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
        // Trivial commands — inline is clearer than a function call
        "help" | "h" | "?" => {
            if arg1.is_empty() {
                print_help()
            } else {
                print_command_help(arg1)
            }
        }
        "quit" | "q" | "exit" => {
            println!("Goodbye!");
            return true;
        }
        "clear" | "cls" => {
            print!("\x1b[2J\x1b[H");
        }

        // Delegated to handler modules
        "status" | "st" => handlers::handle_status(state, template_id, rt),
        "agent" | "a" => handlers::handle_agent(arg1, state, rt),
        "agents" | "ls" => handlers::handle_agents(rt),
        "history" | "hist" => handlers::handle_history(state),
        "pods" => handlers::handle_pods(rt),
        "templates" | "tpl" => handlers::handle_templates(rt),
        "tools" => handlers::handle_tools(state, rt),
        "escalations" | "esc" => handlers::handle_escalations(rt),
        "resolve" => handlers::handle_resolve(arg1, rt),
        "dismiss" => handlers::handle_dismiss(arg1, rt),
        "metacognition" | "meta" => handlers::handle_metacognition(rt),
        "sovereignty" | "sov" => handlers::handle_sovereignty(),
        "ensemble" | "ens" => handlers::handle_ensemble(
            arg1,
            arg2,
            &mut state.active_session,
            &*state.service_context,
            rt,
        ),
        "into" | "i" => {
            handlers::handle_into(arg1, &mut state.active_session, &*state.service_context, rt)
        }
        "filter" | "thresh" => {
            handlers::handle_filter(arg1, &state.active_session, &*state.service_context, rt)
        }
        "mode" => handlers::handle_mode(arg1, &state.active_session, &*state.service_context, rt),
        "ask" => handlers::handle_ask(arg1, arg2, rt, state),
        "invoke" | "inv" => handlers::handle_invoke(arg1, arg2, state, rt),
        "model" | "m" => handlers::handle_model(arg1, rt, state),
        "hhh" | "alignment" | "align" => handlers::handle_hhh(arg1, state),
        "consolidate" | "cons" => {
            let cons_arg = if arg2.is_empty() {
                arg1.to_string()
            } else {
                format!("{} {}", arg1, arg2)
            };
            handlers::handle_consolidate(&cons_arg, state, rt);
        }
        "bundle" | "b" => handlers::handle_bundle(arg1),

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
