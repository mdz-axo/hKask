//! CLI definition — top-level parser, command enums, and re-exports

mod actions;
mod helpers;
mod markdown;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use actions::*;
pub use helpers::{init_logging, parse_template_type};
pub use markdown::generate_cli_markdown;

#[derive(Debug, Parser)]
#[command(name = "kask")]
#[command(author = "hKask Team")]
#[command(version)]
#[command(about = "A Minimal Viable Container for Replicants - CLI", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Output logs as JSON (for OpenTelemetry / structured log ingestion)
    #[arg(long)]
    pub json_logs: bool,

    /// Registry database path (default: in-memory)
    #[arg(short, long)]
    pub registry: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Curator chat interface (interactive by default)
    Chat {
        /// Agent to chat with (default: Curator)
        #[arg(default_value = "Curator")]
        agent: String,

        /// Optional: template ID to use
        #[arg(short, long)]
        template: Option<String>,

        /// Optional: model to use for inference (e.g., "DI/google/gemma-4-9b-it")
        #[arg(short, long)]
        model: Option<String>,

        /// Optional: input file (non-interactive mode)
        #[arg(short = 'f', long)]
        input: Option<PathBuf>,

        /// Launch the TUI workspace instead of the line-based REPL
        #[arg(long)]
        tui: bool,
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

    /// Goal coordination substrate (OCAP-gated, CNS-observed)
    Goal {
        #[command(subcommand)]
        action: GoalAction,
    },

    /// Git archival and CAS actions
    Git {
        #[command(subcommand)]
        action: GitAction,
    },

    /// Backup operations — snapshot, restore, list, prune, verify, config
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },

    /// Specification authoring, curation, and validation (MDS)
    Spec {
        #[command(subcommand)]
        action: SpecAction,
    },

    /// Documentation generation
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },

    /// A2A agent registration and management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Curator governance and metacognition
    Curator {
        #[command(subcommand)]
        action: CuratorAction,
    },

    /// Federation lifecycle — cross-server curator sync
    Federation {
        #[command(subcommand)]
        action: FederationAction,
    },

    /// Token issuance and management
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },

    /// Replicant identity management
    Replicant {
        #[command(subcommand)]
        action: ReplicantAction,
    },

    /// Keystore management (OS keychain)
    Keystore {
        #[command(subcommand)]
        action: KeystoreAction,
    },

    /// Skill bundle management (compose, apply, evolve)
    Bundle {
        #[command(subcommand)]
        action: BundleAction,
    },

    /// Skill management (list, status, publish)
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Style operations — compose prose or embed corpora
    Style {
        #[command(subcommand)]
        action: StyleAction,
    },

    /// Kata operations — list and inspect kata manifests
    Kata {
        #[command(subcommand)]
        action: KataAction,
    },

    /// Kanban board and task coordination
    Kanban {
        #[command(subcommand)]
        action: KanbanAction,
    },

    /// Trained adapter lifecycle — deploy, infer, teardown
    Adapter {
        #[command(subcommand)]
        action: AdapterAction,
    },

    /// List available LLM models
    Models,

    /// Validate all configured providers and API keys
    Doctor,

    /// Add a new replicant to an existing hKask installation
    Onboard,

    /// Manage REPL/CLI inference settings (same as /repl in interactive mode)
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },

    /// Trigger episodic→semantic consolidation with optional semantic cleanup
    Consolidate {
        /// Agent name or WebID whose episodic memory to consolidate
        #[arg(short, long)]
        agent: Option<String>,

        /// Maximum episodic triples to consolidate (default: 100)
        #[arg(short, long, default_value = "100")]
        limit: usize,

        /// Confidence floor — semantic triples at or below this confidence
        /// are deleted after consolidation (default: SemanticLoop threshold, 0.33)
        #[arg(long)]
        confidence_floor: Option<f64>,

        /// Maximum semantic triples to retain after consolidation.
        /// If exceeded, lowest-confidence triples are deleted.
        #[arg(long)]
        max_semantic_triples: Option<usize>,

        /// Master passphrase for authorization (derived via HKDF-SHA256 to produce
        /// the capability_key used as the DB passphrase, matching onboarding flow)
        #[arg(long)]
        passphrase: Option<String>,
    },

    /// Run the 6-loop regulation system
    Loops,

    /// Start the hKask daemon (Unix socket for MCP server auth + CNS monitoring)
    Daemon {
        /// Daemon action
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Run contract tests and report REQ-tagged violations
    Test {
        /// Crate to test (default: all priority crates)
        #[arg(short, long)]
        crate_name: Option<String>,

        /// Output format (default: text)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Watch mode — run continuously at interval (seconds)
        #[arg(long)]
        watch: Option<u64>,
    },

    /// Search the web
    WebSearch {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(long, default_value = "5")]
        max_results: usize,
    },

    /// Initialize hKask server configuration (interactive)
    Init,

    /// Sovereignty export — create and migrate encrypted triple archives
    Export {
        #[command(subcommand)]
        action: ExportAction,
    },

    /// Start the HTTP API server (shares state with CLI)
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Wallet operations — balance, deposits, withdrawals, API keys
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },

    /// List artifacts in a registry (e.g., "styles", "bots", "templates")
    List {
        /// Registry name to list (e.g., "styles")
        registry: String,
    },

    /// Remove an artifact from a registry (e.g., "styles-hemingway")
    Rm {
        /// Registry and artifact name, hyphen-separated (e.g., "styles-hemingway")
        target: String,
        /// Database path
        #[arg(short, long, env = "HKASK_DB_PATH")]
        db: Option<String>,
        /// Database passphrase
        #[arg(long, env = "HKASK_DB_PASSPHRASE")]
        passphrase: Option<String>,
    },

    /// View a transcript bundle with synchronized audio playback (TUI)
    Transcript {
        /// Path to the transcript bundle JSON file
        path: PathBuf,
    },

    /// Matrix messaging — sidecar deployment, agent registration, health checks
    Matrix {
        #[command(subcommand)]
        action: MatrixAction,
    },

    /// Repair encrypted databases — detect and fix passphrase mismatches
    Repair {
        /// Only list broken databases, don't delete anything
        #[arg(long)]
        dry_run: bool,

        /// Delete all broken databases without prompting
        #[arg(long, conflicts_with = "dry_run")]
        force: bool,
    },
}

impl Commands {
    /// Safe label for logging — excludes sensitive arguments.
    pub fn label(&self) -> &'static str {
        match self {
            Commands::Chat { .. } => "chat",
            Commands::Template { .. } => "template",
            Commands::Bot { .. } => "bot",
            Commands::Pod { .. } => "pod",
            Commands::Mcp { .. } => "mcp",
            Commands::Cns { .. } => "cns",
            Commands::Sovereignty { .. } => "sovereignty",
            Commands::Goal { .. } => "goal",
            Commands::Git { .. } => "git",
            Commands::Backup { .. } => "backup",
            Commands::Spec { .. } => "spec",
            Commands::Docs { .. } => "docs",
            Commands::Agent { .. } => "agent",
            Commands::Curator { .. } => "curator",
            Commands::Federation { .. } => "federation",
            Commands::Token { .. } => "token",
            Commands::Replicant { .. } => "replicant",
            Commands::Keystore { .. } => "keystore",
            Commands::Bundle { .. } => "bundle",
            Commands::Skill { .. } => "skill",
            Commands::Style { .. } => "style",
            Commands::Kata { .. } => "kata",
            Commands::Kanban { .. } => "kanban",
            Commands::Adapter { .. } => "adapter",
            Commands::Models => "models",
            Commands::Doctor => "doctor",
            Commands::Onboard => "onboard",
            Commands::Settings { .. } => "settings",
            Commands::Consolidate { .. } => "consolidate",
            Commands::Loops => "loops",
            Commands::Daemon { .. } => "daemon",
            Commands::Test { .. } => "test",
            Commands::WebSearch { .. } => "web_search",
            Commands::Init => "init",
            Commands::Export { .. } => "export",
            Commands::Serve { .. } => "serve",
            Commands::Wallet { .. } => "wallet",
            Commands::List { .. } => "list",
            Commands::Rm { .. } => "rm",
            Commands::Transcript { .. } => "transcript",
            Commands::Matrix { .. } => "matrix",
            Commands::Repair { .. } => "repair",
        }
    }
}
