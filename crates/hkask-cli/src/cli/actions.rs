//! CLI action enums — subcommand types for each top-level command

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
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

#[derive(Debug, Subcommand)]
pub enum BotAction {
    List {
        #[arg(short, long)]
        kind: Option<String>,
    },
    Status {
        #[arg()]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
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
    /// Revert an agent pod to a prior snapshot with safety backup
    Revert {
        /// Pod name to revert
        #[arg()]
        name: String,
        /// Commit hash to revert to
        #[arg(long)]
        commit: String,
        /// Reason for the revert (recorded in CNS)
        #[arg(short, long)]
        reason: String,
    },
    /// Spawn a new agent pod from a prior snapshot (fork)
    SpawnAgent {
        /// Source pod name whose state to clone
        #[arg()]
        source: String,
        /// New pod name for the spawned agent
        #[arg(long = "as")]
        new_name: String,
        /// Commit hash to spawn from
        #[arg(long)]
        commit: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum PodAction {
    Create {
        #[arg(short, long)]
        template: String,
        #[arg(short, long)]
        persona: PathBuf,
        #[arg(short, long)]
        name: Option<String>,
    },
    Activate {
        #[arg()]
        pod_id: String,
    },
    Deactivate {
        #[arg()]
        pod_id: String,
    },
    Status {
        #[arg()]
        pod_id: String,
        #[arg(short, long)]
        verbose: bool,
    },
    List,
    Assign {
        #[arg()]
        name: String,
        #[arg()]
        role: String,
    },
    Mode {
        #[arg()]
        name: String,
        #[arg()]
        mode: String,
        #[arg(short, long)]
        role: Option<String>,
    },
    /// Export a pod as a container build context (Containerfile + pod files)
    ExportContainer {
        #[arg()]
        pod_id: String,
        #[arg(short, long, default_value = "./pod-build")]
        output: PathBuf,
    },
    /// Export a pod as K8s manifests for Hetzner K3s deployment
    ExportK8s {
        #[arg()]
        pod_id: String,
        #[arg(short = 'v', long, default_value = "10")]
        volume_size_gb: u32,
        #[arg(short = 'r', long, default_value = "3")]
        max_replicas: u32,
        #[arg(short, long, default_value = "./k8s-manifests")]
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum McpAction {
    ListServers,
    ListTools,
    GetTool {
        #[arg()]
        name: String,
    },
    Invoke {
        #[arg(long)]
        server: String,
        #[arg(long)]
        tool: String,
        #[arg(long)]
        input: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CnsAction {
    Health,
    Alerts,
    Variety,
    Subscribe {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        spans: String,
    },
    SetPoints {
        #[arg(long)]
        gas_min_remaining: Option<f64>,
        #[arg(long)]
        variety_max_deficit: Option<f64>,
        #[arg(long)]
        error_rate_max: Option<f64>,
        #[arg(long)]
        connector_latency_max_secs: Option<f64>,
        #[arg(long)]
        communication_backpressure_threshold: Option<f64>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SovereigntyAction {
    Status,
    Grant {
        #[arg(long)]
        category: String,
    },
    Revoke,
    Check {
        #[arg(long)]
        category: String,
    },
    Verify {
        #[arg(long)]
        principle: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DocsAction {
    Openapi {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Cli {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    All {
        #[arg(short, long)]
        output: PathBuf,
    },
}

/// Git archival and CAS actions
#[derive(Debug, Subcommand)]
pub enum GitAction {
    Archive {
        #[arg(short, long)]
        owner: String,
        #[arg(short, long)]
        repo: String,
        #[arg(short, long, default_value = "main")]
        branch: String,
        #[arg(short, long, default_value = "registry")]
        path: String,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    Restore {
        #[arg(short, long)]
        owner: String,
        #[arg(short, long)]
        repo: String,
        #[arg(short, long)]
        r#ref: String,
        #[arg(short, long, default_value = ".")]
        target: String,
    },
    List {
        #[arg(short, long)]
        owner: String,
        #[arg(short, long)]
        repo: String,
    },
    Snapshot {
        #[arg(short, long)]
        owner: String,
        #[arg(short, long)]
        repo: String,
        #[arg(short, long)]
        message: String,
    },
    CasVerify {
        #[arg(short, long, default_value = "registry")]
        repo: String,
    },
    CasDiff {
        #[arg(short, long, default_value = "registry")]
        repo: String,
        #[arg(short, long)]
        from: String,
        #[arg(short, long)]
        to: String,
    },
    CasLog {
        #[arg(short, long, default_value = "registry")]
        repo: String,
        #[arg(short, long, default_value = "20")]
        max_count: usize,
    },
    CasSnapshot {
        #[arg(short, long, default_value = "registry")]
        repo: String,
        #[arg(short, long)]
        message: String,
    },
    CasRestore {
        #[arg(short, long, default_value = "registry")]
        repo: String,
        #[arg(short, long)]
        r#ref: Option<String>,
        #[arg(short, long)]
        prefix: Option<String>,
    },
}

/// Backup actions — snapshot, restore, list, prune, verify, config
#[derive(Debug, Subcommand)]
pub enum BackupAction {
    /// Create a backup snapshot
    Snapshot {
        /// Scope: "full", or artifact type label (e.g., "template", "goal")
        #[arg(short, long, default_value = "full")]
        scope: String,
    },
    /// Restore a pod from a backup snapshot
    Restore {
        /// Pod name to restore
        #[arg()]
        pod: String,
        /// Date to restore from (YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS)
        #[arg(short, long)]
        date: Option<String>,
        /// Commit hash to restore from (alternative to --date)
        #[arg(short, long)]
        commit: Option<String>,
    },
    /// List backup snapshots
    List {
        /// Filter by artifact type
        #[arg(short, long)]
        r#type: Option<String>,
        /// Maximum snapshots to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Prune expired snapshots (dry-run by default)
    Prune {
        /// Actually remove (default is dry-run)
        #[arg(long)]
        execute: bool,
    },
    /// Verify backup integrity
    Verify,
    /// Show backup health and daemon status
    Status,
    /// View or update backup configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show current backup configuration
    Show,
    /// Set tracked artifact types
    Set {
        /// Comma-separated artifact types to track (e.g., "template,goal")
        #[arg(short, long)]
        types: String,
        /// Retention duration (e.g., "30d", "24h", "60m")
        #[arg(short, long)]
        retention: Option<String>,
        /// Disable auto-snapshot
        #[arg(long)]
        no_auto: bool,
    },
}

/// Curator governance actions
#[derive(Debug, Subcommand)]
pub enum TokenAction {
    /// Issue a new DelegationToken for a replicant
    Issue {
        #[arg(long)]
        replicant: String,
        #[arg(long, num_args = 1..)]
        capabilities: Vec<String>,
        #[arg(long, default_value = "24h")]
        ttl: String,
    },
    /// List issued tokens (by replicant filter)
    List {
        #[arg(long)]
        replicant: Option<String>,
    },
    /// Revoke a token by ID
    Revoke {
        #[arg()]
        token_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CuratorAction {
    Chat,
    Escalations,
    Resolve {
        #[arg()]
        id: String,
    },
    Dismiss {
        #[arg()]
        id: String,
    },
    Metacognition,
    /// Initialize the hKask system: deploy shared Conduit, create org tokens
    Init {
        /// Domain for the hKask installation (e.g., hkask.example.com)
        #[arg(long, default_value = "localhost")]
        domain: String,
    },
}

/// Federation lifecycle — cross-server curator sync, invite, pause, revoke.
#[derive(Debug, Subcommand)]
pub enum FederationAction {
    /// Invite a remote server to join the federation
    Invite {
        /// Replica ID for the remote server (e.g., "curator.hkask.example.com")
        #[arg(long)]
        peer_replica: String,
        /// Domain of the remote hKask server
        #[arg(long)]
        peer_server_domain: String,
        /// Matrix homeserver domain for the remote Conduit
        #[arg(long)]
        peer_matrix_domain: String,
        /// Matrix user ID for the remote curator
        #[arg(long)]
        peer_curator_matrix_id: String,
        /// Optional invitation message
        #[arg(long)]
        message: Option<String>,
    },
    /// Accept a pending federation invitation
    Accept {
        /// Replica ID of the inviter
        #[arg()]
        invitation_id: String,
    },
    /// Reject a pending federation invitation
    Reject {
        /// Replica ID of the inviter
        #[arg()]
        invitation_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Pause sync with a peer (security measure)
    Pause {
        #[arg()]
        peer_replica: String,
        #[arg(long)]
        reason: String,
    },
    /// Resume sync with a paused peer
    Resume {
        #[arg()]
        peer_replica: String,
    },
    /// Permanently revoke a member
    Revoke {
        #[arg()]
        peer_replica: String,
        #[arg(long)]
        reason: String,
    },
    /// Voluntarily leave the federation
    Leave {
        #[arg(long)]
        reason: String,
    },
    /// Dissolve the federation (revoke all links)
    Dissolve {
        #[arg(long)]
        reason: String,
    },
    /// Show federation link status
    Status,
}

/// Replicant identity actions
#[derive(Debug, Subcommand)]
pub enum ReplicantAction {
    Register {
        #[arg()]
        replicant_name: String,
        #[arg(long)]
        first_name: String,
        #[arg(long)]
        last_name: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        phone: Option<String>,
    },
    Login {
        #[arg()]
        replicant_name: String,
    },

    /// Change a replicant's passphrase
    Passphrase {
        #[arg()]
        replicant_name: String,
    },
    Logout {
        #[arg()]
        session_id: String,
    },
    Sessions {
        #[arg()]
        replicant_name: String,
    },
    List {
        #[arg(long)]
        user_id: Option<String>,
    },
    Show {
        #[arg()]
        replicant_name: String,
    },
    /// Rename a replicant
    Rename {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    /// Delete a replicant and all its data
    Delete {
        #[arg()]
        name: String,
    },
}

/// Sovereignty export actions
#[derive(Debug, Subcommand)]
pub enum ExportAction {
    /// Create an encrypted sovereignty backup archive
    Create {
        /// Passphrase to encrypt the archive
        #[arg(short, long)]
        passphrase: String,
    },
    /// Upload a sovereignty archive to a server (migration)
    Upload {
        /// Path to the archive file
        #[arg(short, long)]
        archive: PathBuf,
        /// Passphrase to decrypt the archive
        #[arg(short, long)]
        passphrase: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeystoreAction {
    Load {
        #[arg(short, long, default_value = ".env")]
        path: PathBuf,
        #[arg(short = 'x', long, default_value = "HKASK_")]
        prefix: String,
        #[arg(long)]
        overwrite: bool,
        /// Securely delete the file after loading keys into the keychain.
        /// Requires explicit confirmation — the system will warn that the
        /// file will be permanently destroyed and ask for consent.
        #[arg(long)]
        shred: bool,
    },
    List,
    Get {
        #[arg()]
        key: String,
    },
    Set {
        #[arg()]
        key: String,
        #[arg()]
        value: String,
    },
    Delete {
        #[arg()]
        key: String,
    },
    /// Rotate the master key version — increments the key version,
    /// derives new secrets, and stores them in the keychain.
    /// Old-version secrets remain derivable for data access.
    /// Requires the current master passphrase.
    Rotate {
        /// New master passphrase (if changing). If not provided,
        /// the current passphrase is used with the incremented version.
        #[arg(short, long)]
        passphrase: Option<String>,
    },
}

/// Specification actions (MDS)
#[derive(Debug, Subcommand)]
pub enum SpecAction {
    Capture {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "domain")]
        category: String,
        #[arg(short, long, default_value = "hkask")]
        domain: String,
        #[arg(short, long)]
        criteria: Option<String>,
    },
    List {
        #[arg(short, long)]
        category: Option<String>,
    },
    Evaluate {
        #[arg()]
        spec_id: String,
    },
    Validate {
        #[arg(short, long)]
        spec_id: String,
    },
    Cultivate {
        #[arg(short, long)]
        spec_id: String,
    },
    Render {
        #[arg()]
        template: String,
        #[arg(short, long)]
        spec_id: Option<String>,
    },
}

/// Style subcommands — compose prose or embed corpora
#[derive(Debug, Subcommand)]
pub enum StyleAction {
    /// Generate prose with exemplar retrieval and centroid validation
    Compose {
        #[arg(short, long)]
        prompt: String,
        #[arg(short, long)]
        cognition: PathBuf,
        #[arg(short, long)]
        db: PathBuf,
        #[arg(long, env = "HKASK_DB_PASSPHRASE")]
        passphrase: String,
        #[arg(long)]
        no_validate: bool,
    },
    /// Download, chunk, embed, and store a style corpus
    EmbedCorpus {
        #[arg(short, long)]
        config: PathBuf,
        #[arg(short, long)]
        replicant: String,
        #[arg(long)]
        passphrase: String,
        #[arg(short, long)]
        db: Option<PathBuf>,
    },
    /// Discover an academic author's works and generate a corpus.yaml
    Discover {
        /// Author name (e.g., "David Dunning")
        author_name: String,
        /// Max works to include
        #[arg(short, long, default_value = "20")]
        max_works: usize,
        /// Output directory for corpus.yaml
        #[arg(short, long)]
        output_dir: Option<String>,
        /// Cache directory for downloaded content
        #[arg(long, default_value = "./.cache")]
        cache_dir: String,
        /// SerpAPI key for web + YouTube transcript search
        #[arg(long, env = "HKASK_SERPAPI_API_KEY")]
        serpapi_key: Option<String>,
        /// Skip YouTube transcript search
        #[arg(long)]
        no_transcripts: bool,
        /// Skip web search
        #[arg(long)]
        no_web: bool,
        /// Skip curation — auto-include all web + YouTube results
        #[arg(long)]
        no_curate: bool,
        /// Search terms for web + YouTube queries (e.g., "Dunning-Kruger effect metacognition overconfidence")
        #[arg(long)]
        search_terms: Option<String>,
        /// Skip LLM-based concept extraction and method inference
        #[arg(long)]
        no_methods: bool,
        /// Biographical details for author disambiguation
        /// (e.g., "professor of psychology at Cornell University")
        #[arg(long)]
        bio: Option<String>,
    },
}

/// Skill bundle management actions
#[derive(Debug, Subcommand)]
pub enum BundleAction {
    Compose {
        #[arg(num_args = 1..)]
        skills: Vec<String>,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long, default_value = "private")]
        visibility: String,
    },
    Apply {
        #[arg()]
        bundle_id: String,
    },
    List,
    Show {
        #[arg()]
        bundle_id: String,
    },
    Evolve {
        #[arg()]
        bundle_id: String,
    },
    Skills,
    Off,
}

/// Goal actions — minimal multi-agent coordination substrate.
///
/// Goal operations are available to anyone with DB access — no token ceremony.
#[derive(Debug, Subcommand)]
pub enum GoalAction {
    Create {
        text: String,
        #[arg(long, default_value = "private")]
        visibility: String,
    },
    List {
        #[arg(long)]
        state: Option<String>,
    },
    SetState {
        id: String,
        state: String,
    },
}

/// Skill management actions — visibility, publish, change detection.
///
/// Two-zone model (src→dist pattern):
/// - `.agents/skills/` — source of truth (private zone)
/// - `skills/` — export surface, generated by `kask skill publish` (public zone)
#[derive(Debug, Subcommand)]
pub enum SkillAction {
    List {
        #[arg(long)]
        visibility: Option<String>,
    },
    Status {
        name: String,
    },
    Publish {
        name: String,
    },
    /// Run the dual-layer skill audit and optionally fail the process for CI.
    Audit {
        /// Fail the process if any skill scores below this threshold.
        #[arg(long, default_value = "0.8")]
        fail_below: f64,
        /// Emit machine-readable JSON instead of human-readable tables.
        #[arg(long)]
        json: bool,
    },
}

/// Kata actions — list, inspect, and execute kata manifests
#[derive(Debug, Subcommand)]
pub enum KataAction {
    /// List available kata manifests
    List,
    /// Show details of a specific kata manifest
    Show {
        /// Manifest name (e.g., "kata-starter", "kata-improvement")
        name: String,
    },
    /// Execute a kata cycle
    Start {
        /// Manifest name (e.g., "kata-improvement", "kata-starter")
        name: String,
        /// Learner bot identity (e.g., "Alice")
        #[arg(short, long)]
        bot: String,
        /// Optional context key=value pairs
        #[arg(short, long = "ctx", num_args = 1..)]
        context: Vec<String>,
        /// Save state to a file after execution
        #[arg(long)]
        save: Option<PathBuf>,
        /// Resume from a previously saved state file
        #[arg(long)]
        resume: Option<PathBuf>,
    },
}

/// REPL settings actions — get, set, list, reset inference parameters.
/// Same settings as the `/repl` slash command in interactive mode.
#[derive(Debug, Subcommand)]
pub enum SettingsAction {
    /// Show all settings (or a single setting if name is given)
    Show {
        #[arg()]
        name: Option<String>,
    },
    /// Set a setting value
    Set { name: String, value: String },
    /// Reset all settings to defaults
    Reset,
}

#[derive(Debug, Subcommand)]
pub enum WalletAction {
    /// Show rJoule balance with USDC and gas equivalents
    Balance {
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// Show or derive a deposit address for receiving USDC
    DepositAddress {
        /// Blockchain network (hinkal or hedera). Defaults to hinkal.
        #[arg(short, long)]
        chain: Option<String>,
        /// Use shielded/privacy mode (default behavior).
        #[arg(short, long, conflicts_with = "transparent")]
        private: bool,
        /// Opt out to transparent mode (public on-chain visibility).
        #[arg(long, conflicts_with = "private")]
        transparent: bool,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// Generate a one-time deposit reference for shielded deposits
    DepositReference {
        /// Blockchain network (hinkal or hedera)
        #[arg(short, long)]
        chain: String,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// Show paginated transaction history
    History {
        /// Maximum transactions to show
        #[arg(short, long)]
        limit: Option<u32>,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// API key management
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
    /// Estimate current network withdrawal fee for a chain
    Fee {
        /// Blockchain network (hinkal or hedera). Defaults to hinkal.
        #[arg(short, long)]
        chain: Option<String>,
    },
    /// Withdraw rJoules as USDC to an external address
    Withdraw {
        /// Amount in rJoules to withdraw
        amount_rj: u64,
        /// Destination address (Hedera 0.0.XXXXX)
        #[arg(short, long)]
        to: String,
        /// Blockchain network (hinkal or hedera). Defaults to hinkal.
        #[arg(short, long)]
        chain: Option<String>,
        /// Use shielded/privacy mode (default behavior).
        #[arg(short, long, conflicts_with = "transparent")]
        private: bool,
        /// Opt out to transparent mode (public on-chain visibility).
        #[arg(long, conflicts_with = "private")]
        transparent: bool,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// Allocate rJoules to an API key for spending
    Encumber {
        /// API key ID to allocate rJoules to
        #[arg(short, long)]
        key_id: String,
        /// Amount in rJoules to allocate
        #[arg(short, long)]
        amount: u64,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// Release an API key's encumbrance (returns unspent rJoules to wallet)
    ReleaseEncumbrance {
        /// API key ID to release
        #[arg(short, long)]
        key_id: String,
    },
    /// Show spending report for an API key
    Report {
        /// API key ID to report on
        #[arg(short, long)]
        key_id: String,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeyAction {
    /// Create a new API key with spending limit
    Create {
        /// Spending limit in rJoules
        #[arg(short, long)]
        limit: u64,
        /// Expiry in days (no expiry if omitted)
        #[arg(short, long)]
        expiry: Option<u32>,
        /// Restrict to shielded/privacy mode (default behavior).
        #[arg(short, long, conflicts_with = "transparent")]
        private: bool,
        /// Opt out to transparent mode (public on-chain visibility).
        #[arg(long, conflicts_with = "private")]
        transparent: bool,
        /// Restrict to a specific chain (defaults to hinkal)
        #[arg(short, long)]
        chain: Option<String>,
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// List active API keys
    List {
        /// Wallet ID (UUID). Defaults to system wallet if omitted.
        #[arg(short, long)]
        wallet: Option<String>,
    },
    /// Revoke an API key (returns unspent rJoules to wallet)
    Revoke {
        /// Key ID to revoke
        key_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum MatrixAction {
    /// Generate Docker sidecar files (docker-compose.yml, Caddyfile, conduit.toml)
    DeploySidecar {
        /// Domain name for the Matrix homeserver (e.g., matrix.example.com)
        #[arg(short, long)]
        domain: String,
        /// Also generate Hydrogen web client config
        #[arg(long)]
        with_web_client: bool,
        /// Output directory (default: ~/.config/hkask/sidecar)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Register a hKask agent on the Matrix homeserver
    RegisterAgent {
        /// Replicant name (e.g., "Alice-Smith")
        agent: String,
        /// Homeserver URL (default: http://localhost:8008)
        #[arg(short, long, default_value = "http://localhost:8008")]
        homeserver: String,
    },
    /// Create a human Matrix user account on the homeserver
    RegisterUser {
        /// Desired username (e.g., "Bob-Jones")
        user: String,
        /// Homeserver URL (default: http://localhost:8008)
        #[arg(short, long, default_value = "http://localhost:8008")]
        homeserver: String,
    },
    /// Check sidecar health (Docker containers, API, database)
    StatusSidecar,
}

#[derive(Subcommand, Debug, Clone)]
pub enum KanbanAction {
    /// Create a new kanban board
    BoardCreate {
        name: String,
        #[arg(short, long, default_value = "")]
        columns: Option<String>,
    },
    /// List all boards
    BoardList,
    /// View a board as a text-based column layout
    BoardView { board_id: String },
    /// Create a new task
    TaskCreate {
        board_id: String,
        title: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        criteria: Vec<String>,
        #[arg(short, long)]
        assign: Option<String>,
    },
    /// List tasks on a board
    TaskList {
        board_id: String,
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Show task details
    TaskShow { task_id: String },
    /// Move a task to a new column
    TaskMove { task_id: String, status: String },
    /// Assign a task to an agent
    TaskAssign { task_id: String, agent: String },
    /// Verify a task against acceptance criteria
    TaskVerify {
        task_id: String,
        #[arg(short, long)]
        evidence: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum DaemonAction {
    /// Start the daemon (binds Unix socket, runs CNS loops, serves until shutdown)
    Start,

    /// Check daemon status (socket existence + health)
    Status,

    /// Stop the daemon (sends shutdown signal via socket)
    Stop,
}

/// Trained adapter lifecycle — deploy, infer, teardown
#[derive(Debug, Subcommand)]
pub enum AdapterAction {
    /// List trained adapters (delegates to training MCP)
    List {
        #[arg(short, long)]
        skill: Option<String>,
    },
    /// Deploy an adapter to a cloud inference provider
    Deploy {
        /// Adapter name or ID
        adapter: String,
        /// Cloud provider (together, runpod, baseten)
        #[arg(short, long, default_value = "together")]
        provider: String,
    },
    /// Check deployment status
    Status {
        /// Deployment ID from deploy command
        deployment_id: String,
    },
    /// Tear down a deployed endpoint
    Teardown {
        /// Deployment ID to tear down
        deployment_id: String,
    },
}
