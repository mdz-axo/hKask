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
use hkask_cns::CnsRuntime;
use hkask_mcp::{McpRuntime, register_builtin_servers};
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

    /// Manifest management
    Manifest {
        #[command(subcommand)]
        action: ManifestAction,
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

    /// Render template with bindings
    Render {
        /// Template ID
        #[arg()]
        id: String,

        /// Input JSON
        #[arg(short, long)]
        input: Option<String>,

        /// Input file
        #[arg(short = 'f', long)]
        input_file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ManifestAction {
    /// List all registered manifests
    List,

    /// Get manifest details
    Get {
        /// Manifest ID
        #[arg()]
        id: String,
    },

    /// Execute manifest
    Execute {
        /// Manifest ID
        #[arg()]
        id: String,

        /// Input JSON
        #[arg(short, long)]
        input: Option<String>,

        /// Input file
        #[arg(short = 'f', long)]
        input_file: Option<PathBuf>,
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
    _cns: &CnsRuntime,
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
            } else if input.contains("create") || input.contains("make") || input.contains("build")
            {
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

    // Initialize CNS runtime
    let cns = CnsRuntime::new();

    match cli.command {
        Commands::Chat {
            template,
            input,
            interactive,
        } => {
            if interactive {
                run_chat_interactive(&registry, &runtime, &cns, template.as_deref());
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
            TemplateAction::Render { id, input, input_file } => {
                // Read input
                let input_json = if let Some(json) = input {
                    json
                } else if let Some(path) = input_file {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("Failed to read input file: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    // Read from stdin
                    let mut buf = String::new();
                    if io::stdin().read_line(&mut buf).is_err() {
                        eprintln!("Failed to read input from stdin");
                        std::process::exit(1);
                    }
                    buf
                };

                // Parse input JSON
                let bindings: serde_json::Value = match serde_json::from_str(&input_json) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Invalid JSON input: {}", e);
                        std::process::exit(1);
                    }
                };

                // Render template
                match commands::render_template(&registry, &id, bindings) {
                    Ok(rendered) => {
                        println!("{}", rendered);
                    }
                    Err(e) => {
                        eprintln!("Failed to render template: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },

        Commands::Manifest { action } => match action {
            ManifestAction::List => {
                let manifest_dir = std::path::PathBuf::from("registry/manifests");
                if manifest_dir.exists() {
                    println!("Registered manifests:\n");
                    for entry in std::fs::read_dir(&manifest_dir).unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                            let id = path.file_stem().unwrap().to_str().unwrap();
                            println!("  {}", id);
                        }
                    }
                } else {
                    println!("No manifests directory found at: {:?}", manifest_dir);
                }
            }
            ManifestAction::Get { id } => {
                let manifest_path = format!("registry/manifests/{}.yaml", id);
                if std::path::Path::new(&manifest_path).exists() {
                    match std::fs::read_to_string(&manifest_path) {
                        Ok(content) => {
                            println!("Manifest: {}\n", id);
                            println!("{}", content);
                        }
                        Err(e) => {
                            eprintln!("Failed to read manifest: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Manifest not found: {}", id);
                    std::process::exit(1);
                }
            }
            ManifestAction::Execute { id, input, input_file } => {
                // Read input
                let input_json = if let Some(json) = input {
                    json
                } else if let Some(path) = input_file {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("Failed to read input file: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    // Read from stdin
                    let mut buf = String::new();
                    if io::stdin().read_line(&mut buf).is_err() {
                        eprintln!("Failed to read input from stdin");
                        std::process::exit(1);
                    }
                    buf
                };

                // Parse input JSON
                let input_value: serde_json::Value = match serde_json::from_str(&input_json) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Invalid JSON input: {}", e);
                        std::process::exit(1);
                    }
                };

                // Execute manifest
                match commands::execute_manifest(&registry, &id, input_value) {
                    Ok(result) => {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    }
                    Err(e) => {
                        eprintln!("Failed to execute manifest: {}", e);
                        std::process::exit(1);
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
                println!("See hkask-agents crate for ACP integration.");
            }
            BotAction::Grant { bot_id, capability } => {
                println!("Grant capability: {} to bot: {}", capability, bot_id);
                println!("Note: Capability granting requires ACP runtime integration.");
                println!("See hkask-agents/ocap.rs for capability token management.");
            }
        },

        Commands::Mcp { action } => {
            let runtime = McpRuntime::new();

            // Register builtin servers
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(register_builtin_servers(&runtime));

            match action {
                McpAction::ListServers => {
                    let servers = rt.block_on(runtime.list_servers());
                    if servers.is_empty() {
                        println!("MCP servers:");
                        println!("  (no servers registered)");
                    } else {
                        println!("MCP servers ({}):\n", servers.len());
                        for server in &servers {
                            println!("  {} ({})", server.name, server.id);
                            println!("    Tools: {}", server.tools.len());
                            println!("    Connected: {}", server.connected);
                        }
                    }
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
                    let tools = rt.block_on(runtime.discover_tools());
                    if tools.is_empty() {
                        println!("Available tools:");
                        println!("  (no tools registered)");
                    } else {
                        println!("Available tools ({}):\n", tools.len());
                        for tool in &tools {
                            println!("  {}", tool);
                        }
                    }
                }
                McpAction::GetTool { name } => {
                    let tool = rt.block_on(runtime.get_tool(&name));
                    match tool {
                        Some(t) => {
                            println!("Tool: {}", t.name);
                            println!("  Description: {}", t.description);
                            println!("  Server: {}", t.server_id);
                            println!("  Input Schema: {}", t.input_schema);
                        }
                        None => {
                            println!("Tool not found: {}", name);
                        }
                    }
                }
            }
        }

        Commands::Cns { action } => match action {
            CnsAction::Health => {
                let cns = CnsRuntime::new();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let health = rt.block_on(cns.health());

                println!("CNS health status:");
                println!("  Overall deficit: {}", health.overall_deficit);
                println!("  Critical alerts: {}", health.critical_count);
                println!("  Warning alerts: {}", health.warning_count);
                println!(
                    "  Status: {}",
                    if health.healthy {
                        "HEALTHY"
                    } else {
                        "DEGRADED"
                    }
                );
            }
            CnsAction::Alerts => {
                let cns = CnsRuntime::new();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let alerts = rt.block_on(cns.critical_alerts());

                if alerts.is_empty() {
                    println!("Algedonic alerts:");
                    println!("  (no critical alerts)");
                } else {
                    println!("Algedonic alerts ({} critical):\n", alerts.len());
                    for alert in &alerts {
                        println!("  [{}] {}: {}", alert.severity, alert.domain, alert.message);
                    }
                }
            }
            CnsAction::Variety => {
                let cns = CnsRuntime::new();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let variety = rt.block_on(cns.variety());

                if variety.is_empty() {
                    println!("Variety counters:");
                    println!("  (no variety data)");
                } else {
                    println!("Variety counters ({} domains):\n", variety.len());
                    for (domain, count) in &variety {
                        println!("  {}: {} states", domain, count);
                    }
                }
            }
        },
    }
}
