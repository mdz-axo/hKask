//! CLI definition — top-level parser, command enums, and re-exports

mod actions;
mod helpers;
mod markdown;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use actions::*;
pub use helpers::{init_logging, parse_data_category, parse_template_type};
pub use markdown::generate_cli_markdown;

#[derive(Parser)]
#[command(name = "kask")]
#[command(author = "hKask Team")]
#[command(version)]
#[command(about = "A Minimal Viable Container for Agents - CLI", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Registry database path (default: in-memory)
    #[arg(short, long)]
    pub registry: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Curator chat interface (interactive by default)
    Chat {
        /// Agent to chat with (default: Curator)
        #[arg(default_value = "Curator")]
        agent: String,

        /// Optional: template ID to use
        #[arg(short, long)]
        template: Option<String>,

        /// Optional: model to use for inference (e.g., "qwen3:8b")
        #[arg(short, long)]
        model: Option<String>,

        /// Optional: input file (non-interactive mode)
        #[arg(short = 'f', long)]
        input: Option<PathBuf>,
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

    /// List available LLM models
    Models,

    /// Run the 6-loop regulation system
    Loops,

    /// Search the web
    WebSearch {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(long, default_value = "5")]
        max_results: usize,
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
}
