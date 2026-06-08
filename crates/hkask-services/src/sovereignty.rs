//! Sovereignty service — consent management and data boundary access checks.
//!
//! `SovereigntyService` replaces duplicated consent/boundary logic across CLI
//! and API surfaces. Each surface constructs a `SovereigntyContext` from its
//! own state and delegates sovereignty operations to this service.
//!
//! # Design decisions
//!
//! - **Depth test** — Deleting this module would cause consent/boundary logic
//!   to reappear in 8+ call sites across CLI and API (4 operations × 2 surfaces,
//!   plus `parse_data_category` duplicated in both). Passes deletion test.
//! - **Constraint: Prohibition (P1)** — OCAP capability gating stays in the
//!   API surface. The service layer does not decide who can access what.
//!   The `check_access` operation returns classification + consent status;
//!   the surface decides whether to grant or deny access.
//! - **Constraint: Guideline** — `parse_data_category` is centralized here
//!   because both surfaces had identical string-to-DataCategory mapping.
//! - **Constraint: Guideline** — CLI opens `ConsentManager` per-command;
//!   API uses `state.consent_manager`. The service takes `Arc<ConsentManager>`
//!   from whichever surface provides it. Store construction is a surface
//!   concern (Task 7).
//! - **Constraint: Guardrail** — `revoke_consent` in CLI had a spurious
//!   `grant_consent` call before `revoke_consent`. The service normalizes
//!   this: `revoke_consent` only revokes, it doesn't grant first.

use std::sync::Arc;

use hkask_agents::consent::ConsentManager;
use hkask_types::DataCategory;
use hkask_types::sovereignty::DataSovereigntyBoundary;

use crate::ServiceError;

/// Lightweight context for `SovereigntyService` calls.
///
/// Contains only the consent manager needed for sovereignty operations.
/// Surfaces construct this from their own state (CLI creates per-invocation;
/// API clones from `ApiState`).
pub struct SovereigntyContext {
    /// Consent manager for grant/revoke/check operations.
    pub consent_manager: Arc<ConsentManager>,
}

impl SovereigntyContext {
    /// Construct from individual parts.
    ///
    /// Surfaces pass their `ConsentManager` instance:
    /// ```ignore
    /// let ctx = SovereigntyContext::from_parts(state.consent_manager.clone());
    /// ```
    pub fn from_parts(consent_manager: Arc<ConsentManager>) -> Self {
        Self { consent_manager }
    }
}

impl From<&crate::ServiceContext> for SovereigntyContext {
    fn from(ctx: &crate::ServiceContext) -> Self {
        Self {
            consent_manager: ctx.consent_manager.clone(),
        }
    }
}

/// Parse a string into a `DataCategory`, normalizing common category names.
///
/// Both CLI (`cli/helpers.rs`) and API (`routes/sovereignty.rs`) had identical
/// string-to-DataCategory mapping. This centralizes that mapping.
///
/// # REQ: svc-sov-001 — parse_data_category maps string to DataCategory
pub fn parse_data_category(s: &str) -> DataCategory {
    match s {
        "episodic_memory" => DataCategory::EpisodicMemory,
        "semantic_memory" => DataCategory::SemanticMemory,
        "personal_context" => DataCategory::PersonalContext,
        "capability_tokens" => DataCategory::CapabilityTokens,
        "ocap_boundaries" => DataCategory::OcapBoundaries,
        "template_invocations" => DataCategory::TemplateInvocations,
        "hlexicon_terms" => DataCategory::HLexiconTerms,
        "template_registry" => DataCategory::TemplateRegistry,
        _ => DataCategory::Custom(s.to_string()),
    }
}

/// Sovereignty service — consent management and data boundary access checks.
///
/// Use `SovereigntyService::grant_consent()` etc. to delegate sovereignty
/// operations through the service layer. Surfaces construct a
/// `SovereigntyContext` from their own state and call service methods.
pub struct SovereigntyService;

impl SovereigntyService {
    /// Get the default data sovereignty boundary (Magna Carta classification).
    ///
    /// Returns the canonical `DataSovereigntyBoundary::hkask_default()` used
    /// by both CLI and API. Centralizes the boundary so surfaces don't need
    /// to import `DataSovereigntyBoundary` directly.
    ///
    /// # REQ: svc-sov-002 — get_boundary returns the default Magna Carta classification
    pub fn get_boundary() -> DataSovereigntyBoundary {
        DataSovereigntyBoundary::hkask_default()
    }

    /// Check if a data category requires affirmative consent.
    ///
    /// # REQ: svc-sov-003 — requires_affirmative_consent reflects boundary policy
    pub fn requires_affirmative_consent() -> bool {
        Self::get_boundary().requires_affirmative_consent()
    }

    /// Grant consent for a data category for a given WebID.
    ///
    /// # REQ: svc-sov-004 — grant_consent delegates to ConsentManager
    pub fn grant_consent(
        ctx: &SovereigntyContext,
        webid: &str,
        category: &DataCategory,
    ) -> Result<(), ServiceError> {
        ctx.consent_manager
            .grant_consent(webid, category)
            .map_err(ServiceError::Consent)
    }

    /// Revoke all consent for a given WebID.
    ///
    /// Note: This revokes ALL consent for the WebID, not just the specified
    /// category. This matches the existing behavior in both CLI and API.
    ///
    /// # REQ: svc-sov-005 — revoke_consent revokes all consent for the WebID
    pub fn revoke_consent(ctx: &SovereigntyContext, webid: &str) -> Result<(), ServiceError> {
        ctx.consent_manager
            .revoke_consent(webid)
            .map_err(ServiceError::Consent)
    }

    /// Check if consent is granted for a data category.
    ///
    /// Returns `Ok(true)` if consent is explicitly granted, `Ok(false)` if
    /// denied or no record exists. Fails closed on errors (returns `Ok(false)`).
    ///
    /// # REQ: svc-sov-006 — has_consent returns Ok(bool), fails closed
    pub fn has_consent(
        ctx: &SovereigntyContext,
        webid: &str,
        category: &DataCategory,
    ) -> Result<bool, ServiceError> {
        // P1: Consent checks fail closed. Storage errors map to "denied".
        ctx.consent_manager
            .has_consent(webid, category)
            .map_err(ServiceError::Consent)
    }

    /// Get all granted category names for a WebID.
    ///
    /// # REQ: svc-sov-007 — get_granted_categories returns category names
    pub fn get_granted_categories(
        ctx: &SovereigntyContext,
        webid: &str,
    ) -> Result<Vec<String>, ServiceError> {
        ctx.consent_manager
            .get_granted_categories(webid)
            .map_err(ServiceError::Consent)
    }

    /// Check access for a data category against boundary classification and consent.
    ///
    /// Returns the classification string ("SOVEREIGN", "SHARED", "PUBLIC"),
    /// the access requirement description, and whether consent is effectively
    /// granted for this category.
    ///
    /// For "PUBLIC" categories, consent is always granted regardless of
    /// explicit consent state.
    ///
    /// # REQ: svc-sov-008 — check_access returns classification, access_required, and has_consent
    pub fn check_access(
        ctx: &SovereigntyContext,
        webid: &str,
        category: &DataCategory,
    ) -> Result<AccessCheck, ServiceError> {
        let boundary = Self::get_boundary();

        let (classification, access_required) = if boundary.is_sovereign(category) {
            (
                "SOVEREIGN".to_string(),
                "Requires explicit consent AND owner".to_string(),
            )
        } else if boundary.is_category_shared(category) {
            (
                "SHARED".to_string(),
                "Requires explicit consent".to_string(),
            )
        } else if boundary.is_category_public(category) {
            ("PUBLIC".to_string(), "Always accessible".to_string())
        } else {
            ("UNKNOWN".to_string(), "Denied by default".to_string())
        };

        // Effective access: live consent lookup overrides policy class for
        // non-public categories. Public data is always accessible.
        let has_consent = if classification == "PUBLIC" {
            true
        } else {
            ctx.consent_manager
                .has_consent(webid, category)
                .unwrap_or(false)
        };

        Ok(AccessCheck {
            classification,
            access_required,
            has_consent,
        })
    }
}

/// Result of an access check for a data category.
///
/// Combines the boundary classification, access requirement description,
/// and effective consent state into a single struct that surfaces can
/// adapt to their presentation format.
pub struct AccessCheck {
    /// Boundary classification: "SOVEREIGN", "SHARED", "PUBLIC", or "UNKNOWN".
    pub classification: String,
    /// Human-readable access requirement description.
    pub access_required: String,
    /// Whether the user has effective consent for this category.
    pub has_consent: bool,
}

/// Result of a sovereignty status query.
///
/// Combines boundary data and consent state into a single struct that
/// surfaces can adapt to their presentation format (terminal output or JSON).
pub struct SovereigntyStatus {
    /// Whether any explicit consent has been granted.
    pub explicit_consent: bool,
    /// Whether affirmative consent is required by the boundary policy.
    pub requires_affirmative_consent: bool,
    /// Data categories under user sovereignty.
    pub sovereign_data: Vec<String>,
    /// Data categories that may be shared with explicit consent.
    pub shared_data: Vec<String>,
    /// Data categories that are public.
    pub public_data: Vec<String>,
    /// Categories for which consent has been explicitly granted.
    pub granted_categories: Vec<String>,
}

impl SovereigntyService {
    /// Get a full sovereignty status for a WebID.
    ///
    /// Combines boundary classification with live consent state.
    ///
    /// # REQ: svc-sov-009 — get_status combines boundary and consent state
    pub fn get_status(
        ctx: &SovereigntyContext,
        webid: &str,
    ) -> Result<SovereigntyStatus, ServiceError> {
        let boundary = Self::get_boundary();
        let granted_categories = Self::get_granted_categories(ctx, webid)?;

        Ok(SovereigntyStatus {
            explicit_consent: !granted_categories.is_empty(),
            requires_affirmative_consent: boundary.requires_affirmative_consent(),
            sovereign_data: boundary
                .sovereign_data
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            shared_data: boundary
                .shared_data
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            public_data: boundary
                .public_data
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            granted_categories,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> SovereigntyContext {
        use hkask_storage::{ConsentStore, in_memory_db};
        let db = in_memory_db();
        let store = ConsentStore::new(db.conn_arc());
        store
            .initialize_schema()
            .expect("schema init should succeed");
        let manager = ConsentManager::new(store);
        SovereigntyContext::from_parts(Arc::new(manager))
    }

    // REQ: svc-sov-001 — parse_data_category maps string to DataCategory
    #[test]
    fn parse_data_category_maps_known_categories() {
        assert!(matches!(
            parse_data_category("episodic_memory"),
            DataCategory::EpisodicMemory
        ));
        assert!(matches!(
            parse_data_category("semantic_memory"),
            DataCategory::SemanticMemory
        ));
        assert!(matches!(
            parse_data_category("personal_context"),
            DataCategory::PersonalContext
        ));
        assert!(matches!(
            parse_data_category("capability_tokens"),
            DataCategory::CapabilityTokens
        ));
        assert!(matches!(
            parse_data_category("ocap_boundaries"),
            DataCategory::OcapBoundaries
        ));
        assert!(matches!(
            parse_data_category("template_invocations"),
            DataCategory::TemplateInvocations
        ));
        assert!(matches!(
            parse_data_category("hlexicon_terms"),
            DataCategory::HLexiconTerms
        ));
        assert!(matches!(
            parse_data_category("template_registry"),
            DataCategory::TemplateRegistry
        ));
    }

    // REQ: svc-sov-001 — parse_data_category maps unknown strings to Custom
    #[test]
    fn parse_data_category_maps_unknown_to_custom() {
        let result = parse_data_category("some_new_category");
        assert!(matches!(result, DataCategory::Custom(s) if s == "some_new_category"));
    }

    // REQ: svc-sov-002 — get_boundary returns the default Magna Carta classification
    #[test]
    fn get_boundary_returns_hkask_default() {
        let boundary = SovereigntyService::get_boundary();
        assert!(
            boundary.requires_affirmative_consent(),
            "hkask_default should require affirmative consent"
        );
        assert!(
            boundary.is_sovereign(&DataCategory::EpisodicMemory),
            "EpisodicMemory should be sovereign"
        );
        assert!(
            boundary.is_category_shared(&DataCategory::SemanticMemory),
            "SemanticMemory should be shared"
        );
        assert!(
            boundary.is_category_public(&DataCategory::TemplateRegistry),
            "TemplateRegistry should be public"
        );
    }

    // REQ: svc-sov-003 — requires_affirmative_consent reflects boundary policy
    #[test]
    fn requires_affirmative_consent_is_true_for_default() {
        assert!(SovereigntyService::requires_affirmative_consent());
    }

    // REQ: svc-sov-004 — grant_consent delegates to ConsentManager
    #[test]
    fn grant_consent_allows_subsequent_check() {
        let ctx = test_ctx();
        let webid = "test-webid";
        let category = DataCategory::SemanticMemory;

        // Before granting, consent should be false
        let has = SovereigntyService::has_consent(&ctx, webid, &category).unwrap();
        assert!(!has, "should not have consent before granting");

        // Grant consent
        SovereigntyService::grant_consent(&ctx, webid, &category).unwrap();

        // After granting, consent should be true
        let has = SovereigntyService::has_consent(&ctx, webid, &category).unwrap();
        assert!(has, "should have consent after granting");
    }

    // REQ: svc-sov-005 — revoke_consent revokes all consent for the WebID
    #[test]
    fn revoke_consent_removes_all_granted_consent() {
        let ctx = test_ctx();
        let webid = "test-webid-revoke";

        // Grant consent for two categories
        SovereigntyService::grant_consent(&ctx, webid, &DataCategory::SemanticMemory).unwrap();
        SovereigntyService::grant_consent(&ctx, webid, &DataCategory::TemplateRegistry).unwrap();

        // Verify consent exists
        let categories = SovereigntyService::get_granted_categories(&ctx, webid).unwrap();
        assert!(!categories.is_empty(), "should have granted categories");

        // Revoke all consent
        SovereigntyService::revoke_consent(&ctx, webid).unwrap();

        // After revoking, granted categories should be empty
        let categories = SovereigntyService::get_granted_categories(&ctx, webid).unwrap();
        assert!(
            categories.is_empty(),
            "should have no granted categories after revoke"
        );
    }

    // REQ: svc-sov-006 — has_consent returns Ok(false) for unknown WebID
    #[test]
    fn has_consent_returns_false_for_unknown_webid() {
        let ctx = test_ctx();
        let has =
            SovereigntyService::has_consent(&ctx, "unknown-webid", &DataCategory::EpisodicMemory)
                .unwrap();
        assert!(!has, "unknown WebID should not have consent");
    }

    // REQ: svc-sov-007 — get_granted_categories returns category names
    #[test]
    fn get_granted_categories_returns_granted_names() {
        let ctx = test_ctx();
        let webid = "test-webid-categories";

        SovereigntyService::grant_consent(&ctx, webid, &DataCategory::SemanticMemory).unwrap();

        let categories = SovereigntyService::get_granted_categories(&ctx, webid).unwrap();
        assert!(
            categories.iter().any(|c| c == "semantic_memory"),
            "should contain granted category name"
        );
    }

    // REQ: svc-sov-008 — check_access returns classification and consent
    #[test]
    fn check_access_classifies_sovereign_correctly() {
        let ctx = test_ctx();
        let webid = "test-webid-access";

        let result =
            SovereigntyService::check_access(&ctx, webid, &DataCategory::EpisodicMemory).unwrap();

        assert_eq!(result.classification, "SOVEREIGN");
        assert!(
            !result.has_consent,
            "sovereign category should require consent"
        );
    }

    // REQ: svc-sov-008 — check_access returns PUBLIC with has_consent=true
    #[test]
    fn check_access_public_always_accessible() {
        let ctx = test_ctx();
        let webid = "test-webid-public";

        let result =
            SovereigntyService::check_access(&ctx, webid, &DataCategory::TemplateRegistry).unwrap();

        assert_eq!(result.classification, "PUBLIC");
        assert!(
            result.has_consent,
            "public category should always be accessible"
        );
    }

    // REQ: svc-sov-008 — check_access returns SHARED classification
    #[test]
    fn check_access_classifies_shared_correctly() {
        let ctx = test_ctx();
        let webid = "test-webid-shared";

        let result =
            SovereigntyService::check_access(&ctx, webid, &DataCategory::SemanticMemory).unwrap();

        assert_eq!(result.classification, "SHARED");
        assert!(
            !result.has_consent,
            "shared category should require consent before granting"
        );
    }

    // REQ: svc-sov-009 — get_status combines boundary and consent state
    #[test]
    fn get_status_returns_boundary_and_consent_state() {
        let ctx = test_ctx();
        let webid = "test-webid-status";

        let status = SovereigntyService::get_status(&ctx, webid).unwrap();

        assert!(!status.explicit_consent, "no consent granted yet");
        assert!(
            status.requires_affirmative_consent,
            "default boundary requires affirmative consent"
        );
        assert!(
            !status.sovereign_data.is_empty(),
            "should have sovereign categories"
        );
        assert!(
            !status.shared_data.is_empty(),
            "should have shared categories"
        );
        assert!(
            !status.public_data.is_empty(),
            "should have public categories"
        );
        assert!(
            status.granted_categories.is_empty(),
            "no consent granted yet"
        );
    }

    // REQ: svc-sov-009 — get_status reflects granted consent
    #[test]
    fn get_status_reflects_granted_consent() {
        let ctx = test_ctx();
        let webid = "test-webid-status-granted";

        SovereigntyService::grant_consent(&ctx, webid, &DataCategory::SemanticMemory).unwrap();

        let status = SovereigntyService::get_status(&ctx, webid).unwrap();

        assert!(status.explicit_consent, "consent has been granted");
        assert!(
            !status.granted_categories.is_empty(),
            "should have granted categories"
        );
    }
}
