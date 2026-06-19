//! Replicant API — list, rename, merge, and delete replicants.
//!
//! REQ: P1-deploy-replicant-manage — P1 User Sovereignty: user manages their replicants.
//! expect: "I can manage my replicants through the API"

use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::ApiState;
use crate::middleware::AuthContext;
use hkask_storage::MergeReceipt;

#[derive(Debug, Serialize)]
pub struct ReplicantInfo {
    pub name: String,
    pub webid: String,
    pub is_primary: bool,
    pub created_at: i64,
    pub last_login: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReplicantListResponse {
    pub replicants: Vec<ReplicantInfo>,
    pub active: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub from: String,
    pub into: String,
}

/// GET /api/v1/replicants
pub async fn list_replicants(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ReplicantListResponse>, (StatusCode, String)> {
    let user_store = state.agent_service.user_store();
    let store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let session_replicant = store
        .get_replicant_by_webid(&auth.webid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .ok_or((StatusCode::NOT_FOUND, "Replicant not found".to_string()))?;
    let replicants = store
        .list_replicants(&session_replicant.user_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?;
    let list: Vec<ReplicantInfo> = replicants
        .into_iter()
        .map(|r| ReplicantInfo {
            name: r.replicant_name,
            webid: r.replicant_webid.to_string(),
            is_primary: r.is_primary,
            created_at: r.created_at,
            last_login: r.last_login,
        })
        .collect();
    let active = list
        .iter()
        .find(|r| r.webid == auth.webid.to_string())
        .map(|r| r.name.clone())
        .unwrap_or_default();
    Ok(Json(ReplicantListResponse {
        replicants: list,
        active,
    }))
}

/// POST /api/v1/replicants/rename
pub async fn rename_replicant(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user_store = state.agent_service.user_store();
    let store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    store
        .rename_replicant(&req.from, &req.to)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{e}")))?;
    Ok(Json(
        serde_json::json!({"status": "renamed", "from": req.from, "to": req.to}),
    ))
}

/// POST /api/v1/replicants/merge
pub async fn merge_replicants(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<MergeRequest>,
) -> Result<Json<MergeReceipt>, (StatusCode, String)> {
    let user_store = state.agent_service.user_store();
    let store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let receipt = store
        .merge_replicant_triples(&req.from, &req.into)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{e}")))?;
    Ok(Json(receipt))
}

/// DELETE /api/v1/replicants/{name}
pub async fn delete_replicant(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user_store = state.agent_service.user_store();
    let store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    store
        .delete_replicant(&name)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{e}")))?;
    Ok(Json(serde_json::json!({"status": "deleted", "name": name})))
}

pub fn replicant_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    OpenApiRouter::new()
        .route("/api/v1/replicants", axum::routing::get(list_replicants))
        .route(
            "/api/v1/replicants/rename",
            axum::routing::post(rename_replicant),
        )
        .route(
            "/api/v1/replicants/merge",
            axum::routing::post(merge_replicants),
        )
        .route(
            "/api/v1/replicants/{name}",
            axum::routing::delete(delete_replicant),
        )
}
