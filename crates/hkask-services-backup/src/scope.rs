//! Backup scope types — what to snapshot, restore, or list.
//! # REQ: P8 (Semantic Grounding) — every type encodes a distinct domain concept.
//! expect: "Backup scope types encode distinct domain concepts"

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, IntoStaticStr};

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
/// - `PodState` → `RepoId::Pods` (pod.db snapshots for revert/spawn_agent)
///
/// Label ↔ variant mapping is derived from `strum` — no duplicate `match` arms.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display, IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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
    /// Agent pod state — pod.db snapshot for revert/spawn_agent operations.
    PodState,
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
            ArtifactType::PodState => RepoId::Pods,
        }
    }

    /// Human-readable label for CLI/API display.
    ///
    /// Uses `strum::IntoStaticStr` — single source of truth, no duplicate match arms.
    /// pre:  self must be a valid ArtifactType variant
    /// post: returns &'static str label (e.g., "template", "pod_state")
    pub fn label(&self) -> &'static str {
        self.into()
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
            BackupScope::ByType(t) => format!("backup: {}", t),
            BackupScope::ByIds { artifact_type, ids } => {
                format!("backup: {} ({})", artifact_type, ids.join(", "))
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

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn strum_roundtrip_all_variants() {
        // Every variant roundtrips through strum Display → FromStr
        let variants = [
            ArtifactType::Template,
            ArtifactType::Style,
            ArtifactType::Goal,
            ArtifactType::Spec,
            ArtifactType::MemoryTriple,
            ArtifactType::Embedding,
            ArtifactType::RegistryEntry,
            ArtifactType::CnsAudit,
            ArtifactType::SovereigntyManifest,
            ArtifactType::Session,
            ArtifactType::WalletState,
            ArtifactType::Settings,
            ArtifactType::AgentArtifact,
            ArtifactType::AgentGallery,
            ArtifactType::AgentLibrary,
            ArtifactType::AgentDocuments,
            ArtifactType::AgentAdapters,
            ArtifactType::PodState,
        ];
        for at in &variants {
            let s: &'static str = at.label();
            let parsed = ArtifactType::from_str(s)
                .unwrap_or_else(|_| panic!("Failed to parse ArtifactType from '{}'", s));
            assert_eq!(&parsed, at, "Roundtrip failed for {:?}", at);
        }
    }

    #[test]
    fn pod_state_maps_to_pods_repo() {
        assert_eq!(
            ArtifactType::PodState.repo_id(),
            hkask_ports::git_cas::RepoId::Pods
        );
    }

    #[test]
    fn pod_state_label_is_snake_case() {
        assert_eq!(ArtifactType::PodState.label(), "pod_state");
    }

    #[test]
    fn parse_from_string() {
        assert_eq!(
            ArtifactType::from_str("pod_state").unwrap(),
            ArtifactType::PodState
        );
        assert_eq!(
            ArtifactType::from_str("memory_triple").unwrap(),
            ArtifactType::MemoryTriple
        );
        assert!(ArtifactType::from_str("nonexistent").is_err());
    }
}
