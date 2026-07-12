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
pub mod scenarios;
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
pub use scenarios::{EventNode, EventTreeDetail, ScenariosDataBridge};
pub use skills::SkillsDataBridge;
pub use training::TrainingDataBridge;
pub use wallet::{WalletDataBridge, WalletTxSummary};

// ── Bridge generation macros ──────────────────────────────────────────
//
// `with_bridges!` takes a sub-macro name and 13 bridge specs, then invokes
// the sub-macro for each. Type names are resolved at the call site (where
// the bridge traits are in scope), avoiding macro-definition-site hygiene
// issues with the callback-pattern approach.

/// Invoke `$sub!` for each bridge spec.
/// Each spec: `$field, $trait, $method`
macro_rules! with_bridges {
    ($sub:ident;
     $($field:ident, $trait:ident, $method:ident);+ $(;)?
    ) => {
        $($sub!($field, $trait, $method);)*
    };
}

/// Generate a `Workspace::with_*` setter method.
macro_rules! workspace_bridge_setter {
    ($field:ident, $trait:ident, $method:ident) => {
        pub fn $method(&mut self, bridge: std::sync::Arc<dyn $trait>) -> &mut Self {
            self.bridges.$field = Some(bridge);
            self
        }
    };
}

/// Generate a `TuiSession::with_*` setter method.
macro_rules! tui_bridge_setter {
    ($field:ident, $trait:ident, $method:ident) => {
        pub fn $method(mut self, bridge: std::sync::Arc<dyn $trait>) -> Self {
            self.workspace.$method(bridge);
            self
        }
    };
}

pub(crate) use {tui_bridge_setter, with_bridges, workspace_bridge_setter};
