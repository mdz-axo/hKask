//! Window implementations for the hKask TUI.

pub mod backup;
pub mod chat;
pub mod cns_monitor;
pub mod configuration;
pub mod curator;
pub mod editor;
pub mod energy;
pub mod matrix;
pub mod media;
pub mod pods;
pub mod registry;
pub mod sidebar;
pub mod skills;
pub mod terminal;
pub mod training;

pub use backup::BackupWindow;
pub use chat::ChatWindow;
pub use cns_monitor::CnsMonitorWindow;
pub use configuration::ConfigurationWindow;
pub use curator::CuratorWindow;
pub use editor::EditorWindow;
pub use energy::EnergyWindow;
pub use matrix::MatrixWindow;
pub use media::MediaWindow;
pub use pods::PodsWindow;
pub use registry::RegistryWindow;
pub use sidebar::SidebarWindow;
pub use skills::SkillsWindow;
pub use terminal::TerminalWindow;
pub use training::TrainingWindow;
