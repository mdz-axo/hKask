//! Window trait — the interface every sub-window implements.
//!
//! Modelled on Zed's Item/View architecture: each window has its own
//! state, render function, and event handler. The workspace manages
//! layout, focus, and event routing.

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
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
    /// Matrix protocol rooms and messages
    Matrix,
    /// Pod status — CuratorPod, TeamPods, ReplicantPods
    Pods,
    /// Kanban board view
    Kanban,
    /// Energy/gas usage analytics
    Energy,
    /// Settings editor (ReplSettings)
    Settings,
    /// Composite sidebar — CNS summary + MCP status + context gauge + pod list
    Sidebar,
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
            WindowKind::Energy => "Energy",
            WindowKind::Settings => "Settings",
            WindowKind::Sidebar => "Sidebar",
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
            WindowKind::Energy => "Gas usage, energy budget, and cost analytics",
            WindowKind::Settings => "Edit REPL inference settings",
            WindowKind::Sidebar => "Composite sidebar with CNS, MCP, and pod status",
        }
    }

    /// Whether this window kind can have multiple instances.
    pub fn allows_multiple(&self) -> bool {
        match self {
            WindowKind::Chat => true,
            WindowKind::Matrix => true,
            WindowKind::Sidebar => false,
            _ => false,
        }
    }

    /// Whether this window is persistent (cannot be closed by user).
    pub fn is_persistent(&self) -> bool {
        matches!(self, WindowKind::Sidebar)
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
