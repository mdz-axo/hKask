//! Backup scope types — what to snapshot, restore, or list.
//! # REQ: P8 (Semantic Grounding) — every type encodes a distinct domain concept.
//! expect: "Backup scope types encode distinct domain concepts"

use serde::{Deserialize, Serialize};

/// Types of artifacts the backup system can track.
///
/// Each variant corresponds to a [`hkask_ports::git_cas::RepoId`]
/// for storage routing. The mapping is:
/// - `Template`, `Style`, `RegistryEntry` → `RepoId::Registry`
/// - `Goal`, `Spec` → `RepoId::GoalsSpecs`
/// - `MemoryTriple`, `Embedding` → `RepoId::Memory`
/// - `CnsAudit` → `RepoId::CnsAudit`
/// - `SovereigntyManifest` → `RepoId::Sovereignty`
/// - `Session` → `RepoId::Sessions`
/// - `WalletState` → `RepoId::Vault`
/// - `Settings` → `RepoId::Registry` (stored alongside templates)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Template,
    Style,
    Goal,
    Spec,
    MemoryTriple,
    Embedding,
    RegistryEntry,
    CnsAudit,
    SovereigntyManifest,
    Session,
    WalletState,
    Settings,
    /// Agent public artifacts — styles, bots, templates published via manifest.json.
    AgentArtifact,
    /// Agent gallery — media assets (images, video, audio).
    AgentGallery,
    /// Agent library — research materials, papers, feeds.
    AgentLibrary,
    /// Agent documents — parsed/extracted documents.
    AgentDocuments,
    /// Agent adapters — trained LoRA weights.
    AgentAdapters,
}

impl ArtifactType {
    /// Map this artifact type to its storage repository.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid ArtifactType variant
    /// post: returns the corresponding RepoId for storage routing
    pub fn repo_id(&self) -> hkask_ports::git_cas::RepoId {
        use hkask_ports::git_cas::RepoId;
        match self {
            ArtifactType::Template
            | ArtifactType::Style
            | ArtifactType::RegistryEntry
            | ArtifactType::Settings
            | ArtifactType::AgentArtifact
            | ArtifactType::AgentGallery
            | ArtifactType::AgentLibrary
            | ArtifactType::AgentDocuments
            | ArtifactType::AgentAdapters => RepoId::Registry,
            ArtifactType::Goal | ArtifactType::Spec => RepoId::GoalsSpecs,
            ArtifactType::MemoryTriple | ArtifactType::Embedding => RepoId::Memory,
            ArtifactType::CnsAudit => RepoId::CnsAudit,
            ArtifactType::SovereigntyManifest => RepoId::Sovereignty,
            ArtifactType::Session => RepoId::Sessions,
            ArtifactType::WalletState => RepoId::Vault,
        }
    }

    /// Human-readable label for CLI/API display.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid ArtifactType variant
    /// post: returns &'static str label (e.g., "template", "goal", "spec")
    pub fn label(&self) -> &'static str {
        match self {
            ArtifactType::Template => "template",
            ArtifactType::Style => "style",
            ArtifactType::Goal => "goal",
            ArtifactType::Spec => "spec",
            ArtifactType::MemoryTriple => "memory_triple",
            ArtifactType::Embedding => "embedding",
            ArtifactType::RegistryEntry => "registry_entry",
            ArtifactType::CnsAudit => "cns_audit",
            ArtifactType::SovereigntyManifest => "sovereignty_manifest",
            ArtifactType::Session => "session",
            ArtifactType::WalletState => "wallet_state",
            ArtifactType::Settings => "settings",
            ArtifactType::AgentArtifact => "agent_artifact",
            ArtifactType::AgentGallery => "agent_gallery",
            ArtifactType::AgentLibrary => "agent_library",
            ArtifactType::AgentDocuments => "agent_documents",
            ArtifactType::AgentAdapters => "agent_adapters",
        }
    }
}

impl std::fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// What to include in a backup snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackupScope {
    /// Snapshot all tracked artifact types.
    Full,
    /// Snapshot all artifacts of a single type.
    ByType(ArtifactType),
    /// Snapshot specific artifacts by ID within a type.
    ByIds {
        artifact_type: ArtifactType,
        ids: Vec<String>,
    },
}

impl BackupScope {
    /// Human-readable description for commit messages.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid BackupScope variant
    /// post: returns String description (e.g., "full backup", "backup: template", "backup: template (id1, id2)")
    pub fn description(&self) -> String {
        match self {
            BackupScope::Full => "full backup".to_string(),
            BackupScope::ByType(t) => format!("backup: {}", t.label()),
            BackupScope::ByIds { artifact_type, ids } => {
                format!("backup: {} ({})", artifact_type.label(), ids.join(", "))
            }
        }
    }
}

/// What to restore from a backup snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestoreScope {
    /// Restore all artifacts from the snapshot.
    Full,
    /// Restore all artifacts of a single type.
    ByType(ArtifactType),
    /// Restore specific artifacts by ID within a type.
    ByIds {
        artifact_type: ArtifactType,
        ids: Vec<String>,
    },
}

/// Filter for listing backup snapshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListFilter {
    /// Filter by artifact type (None = all types).
    pub artifact_type: Option<ArtifactType>,
    /// Maximum number of snapshots to return.
    pub limit: Option<usize>,
}
