//! User sovereignty routes
//!
//! Endpoints read live state from the CNS runtime (`CnsRuntime`) and the
//! persistent `ConsentManager` rather than constructing throwaway
//! `UserSovereigntyState` values. This is the runtime enforcement path for
//! the Magna Carta: the API never reports stale or fabricated sovereignty
//! state.
//!
//! Business logic is delegated to `SovereigntyService`; this module handles
//! HTTP request parsing and response formatting only.

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State, routing::Router};
use hkask_services::{SovereigntyContext, SovereigntyService, parse_data_category};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

/// Map a boolean `requires_affirmative_consent` to an affirmative-consent
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

// Handlers

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
    let ctx = SovereigntyContext::from_parts(state.consent_manager.clone());
    let webid_str = auth.webid.to_string();

    let status = SovereigntyService::get_status(&ctx, &webid_str).map_err(ApiError::from)?;

    Ok(Json(SovereigntyStatusResponse {
        explicit_consent: status.explicit_consent,
        requires_affirmative_consent: consent_name(status.requires_affirmative_consent).to_string(),
        sovereign_data: status.sovereign_data,
        shared_data: status.shared_data,
        public_data: status.public_data,
        granted_categories: status.granted_categories,
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

    let ctx = SovereigntyContext::from_parts(state.consent_manager.clone());

    SovereigntyService::grant_consent(&ctx, &webid_str, &category).map_err(ApiError::from)?;

    let granted = SovereigntyService::get_granted_categories(&ctx, &webid_str).unwrap_or_default();

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

    let ctx = SovereigntyContext::from_parts(state.consent_manager.clone());
    SovereigntyService::revoke_consent(&ctx, &webid_str)?;

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

    let ctx = SovereigntyContext::from_parts(state.consent_manager.clone());

    let access =
        SovereigntyService::check_access(&ctx, &webid_str, &category).map_err(ApiError::from)?;

    // P1 Prohibition: The service returns classification + consent state;
    // the surface decides whether to grant or deny access.
    if !access.has_consent && access.classification != "PUBLIC" {
        return Err(ApiError::Forbidden {
            reason: format!(
                "No consent for category '{category_name}' (class {})",
                access.classification
            ),
        });
    }

    Ok(Json(AccessCheckResponse {
        category: category_name.to_string(),
        classification: access.classification,
        access_required: access.access_required,
    }))
}
