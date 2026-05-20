//! hKask CLI — Command-line interface
//!
//! **Commands:**
//! - `kask chat` — Curator chat interface
//! - `kask template list` — List registered templates
//! - `kask template register` — Register a new template
//! - `kask bot capabilities` — Show bot capabilities
//! - `kask bot grant` — Grant capability to bot
//! - `kask mcp servers` — List MCP servers
//! - `kask mcp tools` — List available tools

use clap::{Parser, Subcommand};
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::{RegistryIndex, SqliteRegistry};
use hkask_types::TemplateType as Type;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod commands;

#[derive(Parser)]
#[command(name = "kask")]
#[command(author = "hKask Team")]
#[command(version = "0.1.0")]
#[command(about = "Planck's Constant of Agent Systems - CLI", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Registry database path (default: in-memory)
    #[arg(short, long)]
    registry: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Curator chat interface
    Chat {
        /// Optional: template ID to use
        #[arg(short, long)]
        template: Option<String>,

        /// Optional: input file
        #[arg(short = 'f', long)]
        input: Option<PathBuf>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },

    /// Template management
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },

    /// Bot capability management
    Bot {
        #[command(subcommand)]
        action: BotAction,
    },

    /// MCP server/tool management
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },

    /// CNS monitoring
    Cns {
        #[command(subcommand)]
        action: CnsAction,
    },
}

#[derive(Subcommand)]
enum TemplateAction {
    /// List all registered templates
    List {
        /// Filter by template type
        #[arg(short, long)]
        r#type: Option<String>,
    },

    /// Register a new template
    Register {
        /// Template ID (e.g., "prompt/selector")
        #[arg(short, long)]
        id: String,

        /// Template file path
        #[arg(short, long)]
        path: PathBuf,

        /// Template type (prompt, cognition, process, etc.)
        #[arg(short, long)]
        r#type: String,

        /// Lexicon terms (comma-separated)
        #[arg(short, long)]
        lexicon: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Get template details
    Get {
        /// Template ID
        #[arg()]
        id: String,
    },

    /// Search templates by lexicon term
    Search {
        /// Lexicon term
        #[arg()]
        term: String,
    },
}

#[derive(Subcommand)]
enum BotAction {
    /// List bot capabilities
    List {
        /// Bot WebID
        #[arg(short, long)]
        bot_id: Option<String>,
    },

    /// Grant capability to bot
    Grant {
        /// Bot WebID
        #[arg(short, long)]
        bot_id: String,

        /// Capability name (e.g., "inference:call")
        #[arg(short, long)]
        capability: String,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// List MCP servers
    ListServers,

    /// Register a new MCP server
    RegisterServer {
        /// Server ID
        #[arg(short, long)]
        id: String,

        /// Server name
        #[arg(short, long)]
        name: String,

        /// Tools provided by this server (comma-separated)
        #[arg(short, long)]
        tools: Option<String>,
    },

    /// List available tools
    ListTools,

    /// Get tool definition
    GetTool {
        /// Tool name
        #[arg()]
        name: String,
    },
}

#[derive(Subcommand)]
enum CnsAction {
    /// Get CNS health status
    Health,

    /// Get algedonic alerts
    Alerts,

    /// Get variety counters
    Variety,
}

fn init_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
    };
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn parse_template_type(type_str: &str) -> Option<Type> {
    match type_str.to_lowercase().as_str() {
        "prompt" => Some(Type::Prompt),
        "cognition" => Some(Type::Cognition),
        "process" => Some(Type::Process),
        _ => None,
    }
}

fn run_chat_interactive(
    registry: &SqliteRegistry,
    _runtime: &McpRuntime,
    template_id: Option<&str>,
) {
    println!("ℏKask Curator Chat - Interactive Mode");
    println!("Template: {}", template_id.unwrap_or("auto-select"));
    println!("Type 'quit' or 'exit' to end session\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        if stdin.lock().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("Goodbye!");
            break;
        }

        // Process input
        let response = process_chat_input(registry, input, template_id);
        println!("Curator: {}\n", response);
    }
}

fn process_chat_input(registry: &SqliteRegistry, input: &str, template_id: Option<&str>) -> String {
    match template_id {
        Some(id) => match registry.get(id) {
            Ok(_entry) => format!("Processing with template '{}': {}", id, input),
            Err(_) => format!("Template '{}' not found. Using default response.", id),
        },
        None => {
            // Auto-select template based on input
            if input.contains('?') || input.contains("what") || input.contains("how") {
                format!("Question detected. Processing: {}", input)
            } else if input.contains("create") || input.contains("make") || input.contains("build") {
                format!("Action request detected. Processing: {}", input)
            } else {
                format!("Received: {}", input)
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    // Initialize registry
    let registry_result = match &cli.registry {
        Some(path) => SqliteRegistry::new(Some(path.to_str().unwrap())),
        None => SqliteRegistry::new(None),
    };

    let mut registry = match registry_result {
        Ok(mut r) => {
            // Load existing templates from database
            let _ = r.load_all();
            r
        }
        Err(e) => {
            eprintln!("Failed to initialize registry: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize MCP runtime
    let runtime = McpRuntime::new();

    match cli.command {
        Commands::Chat {
            template,
            input,
            interactive,
        } => {
            if interactive {
                run_chat_interactive(&registry, &runtime, template.as_deref());
            } else if let Some(input_path) = input {
                // Read from file
                match std::fs::read_to_string(&input_path) {
                    Ok(content) => {
                        let response = process_chat_input(&registry, &content, template.as_deref());
                        println!("Curator: {}", response);
                    }
                    Err(e) => {
                        eprintln!("Failed to read input file: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Read from stdin
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_ok() {
                    let response = process_chat_input(&registry, input.trim(), template.as_deref());
                    println!("Curator: {}", response);
                }
            }
        }

        Commands::Template { action } => match action {
            TemplateAction::List { r#type } => {
                let template_type = r#type.as_deref().and_then(parse_template_type);
                let entries = commands::list_templates(&registry, template_type);

                if entries.is_empty() {
                    println!("No templates registered.");
                } else {
                    println!("Registered templates ({}):\n", entries.len());
                    for entry in entries {
                        println!("  {} ({})", entry.id, entry.template_type.as_str());
                        println!("    Description: {}", entry.description);
                        println!("    Path: {}", entry.source_path);
                        if !entry.lexicon_terms.is_empty() {
                            println!("    Lexicon: {}", entry.lexicon_terms.join(", "));
                        }
                        println!();
                    }
                }
            }
            TemplateAction::Register {
                id,
                path,
                r#type,
                lexicon,
                description,
            } => {
                let template_type = match parse_template_type(&r#type) {
                    Some(t) => t,
                    None => {
                        eprintln!(
                            "Invalid template type: {}. Valid types: prompt, cognition, process",
                            r#type
                        );
                        std::process::exit(1);
                    }
                };

                let lexicon_terms: Vec<String> = lexicon
                    .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                let desc = description.unwrap_or_else(|| format!("Template {}", id));

                match commands::register_template(
                    &mut registry,
                    id.clone(),
                    template_type,
                    path.to_string_lossy().to_string(),
                    lexicon_terms,
                    desc,
                ) {
                    Ok(()) => println!("Registered template: {}", id),
                    Err(e) => {
                        eprintln!("Failed to register template: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            TemplateAction::Get { id } => match commands::get_template(&registry, &id) {
                Ok(entry) => {
                    println!("Template: {}", entry.id);
                    println!("  Type: {}", entry.template_type.as_str());
                    println!("  Description: {}", entry.description);
                    println!("  Path: {}", entry.source_path);
                    println!("  Lexicon: {}", entry.lexicon_terms.join(", "));
                }
                Err(e) => {
                    eprintln!("Template not found: {}", e);
                    std::process::exit(1);
                }
            },
            TemplateAction::Search { term } => {
                let results = commands::search_templates(&registry, &term);
                if results.is_empty() {
                    println!("No templates found with lexicon term: {}", term);
                } else {
                    println!("Templates matching '{}':\n", term);
                    for entry in results {
                        println!("  {} ({})", entry.id, entry.template_type.as_str());
                    }
                }
            }
        },

        Commands::Bot { action } => match action {
            BotAction::List { bot_id } => {
                println!(
                    "Bot capabilities (bot: {})",
                    bot_id.unwrap_or("all".to_string())
                );
                println!("Note: Bot capability management requires ACP runtime integration.");
            }
            BotAction::Grant { bot_id, capability } => {
                println!("Grant capability: {} to bot: {}", capability, bot_id);
                println!("Note: Capability granting requires ACP runtime integration.");
            }
        },

        Commands::Mcp { action } => {
            match action {
                McpAction::ListServers => {
                    println!("MCP servers:");
                    println!("  (no servers registered - use 'mcp register-server' to add one)");
                }
                McpAction::RegisterServer { id, name, tools } => {
                    println!("Registering MCP server: {} ({})", name, id);
                    if let Some(tools_str) = tools {
                        let tools: Vec<&str> = tools_str.split(',').collect();
                        println!("  Tools: {}", tools.join(", "));
                    } else {
                        println!("  Tools: (none specified)");
                    }
                    println!("Note: MCP server registration requires runtime integration.");
                }
                McpAction::ListTools => {
                    println!("Available tools:");
                    println!("  (no tools registered - register an MCP server first)");
                }
                McpAction::GetTool { name } => {
                    println!("Get tool: {}", name);
                    println!("Note: Tool lookup requires MCP server registration.");
                }
            }
        }

        Commands::Cns { action } => match action {
            CnsAction::Health => {
                println!("CNS health status:");
                println!("  Overall deficit: 0");
                println!("  Critical alerts: 0");
                println!("  Warning alerts: 0");
                println!("  Status: HEALTHY");
                println!("\nNote: CNS monitoring requires runtime integration.");
            }
            CnsAction::Alerts => {
                println!("Algedonic alerts:");
                println!("  (no active alerts)");
                println!("\nNote: Alert monitoring requires CNS runtime integration.");
            }
            CnsAction::Variety => {
                println!("Variety counters:");
                println!("  (no variety data)");
                println!("\nNote: Variety monitoring requires CNS runtime integration.");
            }
        },
    }
}
