//! Backup API routes — snapshot, restore, list, prune, verify, config.
//!
//! All backup operations delegate to `hkask_services::BackupService`,
//! constructed from the `GitCASPort` in `ApiState`.
//!
//! Request/response types use simple serde types (strings, enums) rather
//! than domain types from `hkask-services` to avoid coupling the API
//! surface to `utoipa` derives on domain types.

use axum::extract::Extension;
use axum::{Json, extract::State};
use hkask_services::backup::config::RetentionPolicy;
use hkask_services::{
    ArtifactType, BackupScope, BackupService, ListFilter, RestoreScope, ServiceError,
    SnapshotMetadata,
};
use hkask_types::ports::git_cas::CommitHash;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;
use crate::error::ServiceErrorResponse;
use crate::middleware::AuthContext;

// ── Request/Response types (API-surface only, no domain type coupling) ──

/// Backup scope for API requests.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiBackupScope {
    /// Snapshot all tracked artifact types.
    Full,
    /// Snapshot all artifacts of a single type.
    ByType(String),
    /// Snapshot specific artifacts by ID.
    ByIds {
        artifact_type: String,
        ids: Vec<String>,
    },
}

/// Backup snapshot request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SnapshotRequest {
    pub scope: ApiBackupScope,
}

/// Backup snapshot response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SnapshotResponse {
    pub commits: Vec<CommitInfo>,
    pub artifact_count: usize,
    pub trigger: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommitInfo {
    pub repo: String,
    pub commit: String,
}

/// Backup restore scope for API requests.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiRestoreScope {
    Full,
    ByType(String),
    ByIds {
        artifact_type: String,
        ids: Vec<String>,
    },
}

/// Backup restore request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RestoreRequest {
    pub commit_hash: String,
    pub scope: ApiRestoreScope,
}

/// Backup restore response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RestoreResponse {
    pub artifacts: Vec<RestoredArtifact>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RestoredArtifact {
    pub artifact_type: String,
    pub artifact_id: String,
}

/// Backup list query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListQuery {
    /// Filter by artifact type.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Maximum snapshots to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Backup list response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse {
    pub snapshots: Vec<SnapshotResponse>,
}

/// Backup prune request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PruneRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

/// Backup prune response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PruneResponse {
    pub dry_run: bool,
    pub evaluated: usize,
    pub removed: Vec<CommitInfo>,
    pub retained: usize,
}

/// Backup verify response.
#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyResponse {
    pub reports: Vec<RepoVerifyReport>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RepoVerifyReport {
    pub repo: String,
    pub total_blobs: usize,
    pub verified_blobs: usize,
    pub corrupt_hashes: Vec<String>,
    pub ok: bool,
}

/// Backup config response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BackupConfigResponse {
    pub tracked_types: Vec<String>,
    pub auto_snapshot: bool,
    pub verify_after_snapshot: bool,
    pub retention: Option<RetentionConfigResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RetentionConfigResponse {
    pub daily_days: u32,
    pub weekly_weeks: u32,
}

/// Backup config update request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateConfigRequest {
    pub tracked_types: Option<Vec<String>>,
    pub retention: Option<String>,
    pub auto_snapshot: Option<bool>,
    pub verify_after_snapshot: Option<bool>,
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn backup_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(snapshot))
        .routes(routes!(restore))
        .routes(routes!(list_snapshots))
        .routes(routes!(prune))
        .routes(routes!(verify))
        .routes(routes!(get_config))
        .routes(routes!(update_config))
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn backup_service(state: &ApiState) -> BackupService {
    BackupService::new(state.git_cas_port.clone())
}

fn parse_artifact_type(s: &str) -> Option<ArtifactType> {
    match s {
        "template" => Some(ArtifactType::Template),
        "style" => Some(ArtifactType::Style),
        "goal" => Some(ArtifactType::Goal),
        "spec" => Some(ArtifactType::Spec),
        "memory" | "memory_triple" => Some(ArtifactType::MemoryTriple),
        "embedding" => Some(ArtifactType::Embedding),
        "registry" | "registry_entry" => Some(ArtifactType::RegistryEntry),
        "cns" | "cns_audit" => Some(ArtifactType::CnsAudit),
        "sovereignty" | "sovereignty_manifest" => Some(ArtifactType::SovereigntyManifest),
        "session" => Some(ArtifactType::Session),
        "wallet" | "wallet_state" => Some(ArtifactType::WalletState),
        "settings" => Some(ArtifactType::Settings),
        _ => None,
    }
}

fn api_scope_to_domain(scope: ApiBackupScope) -> Result<BackupScope, ServiceError> {
    match scope {
        ApiBackupScope::Full => Ok(BackupScope::Full),
        ApiBackupScope::ByType(s) => {
            let at = parse_artifact_type(&s).ok_or_else(|| {
                ServiceError::ValidationError(format!("Unknown artifact type: {s}"))
            })?;
            Ok(BackupScope::ByType(at))
        }
        ApiBackupScope::ByIds { artifact_type, ids } => {
            let at = parse_artifact_type(&artifact_type).ok_or_else(|| {
                ServiceError::ValidationError(format!("Unknown artifact type: {artifact_type}"))
            })?;
            Ok(BackupScope::ByIds {
                artifact_type: at,
                ids,
            })
        }
    }
}

fn api_restore_scope_to_domain(scope: ApiRestoreScope) -> Result<RestoreScope, ServiceError> {
    match scope {
        ApiRestoreScope::Full => Ok(RestoreScope::Full),
        ApiRestoreScope::ByType(s) => {
            let at = parse_artifact_type(&s).ok_or_else(|| {
                ServiceError::ValidationError(format!("Unknown artifact type: {s}"))
            })?;
            Ok(RestoreScope::ByType(at))
        }
        ApiRestoreScope::ByIds { artifact_type, ids } => {
            let at = parse_artifact_type(&artifact_type).ok_or_else(|| {
                ServiceError::ValidationError(format!("Unknown artifact type: {artifact_type}"))
            })?;
            Ok(RestoreScope::ByIds {
                artifact_type: at,
                ids,
            })
        }
    }
}

fn snapshot_to_response(snap: &SnapshotMetadata) -> SnapshotResponse {
    SnapshotResponse {
        commits: snap
            .commits
            .iter()
            .map(|(repo, commit)| CommitInfo {
                repo: repo.dir_name().to_string(),
                commit: commit.to_string(),
            })
            .collect(),
        artifact_count: snap.artifact_count,
        trigger: format!("{:?}", snap.trigger).to_lowercase(),
        timestamp: snap.timestamp.to_rfc3339(),
    }
}

// ── Route handlers ──────────────────────────────────────────────────────

/// Create a backup snapshot.
#[utoipa::path(
    post,
    path = "/api/v1/backup/snapshot",
    tag = "backup",
    request_body = SnapshotRequest,
    responses(
        (status = 200, description = "Snapshot created", body = SnapshotResponse),
        (status = 400, description = "Invalid scope or untracked type"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn snapshot(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<SnapshotRequest>,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let svc = backup_service(&state);
    let scope = api_scope_to_domain(req.scope)?;

    let result = svc
        .snapshot(scope, &[])
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Snapshot failed: {e}"),
        })?;

    Ok(Json(snapshot_to_response(&result)))
}

/// Restore artifacts from a backup snapshot.
#[utoipa::path(
    post,
    path = "/api/v1/backup/restore",
    tag = "backup",
    request_body = RestoreRequest,
    responses(
        (status = 200, description = "Artifacts restored", body = RestoreResponse),
        (status = 400, description = "Invalid commit hash or scope"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn restore(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<RestoreResponse>, ApiError> {
    let svc = backup_service(&state);
    let scope = api_restore_scope_to_domain(req.scope)?;

    let commit_hash: CommitHash =
        req.commit_hash
            .parse()
            .map_err(|e: String| ApiError::BadRequest {
                message: format!("Invalid commit hash: {e}"),
            })?;

    let artifacts = svc
        .restore(&commit_hash, scope)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Restore failed: {e}"),
        })?;

    Ok(Json(RestoreResponse {
        artifacts: artifacts
            .into_iter()
            .map(|(at, id, _bytes)| RestoredArtifact {
                artifact_type: at.label().to_string(),
                artifact_id: id,
            })
            .collect(),
    }))
}

/// List backup snapshots.
#[utoipa::path(
    get,
    path = "/api/v1/backup/list",
    tag = "backup",
    params(ListQuery),
    responses(
        (status = 200, description = "Snapshots listed", body = ListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn list_snapshots(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    axum::extract::Query(query): axum::extract::Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    let svc = backup_service(&state);

    let filter = ListFilter {
        artifact_type: query.r#type.as_deref().and_then(parse_artifact_type),
        limit: Some(query.limit),
    };

    let snapshots = svc.list(filter).await.map_err(|e| ApiError::Internal {
        message: format!("List failed: {e}"),
    })?;

    Ok(Json(ListResponse {
        snapshots: snapshots.iter().map(snapshot_to_response).collect(),
    }))
}

/// Prune expired backup snapshots.
#[utoipa::path(
    post,
    path = "/api/v1/backup/prune",
    tag = "backup",
    request_body = PruneRequest,
    responses(
        (status = 200, description = "Prune report", body = PruneResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn prune(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<PruneRequest>,
) -> Result<Json<PruneResponse>, ApiError> {
    let svc = backup_service(&state);

    let report = svc
        .prune(req.dry_run)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Prune failed: {e}"),
        })?;

    Ok(Json(PruneResponse {
        dry_run: report.dry_run,
        evaluated: report.evaluated,
        removed: report
            .removed
            .iter()
            .map(|(repo, commit)| CommitInfo {
                repo: repo.dir_name().to_string(),
                commit: commit.to_string(),
            })
            .collect(),
        retained: report.retained,
    }))
}

/// Verify backup integrity.
#[utoipa::path(
    post,
    path = "/api/v1/backup/verify",
    tag = "backup",
    responses(
        (status = 200, description = "Integrity report", body = VerifyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn verify(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<VerifyResponse>, ApiError> {
    let svc = backup_service(&state);

    let reports = svc.verify().await.map_err(|e| ApiError::Internal {
        message: format!("Verify failed: {e}"),
    })?;

    Ok(Json(VerifyResponse {
        reports: reports
            .into_iter()
            .map(|r| RepoVerifyReport {
                repo: r.repo.dir_name().to_string(),
                total_blobs: r.total_blobs,
                verified_blobs: r.verified_blobs,
                corrupt_hashes: r.corrupt_hashes.iter().map(|h| h.to_string()).collect(),
                ok: r.corrupt_hashes.is_empty(),
            })
            .collect(),
    }))
}

/// Get current backup configuration.
#[utoipa::path(
    get,
    path = "/api/v1/backup/config",
    tag = "backup",
    responses(
        (status = 200, description = "Backup configuration", body = BackupConfigResponse),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub(crate) async fn get_config(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<BackupConfigResponse>, ApiError> {
    let svc = backup_service(&state);
    let config = svc.config();

    Ok(Json(BackupConfigResponse {
        tracked_types: config
            .tracked_types
            .iter()
            .map(|at| at.label().to_string())
            .collect(),
        auto_snapshot: config.auto_snapshot,
        verify_after_snapshot: config.verify_after_snapshot,
        retention: config.retention.as_ref().map(|rp| RetentionConfigResponse {
            daily_days: rp.daily_days,
            weekly_weeks: rp.weekly_weeks,
        }),
    }))
}

/// Update backup configuration.
#[utoipa::path(
    put,
    path = "/api/v1/backup/config",
    tag = "backup",
    request_body = UpdateConfigRequest,
    responses(
        (status = 200, description = "Configuration updated", body = BackupConfigResponse),
        (status = 400, description = "Invalid configuration"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub(crate) async fn update_config(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<BackupConfigResponse>, ApiError> {
    let mut svc = backup_service(&state);
    let mut config = svc.config().clone();

    if let Some(types) = &req.tracked_types {
        config.tracked_types = types
            .iter()
            .filter_map(|s| parse_artifact_type(s))
            .collect();
    }

    if let Some(dur_str) = &req.retention {
        let days: u32 = dur_str.trim_end_matches('d').parse().unwrap_or(21);
        config.retention = Some(RetentionPolicy {
            daily_days: days,
            weekly_weeks: 12,
        });
    }

    if let Some(auto) = req.auto_snapshot {
        config.auto_snapshot = auto;
    }

    if let Some(verify) = req.verify_after_snapshot {
        config.verify_after_snapshot = verify;
    }

    svc.update_config(config).map_err(|e| ApiError::Internal {
        message: format!("Failed to update config: {e}"),
    })?;

    let updated = svc.config();
    Ok(Json(BackupConfigResponse {
        tracked_types: updated
            .tracked_types
            .iter()
            .map(|at| at.label().to_string())
            .collect(),
        auto_snapshot: updated.auto_snapshot,
        verify_after_snapshot: updated.verify_after_snapshot,
        retention: updated
            .retention
            .as_ref()
            .map(|rp| RetentionConfigResponse {
                daily_days: rp.daily_days,
                weekly_weeks: rp.weekly_weeks,
            }),
    }))
}
