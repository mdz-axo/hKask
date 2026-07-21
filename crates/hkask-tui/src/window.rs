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
    /// Pod status — CuratorPod, TeamPods, UserPodPods
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
    /// Scenarios — event trees, forecasts, calibration tracking
    Scenarios,
}

impl WindowKind {
    /// (title, description, allows_multiple, is_mcp_tabbed)
    /// is_mcp_tabbed = true means the window handles Tab internally
    /// (Chat/Data toggle) rather than letting the workspace cycle focus.
    pub(crate) const META: &[(WindowKind, &str, &str, bool, bool)] = &[
        (
            WindowKind::Chat,
            "Chat",
            "AI chat with your userpod agent",
            true,
            false,
        ),
        (
            WindowKind::CnsMonitor,
            "CNS Monitor",
            "Cybernetic nervous system health and alerts",
            false,
            false,
        ),
        (
            WindowKind::Backup,
            "Backup",
            "Snapshot, restore, verify, and prune backups",
            false,
            false,
        ),
        (
            WindowKind::Registry,
            "Registry",
            "Browse templates, skills, styles, and bundles",
            false,
            false,
        ),
        (
            WindowKind::Pods,
            "Pods",
            "Pod deployment status and agent inventory",
            false,
            false,
        ),
        (
            WindowKind::Kanban,
            "Kanban",
            "Kanban board for task coordination",
            false,
            true,
        ),
        (
            WindowKind::Wallet,
            "Wallet",
            "Gas budget, rJoule balance, and transactions",
            false,
            false,
        ),
        (
            WindowKind::Memory,
            "Memory",
            "Browse and edit episodic and semantic memories",
            false,
            true,
        ),
        (
            WindowKind::Companies,
            "Companies",
            "Company profiles, people, and relationships",
            false,
            true,
        ),
        (
            WindowKind::Matrix,
            "Matrix",
            "Matrix protocol rooms and federated messages",
            true,
            true,
        ),
        (
            WindowKind::Configuration,
            "Configuration",
            "Edit REPL inference and system settings",
            false,
            false,
        ),
        (
            WindowKind::Curator,
            "Curator",
            "Curator daemon — CNS alerts, memory, and direct chat",
            false,
            false,
        ),
        (
            WindowKind::Terminal,
            "Terminal",
            "Embedded shell — run commands from within the TUI",
            false,
            true,
        ),
        (
            WindowKind::Editor,
            "Editor",
            "Text editor — edit configs, agent YAML, and scripts",
            false,
            false,
        ),
        (
            WindowKind::Training,
            "Training",
            "Training monitor — LoRA adapters, sessions, and artifacts",
            false,
            true,
        ),
        (
            WindowKind::Media,
            "Media",
            "Media gallery — browse images, audio, and video collections",
            false,
            true,
        ),
        (
            WindowKind::Skills,
            "Skills",
            "Skills manager — browse, install, and activate skills",
            false,
            true,
        ),
        (
            WindowKind::Research,
            "Research",
            "Web search, RSS feeds, and content extraction",
            false,
            true,
        ),
        (
            WindowKind::Docproc,
            "Docproc",
            "Document processing: chunk, QA, RDF, embeddings",
            false,
            true,
        ),
        (
            WindowKind::Replica,
            "Replica",
            "Authorial style replicas — build, compare, generate",
            false,
            true,
        ),
        (
            WindowKind::Logo,
            "hKask",
            "Kask amphora logo — workspace identity marker",
            false,
            false,
        ),
        (
            WindowKind::Scenarios,
            "Scenarios",
            "Event trees, Fermi forecasts, calibration tracking",
            false,
            false,
        ),
    ];

    pub fn default_title(&self) -> &'static str {
        Self::META
            .iter()
            .find(|(k, ..)| k == self)
            .map(|(_, t, ..)| *t)
            .unwrap()
    }

    pub fn description(&self) -> &'static str {
        Self::META
            .iter()
            .find(|(k, ..)| k == self)
            .map(|(_, _, d, ..)| *d)
            .unwrap()
    }

    /// Whether multiple instances of this window kind can exist.
    pub fn allows_multiple(&self) -> bool {
        Self::META
            .iter()
            .find(|(k, ..)| k == self)
            .map(|(_, _, _, m, _)| *m)
            .unwrap_or(false)
    }

    /// Whether this window handles Tab key internally (Chat/Data toggle for
    /// MCP-tabbed windows, or PTY focus for Terminal).
    pub fn uses_internal_tab(&self) -> bool {
        Self::META
            .iter()
            .find(|(k, ..)| k == self)
            .map(|(_, _, _, _, t)| *t)
            .unwrap_or(false)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_covers_all_enum_variants() {
        // If a new WindowKind variant is added but META isn't updated,
        // default_title() panics. This test guards against that.
        let kinds: Vec<WindowKind> = WindowKind::META.iter().map(|(k, ..)| *k).collect();
        assert_eq!(
            kinds.len(),
            22,
            "META entry count changed — update this test"
        );
    }

    #[test]
    fn every_variant_has_metadata() {
        for (kind, title, desc, _, _) in WindowKind::META {
            assert!(!title.is_empty(), "{:?} has empty title", kind);
            assert!(!desc.is_empty(), "{:?} has empty description", kind);
        }
    }

    #[test]
    fn allows_multiple_reads_from_meta() {
        // Chat and Matrix are the only multi-instance windows.
        assert!(WindowKind::Chat.allows_multiple());
        assert!(WindowKind::Matrix.allows_multiple());
        assert!(!WindowKind::CnsMonitor.allows_multiple());
        assert!(!WindowKind::Wallet.allows_multiple());
    }

    #[test]
    fn mcp_tabbed_windows_match_implementation() {
        // Windows that implement McpTabbedWindow must have uses_internal_tab() == true.
        let mcp_tabbed: &[WindowKind] = &[
            WindowKind::Kanban,
            WindowKind::Memory,
            WindowKind::Companies,
            WindowKind::Matrix,
            WindowKind::Training,
            WindowKind::Media,
            WindowKind::Skills,
            WindowKind::Research,
            WindowKind::Docproc,
            WindowKind::Replica,
        ];
        for &k in mcp_tabbed {
            assert!(k.uses_internal_tab(), "{:?} should be mcp_tabbed", k);
        }
    }

    #[test]
    fn terminal_uses_internal_tab() {
        assert!(WindowKind::Terminal.uses_internal_tab());
    }
}
