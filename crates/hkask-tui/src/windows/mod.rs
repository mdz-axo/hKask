//! Window implementations for the hKask TUI.

pub mod backup;
pub mod chat;
pub mod companies;
pub mod configuration;
pub mod docproc;
pub mod editor;
pub mod kanban;
pub mod matrix;
pub mod media;
pub mod memory;
pub mod pods;
pub mod reg_monitor;
pub mod registry;
pub mod replica;
pub mod research;
pub mod scenarios;

pub mod skills;
pub mod terminal;
pub mod training;
pub mod wallet;

pub use backup::BackupWindow;
pub use chat::ChatWindow;
pub use companies::CompaniesWindow;
pub use configuration::ConfigurationWindow;
pub use docproc::DocprocWindow;
pub use editor::EditorWindow;
pub use kanban::KanbanWindow;
pub use matrix::MatrixWindow;
pub use media::MediaWindow;
pub use memory::MemoryWindow;
pub use pods::PodsWindow;
pub use reg_monitor::CnsMonitorWindow;
pub use registry::RegistryWindow;
pub use replica::ReplicaWindow;
pub use research::ResearchWindow;
pub use scenarios::ScenariosWindow;

pub use skills::SkillsWindow;
pub use terminal::TerminalWindow;
pub use training::TrainingWindow;
pub use wallet::WalletWindow;
