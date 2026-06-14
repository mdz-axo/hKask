//! User sovereignty routes — call consent manager directly.

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

fn consent_name(value: bool) -> &'static str {
    if value { "required" } else { "open" }
}

fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    hkask_types::DataCategory::parse(s)
}

pub fn sovereignty_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(sovereignty_status))
        .routes(routes!(sovereignty_grant_consent))
        .routes(routes!(sovereignty_revoke_consent))
        .routes(routes!(sovereignty_check_access))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyStatusResponse {
    pub explicit_consent: bool,
    pub requires_affirmative_consent: String,
    pub sovereign_data: Vec<String>,
    pub shared_data: Vec<String>,
    pub public_data: Vec<String>,
    pub granted_categories: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyConsentRequest {
    /// Data category to grant consent for (e.g. "memory", "inference", "all")
    pub category: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyConsentResponse {
    pub consent: bool,
    pub message: String,
    pub categories: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessCheckResponse {
    pub category: String,
    pub classification: String,
    pub access_required: String,
}

/// Get sovereignty status for the authenticated agent — consent state,
/// data category classifications, and granted sharing categories.
#[utoipa::path(
    get, path = "/api/sovereignty/status", tag = "sovereignty",
    responses(
        (status = 200, description = "Sovereignty status", body = SovereigntyStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn sovereignty_status(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<SovereigntyStatusResponse>, ServiceErrorResponse> {
    let cm = &state.agent_service.sovereignty();
    let webid_str = auth.webid.to_string();
    let boundary = hkask_types::sovereignty::DataSovereigntyBoundary::hkask_default();
    let granted = cm
        .get_granted_categories(&webid_str)
        ?;

    Ok(Json(SovereigntyStatusResponse {
        explicit_consent: !granted.is_empty(),
        requires_affirmative_consent: consent_name(boundary.requires_affirmative_consent())
            .to_string(),
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
        granted_categories: granted,
    }))
}

/// Grant explicit consent for a data category, enabling data sharing.
#[utoipa::path(
    post, path = "/api/sovereignty/consent/grant", tag = "sovereignty",
    responses(
        (status = 200, description = "Consent granted", body = SovereigntyConsentResponse),
        (status = 400, description = "Invalid category"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn sovereignty_grant_consent(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SovereigntyConsentRequest>,
) -> Result<Json<SovereigntyConsentResponse>, ServiceErrorResponse> {
    let webid_str = auth.webid.to_string();
    let cat_str = req.category;
    let cat = parse_data_category(&cat_str);
    let cm = &state.agent_service.sovereignty();
    cm.grant_consent(&webid_str, &cat)?;
    let granted = cm
        .get_granted_categories(&webid_str)
        ?;
    Ok(Json(SovereigntyConsentResponse {
        consent: true,
        message: format!("Explicit consent granted for '{cat_str}'. Data sharing enabled."),
        categories: granted,
    }))
}

/// Revoke all explicit consent — only public data remains accessible.
#[utoipa::path(
    post, path = "/api/sovereignty/consent/revoke", tag = "sovereignty",
    responses(
        (status = 200, description = "Consent revoked", body = SovereigntyConsentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Consent not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn sovereignty_revoke_consent(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<SovereigntyConsentResponse>, ServiceErrorResponse> {
    let webid_str = auth.webid.to_string();
    let cm = &state.agent_service.sovereignty();
    cm.revoke_consent(&webid_str)?;
    Ok(Json(SovereigntyConsentResponse {
        consent: false,
        message: "Explicit consent revoked. Only public data is accessible.".into(),
        categories: vec![],
    }))
}

/// Check whether the authenticated agent has access to a specific data category.
#[utoipa::path(
    get, path = "/api/sovereignty/access/check", tag = "sovereignty",
    params(("category" = String, Query, description = "Data category to check")),
    responses(
        (status = 200, description = "Access check result", body = AccessCheckResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Missing category parameter"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn sovereignty_check_access(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AccessCheckResponse>, ServiceErrorResponse> {
    let cat_str = params.get("category").map(|s| s.as_str()).unwrap_or("");
    if cat_str.is_empty() {
        return Err(ApiError::BadRequest {
            message: "Missing required query parameter: category".into(),
        });
    }
    let cat = parse_data_category(cat_str);
    let cat_name = cat.as_str();
    let webid_str = auth.webid.to_string();
    let boundary = hkask_types::sovereignty::DataSovereigntyBoundary::hkask_default();
    let cm = &state.agent_service.sovereignty();

    let class = boundary.classify(&cat);
    let classification = class.label();
    let access_required = class.access_required();
    let has_consent = if classification == "PUBLIC" {
        true
    } else {
        cm.has_consent(&webid_str, &cat)
    };

    if !has_consent && classification != "PUBLIC" {
        return Err(ApiError::Forbidden {
            reason: format!("No consent for category '{cat_name}' (class {classification})"),
        });
    }
    Ok(Json(AccessCheckResponse {
        category: cat_name.to_string(),
        classification: classification.to_string(),
        access_required: access_required.to_string(),
    }))
}
