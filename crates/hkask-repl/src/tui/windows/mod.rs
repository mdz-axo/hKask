//! Window implementations for the hKask TUI.

pub mod chat;
pub mod companies;
pub mod kanban;
pub mod mcp_scoped;
pub mod scenarios;

pub use chat::ChatWindow;
pub use companies::CompaniesWindow;
pub use kanban::KanbanWindow;
pub use scenarios::ScenariosWindow;
