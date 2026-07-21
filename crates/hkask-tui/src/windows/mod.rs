//! Window implementations for the hKask TUI.

pub mod chat;
pub mod companies;
pub mod configuration;
pub mod docproc;
pub mod editor;
pub mod kanban;
pub mod matrix;
pub mod media;
pub mod memory;
pub mod replica;
pub mod research;
pub mod scenarios;

pub mod skills;
pub mod terminal;
pub mod training;
pub mod wallet;

pub use chat::ChatWindow;
pub use companies::CompaniesWindow;
pub use configuration::ConfigurationWindow;
pub use docproc::DocprocWindow;
pub use editor::EditorWindow;
pub use kanban::KanbanWindow;
pub use matrix::MatrixWindow;
pub use media::MediaWindow;
pub use memory::MemoryWindow;
pub use replica::ReplicaWindow;
pub use research::ResearchWindow;
pub use scenarios::ScenariosWindow;

pub use skills::SkillsWindow;
pub use terminal::TerminalWindow;
pub use training::TrainingWindow;
pub use wallet::WalletWindow;
