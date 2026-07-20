//! Loop wiring: cybernetics, inference, episodic, semantic, curation, snapshot, backup.

use super::super::*;
use hkask_database::sqlite::SqliteDriver;

use super::foundation::Foundation;
use crate::cns_store_slo_provider::CnsStoreSloProvider;
use hkask_cns::DEFAULT_SET_POINT_CALIBRATION_INTERVAL;
use hkask_ports::{CnsStoragePort, escalation::EscalationPort};
use hkask_services_core::{DomainKind, ErrorKind, ServiceError};
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Loop wiring: cybernetics, inference, episodic, semantic, curation, snapshot, backup.
pub(super) struct LoopWiring {
    pub loop_system: Arc<LoopSystem>,
    /// Concrete GixCasAdapter for pod-directory backup operations.
    pub pod_backup_adapter: Arc<hkask_git_cas::GixCasAdapter>,
    pub cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    pub inference_port: Option<Arc<dyn InferencePort>>,
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    pub a2a_runtime: Arc<hkask_agents::A2ARuntime>,
    /// CuratorContext — late-bound ManifestExecutor set after MCP pods built.
    pub curator_context: Arc<hkask_agents::CuratorContext>,
    /// Federation link manager — set when federation is enabled.
    pub federation_link_manager: Option<Arc<dyn FederationDispatch>>,
}

impl LoopWiring {
    pub(super) async fn validate(&self) -> Result<(), ServiceError> {
        if self.inference_port.is_some() && !self.curator_context.has_manifest_executor().await {
            return Err(ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: None,
                message: "Loop wiring incomplete: inference enabled but CuratorContext lacks ManifestExecutor"
                    .to_string(),
            });
        }
        Ok(())
    }
}

pub(super) async fn build_loops(
    config: &ServiceConfig,
    f: &mut Foundation,
    system_webid: WebID,
) -> Result<LoopWiring, ServiceError> {
    let loop_system = Arc::new(LoopSystem::new());

    let (curator_directive_tx, curator_directive_rx) =
        tokio::sync::mpsc::unbounded_channel::<CuratorDirective>();

    // Cybernetics loop
    let set_points = load_set_points();
    let cybernetics_loop = CyberneticsLoop::with_set_points(Arc::clone(&f.cns_runtime), set_points)
        .with_event_sink(Arc::clone(&f.cns_event_sink))
        .with_alerts_channel(f.curation_inbox_tx.clone())
        .with_curator_directive_channel(curator_directive_rx)
        .with_slo_provider(Arc::new(CnsStoreSloProvider::new(Arc::clone(
            &f.nu_event_store,
        ))))
        .with_set_point_calibrator(
            Arc::clone(&f.nu_event_store) as Arc<dyn CnsStoragePort>,
            DEFAULT_SET_POINT_CALIBRATION_INTERVAL,
        )
        .with_seam_watcher();
    let cybernetics_loop = Arc::new(RwLock::new(cybernetics_loop));
    loop_system
        .register_loop(Arc::clone(&cybernetics_loop) as Arc<dyn HkaskLoop>)
        .await;

    // Inference loop (optional)
    let inference_port: Option<Arc<dyn InferencePort>> = if config.in_memory {
        None
    } else {
        let governed_port: Arc<dyn InferencePort> = Arc::new(
            hkask_inference::InferenceRouter::new(config.inference_config.clone()).with_governance(
                Arc::clone(&cybernetics_loop),
                Arc::clone(&f.cns_event_sink),
                system_webid,
            ),
        );
        let inference_loop = hkask_agents::InferenceLoop::new()
            .with_energy_budget(config.energy_budget_cap, config.gas_replenish_rate)
            .with_model(&config.default_model);
        loop_system.register_loop(Arc::new(inference_loop)).await;
        Some(governed_port)
    };

    // Episodic + Semantic memory
    let mem_driver: Arc<dyn hkask_database::driver::DatabaseDriver> = if config.in_memory {
        let pool = f.db.sqlite_pool().map_err(|e| ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Storage,
            source: None,
            message: format!("pool error: {e}"),
        })?;
        Arc::new(SqliteDriver::new(pool))
    } else {
        let path = config
            .effective_memory_db_path()
            .expect("effective_memory_db_path returns Some when !in_memory");
        let mem_db =
            Database::open(&path, &config.db_passphrase).map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Storage,
                source: None,
                message: e.to_string(),
            })?;
        let pool = mem_db.sqlite_pool().map_err(|e| ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Storage,
            source: None,
            message: format!("pool error: {e}"),
        })?;
        Arc::new(SqliteDriver::new(pool))
    };
    let h_mem_store = HMemStore::from_driver(Arc::clone(&mem_driver));
    let memory_life_days = config.memory_life_days;
    let episodic_memory = Arc::new(
        EpisodicMemory::new(h_mem_store)
            .with_memory_life_days(memory_life_days)
            .with_cns(Arc::clone(&f.cns_event_sink)),
    );
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    loop_system.register_loop(Arc::new(episodic_loop)).await;

    let h_mem_store2 = HMemStore::from_driver(Arc::clone(&mem_driver));
    let embedding_store = EmbeddingStore::from_driver(Arc::clone(&mem_driver), 1024);
    let semantic_memory = Arc::new(
        SemanticMemory::new(h_mem_store2, embedding_store).with_cns(Arc::clone(&f.cns_event_sink)),
    );
    let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
    loop_system.register_loop(Arc::new(semantic_loop)).await;

    // Memory adapter — reuses configured memory instances (shared store, shared config)
    let memory_adapter = Arc::new(
        hkask_agents::adapters::memory_loop_adapter::MemoryLoopForwarder::new(
            Arc::clone(&episodic_memory),
            Arc::clone(&semantic_memory),
        ),
    );
    let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();
    let semantic_storage: Arc<dyn SemanticStoragePort> = memory_adapter.clone();

    // Curation loop
    let cns_for_curator: Arc<CnsRuntime> = Arc::new(f.cns_runtime.read().await.clone());
    let a2a_runtime = Arc::new(hkask_agents::A2ARuntime::new(&config.a2a_secret));
    let curator_context = Arc::new(
        CuratorContext::with_nu_event_store(
            CuratorHandle::system(),
            cns_for_curator,
            Some(curator_directive_tx.clone()),
            Arc::clone(&f.escalation_queue) as Arc<dyn EscalationPort>,
            Arc::clone(&f.nu_event_store) as Arc<dyn CnsStoragePort>,
        )
        .with_a2a(a2a_runtime.clone())
        .with_consent_manager(Arc::clone(&f.consent_manager)),
    );
    // Clone before move into CuratorAgent — stored in LoopWiring for late-binding
    // ManifestExecutor wiring after MCP pods are built.
    let curator_context_for_loops = Arc::clone(&curator_context);
    let consolidation_bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let curator_agent = CuratorAgent::with_consolidation(
        curator_context,
        Default::default(),
        Arc::clone(&consolidation_bridge),
        Some(
            f.curation_inbox_rx
                .take()
                .expect("curation_inbox_rx consumed once"),
        ),
        Some(f.curation_inbox_tx.clone()),
        config.curator_auto_consolidation_enabled,
    );
    curator_agent.curation_loop().restore_cursor();
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    loop_system.register_loop(curation_loop).await;
    let metacognition_loop: Arc<dyn HkaskLoop> = curator_agent.metacognition().clone();
    loop_system.register_loop(metacognition_loop).await;

    // ── StorageGuard (Loop 7) — autonomous disk space management ──────
    let storage_guard_config = hkask_storage_guard::StorageGuardConfig {
        data_dir: std::env::var("HKASK_DATA_DIR").unwrap_or_else(|_| "/data".to_string()),
        ..Default::default()
    };
    let storage_guard = Arc::new(hkask_storage_guard::StorageGuardLoop::new(
        storage_guard_config,
    ));
    loop_system.register_loop(storage_guard).await;

    // ── Federation (opt-in via HKASK_FEDERATION_ENABLED=1) ─────────────
    let federation_link_manager: Option<Arc<dyn FederationDispatch>> = if std::env::var(
        "HKASK_FEDERATION_ENABLED",
    )
    .as_deref()
        == Ok("1")
    {
        let local_replica = system_webid.to_string();
        let transport: Arc<dyn hkask_ports::federation::FederationTransport> =
            Arc::new(InMemoryFederationTransport::for_replica(
                &InMemoryFederationTransport::new_shared(),
                local_replica.clone(),
            ));
        let link_manager = Arc::new(FederationLinkManager::new(
            local_replica.clone(),
            Arc::clone(&transport),
            Arc::clone(&f.cns_event_sink),
        ));
        let dispatch: Arc<dyn FederationDispatch> = link_manager.clone();
        // Build FederationSync with SemanticIndexSyncPort
        let pool = f.db.sqlite_pool().map_err(|e| ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Storage,
            source: None,
            message: format!("SQLite pool: {e}"),
        })?;
        let mem_driver = Arc::new(SqliteDriver::new(pool));
        let h_mem_store = HMemStore::from_driver(mem_driver);
        let semantic_index = Arc::new(std::sync::Mutex::new(SemanticIndex::new(h_mem_store)));
        let sync_port: Arc<dyn FederationSyncPort> =
            Arc::new(SemanticIndexSyncPort::new(Arc::clone(&semantic_index)));
        let fed_sync = Arc::new(FederationSync::new(
            local_replica.clone(),
            Arc::clone(&transport),
            sync_port,
            link_manager,
            Arc::clone(&f.cns_event_sink),
        ));
        // Spawn background sync loop
        let (fed_cancel_tx, fed_cancel_rx) = tokio::sync::watch::channel(false);
        let fed_sync_clone: Arc<FederationSync> = Arc::clone(&fed_sync);
        tokio::spawn(async move { fed_sync_clone.run(fed_cancel_rx).await });
        drop(fed_cancel_tx);
        tracing::info!(target: "cns.federation.sync", replica = %local_replica, "Federation sync loop started");
        Some(dispatch)
    } else {
        None
    };

    // ── Federation end ─────────────────────────────────────────────────

    // Wire federation dispatcher into CuratorAgent if present.
    if let Some(ref lm) = federation_link_manager {
        let _ = curator_agent.with_federation(Arc::clone(lm));
    }

    // Snapshot + Backup loops — keep concrete GixCasAdapter for pod-directory ops
    let gix_adapter: Arc<hkask_git_cas::GixCasAdapter> =
        match hkask_git_cas::GixCasAdapter::from_env() {
            Ok(adapter) => Arc::new(adapter),
            Err(e) => {
                tracing::warn!(target: "hkask.services", error = %e, "Git CAS port from env failed — using fallback");
                Arc::new(
                    hkask_git_cas::GixCasAdapter::new(PathBuf::from("/tmp/hkask-templates"))
                        .map_err(|e| {
                            ServiceError::Infra(hkask_types::InfrastructureError::Io(e.to_string()))
                        })?,
                )
            }
        };

    Ok(LoopWiring {
        loop_system,
        pod_backup_adapter: gix_adapter,
        cybernetics_loop,
        inference_port,
        episodic_storage,
        semantic_storage,
        a2a_runtime,
        curator_context: curator_context_for_loops,
        federation_link_manager,
    })
}

/// Matrix registration retry loop — self-heals failed pod Matrix registrations.
///
/// Scans active pods for pending Matrix registrations (keychain entry
/// `matrix-pod-pending-{name}`). Retries up to `MAX_RETRIES` per pod per
/// daemon session. On exhaustion stores a permanent `matrix-pod-failed-{name}`
/// marker so the Curator can detect and escalate the degraded state.
///
/// ## Known limitation: stranded entries for inactive pods
///
/// This loop can only retry registrations for pods that are *currently active*.
/// If a pod is decommissioned while a `matrix-pod-pending-{name}` entry exists,
/// the entry becomes stranded — it persists in the keychain but is invisible to
/// this loop because:
///
/// 1. The OS keychain API (`dbus-secret-service` / `security`) does not support
///    listing keys by prefix. We can only retrieve by exact key name.
/// 2. Without a prefix-listing capability, discovering stranded entries would
///    require maintaining a separate inventory of every pod name ever created.
///
/// **Mitigations in place:**
/// - `ActivePods::activate_pod` retries pending registrations on every activation,
///   so a decommissioned-then-reactivated pod will self-heal.
/// - The `retry_counts` map prunes entries for inactive pods at each poll cycle,
///   preventing unbounded memory growth.
/// - Failed registrations store a `matrix-pod-failed-{name}` marker (rather than
///   silently deleting the pending entry), preserving an auditable signal.
/// - On daemon startup, `cleanup_stranded_matrix_entries()` scans the `agents/`
///   directory and removes stale pending/failed keychain entries for pods that
///   still exist on disk (e.g., a pod that was recreated after decommission).
///
/// Stranded entries for truly decommissioned pods (directory deleted) remain
/// undetectable until `kask repair` is run, which scans the agents directory
/// and reports orphaned keychain entries.
///
/// Clean up stale Matrix keychain entries for pods that exist on disk.
///
/// Scans the `agents/` directory to discover all pod names present in the
/// filesystem, then checks each name against keychain entries with the
/// `matrix-pod-pending-{name}` and `matrix-pod-failed-{name}` patterns.
/// If a keychain entry exists for a pod that IS present on disk (e.g., a
/// pod that was decommissioned and recreated), the stale entry is removed.
///
/// Truly stranded entries for pods whose directories have been deleted
/// remain invisible to this scan (the OS keychain does not support
/// prefix-based key listing). Those are reported by `kask repair`.
fn cleanup_stranded_matrix_entries() {
    use std::path::Path;

    let agents_dir = Path::new(hkask_types::agent_paths::AGENTS_DIR);
    if !agents_dir.exists() || !agents_dir.is_dir() {
        return;
    }

    let keychain = hkask_keystore::Keychain::default();
    let pending_prefix = hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX;
    let failed_prefix = "matrix-pod-failed-";
    let mut cleaned = 0u32;

    // Build the set of pod names from the filesystem.
    let disk_pods: std::collections::HashSet<String> = match std::fs::read_dir(agents_dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => return,
    };

    // Check for pending entries that don't match any pod on disk.
    // We can't list keychain keys by prefix, so we iterate over the known
    // pod names from disk and check: is there a pending/failed entry for
    // a pod name NOT found on disk? No — we iterate disk_pods and check
    // if the entry exists. But the entry could also exist for pods NOT
    // on disk (the stranded case). Since we can't discover these entries,
    // we instead rely on the `kask repair` command which already
    // scans the agents directory and reports stranded entries.
    //
    // What we CAN do here: clean up failed markers for pods that ARE on
    // disk (the pod was decommissioned and recreated, or the registration
    // succeeded after the marker was stored). These are harmless but noisy.

    for name in &disk_pods {
        let failed_key = format!("{}-{}", failed_prefix, name);
        if keychain.retrieve_by_key(&failed_key).is_ok() {
            tracing::info!(
                target: "hkask.communication.matrix.cleanup",
                pod = %name,
                "Cleaning up stale matrix-pod-failed marker for active pod"
            );
            let _ = keychain.delete_by_key(&failed_key);
            cleaned += 1;
        }
        let pending_key = format!("{}-{}", pending_prefix, name);
        if keychain.retrieve_by_key(&pending_key).is_ok() {
            tracing::info!(
                target: "hkask.communication.matrix.cleanup",
                pod = %name,
                "Cleaning up stale matrix-pod-pending entry for active pod"
            );
            let _ = keychain.delete_by_key(&pending_key);
            cleaned += 1;
        }
    }

    if cleaned > 0 {
        tracing::info!(
            target: "hkask.communication.matrix.cleanup",
            cleaned = cleaned,
            "Matrix keychain cleanup complete"
        );
    }
}

pub(super) async fn spawn_matrix_retry_loop(
    pod_manager: Arc<hkask_agents::ActivePods>,
    homeserver_url: String,
) {
    const POLL_INTERVAL_SECS: u64 = 60;
    const MAX_RETRIES: u32 = 10;
    use std::collections::HashSet;

    // ── Startup: clean up stranded keychain entries from decommissioned pods ──
    // One-time cost before the main loop; uses blocking I/O (filesystem + keychain).
    let _ = tokio::task::spawn_blocking(cleanup_stranded_matrix_entries).await;

    let mut retry_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let keychain = hkask_keystore::Keychain::default();
        let pending_prefix = hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX;
        let failed_prefix = "matrix-pod-failed-";

        let active_names: HashSet<String> = pod_manager.pod_names().await.into_iter().collect();

        // Prune retry_counts for pods that are no longer active.
        retry_counts.retain(|name, _| active_names.contains(name));

        // ── Retry active pods with pending registrations ──
        for name in &active_names {
            let pending_key = format!("{}-{}", pending_prefix, name);
            let pending_url = match keychain.retrieve_by_key(&pending_key) {
                Ok(url) => url,
                Err(_) => continue,
            };

            let count = retry_counts.entry(name.clone()).or_insert(0);
            *count += 1;

            if *count > MAX_RETRIES {
                tracing::error!(
                    target: "hkask.communication.matrix.failed",
                    pod = %name,
                    retries = *count,
                    "Matrix pod registration exhausted — marking as permanently failed"
                );
                let _ = keychain.delete_by_key(&pending_key);
                let _ = keychain.store_by_key(&format!("{}-{}", failed_prefix, name), "permanent");
                retry_counts.remove(name);
                continue;
            }

            let url = if pending_url.is_empty() {
                &homeserver_url
            } else {
                &pending_url
            };

            match hkask_agents::ActivePods::retry_pod_matrix_registration(url, name).await {
                Ok(()) => {
                    tracing::info!(
                        target: "hkask.communication.matrix.retry",
                        pod = %name,
                        retry = *count,
                        "Retried Matrix pod registration — success"
                    );
                    let _ = keychain.delete_by_key(&pending_key);
                    retry_counts.remove(name);
                }
                Err(e) => {
                    tracing::debug!(
                        target: "hkask.communication.matrix.retry",
                        pod = %name,
                        retry = *count,
                        error = %e,
                        "Retried Matrix pod registration — still failing"
                    );
                }
            }
        }
    }
}

/// Thin pod backup daemon: wake every 24h, snapshot all pod directories via gix.
/// One git repo per pod. The pod directory IS the unit.
pub(super) async fn pod_backup_daemon(
    adapter: Arc<hkask_git_cas::GixCasAdapter>,
    pod_manager: Arc<hkask_agents::pod::ActivePods>,
) {
    const INTERVAL: std::time::Duration = std::time::Duration::from_secs(86400); // 24h

    loop {
        tokio::time::sleep(INTERVAL).await;

        let pod_dirs = pod_manager.pod_db_paths().await;
        if pod_dirs.is_empty() {
            continue;
        }

        tracing::info!(
            target: "cns.backup",
            pod_count = pod_dirs.len(),
            "Pod backup: snapshotting {} pods",
            pod_dirs.len()
        );

        for (pod_name, db_path) in &pod_dirs {
            let pod_dir = match db_path.parent() {
                Some(d) => d.to_path_buf(),
                None => continue,
            };

            match adapter
                .snapshot_pod_dir(&pod_dir, &format!("auto: {}", pod_name))
                .await
            {
                Ok(commit) => {
                    tracing::info!(
                        target: "cns.backup",
                        pod = %pod_name,
                        commit = %commit,
                        "Pod backup: snapshot complete"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.backup",
                        pod = %pod_name,
                        error = %e,
                        "Pod backup: snapshot failed"
                    );
                }
            }
        }
    }
}
