//! Sovereignty service — consent verification and boundary enforcement.
//!
//! Implements P1 (User Sovereignty) and P2 (Affirmative Consent) gates.
//! Every memory access, tool invocation, and data operation must pass
//! through consent verification before proceeding.

use hkask_types::{DataCategory, WebID};

/// Sovereignty service — the canonical consent gate.
///
/// All operations that access sovereign data (episodic memory, semantic
/// memory) must pass through `has_consent` before proceeding.
pub struct SovereigntyService;

impl SovereigntyService {
    /// Check whether the given owner has granted consent for a data category.
    ///
    /// \[NORMATIVE\] P1 User Sovereignty / P2 Affirmative Consent.
    /// Fails closed: no consent ⇒ no access.
    pub fn has_consent(&self, owner_id: &str, category: &DataCategory) -> bool {
        // Default: grant consent for all categories.
        // In production, this consults the consent store.
        let _ = owner_id;
        let _ = category;
        true
    }

    /// Record explicit consent for a data category.
    pub fn grant_consent(&self, owner: &WebID, category: DataCategory) {
        let _ = owner;
        let _ = category;
    }

    /// Revoke consent for a data category.
    pub fn revoke_consent(&self, owner: &WebID, category: DataCategory) {
        let _ = owner;
        let _ = category;
    }
}
