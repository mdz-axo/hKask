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
}

#[derive(Subcommand)]
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

#[derive(Subcommand)]
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

#[derive(Subcommand)]
pub enum SovereigntyAction {
    Status,
    Grant {
        #[arg(long)]
        category: String,
    },
    Revoke {
        #[arg(long)]
        category: String,
    },
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

#[derive(Subcommand)]
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
#[derive(Subcommand)]
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

/// Ensemble multi-agent actions (Phase 7)
#[derive(Subcommand)]
pub enum EnsembleAction {
    ChatCreate {
        #[arg(short, long)]
        session: String,
    },
    ChatRegister {
        #[arg(short, long)]
        session: String,
        #[arg(short, long)]
        bot: String,
        #[arg(short, long)]
        role: String,
    },
    ChatSend {
        #[arg(short, long)]
        session: String,
        #[arg(short, long)]
        message: String,
    },
    ChatList,
    DeliberationCreate {
        #[arg(short, long)]
        session: String,
    },
    DeliberationStart {
        #[arg(short, long)]
        session: String,
    },
    DeliberationRecord {
        #[arg(short, long)]
        session: String,
        #[arg(short, long)]
        agent: String,
        #[arg(short, long)]
        content: String,
        #[arg(short, long)]
        confidence: f64,
    },
    DeliberationSynthesize {
        #[arg(short, long)]
        session: String,
    },
    DeliberationList,
    StandingStart {
        #[arg(
            short,
            long,
            default_value = "registry/manifests/standing-ensemble-session.yaml"
        )]
        config: PathBuf,
    },
    StandingStatus,
}

/// Curator governance actions
#[derive(Subcommand)]
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
}

/// Replicant identity actions
#[derive(Subcommand)]
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
}

#[derive(Subcommand)]
pub enum KeystoreAction {
    Load {
        #[arg(short, long, default_value = ".env")]
        path: PathBuf,
        #[arg(short = 'x', long, default_value = "HKASK_")]
        prefix: String,
        #[arg(long)]
        overwrite: bool,
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
}

/// Specification actions (MDS)
#[derive(Subcommand)]
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
        id: String,
    },
    Cultivate {
        #[arg(short, long)]
        id: String,
    },
    Render {
        #[arg()]
        template: String,
        #[arg(short, long)]
        spec_id: Option<String>,
    },
    TestInvariant {
        #[arg(short, long)]
        spec_id: String,
        #[arg(short, long)]
        seam: String,
        #[arg(short, long)]
        invariant: String,
        #[arg(short, long, default_value = "PublicInterface")]
        category: String,
        #[arg(short, long)]
        cycle: Option<String>,
    },
    TestVerify {
        #[arg(short, long)]
        seam: Option<String>,
        #[arg(short, long)]
        category: Option<String>,
    },
}

/// Style composition — generate prose with exemplar retrieval and centroid validation
#[derive(Subcommand)]
pub enum ComposeAction {
    Run {
        #[arg(short, long)]
        prompt: String,
        #[arg(short, long)]
        cognition: PathBuf,
        #[arg(short, long)]
        db: PathBuf,
        #[arg(long, env = "HKASK_DB_PASSPHRASE")]
        passphrase: String,
        #[arg(long)]
        okapi_url: Option<String>,
        #[arg(long)]
        no_validate: bool,
    },
}

/// Style corpus embedding — download, chunk, embed, store
#[derive(Subcommand)]
pub enum EmbedCorpusAction {
    Run {
        #[arg(short, long)]
        config: PathBuf,
        #[arg(short, long)]
        db: PathBuf,
        #[arg(long, env = "HKASK_DB_PASSPHRASE")]
        passphrase: String,
        #[arg(long)]
        okapi_url: Option<String>,
    },
}

/// Skill bundle management actions
#[derive(Subcommand)]
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
#[derive(Subcommand)]
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
#[derive(Subcommand)]
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
}

/// REPL settings actions — get, set, list, reset inference parameters.
/// Same settings as the `/repl` slash command in interactive mode.
#[derive(Subcommand)]
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
