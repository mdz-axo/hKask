//! CLI action enums — subcommand types for each top-level command

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum TemplateAction {
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
pub enum BotAction {
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
pub enum AgentAction {
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
pub enum PodAction {
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
pub enum McpAction {
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

    /// Invoke an MCP tool directly
    Invoke {
        /// MCP server name
        #[arg(long)]
        server: String,
        /// Tool name to invoke
        #[arg(long)]
        tool: String,
        /// JSON input arguments
        #[arg(long)]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum CnsAction {
    /// Get CNS health status
    Health,

    /// Get algedonic alerts
    Alerts,

    /// Get variety counters
    Variety,

    /// Subscribe to CNS events for an agent
    Subscribe {
        /// Agent WebID to observe events for
        #[arg(long)]
        agent: String,

        /// Span namespaces to subscribe to (comma-separated, e.g., "cns.tool,cns.inference")
        #[arg(long)]
        spans: String,
    },

    /// Display or update CNS set-points
    SetPoints {
        /// Set gas_min_remaining (0.0-1.0)
        #[arg(long)]
        gas_min_remaining: Option<f64>,
        /// Set variety_max_deficit
        #[arg(long)]
        variety_max_deficit: Option<f64>,
        /// Set error_rate_max (0.0-1.0)
        #[arg(long)]
        error_rate_max: Option<f64>,
        /// Set connector_latency_max_secs
        #[arg(long)]
        connector_latency_max_secs: Option<f64>,
    },
}

#[derive(Subcommand)]
pub enum SovereigntyAction {
    /// Get current sovereignty state
    Status,

    /// Grant consent for data sharing (per-category)
    Grant {
        /// Data category to grant consent for
        #[arg(long)]
        category: String,
    },

    /// Revoke consent for data sharing
    Revoke {
        /// Data category to revoke consent for
        #[arg(long)]
        category: String,
    },

    /// Mark acquisition attempt (for testing)
    MarkAcquisition {
        /// VC investment level (0.0-1.0)
        #[arg(short, long, default_value = "0.3")]
        vc_investment: f32,
    },

    /// Check if kill zone is active
    KillZone,

    /// Check data access permissions
    Check {
        /// Data category to check
        #[arg(long)]
        category: String,
    },
}

#[derive(Subcommand)]
pub enum DocsAction {
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
pub enum RegistryAction {
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
pub enum GitAction {
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
pub enum EnsembleAction {
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
pub enum CuratorAction {
    /// Open interactive chat with Curator
    Chat,

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

/// Replicant identity actions
#[derive(Subcommand)]
pub enum ReplicantAction {
    /// Register a new replicant identity
    Register {
        /// Replicant name (login identifier)
        #[arg()]
        replicant_name: String,

        /// Human first name
        #[arg(long)]
        first_name: String,

        /// Human last name
        #[arg(long)]
        last_name: String,

        /// Human email (for recovery only)
        #[arg(long)]
        email: String,

        /// Human phone in E.164 format (optional, for recovery)
        #[arg(long)]
        phone: Option<String>,
    },

    /// Login as a replicant identity
    Login {
        /// Replicant name to login as
        #[arg()]
        replicant_name: String,
    },

    /// Logout from a session
    Logout {
        /// Session ID to invalidate
        #[arg()]
        session_id: String,
    },

    /// List active sessions for a replicant
    Sessions {
        /// Replicant name
        #[arg()]
        replicant_name: String,
    },

    /// List replicant identities for a human user
    List {
        /// User ID
        #[arg(long)]
        user_id: Option<String>,
    },

    /// Show replicant identity info
    Show {
        /// Replicant name
        #[arg()]
        replicant_name: String,
    },
}

#[derive(Subcommand)]
pub enum KeystoreAction {
    /// Load API keys from .env file into OS keychain
    Load {
        /// Path to .env file (default: .env in current directory)
        #[arg(short, long, default_value = ".env")]
        path: PathBuf,

        /// Key prefix to filter (default: HKASK_)
        #[arg(short = 'x', long, default_value = "HKASK_")]
        prefix: String,

        /// Overwrite existing keys
        #[arg(long)]
        overwrite: bool,
    },

    /// List keys stored in OS keychain
    List,

    /// Get a specific key value from OS keychain
    Get {
        /// Key name (e.g. HKASK_BRAVE_API_KEY)
        #[arg()]
        key: String,
    },

    /// Set a specific key value in OS keychain
    Set {
        /// Key name
        #[arg()]
        key: String,

        /// Value to store
        #[arg()]
        value: String,
    },

    /// Delete a key from OS keychain
    Delete {
        /// Key name
        #[arg()]
        key: String,
    },
}

/// Specification actions (DDMVSS)
#[derive(Subcommand)]
pub enum SpecAction {
    /// Capture a goal as a binding specification
    Capture {
        /// Spec name (human-readable goal description)
        #[arg(short, long)]
        name: String,

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

    /// Validate a specification by ID
    Validate {
        /// Specification ID to validate
        #[arg(short, long)]
        id: String,
    },

    /// Cultivate (evaluate) a specification by ID
    Cultivate {
        /// Specification ID to cultivate
        #[arg(short, long)]
        id: String,
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

/// Goal actions — minimal multi-agent coordination substrate.
///
/// All operations are OCAP-gated: a `GoalCapabilityToken` is minted from the
/// resolved OCAP secret, and authority denials are recorded as
/// `cns.tool.goal.capability.denied` CNS events.
#[derive(Subcommand)]
pub enum GoalAction {
    /// Create a new goal owned by the current user.
    Create {
        /// Goal text.
        text: String,

        /// Visibility: private | shared | public.
        #[arg(long, default_value = "private")]
        visibility: String,
    },

    /// List the current user's goals.
    List {
        /// Optional state filter: pending | active | completed | blocked | abandoned.
        #[arg(long)]
        state: Option<String>,
    },

    /// Transition a goal to a new state (e.g. active, completed).
    SetState {
        /// Goal ID.
        id: String,

        /// Target state: pending | active | completed | blocked | abandoned.
        state: String,
    },
}
