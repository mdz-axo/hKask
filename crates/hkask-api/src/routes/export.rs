//! Export routes — sovereignty archive creation and migration for P1 data portability.
//!
//! # REQ: DEP-100 — P1 User Sovereignty: export/upload encrypted triple archive.
//! expect: "My API access is scoped to my sovereignty boundaries" [P1]
//!
//! `POST /api/v1/export/create` — generate encrypted sovereignty archive.
//! `POST /api/v1/export/upload` — upload archive for server migration.

use hkask_rsolidity as rs;
use axum::{Extension, Json, extract::State, http::StatusCode, response::Response};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;
use tracing;

use crate::ApiState;
use crate::middleware::AuthContext;
use hkask_storage::{BackupArchive, MigrationReceipt, Store, TripleStore};

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub passphrase: String,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub archive_path: String,
    pub triple_count: u64,
    pub bytes: u64,
    pub duration_ms: u64,
}

/// POST /api/v1/export/create
pub async fn export_create(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<ExportRequest>,
) -> Result<Json<ExportResponse>, (StatusCode, String)> {
    let start = Instant::now();
    if req.passphrase.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Passphrase must be at least 8 characters".to_string(),
        ));
    }
    let webid = auth.webid;
    let export_dir = PathBuf::from("/var/lib/hkask/exports").join(webid.to_string());
    std::fs::create_dir_all(&export_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create export directory: {e}"),
        )
    })?;
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let archive_path = export_dir.join(format!("{timestamp}.db"));
    let user_store = state.agent_service.user_store();
    let triple_store = {
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lock error: {e}"),
            )
        })?;
        TripleStore::new(store.conn_arc())
    };
    let domain = std::env::var("HKASK_DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    let archive = BackupArchive::create(
        archive_path.clone(),
        &req.passphrase,
        &triple_store,
        &webid,
        &domain,
    )
    .map_err(|e| {
        let status = if matches!(e, hkask_storage::ArchiveError::Empty) {
            StatusCode::BAD_REQUEST
        } else {
            tracing::error!(target: "hkask.api.export", error = %e, "Failed to create archive");
            StatusCode::INTERNAL_SERVER_ERROR
        };
        (status, format!("Archive creation failed: {e}"))
    })?;
    let triple_count = archive
        .triple_count()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        as u64;
    let bytes = std::fs::metadata(archive.path())
        .map(|m| m.len())
        .unwrap_or(0);
    let duration_ms = start.elapsed().as_millis() as u64;
    tracing::info!(target = "cns.backup.export", webid = %webid, triple_count = triple_count, bytes = bytes, duration_ms = duration_ms, "CNS");
    Ok(Json(ExportResponse {
        archive_path: archive.path().to_string_lossy().to_string(),
        triple_count,
        bytes,
        duration_ms,
    }))
}

// ── Upload ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub archive_base64: String,
    pub passphrase: String,
}

/// POST /api/v1/export/upload
///
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
pub async fn export_upload(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<UploadRequest>,
) -> Result<Json<MigrationReceipt>, (StatusCode, String)> {
    let start = Instant::now();
    if req.passphrase.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Passphrase must be at least 8 characters".to_string(),
        ));
    }
    let archive_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.archive_base64)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid base64: {e}")))?;
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("hkask-upload-{}.db", uuid::Uuid::new_v4()));
    std::fs::write(&tmp_path, &archive_bytes).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write temp file: {e}"),
        )
    })?;
    let archive = BackupArchive::open(tmp_path.clone(), &req.passphrase).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        let msg = if e.to_string().to_lowercase().contains("sqlite")
            || e.to_string().contains("not a database")
        {
            "Wrong passphrase or corrupted archive file".to_string()
        } else {
            format!("Failed to open archive: {e}")
        };
        (StatusCode::BAD_REQUEST, msg)
    })?;
    let user_store = state.agent_service.user_store();
    let existing_names: HashSet<String> = {
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lock error: {e}"),
            )
        })?;
        store
            .list_all_replicant_names()
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to list replicants: {e}"),
                )
            })?
            .into_iter()
            .collect()
    };
    let triple_store = {
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lock error: {e}"),
            )
        })?;
        TripleStore::new(store.conn_arc())
    };
    let webid = auth.webid;
    let receipt = archive
        .import_into(&triple_store, &webid, &existing_names)
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Import failed: {e}"),
            )
        })?;
    let duration_ms = start.elapsed().as_millis() as u64;
    tracing::info!(target = "cns.backup.upload", webid = %webid, triple_count = receipt.triple_count, duration_ms = duration_ms, "CNS");
    let _ = std::fs::remove_file(&tmp_path);
    Ok(Json(receipt))
}

/// Build the export router.
pub fn export_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    OpenApiRouter::new()
        .route("/api/v1/export/create", axum::routing::post(export_create))
        .route("/api/v1/export/upload", axum::routing::post(export_upload))
        .route(
            "/api/v1/export/download",
            axum::routing::get(export_download),
        )
}

/// GET /api/v1/export/download — download the latest export archive for the authenticated user.
///
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
pub async fn export_download(
    State(_state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Response, (StatusCode, String)> {
    let webid = auth.webid;
    let export_dir = PathBuf::from("/var/lib/hkask/exports").join(webid.to_string());

    // Find the latest archive
    let mut entries: Vec<_> = std::fs::read_dir(&export_dir)
        .map_err(|_| (StatusCode::NOT_FOUND, "No exports found".to_string()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "db"))
        .collect();
    entries.sort_by_key(|e| std::fs::metadata(e.path()).and_then(|m| m.modified()).ok());
    entries.reverse();

    let latest = entries
        .first()
        .ok_or((StatusCode::NOT_FOUND, "No exports found".to_string()))?;
    let bytes = std::fs::read(latest.path()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read archive: {e}"),
        )
    })?;

    let filename = latest.file_name().to_string_lossy().to_string();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(axum::body::Body::from(bytes))
        .unwrap())
}
