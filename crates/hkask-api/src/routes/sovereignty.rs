//! User sovereignty routes
//!
//! Endpoints read live state from the CNS runtime (`CnsRuntime`) and the
//! persistent `ConsentManager` rather than constructing throwaway
//! `UserSovereigntyState` values. This is the runtime enforcement path for
//! the Magna Carta: the API never reports stale or fabricated sovereignty
//! state.
//!
//! See `docs/architecture/magna-carta.md` for the contract.

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State, routing::Router};
use hkask_types::DataCategory;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

/// Map a boolean `prevents_passive_acquisition` to an affirmative-consent
/// label. The runtime type is a bool after simplification; the API surfaces
/// a stable, doc-aligned string.
fn consent_name(value: bool) -> &'static str {
    if value { "required" } else { "open" }
}

/// Create sovereignty router
pub fn sovereignty_router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/sovereignty/status",
            axum::routing::get(sovereignty_status),
        )
        .route(
            "/api/sovereignty/consent/grant",
            axum::routing::post(sovereignty_grant_consent),
        )
        .route(
            "/api/sovereignty/consent/revoke",
            axum::routing::post(sovereignty_revoke_consent),
        )
        .route(
            "/api/sovereignty/access/check",
            axum::routing::get(sovereignty_check_access),
        )
}

/// Sovereignty status response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyStatusResponse {
    pub explicit_consent: bool,
    pub requires_affirmative_consent: String,
    pub sovereign_data: Vec<String>,
    pub shared_data: Vec<String>,
    pub public_data: Vec<String>,
    pub granted_categories: Vec<String>,
}

/// Sovereignty consent request
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyConsentRequest {
    /// Data category to grant or revoke consent for
    pub category: Option<String>,
}

/// Sovereignty consent response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyConsentResponse {
    pub consent: bool,
    pub message: String,
    pub categories: Vec<String>,
}

/// Access check response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessCheckResponse {
    pub category: String,
    pub classification: String,
    pub access_required: String,
}

/// Sovereignty status endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/status",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Sovereignty status", body = SovereigntyStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<SovereigntyStatusResponse>, ApiError> {
    use hkask_types::sovereignty::DataSovereigntyBoundary;

    let webid_str = auth.webid.to_string();

    // Use the default boundary classification (the only one currently wired
    // into the type system) for the data-category lists. This is a constant
    // view of what the Magna Carta prescribes, surfaced for visibility.
    let boundary = DataSovereigntyBoundary::hkask_default();
    let requires_affirmative_consent = boundary.prevents_passive_acquisition();

    // Enrich status with consent manager state
    let granted_categories: Vec<String> = state
        .consent_manager
        .get_granted_categories(&webid_str)
        .unwrap_or_default()
        .into_iter()
        .collect();

    Ok(Json(SovereigntyStatusResponse {
        explicit_consent: !granted_categories.is_empty(),
        requires_affirmative_consent: consent_name(requires_affirmative_consent).to_string(),
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
    }))
}

/// Grant consent endpoint
#[utoipa::path(
    post,
    path = "/api/sovereignty/consent/grant",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Consent granted", body = SovereigntyConsentResponse),
        (status = 400, description = "Invalid category"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_grant_consent(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SovereigntyConsentRequest>,
) -> Result<Json<SovereigntyConsentResponse>, ApiError> {
    let webid_str = auth.webid.to_string();
    let category_str = req.category.unwrap_or_else(|| "all".to_string());
    let category = parse_data_category(&category_str);

    state
        .consent_manager
        .grant_consent(&webid_str, &category)
        .map_err(|e| ApiError::Internal {
            message: e.to_string(),
        })?;

    let granted = state
        .consent_manager
        .get_granted_categories(&webid_str)
        .unwrap_or_default()
        .into_iter()
        .collect();

    Ok(Json(SovereigntyConsentResponse {
        consent: true,
        message: format!(
            "Explicit consent granted for '{}'. Data sharing enabled.",
            category_str
        ),
        categories: granted,
    }))
}

/// Revoke consent endpoint
#[utoipa::path(
    post,
    path = "/api/sovereignty/consent/revoke",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Consent revoked", body = SovereigntyConsentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Consent not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_revoke_consent(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<SovereigntyConsentResponse>, ApiError> {
    let webid_str = auth.webid.to_string();

    state.consent_manager.revoke_consent(&webid_str)?;

    Ok(Json(SovereigntyConsentResponse {
        consent: false,
        message: "Explicit consent revoked. Only public data is accessible.".to_string(),
        categories: vec![],
    }))
}

/// Check access endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/access/check",
    tag = "sovereignty",
    params(
        ("category" = String, Query, description = "Data category to check"),
    ),
    responses(
        (status = 200, description = "Access check result", body = AccessCheckResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Missing category parameter"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_check_access(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AccessCheckResponse>, ApiError> {
    let category_str = params.get("category").map(|s| s.as_str()).unwrap_or("");
    if category_str.is_empty() {
        return Err(ApiError::BadRequest {
            message: "Missing required query parameter: category".to_string(),
        });
    }

    let category = parse_data_category(category_str);
    let category_name = category.as_str();
    let webid_str = auth.webid.to_string();

    // Use the default boundary classification to map the category to its
    // policy class (Sovereign / Shared / Public), then check the live
    // ConsentManager for the requesting WebID.
    use hkask_types::sovereignty::DataSovereigntyBoundary;
    let boundary = DataSovereigntyBoundary::hkask_default();

    let (classification, access_required) = if boundary.is_sovereign(&category) {
        (
            "SOVEREIGN".to_string(),
            "Requires explicit consent AND owner".to_string(),
        )
    } else if boundary.is_category_shared(&category) {
        (
            "SHARED".to_string(),
            "Requires explicit consent".to_string(),
        )
    } else if boundary.is_category_public(&category) {
        ("PUBLIC".to_string(), "Always accessible".to_string())
    } else {
        ("UNKNOWN".to_string(), "Denied by default".to_string())
    };

    // Effective access: live consent lookup overrides the policy class for
    // non-public categories. Public data is always accessible.
    let has_consent = state
        .consent_manager
        .has_consent(&webid_str, &category)
        .unwrap_or(false);
    if !has_consent && classification != "PUBLIC" {
        return Err(ApiError::Forbidden {
            reason: format!("No consent for category '{category_name}' (class {classification})"),
        });
    }

    Ok(Json(AccessCheckResponse {
        category: category_name.to_string(),
        classification,
        access_required,
    }))
}

/// Parse a string into a DataCategory
fn parse_data_category(s: &str) -> DataCategory {
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
