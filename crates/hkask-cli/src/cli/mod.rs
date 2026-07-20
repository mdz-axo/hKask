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
    /// Launch the interactive ratatui workspace (embeds the REPL)
    Tui {
        /// Agent to chat with (default: Curator)
        #[arg(default_value = "Curator")]
        agent: String,

        /// Optional: template ID to use
        #[arg(short, long)]
        template: Option<String>,

        /// Optional: model to use for inference (e.g., "DI/google/gemma-4-9b-it")
        #[arg(short, long)]
        model: Option<String>,

        /// Optional: input file (non-interactive mode). Use "-" for stdin.
        #[arg(short = 'f', long)]
        input: Option<PathBuf>,
    },

    /// Agent pod management (admin: export-container, export-k8s)
    Pod {
        #[command(subcommand)]
        action: PodAction,
    },

    /// MCP server inventory (read-only). Tool invocation is runtime-only — use the TUI.
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },

    /// User sovereignty management (Magna Carta enforcement)
    Sovereignty {
        #[command(subcommand)]
        action: SovereigntyAction,
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

    /// A2A agent registration and management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Federation lifecycle — cross-server curator sync
    Federation {
        #[command(subcommand)]
        action: FederationAction,
    },

    /// Token issuance and management (OCAP credential provisioning)
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

    /// Trained adapter lifecycle — deploy, infer, teardown
    Adapter {
        #[command(subcommand)]
        action: AdapterAction,
    },

    /// Validate all configured providers and API keys
    Doctor {
        /// Check the full bootstrap chain: daemon, socket, keychain, DB
        /// passphrase, session, MCP servers. Use this to diagnose
        /// "REPL loop stalls" or "No A2A secret" errors.
        #[arg(long)]
        bootstrap: bool,
    },

    /// Add a new replicant to an existing hKask installation
    Onboard,

    /// Manage REPL/CLI inference settings (same as /repl in interactive mode)
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },

    /// Start the hKask daemon (Unix socket for MCP server auth + CNS monitoring)
    Daemon {
        /// Daemon action
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Initialize hKask server configuration (interactive)
    Init,

    /// Sovereignty export — create and migrate encrypted h_mem archives
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

    /// Deploy hKask to a remote cluster (K3s/Hetzner bootstrap)
    Deploy {
        #[command(subcommand)]
        action: DeployAction,
    },
}

impl Commands {
    /// Safe label for logging — excludes sensitive arguments.
    pub fn label(&self) -> &'static str {
        match self {
            Commands::Tui { .. } => "tui",
            Commands::Pod { .. } => "pod",
            Commands::Mcp { .. } => "mcp",
            Commands::Sovereignty { .. } => "sovereignty",
            Commands::Git { .. } => "git",
            Commands::Backup { .. } => "backup",
            Commands::Agent { .. } => "agent",
            Commands::Federation { .. } => "federation",
            Commands::Token { .. } => "token",
            Commands::Replicant { .. } => "replicant",
            Commands::Keystore { .. } => "keystore",
            Commands::Adapter { .. } => "adapter",
            Commands::Doctor { .. } => "doctor",
            Commands::Onboard => "onboard",
            Commands::Settings { .. } => "settings",
            Commands::Daemon { .. } => "daemon",
            Commands::Init => "init",
            Commands::Export { .. } => "export",
            Commands::Serve { .. } => "serve",
            Commands::Wallet { .. } => "wallet",
            Commands::Matrix { .. } => "matrix",
            Commands::Repair { .. } => "repair",
            Commands::Deploy { .. } => "deploy",
        }
    }
}
