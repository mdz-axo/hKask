//! Admin API routes — invite management and session listing.
//!
//! All admin routes are gated by the admin middleware (Role::Admin required).
//! These implement the multi-user contracts from FUNCTIONAL_SPECIFICATION.md §3.16.

use crate::middleware::AuthContext;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::ApiState;

/// POST /api/v1/admin/invite
///
/// expect: "As an admin I can create an invite code for a new member"
pub async fn create_invite(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_store = state.agent_service.storage().users.clone();
    let user_store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let replicant = user_store
        .get_replicant_by_webid(&auth.webid)
        .map_err(|e| (StatusCode::FORBIDDEN, format!("{e}")))?
        .ok_or((StatusCode::FORBIDDEN, "Replicant not found".into()))?;
    let invite = user_store.create_invite(&replicant.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invite creation failed: {e}"),
        )
    })?;
    Ok(Json(InviteResponse { code: invite.code }))
}

/// GET /api/v1/admin/invite
///
/// expect: "As an admin I can see all invites I've sent and their status"
pub async fn list_invites(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_store = state.agent_service.storage().users.clone();
    let user_store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let replicant = user_store
        .get_replicant_by_webid(&auth.webid)
        .map_err(|e| (StatusCode::FORBIDDEN, format!("{e}")))?
        .ok_or((StatusCode::FORBIDDEN, "Replicant not found".into()))?;
    let invites = user_store.list_invites(&replicant.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("List invites failed: {e}"),
        )
    })?;
    Ok(Json(invites))
}

/// GET /api/v1/admin/sessions
///
/// expect: "As an admin I can see all active sessions on my server"
pub async fn list_sessions(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_store = state.agent_service.storage().users.clone();
    let user_store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let sessions = user_store.list_all_sessions().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("List sessions failed: {e}"),
        )
    })?;
    Ok(Json(sessions))
}

/// GET /api/v1/admin/config
///
/// expect: "As an admin I can view the server configuration"
pub async fn get_config(
    State(_): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "session_count": "available via GET /api/v1/admin/sessions",
    })))
}

#[derive(Serialize)]
struct InviteResponse {
    code: String,
}
