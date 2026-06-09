//! User sovereignty routes — call consent manager directly.

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

fn consent_name(value: bool) -> &'static str {
    if value { "required" } else { "open" }
}

fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    use hkask_types::DataCategory;
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
    pub category: Option<String>,
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

#[utoipa::path(
    get, path = "/api/sovereignty/status", tag = "sovereignty",
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
    let cm = &state.service_context.consent_manager;
    let webid_str = auth.webid.to_string();
    let boundary = hkask_types::sovereignty::DataSovereigntyBoundary::hkask_default();
    let granted = cm
        .get_granted_categories(&webid_str)
        .map_err(ApiError::from)?;

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

#[utoipa::path(
    post, path = "/api/sovereignty/consent/grant", tag = "sovereignty",
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
    let cat_str = req.category.unwrap_or_else(|| "all".to_string());
    let cat = parse_data_category(&cat_str);
    let cm = &state.service_context.consent_manager;
    cm.grant_consent(&webid_str, &cat).map_err(ApiError::from)?;
    let granted = cm
        .get_granted_categories(&webid_str)
        .map_err(ApiError::from)?;
    Ok(Json(SovereigntyConsentResponse {
        consent: true,
        message: format!("Explicit consent granted for '{cat_str}'. Data sharing enabled."),
        categories: granted,
    }))
}

#[utoipa::path(
    post, path = "/api/sovereignty/consent/revoke", tag = "sovereignty",
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
    let cm = &state.service_context.consent_manager;
    cm.revoke_consent(&webid_str).map_err(ApiError::from)?;
    Ok(Json(SovereigntyConsentResponse {
        consent: false,
        message: "Explicit consent revoked. Only public data is accessible.".into(),
        categories: vec![],
    }))
}

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
async fn sovereignty_check_access(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AccessCheckResponse>, ApiError> {
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
    let cm = &state.service_context.consent_manager;

    let (classification, access_required) = if boundary.is_sovereign(&cat) {
        ("SOVEREIGN", "Requires explicit consent AND owner")
    } else if boundary.is_category_shared(&cat) {
        ("SHARED", "Requires explicit consent")
    } else if boundary.is_category_public(&cat) {
        ("PUBLIC", "Always accessible")
    } else {
        ("UNKNOWN", "Denied by default")
    };
    let has_consent = if classification == "PUBLIC" {
        true
    } else {
        cm.has_consent(&webid_str, &cat).unwrap_or(false)
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
