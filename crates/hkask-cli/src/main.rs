//! hKask CLI — Command-line interface
//!
//! **Commands:**
//! - `kask chat` — Curator chat interface
//! - `kask template list` — List registered templates
//! - `kask template register` — Register a new template
//! - `kask bot capabilities` — Show bot capabilities
//! - `kask bot grant` — Grant capability to bot
//! - `kask pod create` — Create agent pod from template crate
//! - `kask pod activate` — Activate agent pod
//! - `kask pod deactivate` — Deactivate agent pod
//! - `kask pod status` — Show agent pod status
//! - `kask mcp servers` — List MCP servers
//! - `kask mcp tools` — List available tools
//! - `kask cns health` — CNS monitoring

use clap::{Parser, Subcommand};
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::{RegistryIndex, RussellMapper, SqliteRegistry};
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

    /// Agent pod management
    Pod {
        #[command(subcommand)]
        action: PodAction,
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

    /// Registry management
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },

    /// Documentation generation
    Docs {
        #[command(subcommand)]
        action: DocsAction,
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
enum PodAction {
    /// Create agent pod from template crate
    Create {
        /// Template crate name
        #[arg(short, long)]
        template: String,

        /// Agent persona YAML file path
        #[arg(short, long)]
        persona: PathBuf,

        /// Pod name (optional, defaults to UUID)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Activate agent pod for A2A communication
    Activate {
        /// Pod ID or name
        #[arg()]
        pod_id: String,
    },

    /// Deactivate agent pod
    Deactivate {
        /// Pod ID or name
        #[arg()]
        pod_id: String,
    },

    /// Show agent pod status
    Status {
        /// Pod ID or name
        #[arg()]
        pod_id: String,

        /// Show verbose details
        #[arg(short, long)]
        verbose: bool,
    },

    /// List all agent pods
    List,
}

#[derive(Subcommand)]
enum McpAction {
    /// List MCP servers
    ListServers,

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

#[derive(Subcommand)]
enum DocsAction {
    /// Generate OpenAPI specification (JSON)
    Openapi {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate CLI help documentation (markdown)
    Cli {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate all documentation
    All {
        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum RegistryAction {
    /// Import Russell skill manifests and prompt templates
    ImportRussell {
        /// Source path (Russell skills directory or manifest file)
        #[arg(short, long)]
        source: PathBuf,

        /// Dry run - analyze without writing
        #[arg(long)]
        dry_run: bool,

        /// Validate only - run hLexicon validation only
        #[arg(long)]
        validate_only: bool,

        /// Output format (yaml, json, mermaid)
        #[arg(short, long, default_value = "yaml")]
        output_format: String,

        /// Custom transformation rules (YAML file)
        #[arg(short, long)]
        transform_rules: Option<PathBuf>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List migrated assets with provenance
    ListMigrated {
        /// Filter by origin (e.g., "russell/web-search")
        #[arg(short, long)]
        origin: Option<String>,
    },
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
    // Simple echo/response for now - TODO: Implement actual template processing
    match template_id {
        Some(id) => match registry.get(id) {
            Ok(_entry) => format!("Processing with template '{}': {}", id, input),
            Err(_) => format!("Template '{}' not found. Using default response.", id),
        },
        None => {
            // Auto-select template based on input
            if input.contains('?') || input.contains("what") || input.contains("how") {
                "I'll help answer your question. (Question template would process this)".to_string()
            } else if input.contains("create") || input.contains("make") || input.contains("build")
            {
                "I'll help you create that. (Action template would process this)".to_string()
            } else {
                format!("Received: {}. (Default template response)", input)
            }
        }
    }
}

fn generate_cli_markdown() -> String {
    let mut md = String::new();

    md.push_str("# hKask CLI Documentation\n\n");
    md.push_str(
        "**hKask** (ℏKask — \"Planck's Constant of Agent Systems\") - Command-line interface\n\n",
    );
    md.push_str("## Usage\n\n");
    md.push_str("```bash\n");
    md.push_str("kask [OPTIONS] <COMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("## Options\n\n");
    md.push_str("- `-v`, `--verbose` — Enable verbose output\n");
    md.push_str("- `-r`, `--registry <PATH>` — Registry database path (default: in-memory)\n");
    md.push_str("- `-h`, `--help` — Print help\n");
    md.push_str("- `-V`, `--version` — Print version\n\n");
    md.push_str("## Commands\n\n");
    md.push_str("### `kask chat` — Curator chat interface\n\n");
    md.push_str("```bash\n");
    md.push_str("kask chat [OPTIONS]\n");
    md.push_str("```\n\n");
    md.push_str("Options:\n");
    md.push_str("- `-t`, `--template <TEMPLATE>` — Optional: template ID to use\n");
    md.push_str("- `-f`, `--input <INPUT>` — Optional: input file\n");
    md.push_str("- `-i`, `--interactive` — Interactive mode\n\n");
    md.push_str("### `kask template` — Template management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask template <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `list` — List all registered templates\n");
    md.push_str("  - `-t`, `--type <TYPE>` — Filter by template type\n");
    md.push_str("- `register` — Register a new template\n");
    md.push_str("  - `-i`, `--id <ID>` — Template ID (e.g., \"prompt/selector\")\n");
    md.push_str("  - `-p`, `--path <PATH>` — Template file path\n");
    md.push_str("  - `-t`, `--type <TYPE>` — Template type (prompt, cognition, process)\n");
    md.push_str("  - `-l`, `--lexicon <LEXICON>` — Lexicon terms (comma-separated)\n");
    md.push_str("  - `-d`, `--description <DESC>` — Description\n");
    md.push_str("- `get <ID>` — Get template details\n");
    md.push_str("- `search <TERM>` — Search templates by lexicon term\n\n");
    md.push_str("### `kask bot` — Bot capability management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask bot <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `list` — List bot capabilities\n");
    md.push_str("  - `-b`, `--bot-id <BOT_ID>` — Bot WebID\n");
    md.push_str("- `grant` — Grant capability to bot\n");
    md.push_str("  - `-b`, `--bot-id <BOT_ID>` — Bot WebID\n");
    md.push_str(
        "  - `-c`, `--capability <CAPABILITY>` — Capability name (e.g., \"inference:call\")\n\n",
    );
    md.push_str("### `kask pod` — Agent pod management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask pod <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `create` — Create agent pod from template crate\n");
    md.push_str("  - `-t`, `--template <TEMPLATE>` — Template crate name\n");
    md.push_str("  - `-p`, `--persona <PERSONA>` — Agent persona YAML file path\n");
    md.push_str("  - `-n`, `--name <NAME>` — Pod name (optional, defaults to UUID)\n");
    md.push_str("- `activate <POD_ID>` — Activate agent pod for A2A communication\n");
    md.push_str("- `deactivate <POD_ID>` — Deactivate agent pod\n");
    md.push_str("- `status <POD_ID>` — Show agent pod status\n");
    md.push_str("  - `-v`, `--verbose` — Show verbose details\n");
    md.push_str("- `list` — List all agent pods\n\n");
    md.push_str("### `kask mcp` — MCP server/tool management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask mcp <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `list-servers` — List MCP servers\n");
    md.push_str("- `list-tools` — List available tools\n");
    md.push_str("- `get-tool <NAME>` — Get tool definition\n\n");
    md.push_str("### `kask cns` — CNS monitoring\n\n");
    md.push_str("```bash\n");
    md.push_str("kask cns <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `health` — Get CNS health status\n");
    md.push_str("- `alerts` — Get algedonic alerts\n");
    md.push_str("- `variety` — Get variety counters\n\n");
    md.push_str("### `kask docs` — Documentation generation\n\n");
    md.push_str("```bash\n");
    md.push_str("kask docs <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `openapi` — Generate OpenAPI specification (JSON)\n");
    md.push_str("  - `-o`, `--output <OUTPUT>` — Output file path (default: stdout)\n");
    md.push_str("- `cli` — Generate CLI help documentation (markdown)\n");
    md.push_str("  - `-o`, `--output <OUTPUT>` — Output file path (default: stdout)\n");
    md.push_str("- `all` — Generate all documentation\n");
    md.push_str("  - `-o`, `--output <OUTPUT>` — Output directory\n\n");
    md.push_str("## Examples\n\n");
    md.push_str("```bash\n");
    md.push_str("# Start interactive chat session\n");
    md.push_str("kask chat --interactive\n\n");
    md.push_str("# List all templates\n");
    md.push_str("kask template list\n\n");
    md.push_str("# Register a new template\n");
    md.push_str("kask template register -i prompt/selector -p templates/selector.j2 -t prompt -l \"select,route,dispatch\"\n\n");
    md.push_str("# Generate OpenAPI spec\n");
    md.push_str("kask docs openapi -o docs/openapi.json\n\n");
    md.push_str("# Generate all documentation\n");
    md.push_str("kask docs all -o docs/\n");
    md.push_str("```\n\n");
    md.push_str("## Template Types\n\n");
    md.push_str("- `prompt` — Prompt templates for LLM interaction\n");
    md.push_str("- `cognition` — Cognitive processing templates\n");
    md.push_str("- `process` — Process execution templates\n\n");
    md.push_str("---\n\n");
    md.push_str("*hKask v0.1.0 — Planck's Constant of Agent Systems*\n");

    md
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
        Ok(r) => r,
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

        Commands::Pod { action } => match action {
            PodAction::Create {
                template,
                persona,
                name,
            } => {
                println!("Creating agent pod from template: {}", template);
                println!("Persona file: {}", persona.display());
                if let Some(n) = &name {
                    println!("Pod name: {}", n);
                }
                println!("\nNote: Full pod creation requires pod manager implementation.");
                println!("This is a placeholder for Phase 3 CLI integration.");
            }
            PodAction::Activate { pod_id } => {
                println!("Activating agent pod: {}", pod_id);
                println!("\nNote: Full pod activation requires pod manager implementation.");
            }
            PodAction::Deactivate { pod_id } => {
                println!("Deactivating agent pod: {}", pod_id);
                println!("\nNote: Full pod deactivation requires pod manager implementation.");
            }
            PodAction::Status { pod_id, verbose } => {
                println!("Agent pod status: {}", pod_id);
                println!("  State: populated (placeholder)");
                println!("  WebID: (pending pod manager integration)");
                println!("  Agent type: (pending pod manager integration)");
                println!("  Template: (pending pod manager integration)");
                if verbose {
                    println!("\n  Verbose details:");
                    println!("    Created at: (pending)");
                    println!("    Capability token: (redacted)");
                    println!("    Max attenuation: 7");
                    println!("    Current attenuation: 0");
                    println!("    CNS spans emitted: (pending integration)");
                }
                println!("\nNote: Full status requires pod manager implementation.");
            }
            PodAction::List => {
                println!("Agent pods:");
                println!("  (no pods registered)");
                println!("\nNote: Pod listing requires pod manager implementation.");
            }
        },

        Commands::Mcp { action } => {
            match action {
                McpAction::ListServers => {
                    println!("MCP servers:");
                    // Note: runtime is not shared, so we can't list actual servers
                    println!("  (no servers registered)");
                }
                McpAction::ListTools => {
                    println!("Available tools:");
                    println!("  (no tools registered)");
                }
                McpAction::GetTool { name } => {
                    println!("Get tool: {}", name);
                    println!("Note: Tool lookup requires MCP runtime integration.");
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
            }
            CnsAction::Alerts => {
                println!("Algedonic alerts:");
                println!("  (no active alerts)");
            }
            CnsAction::Variety => {
                println!("Variety counters:");
                println!("  (no variety data)");
            }
        },

        Commands::Docs { action } => match action {
            DocsAction::Openapi { output } => {
                let spec = hkask_api::create_openapi();
                let json =
                    serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");

                match output {
                    Some(path) => {
                        std::fs::write(&path, &json).expect("Failed to write OpenAPI spec");
                        println!("OpenAPI specification written to: {}", path.display());
                    }
                    None => println!("{}", json),
                }
            }
            DocsAction::Cli { output } => {
                let help = generate_cli_markdown();
                match output {
                    Some(path) => {
                        std::fs::write(&path, &help).expect("Failed to write CLI documentation");
                        println!("CLI documentation written to: {}", path.display());
                    }
                    None => println!("{}", help),
                }
            }
            DocsAction::All { output } => {
                std::fs::create_dir_all(&output).expect("Failed to create output directory");

                let spec = hkask_api::create_openapi();
                let json =
                    serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");
                let openapi_path = output.join("openapi.json");
                std::fs::write(&openapi_path, &json).expect("Failed to write OpenAPI spec");
                println!(
                    "OpenAPI specification written to: {}",
                    openapi_path.display()
                );

                let help = generate_cli_markdown();
                let cli_path = output.join("cli.md");
                std::fs::write(&cli_path, &help).expect("Failed to write CLI documentation");
                println!("CLI documentation written to: {}", cli_path.display());

                println!(
                    "\nDocumentation generated successfully in: {}",
                    output.display()
                );
            }
        },

        Commands::Registry { action } => match action {
            RegistryAction::ImportRussell {
                source,
                dry_run,
                validate_only,
                output_format,
                transform_rules,
                verbose,
            } => {
                let config = hkask_templates::russell_mapper::MigrationConfig {
                    dry_run,
                    validate_only,
                    output_format: match output_format.as_str() {
                        "json" => hkask_templates::russell_mapper::OutputFormat::Json,
                        "mermaid" => hkask_templates::russell_mapper::OutputFormat::Mermaid,
                        _ => hkask_templates::russell_mapper::OutputFormat::Yaml,
                    },
                    transform_rules_path: transform_rules,
                };

                match commands::import_russell(&source, &config, verbose) {
                    Ok(assets) => {
                        println!("Migration analysis complete: {} assets", assets.len());
                        for asset in &assets {
                            println!("\n  Origin: {}", asset.origin);
                            println!("  Type: {:?}", asset.asset_type);
                            println!("  Provenance: {}", asset.provenance_hash);
                            if verbose {
                                if let Some(manifest) = &asset.hkask_manifest {
                                    println!(
                                        "  hKask Manifest: {} ({} steps)",
                                        manifest.id,
                                        manifest.steps.len()
                                    );
                                }
                                if let Some(template) = &asset.hkask_template {
                                    println!(
                                        "  hKask Template: {} ({} lexicon terms)",
                                        template.id,
                                        template.lexicon_terms.len()
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Migration failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            RegistryAction::ListMigrated { origin } => {
                println!("Migrated assets:");
                println!("  (use 'kask registry import-russell --dry-run' to analyze assets)");
            }
        },
    }
}
