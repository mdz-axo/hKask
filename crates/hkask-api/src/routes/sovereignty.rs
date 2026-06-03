//! User sovereignty routes

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State, http::StatusCode, routing::Router};
use hkask_types::DataCategory;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::middleware::AuthContext;
use crate::{ApiState, ErrorResponse};

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
            "/api/sovereignty/killzone",
            axum::routing::get(sovereignty_killzone),
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
    pub sovereignty_compromised: bool,
    pub kill_zone_active: bool,
    pub vc_investment: f32,
    pub threshold: f32,
    pub acquisition_resistance: String,
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

/// Kill zone status response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct KillZoneResponse {
    pub active: bool,
    pub acquisition_attempt: bool,
    pub vc_investment: f32,
    pub threshold: f32,
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
) -> Result<Json<SovereigntyStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    use hkask_types::UserSovereigntyState;
    let sovereignty_state = UserSovereigntyState::new();

    let webid_str = auth.webid.to_string();

    // Enrich status with consent manager state
    let granted_categories = state
        .consent_manager
        .get_granted_categories(&webid_str)
        .unwrap_or_default()
        .into_iter()
        .collect();

    Ok(Json(SovereigntyStatusResponse {
        explicit_consent: sovereignty_state.explicit_consent,
        sovereignty_compromised: sovereignty_state.is_compromised(),
        kill_zone_active: sovereignty_state.kill_zone_state.kill_zone_active,
        vc_investment: sovereignty_state.kill_zone_state.vc_investment,
        threshold: sovereignty_state.kill_zone_threshold,
        acquisition_resistance: sovereignty_state.boundary.resistance.to_string(),
        sovereign_data: sovereignty_state
            .boundary
            .sovereign_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        shared_data: sovereignty_state
            .boundary
            .shared_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        public_data: sovereignty_state
            .boundary
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
) -> Result<Json<SovereigntyConsentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let webid_str = auth.webid.to_string();
    let category_str = req.category.unwrap_or_else(|| "all".to_string());
    let category = parse_data_category(&category_str);

    state
        .consent_manager
        .grant_consent(&webid_str, &category)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "consent_grant_failed".to_string(),
                    code: "SOVEREIGNTY_ERROR".to_string(),
                    details: Some(serde_json::json!({ "message": e.to_string() })),
                }),
            )
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
) -> Result<Json<SovereigntyConsentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let webid_str = auth.webid.to_string();

    state
        .consent_manager
        .revoke_consent(&webid_str)
        .map_err(|e| match e {
            hkask_agents::ConsentError::ConsentNotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "consent_not_found".to_string(),
                    code: "SOVEREIGNTY_NOT_FOUND".to_string(),
                    details: Some(serde_json::json!({
                        "message": format!("No consent record found for WebID: {}", webid_str)
                    })),
                }),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "consent_revoke_failed".to_string(),
                    code: "SOVEREIGNTY_ERROR".to_string(),
                    details: Some(serde_json::json!({ "message": e.to_string() })),
                }),
            ),
        })?;

    Ok(Json(SovereigntyConsentResponse {
        consent: false,
        message: "Explicit consent revoked. Only public data is accessible.".to_string(),
        categories: vec![],
    }))
}

/// Kill zone status endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/killzone",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Kill zone status", body = KillZoneResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_killzone(
    State(_state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Json<KillZoneResponse> {
    use hkask_types::UserSovereigntyState;
    let state = UserSovereigntyState::new();

    Json(KillZoneResponse {
        active: state.kill_zone_state.kill_zone_active,
        acquisition_attempt: state.kill_zone_state.acquisition_attempt,
        vc_investment: state.kill_zone_state.vc_investment,
        threshold: state.kill_zone_threshold,
    })
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
    State(_state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AccessCheckResponse>, (StatusCode, Json<ErrorResponse>)> {
    use hkask_types::UserSovereigntyState;

    let category_str = params.get("category").map(|s| s.as_str()).unwrap_or("");
    if category_str.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_parameter".to_string(),
                code: "SOVEREIGNTY_BAD_REQUEST".to_string(),
                details: Some(serde_json::json!({
                    "message": "Missing required query parameter: category"
                })),
            }),
        ));
    }

    let state = UserSovereigntyState::new();
    let category = parse_data_category(category_str);
    let category_name = category.as_str();

    let (classification, access_required) = if state.boundary.is_sovereign(&category) {
        (
            "SOVEREIGN".to_string(),
            "Requires explicit consent AND owner".to_string(),
        )
    } else if state.boundary.is_shared(&category) {
        (
            "SHARED".to_string(),
            "Requires explicit consent".to_string(),
        )
    } else if state.boundary.is_public(&category) {
        ("PUBLIC".to_string(), "Always accessible".to_string())
    } else {
        ("UNKNOWN".to_string(), "Denied by default".to_string())
    };

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
