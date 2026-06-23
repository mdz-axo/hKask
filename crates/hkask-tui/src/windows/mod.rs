//! Window implementations for the hKask TUI.
//!
//! Each window implements the `Window` trait and renders a specific
//! hKask subsystem: chat, CNS monitor, backup, registry, matrix, pods,
//! kanban, energy, settings, and sidebar.

pub mod chat;
pub mod logo;
pub mod sidebar;

pub use chat::ChatWindow;
pub use logo::LogoWindow;
pub use sidebar::SidebarWindow;
