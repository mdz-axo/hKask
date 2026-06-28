//! Window trait — the interface every sub-window implements.
//!
//! Modelled on Zed's Item/View architecture: each window has its own
//! state, render function, and event handler. The workspace manages
//! layout, focus, and event routing.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use uuid::Uuid;

/// Unique identifier for a window instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub Uuid);

/// The kind of window — determines icon, default title, and creation behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    /// AI chat interface — the primary interaction window
    Chat,
    /// CNS health monitor — variety counters, alerts, span trace
    CnsMonitor,
    /// Backup operations — snapshots, restore, verify, prune
    Backup,
    /// Registry browser — templates, skills, styles, bundles
    Registry,
    /// Pod status — CuratorPod, TeamPods, ReplicantPods
    Pods,
    /// Kanban board view
    Kanban,
    /// Wallet — gas, rJoule balance, transactions
    Wallet,
    /// Memory — episodic/semantic memory browser
    Memory,
    /// Companies — organization data and entities
    Companies,
    /// Matrix chat — federated messaging rooms
    Matrix,
    /// Settings editor (ReplSettings)
    Configuration,

    /// Curator daemon window — P12.1 dual-presence
    Curator,
    /// Embedded terminal — run shell commands
    Terminal,
    /// Text editor — edit configs, agent YAML, etc.
    Editor,
    /// Training monitor — LoRA adapters, sessions, artifacts
    Training,
    /// Media gallery — images, audio, video collections
    Media,
    /// Skills manager — browse and manage skill corpus
    Skills,
    /// Research — web search, RSS feeds, content extraction
    Research,
    /// Docproc — document processing, chunking, QA, RDF extraction
    Docproc,
    /// Replica — authorial style replica management
    Replica,
    /// Logo — persistent Kask amphora logo window
    Logo,
}

impl WindowKind {
    /// Default display title for this window kind.
    pub fn default_title(&self) -> &'static str {
        match self {
            WindowKind::Chat => "Chat",
            WindowKind::CnsMonitor => "CNS Monitor",
            WindowKind::Backup => "Backup",
            WindowKind::Registry => "Registry",
            WindowKind::Matrix => "Matrix",
            WindowKind::Pods => "Pods",
            WindowKind::Kanban => "Kanban",
            WindowKind::Wallet => "Wallet",
            WindowKind::Memory => "Memory",
            WindowKind::Companies => "Companies",
            WindowKind::Configuration => "Configuration",
            WindowKind::Curator => "Curator",
            WindowKind::Terminal => "Terminal",
            WindowKind::Editor => "Editor",
            WindowKind::Training => "Training",
            WindowKind::Media => "Media",
            WindowKind::Skills => "Skills",
            WindowKind::Research => "Research",
            WindowKind::Docproc => "Docproc",
            WindowKind::Replica => "Replica",
            WindowKind::Logo => "hKask",
        }
    }

    /// Short description for the command palette.
    pub fn description(&self) -> &'static str {
        match self {
            WindowKind::Chat => "AI chat with your replicant agent",
            WindowKind::CnsMonitor => "Cybernetic nervous system health and alerts",
            WindowKind::Backup => "Snapshot, restore, verify, and prune backups",
            WindowKind::Registry => "Browse templates, skills, styles, and bundles",
            WindowKind::Matrix => "Matrix protocol rooms and federated messages",
            WindowKind::Pods => "Pod deployment status and agent inventory",
            WindowKind::Kanban => "Kanban board for task coordination",
            WindowKind::Wallet => "Gas budget, rJoule balance, and transactions",
            WindowKind::Memory => "Browse and edit episodic and semantic memories",
            WindowKind::Companies => "Company profiles, people, and relationships",
            WindowKind::Configuration => "Edit REPL inference and system settings",
            WindowKind::Curator => "Curator daemon — CNS alerts, memory, and direct chat",
            WindowKind::Terminal => "Embedded shell — run commands from within the TUI",
            WindowKind::Editor => "Text editor — edit configs, agent YAML, and scripts",
            WindowKind::Training => "Training monitor — LoRA adapters, sessions, and artifacts",
            WindowKind::Media => "Media gallery — browse images, audio, and video collections",
            WindowKind::Skills => "Skills manager — browse, install, and activate skills",
            WindowKind::Research => "Web search, RSS feeds, and content extraction",
            WindowKind::Docproc => "Document processing: chunk, QA, RDF, embeddings",
            WindowKind::Replica => "Authorial style replicas — build, compare, generate",
            WindowKind::Logo => "Kask amphora logo — workspace identity marker",
        }
    }

    /// Whether this window kind can have multiple instances.
    pub fn allows_multiple(&self) -> bool {
        match self {
            WindowKind::Chat => true,
            WindowKind::Matrix => true,
            WindowKind::Logo => false,
            _ => false,
        }
    }

    /// Whether this window is persistent (cannot be closed by user).
    pub fn is_persistent(&self) -> bool {
        matches!(self, WindowKind::Logo)
    }
}

/// Every sub-window implements this trait.
///
/// This is the single interface the workspace uses to render and
/// interact with windows. The trait is object-safe so windows can
/// be stored heterogeneously.
pub trait Window {
    /// Unique identifier for this window instance.
    fn id(&self) -> WindowId;

    /// Display title shown in the window border and tab bar.
    fn title(&self) -> &str;

    /// The kind of window.
    fn kind(&self) -> WindowKind;

    /// Render this window into the given rectangle.
    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool);

    /// Handle a key event. Return `true` if the event was consumed.
    fn handle_key(&mut self, key: KeyEvent) -> bool;

    /// Whether this window can be closed by the user.
    fn can_close(&self) -> bool {
        !self.kind().is_persistent()
    }

    /// Called when this window gains keyboard focus.
    fn on_focus(&mut self) {}

    /// Called when this window loses keyboard focus.
    fn on_blur(&mut self) {}

    /// Called periodically (every tick) for background updates.
    fn tick(&mut self) {}
}
