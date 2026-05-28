//! User sovereignty routes

use axum::{Json, extract::Query, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiState;

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
}

/// Sovereignty consent response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyConsentResponse {
    pub consent: bool,
    pub message: String,
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
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_status(State(_state): State<ApiState>) -> Json<SovereigntyStatusResponse> {
    use hkask_types::UserSovereigntyState;
    let state = UserSovereigntyState::new();

    Json(SovereigntyStatusResponse {
        explicit_consent: state.explicit_consent,
        sovereignty_compromised: state.is_compromised(),
        kill_zone_active: state.detector.kill_zone_active,
        vc_investment: state.detector.vc_investment,
        threshold: state.detector.threshold,
        acquisition_resistance: format!("{:?}", state.boundary.resistance),
        sovereign_data: state
            .boundary
            .sovereign_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        shared_data: state
            .boundary
            .shared_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        public_data: state
            .boundary
            .public_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
    })
}

/// Grant consent endpoint
async fn sovereignty_grant_consent(
    State(_state): State<ApiState>,
) -> Json<SovereigntyConsentResponse> {
    Json(SovereigntyConsentResponse {
        consent: true,
        message: "Explicit consent granted. Data sharing enabled for shared categories."
            .to_string(),
    })
}

/// Revoke consent endpoint
async fn sovereignty_revoke_consent(
    State(_state): State<ApiState>,
) -> Json<SovereigntyConsentResponse> {
    Json(SovereigntyConsentResponse {
        consent: false,
        message: "Explicit consent revoked. Only public data accessible.".to_string(),
    })
}

/// Kill zone status endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/killzone",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Kill zone status", body = KillZoneResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_killzone(State(_state): State<ApiState>) -> Json<KillZoneResponse> {
    use hkask_types::UserSovereigntyState;
    let state = UserSovereigntyState::new();

    Json(KillZoneResponse {
        active: state.detector.kill_zone_active,
        acquisition_attempt: state.detector.acquisition_attempt,
        vc_investment: state.detector.vc_investment,
        threshold: state.detector.threshold,
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
        (status = 400, description = "Missing category parameter"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_check_access(
    State(_state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<AccessCheckResponse> {
    use hkask_types::UserSovereigntyState;

    let category_str = params.get("category").map(|s| s.as_str()).unwrap_or("");
    let state = UserSovereigntyState::new();

    // Parse category string to DataCategory
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

    Json(AccessCheckResponse {
        category: category_name.to_string(),
        classification,
        access_required,
    })
}

/// Parse a string into a DataCategory
fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    match s {
        "episodic_memory" => hkask_types::DataCategory::EpisodicMemory,
        "semantic_memory" => hkask_types::DataCategory::SemanticMemory,
        "personal_context" => hkask_types::DataCategory::PersonalContext,
        "capability_tokens" => hkask_types::DataCategory::CapabilityTokens,
        "ocap_boundaries" => hkask_types::DataCategory::OcapBoundaries,
        "template_invocations" => hkask_types::DataCategory::TemplateInvocations,
        "hlexicon_terms" => hkask_types::DataCategory::HLexiconTerms,
        "template_registry" => hkask_types::DataCategory::TemplateRegistry,
        _ => hkask_types::DataCategory::Custom(s.to_string()),
    }
}
