//! Git CAS archival and resolution routes

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::State, http::StatusCode, routing::Router};
use hkask_storage::sanitize_path;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::middleware::AuthContext;
use crate::{ApiState, ErrorResponse};

// ── Request / Response types ──────────────────────────────────────────────

/// Archive repository request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ArchiveRequest {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub path: String,
}

/// Archive entry response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArchiveEntry {
    pub name: String,
    pub path: String,
    pub content: String,
    pub template_type: String,
}

/// Archive response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArchiveResponse {
    pub crate_name: String,
    pub git_sha: String,
    pub templates: Vec<ArchiveEntry>,
}

/// SHA resolve response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResolveShaResponse {
    pub sha: String,
}

// ── Router ───────────────────────────────────────────────────────────────

/// Create git router
pub fn git_router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/git/archive", axum::routing::post(archive))
        .route("/api/v1/git/resolve/:sha", axum::routing::get(resolve_sha))
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// Archive a repository template crate
#[utoipa::path(
    post,
    path = "/api/v1/git/archive",
    tag = "git",
    request_body = ArchiveRequest,
    responses(
        (status = 200, description = "Template crate archived", body = ArchiveResponse),
        (status = 400, description = "Invalid path or missing crate"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn archive(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<ArchiveRequest>,
) -> Result<Json<ArchiveResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.git.archive.start"),
        Phase::Observe,
        serde_json::json!({
            "owner": req.owner,
            "repo": req.repo,
            "branch": req.branch,
            "path": req.path,
        }),
    );

    // Validate path to prevent directory traversal
    let base = std::path::PathBuf::from("/tmp/hkask-templates");
    sanitize_path(&base, &req.path).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_path".to_string(),
                code: "GIT_BAD_REQUEST".to_string(),
                details: Some(serde_json::json!({
                    "message": format!("Path validation failed: {}", e)
                })),
            }),
        )
    })?;

    // Construct crate name from owner/repo
    let crate_name = format!("{}/{}", req.owner, req.repo);

    let git_cas = state.git_cas.clone();
    let template_crate = git_cas.load_template_crate(&crate_name).map_err(|e| {
        state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.git.archive.error"),
        Phase::Observe,
            serde_json::json!({ "error": e.to_string() }),
        );
        match e {
            hkask_agents::GitError::CrateNotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "crate_not_found".to_string(),
                    code: "GIT_NOT_FOUND".to_string(),
                    details: Some(serde_json::json!({
                        "message": format!("Template crate '{}' not found", crate_name)
                    })),
                }),
            ),
            hkask_agents::GitError::InvalidPath(_) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_path".to_string(),
                    code: "GIT_BAD_REQUEST".to_string(),
                    details: Some(serde_json::json!({
                        "message": format!("Invalid path: {}", e)
                    })),
                }),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "archive_failed".to_string(),
                    code: "GIT_ERROR".to_string(),
                    details: Some(serde_json::json!({
                        "message": e.to_string()
                    })),
                }),
            ),
        }
    })?;

    let sha = git_cas
        .resolve_sha(&crate_name)
        .unwrap_or_else(|_| "0000000000000000000000000000000000000000".to_string());

    let templates: Vec<ArchiveEntry> = template_crate
        .templates
        .iter()
        .map(|t| ArchiveEntry {
            name: t.path.clone(),
            path: t.path.clone(),
            content: t.content.clone(),
            template_type: t.template_type.clone(),
        })
        .collect();

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.git.archive.success"),
        Phase::Observe,
        serde_json::json!({
            "crate_name": crate_name,
            "sha": sha,
            "template_count": templates.len(),
        }),
    );

    Ok(Json(ArchiveResponse {
        crate_name: template_crate.name,
        git_sha: sha,
        templates,
    }))
}

/// Resolve a git SHA
#[utoipa::path(
    get,
    path = "/api/v1/git/resolve/{sha}",
    tag = "git",
    responses(
        (status = 200, description = "SHA resolved", body = ResolveShaResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn resolve_sha(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(sha): Path<String>,
) -> Result<Json<ResolveShaResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.git.resolve.start"),
        Phase::Observe,
        serde_json::json!({
            "sha": sha,
        }),
    );

    // The SHA parameter here is used as a crate identifier for the resolve call.
    // GitCasAdapter.resolve_sha runs `git rev-parse HEAD` against the base path,
    // so we pass the crate name to resolve the current HEAD SHA for that crate.
    let git_cas = state.git_cas.clone();
    let resolved = git_cas.resolve_sha(&sha).map_err(|e| {
        state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.git.resolve.error"),
        Phase::Observe,
            serde_json::json!({ "error": e.to_string() }),
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "resolve_failed".to_string(),
                code: "GIT_ERROR".to_string(),
                details: Some(serde_json::json!({
                    "message": format!("Failed to resolve SHA: {}", e)
                })),
            }),
        )
    })?;

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.git.resolve.success"),
        Phase::Observe,
        serde_json::json!({
            "resolved_sha": resolved,
        }),
    );

    Ok(Json(ResolveShaResponse { sha: resolved }))
}
