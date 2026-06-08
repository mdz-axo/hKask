//! Database store initialization for the API server.
//!
//! Extracted from `ApiState::new()` so that store creation is composable
//! and independently testable. Each store gets its own database connection
//! (and therefore its own connection pool) so a slow store cannot starve another.

use std::sync::Arc;

use hkask_agents::consent::ConsentManager;
use hkask_agents::escalation::EscalationQueue;
use hkask_types::ports::git_cas::GitCASPort;

use crate::error::ApiError;

/// Database configuration for persistent storage.
pub struct DbConfig {
    pub path: Option<String>,
    pub passphrase: Option<String>,
}

/// Database-backed stores initialized from a `DbConfig`.
///
/// Extracted from `ApiState::new()` so that store creation is composable
/// and independently testable.
pub(crate) struct Stores {
    pub consent_manager: Arc<ConsentManager>,
    pub escalation_queue: Arc<EscalationQueue>,
    pub goal_repo: Arc<hkask_storage::SqliteGoalRepository>,
    pub standing_session_store: Arc<hkask_storage::StandingSessionStore>,
}

impl Stores {
    /// Open and initialise all persistent stores.
    ///
    /// Each store gets its own database connection (and therefore its own
    /// connection pool) so a slow store cannot starve another.
    ///
    /// The `git_cas_port` is injected into stores that support CAS write-through
    /// via their `.with_cas()` builder methods, enabling per-mutation audit
    /// trails alongside batch snapshots from the SnapshotLoop.
    pub(crate) fn init(
        db_config: Option<&DbConfig>,
        git_cas_port: Arc<dyn GitCASPort>,
    ) -> Result<Stores, ApiError> {
        let consent_conn = open_db(db_config, "consent")?.conn_arc();
        let consent_store =
            hkask_storage::ConsentStore::new(consent_conn).with_cas(Arc::clone(&git_cas_port));
        consent_store
            .initialize_schema()
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to initialize consent store schema: {e}"),
            })?;
        let consent_manager = Arc::new(ConsentManager::new(consent_store));

        let escalation_conn = open_db(db_config, "escalation")?.conn_arc();
        let escalation_queue =
            Arc::new(
                EscalationQueue::new(escalation_conn).map_err(|e| ApiError::Internal {
                    message: format!("Failed to initialize escalation queue: {e}"),
                })?,
            );

        let goal_conn = open_db(db_config, "goal")?.conn_arc();
        let goal_sink: Arc<dyn hkask_types::event::NuEventSink> =
            Arc::new(hkask_storage::NuEventStore::new(Arc::clone(&goal_conn)));
        let goal_repo = Arc::new(
            hkask_storage::SqliteGoalRepository::new(goal_conn)
                .with_telemetry(goal_sink)
                .with_cas(Arc::clone(&git_cas_port)),
        );

        let standing_conn = open_db(db_config, "standing session")?.conn_arc();
        let standing_session_store = hkask_storage::StandingSessionStore::new(standing_conn)
            .with_cas(Arc::clone(&git_cas_port));
        standing_session_store
            .initialize_schema()
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to initialize standing session store schema: {e}"),
            })?;
        let standing_session_store = Arc::new(standing_session_store);

        Ok(Stores {
            consent_manager,
            escalation_queue,
            goal_repo,
            standing_session_store,
        })
    }
}

/// Open a persistent database, or fall back to in-memory with a warning.
///
/// Extracts the repeated pattern of `db_config.and_then(...)` → `Database::open`
/// that appeared 4 times in `ApiState::new()`. Returns the `Database`,
/// so callers can extract `.conn_arc()` or use it directly.
pub(crate) fn open_db(
    db_config: Option<&DbConfig>,
    purpose: &str,
) -> Result<hkask_storage::Database, ApiError> {
    match db_config.and_then(|c| c.path.as_deref().zip(c.passphrase.as_deref())) {
        Some((path, passphrase)) => {
            hkask_storage::Database::open(path, passphrase).map_err(|e| ApiError::Internal {
                message: format!("Failed to open {purpose} database: {e}"),
            })
        }
        None => {
            tracing::warn!(
                target: "hkask.api",
                "No persistent database configured — {purpose} store is in-memory and will be lost on restart. \
                 Set HKASK_DB_PATH and HKASK_DB_PASSPHRASE for sovereign persistence."
            );
            Ok(hkask_storage::in_memory_db())
        }
    }
}
