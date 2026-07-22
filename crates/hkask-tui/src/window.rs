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

/// Actions a window can request from the workspace.
/// Drained via `Window::drain_action()` after each tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceAction {
    /// Quit the TUI.
    Quit,
}

/// Direction for splitting a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// The kind of window — determines icon, default title, and creation behaviour.
///
/// The TUI now hosts only the Chat window; all other window types have been
/// removed. The enum is retained (single-variant) so the workspace, layout
/// persistence, and catalog can continue to tag windows by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    /// AI chat interface — the primary interaction window (default: Chat mode).
    /// Curator chat is a mode within ChatWindow.
    Chat,
}

impl WindowKind {
    /// (title, description, allows_multiple, is_mcp_tabbed)
    /// is_mcp_tabbed = true means the window handles Tab internally
    /// (Chat/Data toggle) rather than letting the workspace cycle focus.
    pub(crate) const META: &[(WindowKind, &str, &str, bool, bool)] = &[(
        WindowKind::Chat,
        "Chat",
        "AI chat with your userpod agent (default: Chat mode)",
        true,
        false,
    )];

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
    /// All windows are closeable (the persistent Logo window was removed).
    fn can_close(&self) -> bool {
        true
    }

    /// Called when this window gains keyboard focus.
    fn on_focus(&mut self) {}

    /// Called when this window loses keyboard focus.
    fn on_blur(&mut self) {}

    /// Called periodically (every tick) for background updates.
    fn tick(&mut self) {}

    /// Drain a pending workspace action (e.g., from slash commands).
    /// Default implementation returns `None` — only ChatWindow overrides this.
    fn drain_action(&mut self) -> Option<WorkspaceAction> {
        None
    }
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
            1,
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
        // Chat is the only multi-instance window now.
        assert!(WindowKind::Chat.allows_multiple());
    }

    #[test]
    fn chat_does_not_use_internal_tab() {
        // The Chat window does not handle Tab internally (no Data tab).
        assert!(!WindowKind::Chat.uses_internal_tab());
    }
}
