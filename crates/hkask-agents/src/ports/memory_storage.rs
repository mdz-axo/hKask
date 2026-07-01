//! Memory Storage Ports — Re-export shim
//!
//! These port traits have been promoted to `hkask-memory` under ADR-042
//! (port promotion rule). This module re-exports them for backward
//! compatibility. New code should import from `hkask_memory::ports`.

pub use hkask_memory::ports::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};

// Conv re-export so old `use crate::ports::memory_storage::RecalledEpisode` works.
// Can be removed once all internal callers migrate to `hkask_memory::ports`.
