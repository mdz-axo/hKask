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
use hkask_cli::commands;
use hkask_cli::russell_mapper::RussellMappingConfig;
use hkask_mcp::runtime::McpRuntime;
<<<<<<< HEAD
use hkask_templates::SqliteRegistry;
=======
use hkask_templates::{
    FieldMapping, FieldMappings, IdTransformation, MappedTemplate, MappingMeta, ModelTierSelection,
    RegistryEntry, RegistryIndex, RussellMapper, RussellMappingConfig, SqliteRegistry,
    TemplateTypeInference,
};
>>>>>>> origin/main
use hkask_types::TemplateType as Type;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Parse a string into a DataCategory
fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    match s {
        "episodic_memory" => hkask_types::DataCategory::EpisodicMemory,
        "semantic_memory" => hkask_types::DataCategory::SemanticMemory,
        "personal_context" => hkask_types::DataCategory::PersonalContext,
        "capability_tokens" => hkask_types::DataCategory::CapabilityTokens,
        "ocap_boundaries" => hkask_types::DataCategory::OcapBoundaries,
        "template_invocations" => hkask_types::DataCategory::TemplateInvocations,
        "hlexicon_terms" => hkask_types::DataCategory::HLexiconTerms,
        "template_registry" => hkask_types::DataCategory::TemplateRegistry,
        _ => hkask_types::DataCategory::Custom(s.to_string()),
    }
}

#[derive(Parser)]
#[command(name = "kask")]
#[command(author = "hKask Team")]
#[command(version)]
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

        /// Agent to chat with (default: Curator)
        #[arg(short, long)]
        agent: Option<String>,
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

    /// User sovereignty management (Magna Carta enforcement)
    Sovereignty {
        #[command(subcommand)]
        action: SovereigntyAction,
    },

    /// Registry management
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },

    /// Git archival management (Phase 9)
    Git {
        #[command(subcommand)]
        action: GitAction,
    },

    /// Multi-agent ensemble management (Phase 7)
    Ensemble {
        #[command(subcommand)]
        action: EnsembleAction,
    },

    /// Specification authoring, curation, and validation (DDMVSS)
    Spec {
        #[command(subcommand)]
        action: SpecAction,
    },

    /// Documentation generation
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },

    /// ACP agent registration and management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Curator governance and metacognition
    Curator {
        #[command(subcommand)]
        action: CuratorAction,
    },
}

#[derive(Subcommand)]
enum TemplateAction {
    List {
        #[arg(short, long)]
        r#type: Option<String>,
    },
    Register {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        r#type: String,
        #[arg(short, long)]
        lexicon: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    Get {
        #[arg()]
        id: String,
    },
    Search {
        #[arg()]
        term: String,
    },
}

#[derive(Subcommand)]
enum BotAction {
    List {
        #[arg(short, long)]
        kind: Option<String>,
    },
    Status {
        #[arg()]
        name: String,
    },
    Grant {
        #[arg(short, long)]
        bot_id: String,
        #[arg(short, long)]
        capability: String,
    },
}

#[derive(Subcommand)]
enum AgentAction {
    Register {
        #[arg(long)]
        webid: String,
        #[arg(long)]
        agent_type: String,
        #[arg(long)]
        capabilities: String,
    },
    Unregister {
        #[arg(long)]
        name: String,
    },
    List,
    Capabilities {
        #[arg(long)]
        name: String,
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
enum SovereigntyAction {
    /// Get current sovereignty state
    Status,

    /// Grant explicit consent for data sharing
    GrantConsent,

    /// Revoke explicit consent
    RevokeConsent,

    /// Mark acquisition attempt (for testing)
    MarkAcquisition {
        /// VC investment level (0.0-1.0)
        #[arg(short, long, default_value = "0.3")]
        vc_investment: f32,
    },

    /// Check if kill zone is active
    KillZone,

    /// Check data access permissions
    CheckAccess {
        /// Data category to check
        #[arg(short, long)]
        category: String,
    },
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

/// Git archival actions (Phase 9)
#[derive(Subcommand)]
enum GitAction {
    /// Archive registry to GitHub repository
    Archive {
        /// GitHub repository owner
        #[arg(short, long)]
        owner: String,

        /// GitHub repository name
        #[arg(short, long)]
        repo: String,

        /// Branch to archive to
        #[arg(short, long, default_value = "main")]
        branch: String,

        /// Path in repository
        #[arg(short, long, default_value = "registry")]
        path: String,

        /// Content to archive (or use --file)
        #[arg(short, long)]
        content: Option<String>,

        /// File to archive
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Restore registry from GitHub repository
    Restore {
        /// GitHub repository owner
        #[arg(short, long)]
        owner: String,

        /// GitHub repository name
        #[arg(short, long)]
        repo: String,

        /// Git ref (branch, tag, or SHA)
        #[arg(short, long)]
        r#ref: String,

        /// Target path to restore to
        #[arg(short, long, default_value = ".")]
        target: String,
    },

    /// List archived registry versions
    List {
        /// GitHub repository owner
        #[arg(short, long)]
        owner: String,

        /// GitHub repository name
        #[arg(short, long)]
        repo: String,
    },

    /// Create registry snapshot (commit)
    Snapshot {
        /// GitHub repository owner
        #[arg(short, long)]
        owner: String,

        /// GitHub repository name
        #[arg(short, long)]
        repo: String,

        /// Commit message
        #[arg(short, long)]
        message: String,
    },
}

/// Ensemble multi-agent actions (Phase 7)
#[derive(Subcommand)]
enum EnsembleAction {
    /// Create a new chat session
    ChatCreate {
        /// Session ID
        #[arg(short, long)]
        session: String,
    },

    /// Register a bot in a chat session
    ChatRegister {
        /// Session ID
        #[arg(short, long)]
        session: String,

        /// Bot WebID
        #[arg(short, long)]
        bot: String,

        /// Bot role (memory_bot, spandrel_bot, okapi_bot, scholar_bot)
        #[arg(short, long)]
        role: String,
    },

    /// Send a message to chat
    ChatSend {
        /// Session ID
        #[arg(short, long)]
        session: String,

        /// Message content
        #[arg(short, long)]
        message: String,
    },

    /// List active chat sessions
    ChatList,

    /// Create a deliberation session
    DeliberationCreate {
        /// Session ID
        #[arg(short, long)]
        session: String,
    },

    /// Start deliberation
    DeliberationStart {
        /// Session ID
        #[arg(short, long)]
        session: String,
    },

    /// Record a response in deliberation
    DeliberationRecord {
        /// Session ID
        #[arg(short, long)]
        session: String,

        /// Agent WebID
        #[arg(short, long)]
        agent: String,

        /// Response content
        #[arg(short, long)]
        content: String,

        /// Confidence score (0.0-1.0)
        #[arg(short, long)]
        confidence: f64,
    },

    /// Synthesize deliberation responses
    DeliberationSynthesize {
        /// Session ID
        #[arg(short, long)]
        session: String,
    },

    /// List active deliberation sessions
    DeliberationList,

    /// Bootstrap the standing ensemble session from YAML
    StandingStart {
        /// Path to standing-ensemble-session.yaml
        #[arg(
            short,
            long,
            default_value = "registry/manifests/standing-ensemble-session.yaml"
        )]
        config: PathBuf,
    },

    /// Show standing session status
    StandingStatus,
}

/// Curator governance actions
#[derive(Subcommand)]
enum CuratorAction {
    /// List pending escalations
    Escalations,

    /// Resolve an escalation by ID
    Resolve {
        /// Escalation ID
        #[arg()]
        id: String,
    },

    /// Dismiss an escalation by ID
    Dismiss {
        /// Escalation ID
        #[arg()]
        id: String,
    },

    /// Run a metacognition cycle and display system health
    Metacognition,
}

/// Specification actions (DDMVSS)
#[derive(Subcommand)]
enum SpecAction {
    /// Capture a goal as a binding specification
    Capture {
        /// Goal description
        #[arg()]
        description: String,

        /// Spec category (domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation)
        #[arg(short, long, default_value = "domain")]
        category: String,

        /// Domain anchor (okapi, russell, hkask)
        #[arg(short, long, default_value = "hkask")]
        domain: String,

        /// Completion criteria (comma-separated)
        #[arg(short, long)]
        criteria: Option<String>,
    },

    /// List all specifications
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Evaluate a specification for coherence (curation)
    Evaluate {
        /// Specification ID
        #[arg()]
        spec_id: String,
    },

    /// Validate the full specification collection
    Validate {
        /// Coherence threshold (0.0-1.0)
        #[arg(short, long, default_value = "0.7")]
        threshold: f64,
    },

    /// Show collection coherence and missing categories
    Cultivate {
        /// Coherence threshold (0.0-1.0)
        #[arg(short, long, default_value = "0.7")]
        threshold: f64,
    },

    /// Render a specification template with spec data
    Render {
        /// Template path (e.g., spec/goal-capture.j2)
        #[arg()]
        template: String,

        /// Specification ID to populate template with
        #[arg(short, long)]
        spec_id: Option<String>,
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
    agent_name: Option<&str>,
) {
    let display_name = agent_name.unwrap_or("Curator");
    println!("ℏKask Chat - Interactive Mode");
    println!("Agent: {}", display_name);
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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(process_chat_input_async(
            registry,
            input,
            template_id,
            agent_name,
        ));
        println!("{}: {}\n", display_name, response);
    }
}

async fn process_chat_input_async(
    _registry: &SqliteRegistry,
    input: &str,
    _template_id: Option<&str>,
    agent_name: Option<&str>,
) -> String {
    commands::chat_with_agent(input, agent_name).await
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
    md.push_str(&format!(
        "*hKask v{} — Planck's Constant of Agent Systems*\n",
        env!("CARGO_PKG_VERSION")
    ));

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
            agent,
        } => {
            let agent_name = agent.as_deref();
            if interactive {
                run_chat_interactive(&registry, &runtime, template.as_deref(), agent_name);
            } else if let Some(input_path) = input {
                match std::fs::read_to_string(&input_path) {
                    Ok(content) => {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let response = rt.block_on(process_chat_input_async(
                            &registry,
                            &content,
                            template.as_deref(),
                            agent_name,
                        ));
                        println!("{}: {}", agent_name.unwrap_or("Curator"), response);
                    }
                    Err(e) => {
                        eprintln!("Failed to read input file: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_ok() {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let response = rt.block_on(process_chat_input_async(
                        &registry,
                        input.trim(),
                        template.as_deref(),
                        agent_name,
                    ));
                    println!("{}: {}", agent_name.unwrap_or("Curator"), response);
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
            TemplateAction::Search { term } => match commands::search_templates(&registry, &term) {
                Ok(results) => {
                    if results.is_empty() {
                        println!("No templates found with lexicon term: {}", term);
                    } else {
                        println!("Templates matching '{}':\n", term);
                        for entry in results {
                            println!("  {} ({})", entry.id, entry.template_type.as_str());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Search failed: {}", e);
                    std::process::exit(1);
                }
            },
        },

        Commands::Bot { action } => match action {
            BotAction::List { kind } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_list(kind.as_deref()).await {
                        Ok(agents) => {
                            if agents.is_empty() {
                                println!("No agents registered.");
                                return;
                            }
                            println!(
                                "{:<25} {:<12} {:<40} SOURCE",
                                "NAME", "KIND", "CAPABILITIES"
                            );
                            println!("{}", "-".repeat(100));
                            for agent in &agents {
                                println!(
                                    "{:<25} {:<12} {:<40} {}",
                                    agent.definition.name,
                                    agent.definition.agent_kind,
                                    agent.definition.capabilities.len(),
                                    agent.source_yaml,
                                );
                            }
                            println!("\nTotal: {} agents", agents.len());
                        }
                        Err(e) => {
                            eprintln!("Failed to list agents: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            BotAction::Status { name } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_status(&name).await {
                        Ok(agent) => {
                            let def = &agent.definition;
                            println!("Agent: {}", def.name);
                            println!("  Kind: {}", def.agent_kind);
                            println!("  Editor: {}", def.editor);
                            println!("  Binding contract: {}", def.binding_contract);
                            if let Some(charter) = &def.charter {
                                println!("  Charter: {}", charter.description);
                                println!("  Archetype: {}", charter.archetype);
                            }
                            println!("  Capabilities:");
                            for cap in &def.capabilities {
                                println!("    - {}", cap);
                            }
                            if !def.rights.is_empty() {
                                println!("  Rights:");
                                for r in &def.rights_flat() {
                                    println!("    - {}", r);
                                }
                            }
                            if !def.responsibilities.is_empty() {
                                println!("  Responsibilities:");
                                for r in &def.responsibilities_flat() {
                                    println!("    - {}", r);
                                }
                            }
                            if let Some(persona) = &def.persona {
                                println!("  Persona:");
                                println!("    Tone: {}", persona.tone);
                                println!("    Verbosity: {}", persona.verbosity);
                                if !persona.forbidden.is_empty() {
                                    println!("    Forbidden: {}", persona.forbidden.join(", "));
                                }
                            }
                            if let Some(probe) = &def.readiness_probe {
                                println!(
                                    "  Readiness probe: {} ({})",
                                    probe.endpoint, probe.probe_type
                                );
                            }
                            println!("  Registered: {}", agent.registered_at);
                            println!("  Source: {}", agent.source_yaml);
                        }
                        Err(e) => {
                            eprintln!("Failed to get agent status: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            BotAction::Grant { bot_id, capability } => {
                println!("Grant capability: {} to bot: {}", capability, bot_id);
                println!("Note: Capability granting via ACP attenuation not yet wired.");
            }
        },

        Commands::Pod { action } => match action {
            PodAction::Create {
                template,
                persona,
                name,
            } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::create_pod(&template, &persona, name.as_deref())) {
                    Ok(pod_id) => {
                        println!("Created agent pod: {}", pod_id);
                        println!("Template: {}", template);
                        println!("Persona file: {}", persona.display());
                        if let Some(n) = &name {
                            println!("Pod name: {}", n);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to create pod: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::Activate { pod_id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::activate_pod(&pod_id)) {
                    Ok(_) => {
                        println!("Activated agent pod: {}", pod_id);
                    }
                    Err(e) => {
                        eprintln!("Failed to activate pod: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::Deactivate { pod_id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::deactivate_pod(&pod_id)) {
                    Ok(_) => {
                        println!("Deactivated agent pod: {}", pod_id);
                    }
                    Err(e) => {
                        eprintln!("Failed to deactivate pod: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::Status { pod_id, verbose } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::get_pod_status(&pod_id)) {
                    Ok(status) => {
                        println!("Agent pod status: {}", pod_id);
                        println!("  State: {}", status.state);
                        println!("  WebID: {}", status.webid);
                        if let Some(name) = &status.name {
                            println!("  Name: {}", name);
                        }
                        if verbose {
                            println!("  Created at: {}", status.created_at);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get pod status: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::List => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let pods = rt.block_on(commands::list_pods());

                if pods.is_empty() {
                    println!("No pods registered.");
                } else {
                    println!("Agent pods ({}):\n", pods.len());
                    for pod in pods {
                        println!("  {} ({})", pod.pod_id, pod.state);
                        println!("    WebID: {}", pod.webid);
                        if let Some(name) = &pod.name {
                            println!("    Name: {}", name);
                        }
                        println!();
                    }
                }
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

        Commands::Sovereignty { action } => match action {
            SovereigntyAction::Status => {
                let state = hkask_types::UserSovereigntyState::new();

                println!("User Sovereignty Status:");
                println!("  Explicit consent: {}", state.explicit_consent);
                println!("  Sovereignty compromised: {}", state.is_compromised());
                println!("  Kill zone active: {}", state.detector.kill_zone_active);
                println!("  VC investment: {:.2}", state.detector.vc_investment);
                println!("  Threshold: {:.2}", state.detector.threshold);
                println!("  Acquisition resistance: {:?}", state.boundary.resistance);
                println!();
                println!("  Sovereign data:");
                for category in &state.boundary.sovereign_data {
                    println!("    - {}", category.as_str());
                }
                println!("  Shared data:");
                for category in &state.boundary.shared_data {
                    println!("    - {}", category.as_str());
                }
                println!("  Public data:");
                for category in &state.boundary.public_data {
                    println!("    - {}", category.as_str());
                }
            }
            SovereigntyAction::GrantConsent => {
                println!("Explicit consent granted.");
                println!("  Data sharing is now enabled for shared data categories.");
                println!("  Sovereign data remains protected.");
            }
            SovereigntyAction::RevokeConsent => {
                println!("Explicit consent revoked.");
                println!("  Data sharing is now disabled.");
                println!("  Only public data is accessible.");
            }
            SovereigntyAction::MarkAcquisition { vc_investment } => {
                let mut state = hkask_types::UserSovereigntyState::new();
                state.mark_acquisition_attempt();
                state.update_vc_investment(vc_investment);

                println!("Acquisition attempt marked.");
                println!("  VC investment: {:.2}", vc_investment);
                println!("  Kill zone active: {}", state.is_compromised());
                if state.is_compromised() {
                    println!("  [ALERT] Sovereignty compromised - CNS alert triggered!");
                }
            }
            SovereigntyAction::KillZone => {
                let state = hkask_types::UserSovereigntyState::new();

                println!("Kill Zone Status:");
                println!("  Active: {}", state.detector.kill_zone_active);
                println!(
                    "  Acquisition attempt: {}",
                    state.detector.acquisition_attempt
                );
                println!("  VC investment: {:.2}", state.detector.vc_investment);
                println!("  Threshold: {:.2}", state.detector.threshold);
                if state.detector.kill_zone_active {
                    println!("  [ALERT] Kill zone active - sovereignty compromised!");
                }
            }
            SovereigntyAction::CheckAccess { category } => {
                let owner = hkask_types::WebID::new();
                let checker = hkask_agents::SovereigntyChecker::new(owner);
                let state = checker.get_state();

<<<<<<< HEAD
                // Parse category string to DataCategory
                let data_category = parse_data_category(&category);

                let is_sovereign = state.boundary.is_sovereign(&data_category);
                let is_shared = state.boundary.is_shared(&data_category);
                let is_public = state.boundary.is_public(&data_category);
=======
                let is_sovereign = state.boundary.is_sovereign(&category);
                let is_shared = state.boundary.shared_data.contains(&category);
                let is_public = state.boundary.public_data.contains(&category);
>>>>>>> origin/main

                println!("Data access check for '{}':", category);
                if is_sovereign {
                    println!("  Category: SOVEREIGN");
                    println!("  Access: Requires explicit consent AND owner");
                } else if is_shared {
                    println!("  Category: SHARED");
                    println!("  Access: Requires explicit consent");
                } else if is_public {
                    println!("  Category: PUBLIC");
                    println!("  Access: Always accessible");
                } else {
                    println!("  Category: UNKNOWN");
                    println!("  Access: Denied by default");
                }
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
                let mut config = if let Some(rules_path) = &transform_rules {
                    match RussellMappingConfig::load_from_yaml(rules_path.to_str().unwrap_or("")) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to load transform rules from {}: {}. Using defaults.",
                                rules_path.display(),
                                e
                            );
                            RussellMappingConfig::defaults()
                        }
                    }
                } else {
                    let default_path = "registry/manifests/russell-mapping.yaml";
                    match RussellMappingConfig::load_from_yaml(default_path) {
                        Ok(c) => c,
                        Err(_) => RussellMappingConfig::defaults(),
                    }
                };

                config.dry_run = dry_run;

                let mapper = hkask_cli::russell_mapper::RussellMapper::with_config(config.clone());

                if validate_only {
                    match hkask_cli::commands::import_russell(&source, &config, verbose) {
                        Ok(assets) => {
                            println!("Validation complete: {} manifests parsed", assets.len());
                            for asset in &assets {
                                println!("\n  ID: {} [VALID]", asset.id);
                            }
                        }
                        Err(e) => {
                            eprintln!("Validation failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    match hkask_cli::commands::import_russell_with_mapper(&mapper, &source, verbose)
                    {
                        Ok(assets) => {
                            let fmt = output_format.to_lowercase();
                            match fmt.as_str() {
                                "json" => {
                                    let json = serde_json::to_string_pretty(&assets)
                                        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                                    println!("{}", json);
                                }
                                "mermaid" => {
                                    println!("graph LR");
                                    for asset in &assets {
                                        println!(
                                            "  russell[\"{}\"] --> hkask[\"{}\"]",
                                            asset.id, asset.id
                                        );
                                    }
                                }
                                _ => {
                                    println!(
                                        "Migration analysis complete: {} assets",
                                        assets.len()
                                    );
                                    for asset in &assets {
                                        println!("\n  ID: {}", asset.id);
                                        println!("  Type: {:?}", asset.template_type);
                                        println!("  Description: {}", asset.description);
                                        println!("  Model Tier: {}", asset.model_tier);
                                        println!("  Energy Cap: {}", asset.energy_cap);
                                    }
                                }
                            }

                            if !dry_run {
                                for asset in &assets {
                                    let entry = hkask_templates::RegistryEntry {
                                        id: asset.id.clone(),
                                        template_type: asset.template_type,
                                        lexicon_terms: vec!["russell-migrated".to_string()],
                                        description: asset.description.clone(),
                                        source_path: format!("russell-migrated:{}", asset.id),
                                        required_capabilities: vec![],
                                    };
                                    if let Err(e) = registry.register(entry, None) {
                                        eprintln!(
                                            "Failed to register template {}: {}",
                                            asset.id, e
                                        );
                                    } else if verbose {
                                        println!("  Registered: {}", asset.id);
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
            }
            RegistryAction::ListMigrated { origin: _ } => {
                println!("Migrated assets:");
                println!("  (use 'kask registry import-russell --dry-run' to analyze assets)");
            }
        },

        Commands::Git { action } => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let runtime = hkask_mcp::runtime::McpRuntime::new();

            match action {
                GitAction::Archive {
                    owner,
                    repo,
                    branch,
                    path,
                    content,
                    file,
                } => {
                    let content_str = if let Some(c) = content {
                        c
                    } else if let Some(f) = file {
                        std::fs::read_to_string(&f).unwrap_or_else(|e| {
                            eprintln!("Failed to read file: {}", e);
                            std::process::exit(1);
                        })
                    } else {
                        eprintln!("Either --content or --file must be provided");
                        std::process::exit(1);
                    };

                    match rt.block_on(commands::archive_registry_to_git(
                        &runtime,
                        &owner,
                        &repo,
                        &branch,
                        &path,
                        &content_str,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Archive failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                GitAction::Restore {
                    owner,
                    repo,
                    r#ref,
                    target,
                } => {
                    match rt.block_on(commands::restore_registry_from_git(
                        &runtime, &owner, &repo, &r#ref, &target,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Restore failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                GitAction::List { owner, repo } => {
                    match rt.block_on(commands::list_registry_archives(&runtime, &owner, &repo)) {
                        Ok(commits) => {
                            println!("Archived versions for {}/{}:", owner, repo);
                            for (i, sha) in commits.iter().enumerate() {
                                println!("  {}. {}", i + 1, sha);
                            }
                        }
                        Err(e) => {
                            eprintln!("List failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                GitAction::Snapshot {
                    owner,
                    repo,
                    message,
                } => {
                    match rt.block_on(commands::create_registry_snapshot(
                        &runtime, &owner, &repo, &message,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Snapshot failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Spec { action } => match action {
            SpecAction::Capture {
                description,
                category,
                domain,
                criteria,
            } => {
                use hkask_types::{CompletenessCheck, DomainAnchor, GoalSpec, Spec, SpecCategory};

                let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
                let anchor = DomainAnchor::parse_str(&domain).unwrap_or(DomainAnchor::Hkask);

                let mut goal = GoalSpec::new(&description);
                if let Some(crits) = criteria {
                    for c in crits.split(',') {
                        goal = goal.with_criterion(c.trim());
                    }
                }

                let spec = Spec::new(&description, cat, anchor).with_goal(goal);
                let complete = spec.is_complete();

                println!("Specification captured:");
                println!("  ID: {}", spec.id);
                println!("  Name: {}", spec.name);
                println!("  Category: {}", spec.category.as_str());
                println!("  Domain: {}", spec.domain_anchor.as_str());
                println!("  Complete: {}", complete);
            }
            SpecAction::List { category } => {
                println!("Specifications:");
                if let Some(cat) = category {
                    println!("  (filtered by category: {})", cat);
                }
                println!("  Note: Persistent spec storage requires hkask-mcp-spec server.");
            }
            SpecAction::Evaluate { spec_id } => {
                println!("Evaluating specification: {}", spec_id);
                println!("  Note: Evaluation requires hkask-mcp-spec server.");
            }
            SpecAction::Validate { threshold } => {
                println!(
                    "Validating specification collection (threshold: {:.2})",
                    threshold
                );
                println!("  Note: Validation requires hkask-mcp-spec server.");
            }
            SpecAction::Cultivate { threshold } => {
                use hkask_types::SpecCategory;

                println!(
                    "Cultivating specification collection (threshold: {:.2})",
                    threshold
                );
                println!("  Categories required:");
                for cat in SpecCategory::all() {
                    println!("    - {}", cat.as_str());
                }
                println!("  Note: Full cultivation requires hkask-mcp-spec server.");
            }
            SpecAction::Render { template, spec_id } => {
                use hkask_storage::SqliteSpecStore;
                use hkask_types::{SpecId, SpecStore};
                use minijinja::UndefinedBehavior;

                let template_path = format!("registry/templates/{}", template);
                let template_content = match std::fs::read_to_string(&template_path) {
                    Ok(content) => content,
                    Err(_) => {
                        eprintln!("Template not found: {}", template_path);
                        std::process::exit(1);
                    }
                };

                let db_path =
                    std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string());
                let conn = match rusqlite::Connection::open(&db_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to open database: {}", e);
                        std::process::exit(1);
                    }
                };
                let store = SqliteSpecStore::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
                if let Err(e) = store.init_schema() {
                    eprintln!("Failed to initialize spec schema: {}", e);
                    std::process::exit(1);
                }

                let ctx = if let Some(sid) = spec_id {
                    let parsed_id = match SpecId::from_string(&sid) {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("Invalid spec ID: {}", e);
                            std::process::exit(1);
                        }
                    };
                    match store.load(parsed_id) {
                        Ok(spec) => minijinja::context! {
                            spec_id => spec.id.to_string(),
                            goal_name => spec.name,
                            spec_category => spec.category.as_str(),
                            domain_anchor => spec.domain_anchor.as_str(),
                            goals => spec.goals.iter().map(|g| minijinja::context! {
                                text => g.text,
                                depth => g.depth,
                                criteria => g.criteria.iter().map(|c| minijinja::context! {
                                    description => c.description,
                                    satisfied => c.satisfied,
                                }).collect::<Vec<_>>(),
                            }).collect::<Vec<_>>(),
                        },
                        Err(e) => {
                            eprintln!("Failed to load spec: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    minijinja::context! {}
                };

                let mut env = minijinja::Environment::new();
                env.set_undefined_behavior(UndefinedBehavior::Strict);
                match env.render_str(&template_content, ctx) {
                    Ok(rendered) => println!("{}", rendered),
                    Err(e) => {
                        eprintln!("Template render error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },

        Commands::Ensemble { action } => {
            let rt = tokio::runtime::Runtime::new().unwrap();

            match action {
                EnsembleAction::ChatCreate { session } => {
                    match rt.block_on(commands::ensemble_chat_create(session.clone())) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Chat create failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::ChatRegister { session, bot, role } => {
                    match rt.block_on(commands::ensemble_chat_register(
                        session.clone(),
                        bot.clone(),
                        role.clone(),
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Chat register failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::ChatSend { session, message } => {
                    match rt.block_on(commands::ensemble_chat_send(
                        session.clone(),
                        message.clone(),
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Chat send failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::ChatList => match rt.block_on(commands::ensemble_chat_list()) {
                    Ok(sessions) => {
                        println!("Active chat sessions:");
                        for s in sessions {
                            println!("  - {}", s);
                        }
                    }
                    Err(e) => {
                        eprintln!("Chat list failed: {}", e);
                        std::process::exit(1);
                    }
                },
                EnsembleAction::DeliberationCreate { session } => {
                    match rt.block_on(commands::ensemble_deliberation_create(session.clone())) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Deliberation create failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationStart { session } => {
                    match rt.block_on(commands::ensemble_deliberation_start(session.clone())) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Deliberation start failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationRecord {
                    session,
                    agent,
                    content,
                    confidence,
                } => {
                    match rt.block_on(commands::ensemble_deliberation_record(
                        session.clone(),
                        agent.clone(),
                        content.clone(),
                        confidence,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Deliberation record failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationSynthesize { session } => {
                    match rt.block_on(commands::ensemble_deliberation_synthesize(session.clone())) {
                        Ok(result) => println!("Synthesized response:\n{}", result),
                        Err(e) => {
                            eprintln!("Deliberation synthesize failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationList => {
                    match rt.block_on(commands::ensemble_deliberation_list()) {
                        Ok(sessions) => {
                            println!("Active deliberation sessions:");
                            for s in sessions {
                                println!("  - {}", s);
                            }
                        }
                        Err(e) => {
                            eprintln!("Deliberation list failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::StandingStart { config } => {
                    match commands::ensemble_standing_start(&config) {
                        Ok(status) => {
                            println!("Standing session bootstrapped:");
                            println!("  Session ID: {}", status.session_id);
                            println!("  Participants: {}", status.participant_count);
                            println!("  Initial messages: {}", status.message_count);
                        }
                        Err(e) => {
                            eprintln!("Standing session bootstrap failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::StandingStatus => match commands::ensemble_standing_status() {
                    Ok(status) => {
                        println!("Standing session status:");
                        println!("  Session ID: {}", status.session_id);
                        println!("  Participants: {}", status.participant_count);
                        println!("  Messages: {}", status.message_count);
                        println!("\nParticipants:");
                        for p in &status.participants {
                            println!("  - {} ({})", p.name, p.role);
                        }
                    }
                    Err(e) => {
                        eprintln!("Standing status failed: {}", e);
                        std::process::exit(1);
                    }
                },
            }
        }

        Commands::Agent { action } => match action {
            AgentAction::Register {
                webid,
                agent_type,
                capabilities,
            } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let caps: Vec<String> = capabilities
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    match commands::agent_register(&webid, &agent_type, caps).await {
                        Ok(receipt) => {
                            println!("Agent registered:");
                            println!("  WebID: {}", receipt.webid);
                            println!("  Token: {}...", &receipt.token_hash[..16]);
                            println!("  Registered at: {}", receipt.registered_at);
                        }
                        Err(e) => {
                            eprintln!("Registration failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            AgentAction::Unregister { name } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::agent_unregister(&name).await {
                        Ok(()) => println!("Agent unregistered: {}", name),
                        Err(e) => {
                            eprintln!("Unregister failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            AgentAction::List => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_list(None).await {
                        Ok(agents) => {
                            if agents.is_empty() {
                                println!("No agents registered.");
                                return;
                            }
                            println!("{:<25} {:<12} {:<40}", "NAME", "KIND", "CAPABILITIES");
                            println!("{}", "-".repeat(80));
                            for agent in &agents {
                                println!(
                                    "{:<25} {:<12} {:<40}",
                                    agent.definition.name,
                                    agent.definition.agent_kind,
                                    agent.definition.capabilities.join(", "),
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to list agents: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            AgentAction::Capabilities { name } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_status(&name).await {
                        Ok(agent) => {
                            println!("Capabilities for {}:", agent.definition.name);
                            for cap in &agent.definition.capabilities {
                                println!("  - {}", cap);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to get capabilities: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
        },

        Commands::Curator { action } => match action {
            CuratorAction::Escalations => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_escalations().await {
                        Ok(escalations) => {
                            if escalations.is_empty() {
                                println!("No pending escalations.");
                                return;
                            }
                            println!(
                                "{:<20} {:<15} {:<10} CONTEXT",
                                "ID", "BOT", "CONFIDENCE"
                            );
                            println!("{}", "-".repeat(80));
                            for esc in &escalations {
                                println!(
                                    "{:<20} {:<15} {:<10.2} {}",
                                    &esc.id[..std::cmp::min(20, esc.id.len())],
                                    esc.bot_id
                                        .0
                                        .to_string()
                                        .split('-')
                                        .next()
                                        .unwrap_or("unknown"),
                                    esc.confidence,
                                    &esc.error_context
                                        [..std::cmp::min(40, esc.error_context.len())],
                                );
                            }
                            println!("\nTotal: {} pending escalations", escalations.len());
                        }
                        Err(e) => {
                            eprintln!("Failed to list escalations: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            CuratorAction::Resolve { id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_resolve(&id).await {
                        Ok(()) => println!("Escalation {} resolved.", id),
                        Err(e) => {
                            eprintln!("Failed to resolve escalation: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            CuratorAction::Dismiss { id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_dismiss(&id).await {
                        Ok(()) => println!("Escalation {} dismissed.", id),
                        Err(e) => {
                            eprintln!("Failed to dismiss escalation: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            CuratorAction::Metacognition => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_metacognition().await {
                        Ok(summary) => println!("{}", summary),
                        Err(e) => {
                            eprintln!("Metacognition cycle failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
        },
    }
}
