//! Domain-specific bridge traits for TUI windows.
//!
//! Each trait provides a focused surface (≤7 methods) for a single
//! service domain, following deep-module discipline. The CLI crate
//! implements these traits to wire live service data into the TUI.

pub mod backup;
pub mod config;
pub mod kanban;
pub mod memory;
pub mod registry;
pub mod wallet;

pub use backup::BackupDataBridge;
pub use config::ConfigDataBridge;
pub use kanban::KanbanDataBridge;
pub use memory::MemoryDataBridge;
pub use registry::RegistryDataBridge;
pub use wallet::{WalletDataBridge, WalletTxSummary};
