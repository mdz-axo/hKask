//! Loop wiring: cybernetics, inference, episodic, semantic, curation, snapshot, backup.

use super::super::*;
use super::foundation::Foundation;
use hkask_services_core::ServiceError;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Loop wiring: cybernetics, inference, episodic, semantic, curation, snapshot, backup.
pub(super) struct LoopWiring {
    pub loop_system: Arc<LoopSystem>,
    /// Concrete GixCasAdapter for pod-directory backup operations.
    pub pod_backup_adapter: Arc<hkask_mcp::GixCasAdapter>,
    pub cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    pub inference_port: Option<Arc<dyn InferencePort>>,
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    pub tool_consumption_tx: tokio::sync::mpsc::UnboundedSender<ToolConsumptionEvent>,
    pub a2a_runtime: Arc<hkask_agents::A2ARuntime>,
    /// CuratorContext — late-bound ManifestExecutor set after MCP pods built.
    pub curator_context: Arc<hkask_agents::CuratorContext>,
    /// Federation link manager — set when federation is enabled.
    pub federation_link_manager: Option<Arc<dyn FederationDispatch>>,
}

impl LoopWiring {
    pub(super) async fn validate(&self) -> Result<(), ServiceError> {
        if self.inference_port.is_some() && !self.curator_context.has_manifest_executor().await {
            return Err(ServiceError::Config {
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

    let (tool_consumption_tx, tool_consumption_rx) =
        tokio::sync::mpsc::unbounded_channel::<ToolConsumptionEvent>();
    let (curator_directive_tx, curator_directive_rx) =
        tokio::sync::mpsc::unbounded_channel::<CuratorDirective>();

    // Cybernetics loop
    let set_points = load_set_points();
    let cybernetics_loop = CyberneticsLoop::with_set_points(Arc::clone(&f.cns_runtime), set_points)
        .with_event_sink(Arc::clone(&f.cns_event_sink))
        .with_alerts_channel(f.curation_inbox_tx.clone())
        .with_tool_consumption_channel(tool_consumption_rx)
        .with_curator_directive_channel(curator_directive_rx);
    let cybernetics_loop = Arc::new(RwLock::new(cybernetics_loop));
    loop_system
        .register_loop(Arc::new(CyberneticsLoopHandle(Arc::clone(
            &cybernetics_loop,
        ))))
        .await;

    // Inference loop (optional)
    let inference_port: Option<Arc<dyn InferencePort>> = if config.in_memory {
        None
    } else {
        let router = hkask_inference::InferenceRouter::new(config.inference_config.clone());
        let raw_port: Arc<dyn InferencePort> = Arc::new(router);
        let governed_port: Arc<dyn InferencePort> = Arc::new(hkask_cns::GovernedInference::new(
            raw_port,
            Arc::clone(&cybernetics_loop),
            Arc::clone(&f.cns_event_sink),
            system_webid,
        ));
        let inference_loop = hkask_agents::InferenceLoop::new()
            .with_energy_budget(config.energy_budget_cap, config.gas_replenish_rate)
            .with_model(&config.default_model);
        loop_system.register_loop(Arc::new(inference_loop)).await;
        Some(governed_port)
    };

    // Episodic + Semantic memory
    let mem_conn = if config.in_memory {
        Arc::clone(&f.db.conn_arc())
    } else {
        let path = config
            .effective_memory_db_path()
            .expect("effective_memory_db_path returns Some when !in_memory");
        let passphrase = config
            .memory_passphrase
            .as_deref()
            .unwrap_or(&config.db_passphrase);
        Database::open(&path, passphrase)
            .map_err(|e| ServiceError::Storage {
                source: None,
                message: e.to_string(),
            })?
            .conn_arc()
    };
    let triple_store = TripleStore::new(Arc::clone(&mem_conn));
    let memory_life_days = config.memory_life_days;
    let episodic_memory = Arc::new(
        EpisodicMemory::new(triple_store)
            .with_memory_life_days(memory_life_days)
            .with_cns(Arc::clone(&f.cns_event_sink)),
    );
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    loop_system.register_loop(Arc::new(episodic_loop)).await;

    let triple_store2 = TripleStore::new(Arc::clone(&mem_conn));
    let embedding_store = EmbeddingStore::new(Arc::clone(&mem_conn));
    let semantic_memory = Arc::new(
        SemanticMemory::new(triple_store2, embedding_store).with_cns(Arc::clone(&f.cns_event_sink)),
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
        CuratorContext::new(
            CuratorHandle::system(),
            cns_for_curator,
            Some(curator_directive_tx.clone()),
            Arc::clone(&f.escalation_queue),
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
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    loop_system.register_loop(curation_loop).await;

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
        let triple_store = TripleStore::new(Arc::clone(&mem_conn));
        let semantic_index = Arc::new(std::sync::Mutex::new(SemanticIndex::new(triple_store)));
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
    let gix_adapter: Arc<hkask_mcp::GixCasAdapter> = match hkask_mcp::GixCasAdapter::from_env() {
        Ok(adapter) => Arc::new(adapter),
        Err(e) => {
            tracing::warn!(target: "hkask.services", error = %e, "Git CAS port from env failed — using fallback");
            Arc::new(
                hkask_mcp::GixCasAdapter::new(PathBuf::from("/tmp/hkask-templates")).map_err(
                    |e| ServiceError::Infra(hkask_types::InfrastructureError::Io(e.to_string())),
                )?,
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
        tool_consumption_tx,
        a2a_runtime,
        curator_context: curator_context_for_loops,
        federation_link_manager,
    })
}

/// Matrix registration retry loop — self-heals failed pod Matrix registrations.
pub(super) async fn spawn_matrix_retry_loop(
    _pod_manager: Arc<hkask_agents::ActivePods>,
    homeserver_url: String,
) {
    const POLL_INTERVAL_SECS: u64 = 60;
    const MAX_RETRIES: u32 = 10;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let keychain = hkask_keystore::Keychain::default();
        let prefix = hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX;

        let _ = (&keychain, prefix, &homeserver_url, MAX_RETRIES);
    }
}

/// Thin pod backup daemon: wake every 24h, snapshot all pod directories via gix.
/// One git repo per pod. The pod directory IS the unit.
pub(super) async fn pod_backup_daemon(
    adapter: Arc<hkask_mcp::GixCasAdapter>,
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
