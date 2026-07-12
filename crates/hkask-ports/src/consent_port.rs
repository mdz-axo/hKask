//! ConsentPort — trait boundary for consent record persistence.
//!
//! Decouples agent pods from the concrete `ConsentStore` in hkask-storage.

use hkask_types::InfrastructureError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A stored consent record at the port boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConsentRecord {
    pub id: String,
    pub webid: String,
    pub granted_categories: HashSet<String>,
    pub granted_at: i64,
    pub revoked_at: Option<i64>,
    pub active: bool,
}

/// Port trait for consent record persistence.
pub trait ConsentPort: Send + Sync {
    /// Initialize the consent store schema.
    fn initialize_schema(&self) -> Result<(), InfrastructureError>;

    /// Store (upsert) a consent record.
    fn store(&self, record: &StoredConsentRecord) -> Result<(), InfrastructureError>;

    /// List all active consent records.
    fn list_active(&self) -> Result<Vec<StoredConsentRecord>, InfrastructureError>;
}
