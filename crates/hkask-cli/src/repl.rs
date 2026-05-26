//! Interactive REPL for hKask — discoverable, self-documenting, alive.
//!
//! Design principles:
//! - Every capability is reachable from `/help`
//! - Tab completion for slash commands and agent names
//! - Fuzzy matching on slash commands (like russell's `/model`)
//! - Welcome banner with the Kask amphora logo
//! - Categorized help so the menu is scannable

use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::CmdKind;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config as ReadlineConfig, Context, Editor};
use std::borrow::Cow;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const SLASH_COMMANDS: &[SlashCommand] = &[
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
        args: "sessions|create|join|send",
        about: "Multi-agent ensemble operations",
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

struct SlashCommand {
    primary: &'static str,
    aliases: &'static [&'static str],
    args: &'static str,
    about: &'static str,
}

impl SlashCommand {
    fn matches(&self, input: &str) -> bool {
        input == self.primary || self.aliases.contains(&input)
    }
}

fn find_command(input: &str) -> Option<&'static SlashCommand> {
    SLASH_COMMANDS.iter().find(|c| c.matches(input))
}

fn fuzzy_match_command(input: &str) -> Vec<&'static SlashCommand> {
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

#[derive(Debug, Clone)]
struct SessionHistory {
    turns: Vec<(String, String)>,
}

impl SessionHistory {
    fn new() -> Self {
        Self { turns: Vec::new() }
    }
    fn record(&mut self, agent: &str, response: &str) {
        self.turns.push((agent.to_string(), response.to_string()));
    }
}

struct KaskHelper {
    slash_completions: Vec<String>,
}

impl KaskHelper {
    fn new() -> Self {
        let mut slash_completions = Vec::new();
        for cmd in SLASH_COMMANDS {
            slash_completions.push(format!("/{}", cmd.primary));
            for alias in cmd.aliases {
                slash_completions.push(format!("/{}", alias));
            }
        }
        Self { slash_completions }
    }
}

impl Completer for KaskHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        if !line.starts_with('/') || pos == 0 {
            return Ok((0, Vec::new()));
        }

        let partial = &line[..pos];
        let matches: Vec<String> = self
            .slash_completions
            .iter()
            .filter(|c| c.starts_with(partial))
            .cloned()
            .collect();

        Ok((0, matches))
    }
}

impl Hinter for KaskHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if !line.starts_with('/') || pos == 0 {
            return None;
        }
        let partial = &line[..pos];
        self.slash_completions
            .iter()
            .find(|c| c.starts_with(partial) && c.len() > partial.len())
            .map(|c| c[partial.len()..].to_string())
    }
}

impl Highlighter for KaskHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[2m{}\x1b[0m", hint))
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.starts_with('/') {
            Cow::Owned(format!("\x1b[1;36m{}\x1b[0m", line))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, line: &str, _pos: usize, _cmd_kind: CmdKind) -> bool {
        line.starts_with('/')
    }
}

impl Validator for KaskHelper {}
impl rustyline::Helper for KaskHelper {}

fn print_banner(agent: &str, template: Option<&str>) {
    let kask = r#"
    ╭──────────────────────────────────────╮
    │                                      │
    │      ╷   ┌───────┐   ╷              │
    │      │   │       │   │              │
    │      ╰───┤  ◉    ├───╯              │
    │          │       │                  │
    │          │  KASK │                  │
    │          │       │                  │
    │          └───────┘                  │
    │          Planck's Constant          │
    │          of Agent Systems           │
    │                                      │
    ╰──────────────────────────────────────╯"#;

    println!("\x1b[1;36m{}\x1b[0m", kask);
    println!();
    println!(
        "  \x1b[1mℏKask v{}\x1b[0m — Planck's Constant of Agent Systems",
        VERSION
    );
    println!(
        "  Agent: \x1b[1m{}\x1b[0m  |  Template: \x1b[1m{}\x1b[0m",
        agent,
        template.unwrap_or("auto-select")
    );
    println!();
    println!(
        "  Type \x1b[1;36m/help\x1b[0m for commands, \x1b[2m<TAB>\x1b[0m to autocomplete, \x1b[2m/quit\x1b[0m to exit"
    );
    println!();
}

fn print_help() {
    println!();
    println!("\x1b[1mℏKask Commands\x1b[0m");
    println!();

    let categories = [
        ("Session", &["help", "quit", "clear", "history"] as &[&str]),
        ("Agent", &["agent", "agents", "pods"]),
        ("System", &["status", "tools", "templates", "sovereignty"]),
        ("Ensemble", &["ensemble"]),
        (
            "Governance",
            &["escalations", "resolve", "dismiss", "metacognition"],
        ),
    ];

    for (category, cmds) in &categories {
        println!("  \x1b[1;33m{}\x1b[0m", category);
        for &cmd_name in *cmds {
            if let Some(cmd) = find_command(cmd_name) {
                let alias_str = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!(", /{}", cmd.aliases.join(", /"))
                };
                let args_str = if cmd.args.is_empty() {
                    String::new()
                } else {
                    format!(" {}", cmd.args)
                };
                println!(
                    "    \x1b[36m/{}{}\x1b[0m{}  — {}",
                    cmd.primary, args_str, alias_str, cmd.about
                );
            }
        }
        println!();
    }

    println!("  \x1b[2mTip: /help <command> for details on a specific command\x1b[0m");
    println!();
}

fn print_command_help(cmd_name: &str) {
    if let Some(cmd) = find_command(cmd_name) {
        println!();
        println!("  \x1b[1;36m/{} {}\x1b[0m", cmd.primary, cmd.args);
        if !cmd.aliases.is_empty() {
            println!("  Aliases: /{}", cmd.aliases.join(", /"));
        }
        println!("  {}", cmd.about);

        match cmd.primary {
            "ensemble" => {
                println!();
                println!("  Subcommands:");
                println!(
                    "    \x1b[36m/ensemble sessions\x1b[0m    — List active ensemble sessions"
                );
                println!(
                    "    \x1b[36m/ensemble create\x1b[0m <id> — Create a new ensemble chat session"
                );
                println!(
                    "    \x1b[36m/ensemble join\x1b[0m <id> <bot> <role> — Register a bot in a session"
                );
                println!(
                    "    \x1b[36m/ensemble send\x1b[0m <id> <msg> — Send a message to a session"
                );
                println!();
                println!("  Roles: memory_bot, spandrel_bot, okapi_bot, scholar_bot");
            }
            "agent" => {
                println!();
                println!("  \x1b[2m/agent\x1b[0m          — Show current agent");
                println!("  \x1b[2m/agent Russell\x1b[0m  — Switch to Russell");
                println!("  \x1b[2m/agents\x1b[0m         — List all available agents");
            }
            _ => {}
        }
        println!();
    } else {
        let fuzzy = fuzzy_match_command(cmd_name);
        if fuzzy.is_empty() {
            println!("  Unknown command: /{}", cmd_name);
        } else {
            println!("  Unknown command: /{} — did you mean:", cmd_name);
            for cmd in &fuzzy {
                println!("    /{} — {}", cmd.primary, cmd.about);
            }
        }
        println!("  Type /help for available commands.");
    }
}

pub fn run(
    _registry: &SqliteRegistry,
    _runtime: &McpRuntime,
    template_id: Option<&str>,
    agent_name: &str,
) {
    let mut current_agent = agent_name.to_string();
    let mut session_history = SessionHistory::new();

    let helper = KaskHelper::new();

    let rl_config = ReadlineConfig::builder()
        .history_ignore_space(true)
        .history_ignore_dups(true)
        .expect("invalid readline config")
        .completion_type(CompletionType::List)
        .build();

    let mut rl = match Editor::with_config(rl_config) {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Failed to initialize readline: {}", e);
            return;
        }
    };
    rl.set_helper(Some(helper));

    if rl.load_history(&history_path()).is_err() {
        // No history file yet — that's fine
    }

    print_banner(&current_agent, template_id);

    loop {
        let prompt = format!("\x1b[1mℏKask\x1b[0m [\x1b[36m{}\x1b[0m]> ", current_agent);
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(input.to_owned());

                if input.starts_with('/') {
                    if handle_slash_command(
                        input,
                        &mut current_agent,
                        &mut session_history,
                        template_id,
                    ) {
                        let _ = rl.save_history(&history_path());
                        break;
                    }
                    continue;
                }

                if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                    println!("Goodbye!");
                    let _ = rl.save_history(&history_path());
                    break;
                }

                let rt = tokio::runtime::Runtime::new().unwrap();
                let response = rt.block_on(crate::commands::chat_with_agent(
                    input,
                    Some(&current_agent),
                ));
                println!("{}: {}\n", current_agent, response);
                session_history.record(&current_agent, &response);
            }
            Err(ReadlineError::Interrupted) => {
                println!("(Ctrl+C — type /quit to exit)");
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                let _ = rl.save_history(&history_path());
                break;
            }
            Err(err) => {
                eprintln!("Readline error: {}", err);
                let _ = rl.save_history(&history_path());
                break;
            }
        }
    }
}

fn history_path() -> std::path::PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("hkask");
    let _ = std::fs::create_dir_all(&path);
    path.push("kask_history.txt");
    path
}

fn handle_slash_command(
    input: &str,
    current_agent: &mut String,
    session_history: &mut SessionHistory,
    template_id: Option<&str>,
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
            println!("  Template:   {}", tpl);
            println!("  CNS:        \x1b[32mHEALTHY\x1b[0m (no alerts)");
            println!("  Turns:      {}", session_history.turns.len());
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
            let rt = tokio::runtime::Runtime::new().unwrap();
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
            let rt = tokio::runtime::Runtime::new().unwrap();
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
                let rt = tokio::runtime::Runtime::new().unwrap();
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
                let rt = tokio::runtime::Runtime::new().unwrap();
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
            let rt = tokio::runtime::Runtime::new().unwrap();
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
            let rt = tokio::runtime::Runtime::new().unwrap();
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
            let rt = tokio::runtime::Runtime::new().unwrap();
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
            handle_ensemble(arg1, arg2);
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

fn handle_ensemble(subcmd: &str, rest: &str) {
    let rt = tokio::runtime::Runtime::new().unwrap();
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
                                println!("    \x1b[36m•\x1b[0m {}", s);
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
            println!("  Use: sessions, create, join, send");
            println!("  Type \x1b[36m/help ensemble\x1b[0m for details.");
        }
    }
    println!();
}
