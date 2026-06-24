//! Domain-specific bridge traits for TUI windows.
//!
//! Each trait provides a focused surface (≤7 methods) for a single
//! service domain, following deep-module discipline. The CLI crate
//! implements these traits to wire live service data into the TUI.

pub mod backup;
pub mod companies;
pub mod config;
pub mod docproc;
pub mod kanban;
pub mod matrix;
pub mod media;
pub mod memory;
pub mod registry;
pub mod replica;
pub mod research;
pub mod skills;
pub mod training;
pub mod wallet;

pub use backup::BackupDataBridge;
pub use companies::CompaniesDataBridge;
pub use config::ConfigDataBridge;
pub use docproc::DocprocDataBridge;
pub use kanban::KanbanDataBridge;
pub use matrix::MatrixDataBridge;
pub use media::MediaDataBridge;
pub use memory::MemoryDataBridge;
pub use registry::RegistryDataBridge;
pub use replica::ReplicaDataBridge;
pub use research::ResearchDataBridge;
pub use skills::SkillsDataBridge;
pub use training::TrainingDataBridge;
pub use wallet::{WalletDataBridge, WalletTxSummary};
