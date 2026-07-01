//! Foundation: database connections, stores, CNS runtime, seam watcher.

use super::super::*;
use hkask_services_core::ServiceError;
use tokio::sync::RwLock;

/// Foundation: database connections, stores, CNS runtime, seam watcher.
pub(super) struct Foundation {
    pub db: Database,
    pub primary_conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
    pub curation_inbox_tx: tokio::sync::mpsc::UnboundedSender<CurationInput>,
    pub curation_inbox_rx: Option<tokio::sync::mpsc::UnboundedReceiver<CurationInput>>,
    pub consent_manager: Arc<ConsentManager>,
    pub escalation_queue: Arc<EscalationQueue>,
    pub goal_repo: Arc<SqliteGoalRepository>,
    pub sovereignty_boundary_store: SovereigntyBoundaryStore,
    pub spec_store: SqliteSpecStore,
    pub user_store: Arc<std::sync::Mutex<UserStore>>,
    pub cns_runtime: Arc<RwLock<CnsRuntime>>,
    pub seam_watcher: Arc<RwLock<Option<SeamWatcher>>>,
    pub cns_event_sink: Arc<dyn NuEventSink>,
    /// Abstracted event store for gas report queries and calibration.
    pub gas_event_store: Arc<dyn CnsStoragePort>,
}

pub(super) async fn build_foundation(config: &ServiceConfig) -> Result<Foundation, ServiceError> {
    let db = if config.in_memory {
        in_memory_db()
    } else {
        Database::open(&config.db_path, &config.db_passphrase).map_err(|e| {
            ServiceError::Storage {
                source: None,
                message: e.to_string(),
            }
        })?
    };
    let shared_conn = db.conn_arc();

    let primary_conn = Arc::clone(&shared_conn);
    let gas_store: Arc<NuEventStore> = Arc::new(NuEventStore::new(Arc::clone(&primary_conn)));
    let gas_event_store: Arc<dyn CnsStoragePort> =
        Arc::clone(&gas_store) as Arc<dyn CnsStoragePort>;
    let cns_event_sink: Arc<dyn NuEventSink> = Arc::clone(&gas_store) as Arc<dyn NuEventSink>;

    let consent_conn = Arc::clone(&shared_conn);
    let escalation_conn = Arc::clone(&shared_conn);
    let goal_conn = Arc::clone(&shared_conn);
    let sovereignty_conn = Arc::clone(&shared_conn);
    let spec_conn = Arc::clone(&shared_conn);
    let user_conn = Arc::clone(&shared_conn);

    // Shared channel for CurationInput.
    let (curation_inbox_tx, curation_inbox_rx) =
        tokio::sync::mpsc::unbounded_channel::<CurationInput>();

    let consent_store = ConsentStore::new(consent_conn);
    consent_store
        .initialize_schema()
        .map_err(|e| ServiceError::ConsentStore {
            source: None,
            message: e.to_string(),
        })?;
    let consent_manager =
        Arc::new(ConsentManager::new(consent_store).with_event_sink(Arc::clone(&cns_event_sink)));

    let escalation_queue =
        Arc::new(
            EscalationQueue::new(escalation_conn).map_err(|e| ServiceError::Escalation {
                source: None,
                message: e.to_string(),
            })?,
        );

    let goal_sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&goal_conn)));
    let goal_repo = Arc::new(SqliteGoalRepository::new(goal_conn).with_telemetry(goal_sink));

    let sovereignty_boundary_store = SovereigntyBoundaryStore::new(sovereignty_conn);
    sovereignty_boundary_store
        .initialize_schema()
        .map_err(|e| ServiceError::SovereigntyStore {
            source: None,
            message: e.to_string(),
        })?;

    let spec_store = SqliteSpecStore::new(spec_conn);
    spec_store.init_schema().map_err(|e| ServiceError::Spec {
        source: None,
        message: e.to_string(),
    })?;

    let user_store = Arc::new(std::sync::Mutex::new(UserStore::new(user_conn)));
    {
        let guard = user_store.lock().map_err(|_| ServiceError::UserStore {
            source: None,
            message: hkask_types::InfrastructureError::LockPoisoned.to_string(),
        })?;
        guard
            .initialize_schema()
            .map_err(|e| ServiceError::UserStore {
                source: None,
                message: e.to_string(),
            })?;
    }

    // CNS runtime
    let cns_runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(
        config.cns_threshold,
    )));

    // Seam watcher — non-fatal if inventory unavailable.
    let seam_watcher: Arc<RwLock<Option<SeamWatcher>>> = {
        let cns = cns_runtime.read().await;
        if let Some(watcher) = SeamWatcher::load() {
            watcher.register_domains(&cns).await;
            let summary = watcher.summary();
            tracing::info!(
                target: "bootstrap",
                crates = %summary.crate_count,
                coverage_pct = %summary.coverage_pct,
                total_items = %summary.total_items,
                "Seam watcher initialized — watching the public seam"
            );
            Arc::new(RwLock::new(Some(watcher)))
        } else {
            tracing::info!(
                target: "bootstrap",
                "Seam watcher skipped — no inventory available (non-fatal)"
            );
            Arc::new(RwLock::new(None))
        }
    };

    // Spawn periodic seam drift check (background watcher).
    seam_monitor::spawn_seam_drift_check(&seam_watcher, &cns_runtime, &cns_event_sink);

    Ok(Foundation {
        db,
        primary_conn,
        curation_inbox_tx,
        curation_inbox_rx: Some(curation_inbox_rx),
        consent_manager,
        escalation_queue,
        goal_repo,
        sovereignty_boundary_store,
        spec_store,
        user_store,
        cns_runtime,
        seam_watcher,
        cns_event_sink,
        gas_event_store,
    })
}
