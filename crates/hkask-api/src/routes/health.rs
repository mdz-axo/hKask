//! Health check endpoint — Kubernetes readiness probe.
//!
//! Verifies:
//! - Database is reachable (real SQL query via AgentRegistryStore)
//! - Matrix/Conduit is reachable (HTTP to Conduit's versions endpoint)
//!
//! Returns 503 with a JSON body listing failures if either check fails.

use crate::ApiState;
use axum::extract::State;
use axum::response::{IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    healthy: bool,
    db: bool,
    conduit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// GET /health — readiness probe. Checks DB + Matrix connectivity.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "All checks passed"),
        (status = 503, description = "One or more checks failed"),
    ),
)]
pub async fn health_check(State(state): State<ApiState>) -> impl IntoResponse {
    let db_ok = check_db(&state);
    let conduit_ok = check_conduit().await;

    let healthy = db_ok && conduit_ok;

    let error = if !healthy {
        let mut parts = Vec::new();
        if !db_ok {
            parts.push("database unreachable");
        }
        if !conduit_ok {
            parts.push("conduit unreachable");
        }
        Some(parts.join("; "))
    } else {
        None
    };

    let body = HealthResponse {
        healthy,
        db: db_ok,
        conduit: conduit_ok,
        error,
    };

    let status = if healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(body)).into_response()
}

fn check_db(state: &ApiState) -> bool {
    // AgentRegistryStore::get_user_profile() performs a real SELECT
    // against the SQLite database — it exercises lock → prepare → query.
    // The result (Some/None) doesn't matter; we just need the query to succeed.
    state
        .agent_service
        .storage()
        .agents
        .get_user_profile()
        .is_ok()
}

async fn check_conduit() -> bool {
    let url = match std::env::var("HKASK_MATRIX_URL") {
        Ok(u) => format!("{u}/_matrix/client/versions"),
        Err(_) => return false,
    };
    match reqwest::get(&url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
