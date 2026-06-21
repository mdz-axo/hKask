//! Git CAS value types — content hashes, commit hashes, repo identifiers, and tree/diff types.
//!
//! Content is addressed by BLAKE3 hash. Snapshots are git commits.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Value Types ──────────────────────────────────────────────────────────────

/// BLAKE3 content hash — 32 bytes, displayed as hex.
///
/// Addresses blob content within a CAS repository. Produced by
/// [`ContentHash::from_blake3`] which wraps the crate's BLAKE3 hashing helper.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Compute a BLAKE3 content hash from arbitrary data.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  data is any byte slice (including empty)
    /// post: returns a [`ContentHash`] containing the 32-byte BLAKE3 digest of data;
    ///       same data → same hash (deterministic)
    pub fn from_blake3(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    /// Return the raw 32-byte hash.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any valid [`ContentHash`]
    /// post: returns a reference to the inner 32-byte array unchanged
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", hex::encode(self.0))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl std::str::FromStr for ContentHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("expected 32 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Git commit SHA — 20 bytes, displayed as hex.
///
/// Addresses a snapshot commit within a CAS repository.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitHash(pub [u8; 20]);

impl CommitHash {
    /// Create from a raw 20-byte SHA.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  bytes is any 20-byte array
    /// post: returns a [`CommitHash`] wrapping the given bytes unchanged
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Return the raw 20-byte SHA.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any valid [`CommitHash`]
    /// post: returns a reference to the inner 20-byte array unchanged
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// The null commit hash (all zeros), used as a sentinel for "no parent".
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  (no inputs)
    /// post: returns a [`CommitHash`] with all 20 bytes set to zero
    pub fn null() -> Self {
        Self([0u8; 20])
    }
}

impl fmt::Debug for CommitHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommitHash({})", hex::encode(self.0))
    }
}

impl fmt::Display for CommitHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl std::str::FromStr for CommitHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 20 {
            return Err(format!("expected 20 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Repository identifier — one of the 7 snapshot repos.
///
/// Each variant names a distinct git repository that stores a specific
/// category of hKask state. Repos are isolated from each other.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepoId {
    /// Agent registry (templates, personas, dispatch manifests)
    Registry,
    /// Semantic memory (triples, knowledge graph)
    Memory,
    /// CNS audit trail (ν-events, variety counters, algedonic alerts)
    CnsAudit,
    /// User sovereignty (consent records, OCAP tokens)
    Sovereignty,
    /// Goals and specifications
    GoalsSpecs,
    /// Standing sessions (conversation history)
    Sessions,
    /// Vault (encrypted master key material)
    Vault,
}

impl RepoId {
    /// Return the directory name used for this repo on disk.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`RepoId`] variant
    /// post: returns a `&'static str` directory name; each variant maps to a distinct name;
    ///       never panics
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::Memory => "memory",
            Self::CnsAudit => "cns-audit",
            Self::Sovereignty => "sovereignty",
            Self::GoalsSpecs => "goals-specs",
            Self::Sessions => "sessions",
            Self::Vault => "vault",
        }
    }

    /// Iterate all 7 repo variants.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  (no inputs)
    /// post: returns a static slice containing all 7 [`RepoId`] variants exactly once;
    ///       order is stable across calls
    pub fn all() -> &'static [RepoId] {
        &[
            RepoId::Registry,
            RepoId::Memory,
            RepoId::CnsAudit,
            RepoId::Sovereignty,
            RepoId::GoalsSpecs,
            RepoId::Sessions,
            RepoId::Vault,
        ]
    }
}

// ── CAS Domain Types ─────────────────────────────────────────────────────────

/// A file entry in a git tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// File path relative to repo root.
    pub path: String,
    /// BLAKE3 content hash of the file content.
    pub content_hash: ContentHash,
    /// Whether this is a blob (file) or tree (directory).
    pub kind: TreeEntryKind,
}

/// Kind of tree entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreeEntryKind {
    Blob,
    Tree,
}

/// A file diff between two commits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// File path relative to repo root.
    pub path: String,
    /// Kind of change.
    pub kind: DiffKind,
    /// Unified diff content.
    pub content: String,
}

/// Kind of file change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
}
