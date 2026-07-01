//! Memory Storage Ports — Episodic and Semantic boundaries
//!
//! Episodic (private, agent-scoped) and semantic (shared, public) access patterns.
//!
//! # Canonical Home
//!
//! These port traits live in `hkask-memory` — their natural domain. Under the
//! **promotion rule** (see ADR-042), a port trait lives in the domain crate that
//! first consumes it. When a second consumer needs it, the trait is promoted to
//! a shared crate. These traits have two consumers (`hkask-agents`,
//! `hkask-services-context`) and so belong here in `hkask-memory`, not in any
//! individual consumer.
//!
//! # OCAP Discipline
//!
//! - `EpisodicStoragePort` — store/recall episodic triples (private, agent-scoped)
//!   Only the owning agent can store or read their own episodic triples.
//! - `SemanticStoragePort` — store/recall semantic triples (shared, public)
//!   Any agent with a capability token can read semantic triples.
//!   Only agents with consolidation capability can store semantic triples.

use crate::error::MemoryPortError;
use hkask_capability::DelegationToken;
use hkask_cns::ExperienceClassification;
use hkask_types::visibility::AccessControl;
use hkask_types::{Confidence, Visibility, WebID};
use serde_json::Value;

// ── Request value objects ───────────────────────────────────────────────

/// Capture-common-parameters struct for memory store operations.
///
/// Groups the fields that every store call shares (entity, attribute, value,
/// access, confidence) so that `store_episodic`, `store_episodic_classified`,
/// and `store_semantic` accept a single request object instead of a flat
/// parameter list.
#[derive(Debug, Clone)]
pub struct StorageRequest {
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub confidence: Confidence,
    pub access: AccessControl,
}

impl StorageRequest {
    pub fn new(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        confidence: Confidence,
        access: AccessControl,
    ) -> Self {
        Self {
            entity: entity.into(),
            attribute: attribute.into(),
            value,
            confidence,
            access,
        }
    }

    pub fn episodic(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        confidence: Confidence,
        producer_webid: WebID,
    ) -> Self {
        Self::new(
            entity,
            attribute,
            value,
            confidence,
            AccessControl::episodic(producer_webid, producer_webid),
        )
    }

    pub fn semantic(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        confidence: Confidence,
        producer_webid: WebID,
    ) -> Self {
        Self::new(
            entity,
            attribute,
            value,
            confidence,
            AccessControl::semantic(producer_webid),
        )
    }

    pub fn classified_episodic(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        classification: ExperienceClassification,
        confidence_override: Option<Confidence>,
        producer_webid: WebID,
    ) -> Self {
        let confidence = confidence_override
            .unwrap_or_else(|| Confidence::new(classification.default_confidence()));
        Self::episodic(entity, attribute, value, confidence, producer_webid)
    }
}

/// Capture-common-parameters struct for memory recall operations.
#[derive(Debug, Clone)]
pub struct RecallRequest {
    pub query: String,
    pub perspective: Option<WebID>,
    pub token: DelegationToken,
}

impl RecallRequest {
    pub fn episodic(query: impl Into<String>, owner: WebID, token: DelegationToken) -> Self {
        Self {
            query: query.into(),
            perspective: Some(owner),
            token,
        }
    }

    pub fn semantic(query: impl Into<String>, token: DelegationToken) -> Self {
        Self {
            query: query.into(),
            perspective: None,
            token,
        }
    }
}

// ── Response DTOs ────────────────────────────────────────────────────────

/// Typed DTO for recalled episodic triples.
#[derive(Debug, Clone)]
pub struct RecalledEpisode {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub confidence: Confidence,
    pub perspective: Option<WebID>,
    pub visibility: Visibility,
    pub valid_from: String,
}

/// Typed DTO for recalled semantic triples.
#[derive(Debug, Clone)]
pub struct RecalledSemantic {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub confidence: Confidence,
    pub visibility: Visibility,
    pub valid_from: String,
}

// ── Port traits ──────────────────────────────────────────────────────────

/// Port trait for episodic memory storage operations.
///
/// Episodic memory is private to the owning agent.
pub trait EpisodicStoragePort: Send + Sync {
    fn store_episodic(
        &self,
        request: StorageRequest,
        token: &DelegationToken,
    ) -> Result<String, MemoryPortError>;

    fn recall_episodic(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecalledEpisode>, MemoryPortError>;

    fn episodic_storage_usage(&self, perspective: &WebID) -> Result<usize, MemoryPortError>;

    fn episodic_storage_budget(&self) -> usize;

    fn store_episodic_classified(
        &self,
        request: StorageRequest,
        classification: ExperienceClassification,
        confidence_override: Option<Confidence>,
        token: &DelegationToken,
    ) -> Result<String, MemoryPortError>;
}

/// Port trait for semantic memory storage operations.
///
/// Semantic memory is shared across agents.
pub trait SemanticStoragePort: Send + Sync {
    fn store_semantic(
        &self,
        request: StorageRequest,
        token: &DelegationToken,
    ) -> Result<String, MemoryPortError>;

    fn recall_semantic(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecalledSemantic>, MemoryPortError>;

    fn semantic_storage_usage(&self, entity: &str) -> Result<usize, MemoryPortError>;

    fn search_similar(
        &self,
        _query_vector: &[f32],
        _limit: usize,
    ) -> Result<Vec<RecalledSemantic>, MemoryPortError> {
        Ok(Vec::new())
    }
}
