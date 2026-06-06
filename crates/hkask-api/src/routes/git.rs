//! Git CAS archival and resolution routes

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::State, routing::Router};
use hkask_storage::sanitize_path;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

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

/// Create git router
pub fn git_router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/git/archive", axum::routing::post(archive))
        .route("/api/v1/git/resolve/:sha", axum::routing::get(resolve_sha))
}

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
) -> Result<Json<ArchiveResponse>, ApiError> {
    // Validate path to prevent directory traversal
    sanitize_path(&std::path::PathBuf::from("/tmp/hkask-templates"), &req.path).map_err(|e| {
        ApiError::BadRequest {
            message: format!("Path validation failed: {}", e),
        }
    })?;

    // Construct crate name from owner/repo
    let crate_name = format!("{}/{}", req.owner, req.repo);

    let git_cas = state.git_cas.clone();
    let template_crate = git_cas.load_template_crate(&crate_name)?;

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
) -> Result<Json<ResolveShaResponse>, ApiError> {
    // The SHA parameter here is used as a crate identifier for the resolve call.
    // GitCasAdapter.resolve_sha runs `git rev-parse HEAD` against the base path,
    // so we pass the crate name to resolve the current HEAD SHA for that crate.
    let git_cas = state.git_cas.clone();
    let resolved = git_cas.resolve_sha(&sha).map_err(|e| ApiError::Internal {
        message: format!("Failed to resolve SHA: {}", e),
    })?;

    Ok(Json(ResolveShaResponse { sha: resolved }))
}
