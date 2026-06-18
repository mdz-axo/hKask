//! Export routes — sovereignty archive creation for P1 data portability.
//!
//! # REQ: DEP-100 — P1 User Sovereignty: export encrypted triple archive.
//!
//! `POST /api/v1/export/create` — generate and return encrypted sovereignty archive.

use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use tracing;

use crate::ApiState;
use crate::middleware::AuthContext;
use hkask_storage::{BackupArchive, Store, TripleStore};

/// Request body for archive export.
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// User-chosen passphrase for archive encryption.
    pub passphrase: String,
}

/// Response body for archive export.
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    /// File path of the generated archive.
    pub archive_path: String,
    /// Number of triples exported.
    pub triple_count: u64,
    /// Size of the archive in bytes.
    pub bytes: u64,
    /// Duration of the export in milliseconds.
    pub duration_ms: u64,
}

/// POST /api/v1/export/create
///
/// REQ: DEP-105 — generates encrypted sovereignty archive for the authenticated user.
/// pre:  request contains valid AuthContext (session or capability token)
/// pre:  passphrase is ≥8 characters
/// post: archive file created at /var/lib/hkask/exports/{webid}/{timestamp}.db
/// post: returns ExportResponse with path, triple_count, bytes, duration_ms
/// post: CnsSpan::BackupExport emitted
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

    // Build export path
    let export_dir = PathBuf::from("/var/lib/hkask/exports").join(webid.to_string());
    std::fs::create_dir_all(&export_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create export directory: {e}"),
        )
    })?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let archive_path = export_dir.join(format!("{timestamp}.db"));

    // Get TripleStore via UserStore's shared database connection
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

    // Create archive
    let archive = BackupArchive::create(
        archive_path.clone(),
        &req.passphrase,
        &triple_store,
        &webid,
        &domain,
    )
    .map_err(|e| {
        tracing::error!(target: "hkask.api.export", error = %e, "Failed to create archive");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Archive creation failed: {e}"),
        )
    })?;

    let triple_count = archive
        .triple_count()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        as u64;

    let bytes = std::fs::metadata(archive.path())
        .map(|m| m.len())
        .unwrap_or(0);
    let duration_ms = start.elapsed().as_millis() as u64;

    // CNS span
    tracing::info!(
        target = "cns.backup.export",
        webid = %webid,
        triple_count = triple_count,
        bytes = bytes,
        duration_ms = duration_ms,
        "CNS"
    );

    Ok(Json(ExportResponse {
        archive_path: archive.path().to_string_lossy().to_string(),
        triple_count,
        bytes,
        duration_ms,
    }))
}

/// Build the export router.
///
/// REQ: DEP-106
pub fn export_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    OpenApiRouter::new().route("/api/v1/export/create", axum::routing::post(export_create))
}
