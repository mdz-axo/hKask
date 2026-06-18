//! Artifact serialization for git blob storage.
//! # REQ: P7-svc-backup-serialization-f1 (Snapshot Serialization Format) — deterministic byte representation.
//!
//! Each artifact type serializes to a deterministic byte sequence so that
//! identical artifact state produces identical blob hashes (git deduplication).
//! Initial implementation uses JSON for all types — simplest, diffable, human-readable.
//! Per-type format optimization deferred to F1 resolution.

use serde::Serialize;
use tracing::info;

use crate::scope::ArtifactType;

/// Serialize an artifact to deterministic bytes for git blob storage.
///
/// The serialization must be deterministic: same artifact → same bytes → same
/// BLAKE3 hash → git deduplication works. JSON with sorted keys satisfies this.
///
/// REQ: P7-svc-backup-serialization-svc-159
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// REQ: P9-CNS-BK-012 pre: valid artifact, post: cns.backup span emitted
/// pre:  artifact_type must be a valid ArtifactType; artifact_id must be non-empty; data must be Serialize
/// post: returns Vec<u8> of JSON-encoded ArtifactEnvelope; Err on serialization failure
pub fn serialize_artifact(
    artifact_type: &ArtifactType,
    artifact_id: &str,
    data: &impl Serialize,
) -> Result<Vec<u8>, serde_json::Error> {
    // Wrap in an envelope so the blob self-describes its type and ID.
    let envelope = ArtifactEnvelope {
        artifact_type: artifact_type.label().to_string(),
        artifact_id: artifact_id.to_string(),
        payload: data,
    };
    let bytes = serde_json::to_vec(&envelope)?;

    // P9: CNS span
    // REQ: P9-CNS-BK-012 pre: valid state, post: cns.backup span emitted
    info!(target: "cns.backup", operation = "serialize_artifact", artifact_type = %artifact_type.label(), artifact_id = %artifact_id, byte_len = bytes.len(), "CNS");

    Ok(bytes)
}

/// Deserialize an artifact blob back to its JSON value.
///
/// Returns the raw JSON value — callers interpret based on artifact type.
///
/// REQ: P7-svc-backup-serialization-svc-160
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// REQ: P9-CNS-BK-013 pre: valid blob, post: cns.backup span emitted
/// pre:  blob must be valid JSON matching ArtifactEnvelopeValue schema
/// post: returns ArtifactEnvelopeValue with artifact_type, artifact_id, and payload; Err on invalid JSON
pub fn deserialize_artifact(blob: &[u8]) -> Result<ArtifactEnvelopeValue, serde_json::Error> {
    let envelope: ArtifactEnvelopeValue = serde_json::from_slice(blob)?;

    // P9: CNS span
    // REQ: P9-CNS-BK-013 pre: valid state, post: cns.backup span emitted
    info!(target: "cns.backup", operation = "deserialize_artifact", artifact_type = %envelope.artifact_type, artifact_id = %envelope.artifact_id, byte_len = blob.len(), "CNS");

    Ok(envelope)
}

/// Envelope wrapping an artifact for git blob storage.
///
/// Self-describing: the blob carries its type and ID so restore
/// operations can route it to the correct store without external metadata.
///
/// This is the serialization-only type. Deserialization uses
/// [`ArtifactEnvelopeValue`] which owns its data.
#[derive(Serialize)]
struct ArtifactEnvelope<'a, T: Serialize> {
    artifact_type: String,
    artifact_id: String,
    #[serde(flatten)]
    payload: &'a T,
}

/// Deserialized artifact envelope (payload as raw JSON value).
#[derive(Debug, serde::Deserialize)]
pub struct ArtifactEnvelopeValue {
    pub artifact_type: String,
    pub artifact_id: String,
    /// The payload as a raw `serde_json::Value` — callers downcast
    /// based on `artifact_type`.
    #[serde(flatten)]
    pub payload: serde_json::Value,
}

/// Compute the git tree path for an artifact within its repository.
///
/// Path format: `<artifact_type_label>/<artifact_id>.json`
/// This organizes blobs hierarchically in the git tree, enabling
/// scoped list_tree operations (e.g., `prefix = "template/"`).
///
/// REQ: P7-svc-backup-serialization-svc-161
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  artifact_type must be a valid ArtifactType; artifact_id must be non-empty
/// post: returns String path in format "{label}/{id}.json"
pub fn artifact_git_path(artifact_type: &ArtifactType, artifact_id: &str) -> String {
    format!("{}/{}.json", artifact_type.label(), artifact_id)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // REQ: P7-svc-backup-serialization-backup-serialize-001 — Same artifact produces same bytes (deterministic)
    #[test]
    fn same_artifact_produces_same_bytes() {
        let data = json!({"name": "test", "value": 42});
        let bytes1 = serialize_artifact(&ArtifactType::Template, "tpl-1", &data).unwrap();
        let bytes2 = serialize_artifact(&ArtifactType::Template, "tpl-1", &data).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    // REQ: P7-svc-backup-serialization-backup-serialize-002 — Different IDs produce different bytes
    #[test]
    fn different_ids_produce_different_bytes() {
        let data = json!({"name": "test"});
        let bytes1 = serialize_artifact(&ArtifactType::Template, "tpl-1", &data).unwrap();
        let bytes2 = serialize_artifact(&ArtifactType::Template, "tpl-2", &data).unwrap();
        assert_ne!(bytes1, bytes2);
    }

    // REQ: P7-svc-backup-serialization-backup-serialize-003 — Round-trip: serialize → deserialize preserves data
    #[test]
    fn roundtrip_preserves_data() {
        let data = json!({"name": "test", "value": 42});
        let bytes = serialize_artifact(&ArtifactType::Template, "tpl-1", &data).unwrap();
        let envelope: ArtifactEnvelopeValue = deserialize_artifact(&bytes).unwrap();
        assert_eq!(envelope.artifact_type, "template");
        assert_eq!(envelope.artifact_id, "tpl-1");
        assert_eq!(envelope.payload, data);
    }

    // REQ: P7-svc-backup-serialization-backup-serialize-004 — Git path follows convention
    #[test]
    fn git_path_follows_convention() {
        let path = artifact_git_path(&ArtifactType::Template, "my-template");
        assert_eq!(path, "template/my-template.json");
    }
}
