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

    /// Git archival and CAS actions
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

    /// Style composition — generate prose with exemplar retrieval
    Compose {
        #[command(subcommand)]
        action: ComposeAction,
    },

    /// Style corpus embedding (download, chunk, embed, store)
    EmbedCorpus {
        #[command(subcommand)]
        action: EmbedCorpusAction,
    },

    /// List available LLM models
    Models,

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
