//! Git CAS Port — Hexagonal boundary for content-addressable git storage
//!
//! Defines the trait and value types for a content-addressed git storage system.
//! Content is addressed by BLAKE3 hash. Snapshots are git commits.
//!
//! Each method operates on a named repository (`RepoId`), providing isolation
//! between the 7 snapshot repos.

pub mod error;
pub mod port;
pub mod snapshot;
pub mod types;

pub use error::GitCasError;
pub use port::{GitCASPort, GitCasVerificationReport, LogEntry, MockGitCas};
pub use snapshot::{CasRetentionPolicy, CasRetentionTier, RepoSnapshotPolicy, TripleEntry};
pub use types::{
    CommitHash, ContentHash, DiffKind, FileDiff, ParseHashError, RepoId, TreeEntry, TreeEntryKind,
};
