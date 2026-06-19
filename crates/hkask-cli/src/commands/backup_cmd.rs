//! Backup command handlers for `kask backup`
//!
//! Implements CLI display logic for backup operations. All business logic
//! delegates to `hkask_services::BackupService`.


use std::sync::Arc;

use hkask_services::RetentionPolicy;
use hkask_services::{ArtifactType, BackupScope, BackupService, ListFilter, RestoreScope};
use hkask_types::ports::git_cas::GitCASPort;

use crate::block_on;
use crate::cli::BackupAction;

/// Resolve the hexagonal `GitCASPort` from the environment.
fn resolve_git_cas_port() -> Arc<dyn GitCASPort> {
    let adapter = super::helpers::or_exit(
        hkask_mcp::GixCasAdapter::from_env(),
        "Failed to initialize CAS adapter",
    );
    Arc::new(adapter) as Arc<dyn GitCASPort>
}

/// Parse an artifact type from a CLI string.
fn parse_artifact_type(s: &str) -> Option<ArtifactType> {
    match s {
        "template" => Some(ArtifactType::Template),
        "style" => Some(ArtifactType::Style),
        "goal" => Some(ArtifactType::Goal),
        "spec" => Some(ArtifactType::Spec),
        "memory" | "memory_triple" => Some(ArtifactType::MemoryTriple),
        "embedding" => Some(ArtifactType::Embedding),
        "registry" | "registry_entry" => Some(ArtifactType::RegistryEntry),
        "cns" | "cns_audit" => Some(ArtifactType::CnsAudit),
        "sovereignty" | "sovereignty_manifest" => Some(ArtifactType::SovereigntyManifest),
        "session" => Some(ArtifactType::Session),
        "wallet" | "wallet_state" => Some(ArtifactType::WalletState),
        "settings" => Some(ArtifactType::Settings),
        _ => None,
    }
}

/// Parse a comma-separated list of artifact types.
fn parse_artifact_types(s: &str) -> Vec<ArtifactType> {
    s.split(',')
        .map(|s| s.trim())
        .filter_map(parse_artifact_type)
        .collect()
}

/// Parse a backup scope from a CLI string.
fn parse_scope(s: &str) -> BackupScope {
    match s {
        "full" | "" => BackupScope::Full,
        other => {
            if let Some(at) = parse_artifact_type(other) {
                BackupScope::ByType(at)
            } else {
                eprintln!("Unknown scope '{}', defaulting to full", other);
                BackupScope::Full
            }
        }
    }
}

/// Parse a restore scope from a CLI string.
fn parse_restore_scope(s: &str) -> RestoreScope {
    match s {
        "full" | "" => RestoreScope::Full,
        other => {
            if let Some(at) = parse_artifact_type(other) {
                RestoreScope::ByType(at)
            } else {
                eprintln!("Unknown scope '{}', defaulting to full", other);
                RestoreScope::Full
            }
        }
    }
}

/// Run a backup operation.
///
