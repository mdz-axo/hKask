use super::*;
use hkask_services_core::{ServiceConfig, ServiceError};

impl AgentService {
    /// Assemble all shared infrastructure from a `ServiceConfig`.
    ///
    /// This is the canonical construction path that replaces the four
    /// independent assemblies currently in the codebase. It resolves
    /// secrets, opens databases, constructs CNS/loop system, governed
    /// tool membrane, and session manager in the correct dependency order.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be a valid ServiceConfig with resolved secrets
    /// post: returns fully assembled AgentService with all infrastructure wired; Err on any construction step failure
    /// # Dependency order
    ///
    /// 1. Database connections (primary + per-purpose)
    /// 2. Stores (consent, escalation, goals, standing sessions)
    /// 3. CNS runtime + event sink
    /// 4. Loop system + cybernetics loop
    /// 5. GovernedTool membrane + MCP dispatcher
    /// 6. A2A runtime + pod manager
    /// 7. Inference port (optional, based on config)
    /// 8. Memory adapters (episodic + semantic)
    pub async fn build(config: ServiceConfig) -> Result<Self, ServiceError> {
        let system_webid = WebID::from_persona(config.agent_name.as_bytes());

        // ── Foundation: database, stores, CNS, seam watcher ──────────────
        let mut foundation = build_foundation(&config).await?;

        // ── Loops: cybernetics, inference, episodic, semantic, curation ──
        let loops = build_loops(&config, &mut foundation, system_webid).await?;

        // ── MCP + pods: governed tool, dispatcher, pod manager, daemon ───
        let mcp_pods = build_mcp_and_pods(&config, &loops, &foundation, system_webid).await?;

        // ── Matrix transport + 7R7 listener ──────────────────────────────
        let matrix_transport =
            self::matrix::build_matrix(Some(Arc::clone(&foundation.cns_event_sink))).await;

        // Communication events are now pushed directly in CurationLoop.sense()
        // from the NuEventStore query_algedonic results — no separate watcher needed.
        // See curator/curation_loop.rs for the integrated push.

        // Spawn Matrix registration retry loop — retries pending pod Matrix
        // registrations with exponential backoff for self-healing.
        if let Some(url) = mcp_pods
            .pod_manager
            .matrix_homeserver_url()
            .map(String::from)
        {
            let pod_manager = Arc::clone(&mcp_pods.pod_manager);
            tokio::spawn(async move {
                spawn_matrix_retry_loop(pod_manager, url).await;
            });
        }

        // ── Registry + wallet: agent records, A2A restore, rJoule ───────
        let reg_wallet = build_registry_and_wallet(&config, &foundation, &loops, &mcp_pods).await?;

        Ok(Self {
            registry: reg_wallet.registry,
            mcp_runtime: mcp_pods.mcp_runtime,
            mcp_dispatcher: mcp_pods.mcp_dispatcher,
            cns_runtime: foundation.cns_runtime,
            cybernetics_loop: loops.cybernetics_loop,
            loop_system: loops.loop_system,
            pod_backup_adapter: loops.pod_backup_adapter.clone(),
            inference_port: loops.inference_port,
            episodic_storage: loops.episodic_storage,
            semantic_storage: loops.semantic_storage,
            escalation_queue: foundation.escalation_queue,
            consent_manager: foundation.consent_manager,
            goal_repo: foundation.goal_repo,
            curation_inbox_tx: Some(foundation.curation_inbox_tx.clone()),
            pod_manager: mcp_pods.pod_manager,
            capability_checker: mcp_pods.capability_checker,
            system_webid,
            event_sink: foundation.cns_event_sink,
            energy_estimator: mcp_pods.energy_estimator,
            sovereignty_boundary_store: foundation.sovereignty_boundary_store,
            spec_store: foundation.spec_store,
            a2a_runtime: loops.a2a_runtime,
            agent_registry_store: reg_wallet.agent_registry_store,
            user_store: foundation.user_store,
            daemon_handler: mcp_pods.daemon_handler,
            matrix_transport,
            curator_ready: Some(mcp_pods.curator_ready),
            seam_watcher: foundation.seam_watcher,
            config,
            wallet_service: reg_wallet.wallet_service,
            wallet_store: reg_wallet.wallet_store,
            wallet_gas_calibrator: reg_wallet.wallet_gas_calibrator,
            federation_link_manager: loops.federation_link_manager,
        })
    }
}

// ── Build helpers ──────────────────────────────────────────────────────────
// Extracted from build() for readability. Each helper constructs one
// subsystem and returns an intermediate struct consumed by the next step.

/// Foundation: database connections, stores, CNS runtime, seam watcher.
struct Foundation {
    db: Database,
    primary_conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
    curation_inbox_tx: tokio::sync::mpsc::UnboundedSender<CurationInput>,
    curation_inbox_rx: Option<tokio::sync::mpsc::UnboundedReceiver<CurationInput>>,
    consent_manager: Arc<ConsentManager>,
    escalation_queue: Arc<EscalationQueue>,
    goal_repo: Arc<SqliteGoalRepository>,
    sovereignty_boundary_store: SovereigntyBoundaryStore,
    spec_store: SqliteSpecStore,
    user_store: Arc<std::sync::Mutex<UserStore>>,
    cns_runtime: Arc<RwLock<CnsRuntime>>,
    seam_watcher: Arc<RwLock<Option<SeamWatcher>>>,
    cns_event_sink: Arc<dyn NuEventSink>,
    /// Concrete event store used for gas report queries and calibration.
    gas_event_store: Arc<NuEventStore>,
}

async fn build_foundation(config: &ServiceConfig) -> Result<Foundation, ServiceError> {
    let db = if config.in_memory {
        in_memory_db()
    } else {
        Database::open(&config.db_path, &config.db_passphrase).map_err(|e| {
            ServiceError::Storage {
                message: e.to_string(),
            }
        })?
    };
    let shared_conn = db.conn_arc();

    let primary_conn = Arc::clone(&shared_conn);
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
            message: e.to_string(),
        })?;
    let consent_manager = Arc::new(ConsentManager::new(consent_store));

    let escalation_queue =
        Arc::new(
            EscalationQueue::new(escalation_conn).map_err(|e| ServiceError::Escalation {
                message: e.to_string(),
            })?,
        );

    let goal_sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&goal_conn)));
    let goal_repo = Arc::new(SqliteGoalRepository::new(goal_conn).with_telemetry(goal_sink));

    let sovereignty_boundary_store = SovereigntyBoundaryStore::new(sovereignty_conn);
    sovereignty_boundary_store
        .initialize_schema()
        .map_err(|e| ServiceError::SovereigntyStore {
            message: e.to_string(),
        })?;

    let spec_store = SqliteSpecStore::new(spec_conn);
    spec_store.init_schema().map_err(|e| ServiceError::Spec {
        message: e.to_string(),
    })?;

    let user_store = Arc::new(std::sync::Mutex::new(UserStore::new(user_conn)));
    {
        let guard = user_store.lock().map_err(|_| ServiceError::UserStore {
            message: hkask_types::InfrastructureError::LockPoisoned.to_string(),
        })?;
        guard
            .initialize_schema()
            .map_err(|e| ServiceError::UserStore {
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

    // CNS event sink + gas event store — share one NuEventStore on primary DB.
    let gas_event_store: Arc<NuEventStore> = Arc::new(NuEventStore::new(Arc::clone(&primary_conn)));
    let cns_event_sink: Arc<dyn NuEventSink> = Arc::clone(&gas_event_store) as Arc<dyn NuEventSink>;

    // Triple store for kanban task bridge — contract violations create tasks here.
    let _triple_store = Arc::new(TripleStore::new(Arc::clone(&primary_conn)));

    // Spawn periodic seam drift check (background watcher).
    self::seam_monitor::spawn_seam_drift_check(&seam_watcher, &cns_runtime, &cns_event_sink);

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

/// Loops: cybernetics, inference, episodic, semantic, curation, snapshot, backup.
struct Loops {
    loop_system: Arc<LoopSystem>,
    /// Concrete GixCasAdapter for pod-directory backup operations.
    pod_backup_adapter: Arc<hkask_mcp::GixCasAdapter>,
    cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    inference_port: Option<Arc<dyn InferencePort>>,
    episodic_storage: Arc<dyn EpisodicStoragePort>,
    semantic_storage: Arc<dyn SemanticStoragePort>,
    tool_consumption_tx: tokio::sync::mpsc::UnboundedSender<ToolConsumptionEvent>,
    a2a_runtime: Arc<hkask_agents::A2ARuntime>,
    /// CuratorContext — late-bound ManifestExecutor set after MCP pods built.
    curator_context: Arc<hkask_agents::CuratorContext>,
    /// Federation link manager — set when federation is enabled.
    federation_link_manager: Option<Arc<dyn FederationDispatch>>,
}

async fn build_loops(
    config: &ServiceConfig,
    f: &mut Foundation,
    system_webid: WebID,
) -> Result<Loops, ServiceError> {
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

    // Memory adapter — with CNS observability on its own store instances
    let memory_adapter = Arc::new(
        hkask_agents::adapters::memory_loop_adapter::MemoryLoopForwarder::new(
            EpisodicMemory::new(TripleStore::new(Arc::clone(&mem_conn)))
                .with_cns(Arc::clone(&f.cns_event_sink)),
            SemanticMemory::new(
                TripleStore::new(Arc::clone(&mem_conn)),
                EmbeddingStore::new(Arc::clone(&mem_conn)),
            )
            .with_cns(Arc::clone(&f.cns_event_sink)),
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
        .with_a2a(a2a_runtime.clone()),
    );
    // Clone before move into CuratorAgent — stored in Loops for late-binding
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
    // The CLI path uses federation_dispatch() directly; this enables
    // the internal directive dispatch path (CurationLoop → CuratorAgent).
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
    // pod_backup_daemon handles all pod snapshots now.
    // GixCasAdapter is held in Loops for TUI/CLI access.

    Ok(Loops {
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

/// MCP + pods: governed tool, dispatcher, pod manager, daemon handler.
struct McpPods {
    mcp_runtime: Arc<McpRuntime>,
    mcp_dispatcher: Arc<McpDispatcher>,
    pod_manager: Arc<ActivePods>,
    capability_checker: Arc<CapabilityChecker>,
    daemon_handler: Arc<hkask_services_runtime::ServiceDaemonHandler>,
    energy_estimator: Arc<hkask_cns::CalibratedEnergyEstimator>,
    /// Keeps the CuratorSync cancellation channel alive.
    /// The channel sender is held only for its Drop impl — when McpPods is dropped,
    /// the receiver detects the closed channel and cancels the CuratorSync polling loop.
    _curator_cancel: tokio::sync::watch::Sender<bool>,
    curator_ready: tokio::sync::oneshot::Receiver<()>,
}

async fn build_mcp_and_pods(
    config: &ServiceConfig,
    l: &Loops,
    f: &Foundation,
    system_webid: WebID,
) -> Result<McpPods, ServiceError> {
    // GovernedTool membrane
    let mcp_runtime = McpRuntime::new();
    let raw_tool_port = Arc::new(RawMcpToolPort::new(mcp_runtime.clone()));
    let energy_estimator: Arc<CalibratedEnergyEstimator> = Arc::new(
        CalibratedEnergyEstimator::new(Arc::clone(&f.gas_event_store))
            .with_event_sink(Arc::clone(&f.cns_event_sink)),
    );
    energy_estimator
        .clone()
        .spawn_calibration(hkask_cns::DEFAULT_CALIBRATION_INTERVAL);
    let estimator: Arc<dyn EnergyEstimator> =
        Arc::clone(&energy_estimator) as Arc<dyn EnergyEstimator>;
    let governed_tool = Arc::new(
        GovernedTool::new(
            raw_tool_port,
            Arc::clone(&l.cybernetics_loop),
            Arc::clone(&f.cns_event_sink),
            estimator,
            system_webid,
        )
        .with_tool_consumption_channel(l.tool_consumption_tx.clone()),
    );
    let mcp_dispatcher = Arc::new(McpDispatcher::with_governed_tool(
        mcp_runtime.clone(),
        governed_tool.clone(),
    ));
    let mcp_runtime = Arc::new(mcp_runtime);

    // Wire ManifestExecutor into CuratorContext for template-driven metacognition.
    // Per P3 (Generative Space), the Curator's calibrated decisions are produced
    // by KnowAct templates, not Rust code. The executor is late-bound because
    // McpDispatcher depends on GovernedTool which depends on CyberneticsLoop.
    if let Some(inference_port) = l.inference_port.clone() {
        let executor = Arc::new(hkask_templates::ManifestExecutor::new(
            inference_port,
            mcp_dispatcher.clone() as Arc<dyn hkask_templates::McpPort>,
            hkask_types::LLMParameters::default(),
            config.a2a_secret.clone(),
        ));
        l.curator_context.set_manifest_executor(executor).await;
        tracing::info!(target: "hkask.startup", "ManifestExecutor wired into CuratorContext — template-driven metacognition enabled");
    }

    // Pod manager — anchor the capability checker to BOTH the system OCAP
    // authority (pre-registration pod tokens) and the A2A root (post-registration
    // tokens), so legitimate pod tokens verify while forged tokens are rejected.
    // Fails the build if the system OCAP key is unavailable (P4 — fail closed).
    let capability_checker = Arc::new(
        hkask_agents::pod::system_capability_checker()
            .map_err(|e| {
                ServiceError::Infra(hkask_types::InfrastructureError::Io(format!(
                    "OCAP authority key unavailable: {e}"
                )))
            })?
            .trust_root(l.a2a_runtime.root_public_key()),
    );
    let mcp_runtime_adapter = hkask_agents::adapters::mcp_runtime::FullMcpAdapter::new(
        Arc::clone(&capability_checker),
        Arc::new((*mcp_runtime).clone()),
        tokio::runtime::Handle::current(),
    );
    let mut pods = hkask_agents::pod::ActivePods::new()
        .with_a2a_runtime(l.a2a_runtime.clone())
        .with_factory_and_ports(
            Arc::new(hkask_agents::pod::PodFactory::new(
                Arc::new(hkask_templates::TemplateCrateLoader::from_path(
                    std::path::PathBuf::from(&config.template_cache_path),
                )),
                Arc::new(hkask_agents::DenyAllConsent),
                // Pod storage lives alongside the main DB, not inside it.
                std::path::Path::new(&config.db_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf(),
            )),
            Arc::new(mcp_runtime_adapter),
            Some(governed_tool.clone()),
            Some(Arc::clone(&capability_checker)),
            None,
            Arc::clone(&l.episodic_storage) as Arc<dyn EpisodicStoragePort>,
            Arc::clone(&l.semantic_storage) as Arc<dyn SemanticStoragePort>,
        );
    if let Some(inf) = l.inference_port.clone() {
        pods = pods.with_inference_port(inf);
    }
    pods = pods.with_matrix_homeserver(
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string()),
    );
    let pod_manager: Arc<hkask_agents::pod::ActivePods> = Arc::new(pods);

    // Thin pod backup: iterate pods, snapshot each directory via GixCasAdapter.
    // Replaces the old BackupService/BackupConfig/ArtifactProducer/BackupLoop stack.
    {
        let adapter = Arc::clone(&l.pod_backup_adapter);
        let pm = Arc::clone(&pod_manager);
        tokio::spawn(async move {
            pod_backup_daemon(adapter, pm).await;
        });
    }

    // Start CuratorPod + CuratorSync (semantic aggregation loop).
    // Runs as a background task for the lifetime of the service.
    // A oneshot signals readiness so callers can await curator activation.
    let (curator_cancel_tx, curator_cancel_rx) = tokio::sync::watch::channel(false);
    let (curator_ready_tx, curator_ready_rx) = tokio::sync::oneshot::channel();
    let curator_pm = Arc::clone(&pod_manager);
    let curator_data_dir = std::path::Path::new(&config.db_path)
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    tokio::spawn(async move {
        match curator_pm
            .ensure_curator(curator_data_dir, curator_cancel_rx)
            .await
        {
            Ok(Some(_)) => {
                tracing::info!(target: "hkask.startup", "CuratorPod activated and CuratorSync running");
                let _ = curator_ready_tx.send(());
            }
            Ok(None) => {
                tracing::info!(target: "hkask.startup", "CuratorPod already active");
                let _ = curator_ready_tx.send(());
            }
            Err(e) => {
                tracing::error!(target: "hkask.startup", error = %e, "Failed to start CuratorPod");
                // Don't send on oneshot — callers awaiting curator_ready
                // will observe the sender dropped (recv_err).
            }
        }
    });
    // cancel_tx stored in McpPods for lifecycle

    // Matrix auto-registration — deferred to per-pod activation

    // Daemon handler + listener (skip socket in test mode)
    let daemon_handler = Arc::new(hkask_services_runtime::ServiceDaemonHandler::new(
        Arc::clone(&pod_manager),
        Arc::clone(&f.user_store),
        Some(Arc::clone(&f.cns_runtime)),
        Some(Arc::new(f.spec_store.clone())),
        l.inference_port.clone(),
    ));
    if !config.in_memory {
        let mut daemon_listener = hkask_mcp::daemon::DaemonListener::new();
        daemon_listener.bind().await.map_err(|e| {
            ServiceError::Infra(hkask_types::InfrastructureError::Io(format!(
                "Failed to bind daemon socket: {}",
                e
            )))
        })?;
        let serve_handler = Arc::clone(&daemon_handler);
        tokio::spawn(async move {
            if let Err(e) = daemon_listener.serve(serve_handler).await {
                tracing::error!(
                    target: "hkask.daemon",
                    error = %e,
                    "Daemon listener serve loop exited with error"
                );
            }
        });
    }

    Ok(McpPods {
        mcp_runtime,
        mcp_dispatcher,
        pod_manager,
        capability_checker,
        daemon_handler,
        energy_estimator,
        _curator_cancel: curator_cancel_tx,
        curator_ready: curator_ready_rx,
    })
}

/// Registry + wallet: agent records, A2A restore, rJoule payments.
struct RegWallet {
    registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    agent_registry_store: hkask_storage::AgentRegistryStore,
    wallet_service: Option<Arc<WalletService>>,
    wallet_store: Option<Arc<WalletStore>>,
    wallet_gas_calibrator: Option<Arc<hkask_cns::WalletGasCalibrator>>,
}

async fn build_registry_and_wallet(
    config: &ServiceConfig,
    f: &Foundation,
    l: &Loops,
    _mcp: &McpPods,
) -> Result<RegWallet, ServiceError> {
    // Registry
    let registry = Arc::new(tokio::sync::Mutex::new(
        SqliteRegistry::new_with_conn(f.primary_conn.clone()).map_err(|e| {
            ServiceError::Template {
                message: e.to_string(),
            }
        })?,
    ));

    // Agent registry store
    let agent_registry_store = hkask_storage::AgentRegistryStore::new(f.primary_conn.clone());
    agent_registry_store
        .initialize_schema()
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })?;

    // Restore A2A state from persistent storage
    let registered_agents =
        agent_registry_store
            .list()
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })?;
    if !registered_agents.is_empty() {
        use std::str::FromStr;
        let agents: Vec<hkask_agents::a2a::A2AAgent> = registered_agents
            .iter()
            .map(|ra| hkask_agents::a2a::A2AAgent {
                webid: hkask_types::WebID::from_str(&ra.definition.name).unwrap_or_else(|_| {
                    hkask_types::WebID::from_persona(ra.definition.name.as_bytes())
                }),
                agent_type: ra.definition.agent_kind,
                capabilities: ra.definition.capabilities.clone(),
                registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                active: true,
            })
            .collect();
        let tokens = std::collections::HashMap::new();
        // A2A restore is async — await directly since build() is async.
        l.a2a_runtime
            .restore_from_storage(agents, tokens)
            .await
            .map_err(|e| ServiceError::A2A {
                message: e.to_string(),
            })?;
    }

    // Wallet — non-fatal if config or build fails (daemon can run without wallet)
    let (wallet_service, wallet_store, wallet_gas_calibrator) = match build_wallet(config, f, l) {
        Ok(tuple) => tuple,
        Err(e) => {
            tracing::warn!(target: "cns.wallet", error = %e, "Wallet unavailable — running without rJoule");
            (None, None, None)
        }
    };

    Ok(RegWallet {
        registry,
        agent_registry_store,
        wallet_service,
        wallet_store,
        wallet_gas_calibrator,
    })
}

/// Build wallet subsystem — returns (service, store, gas_calibrator) or error.
#[allow(clippy::type_complexity)]
fn build_wallet(
    config: &ServiceConfig,
    f: &Foundation,
    l: &Loops,
) -> Result<
    (
        Option<Arc<WalletService>>,
        Option<Arc<WalletStore>>,
        Option<Arc<hkask_cns::WalletGasCalibrator>>,
    ),
    ServiceError,
> {
    // Per-agent wallet: each agent gets their own wallet database at
    // agents/{name}/wallet.db, encrypted with the same passphrase as pod.db.
    // This gives each replicant sovereign control over their rJoule balances,
    // API keys, and encumbrances — no shared wallet state across agents.
    let wallet_db_path = if config.in_memory {
        None
    } else {
        let path = hkask_types::agent_paths::agent_wallet_db(&config.agent_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Some(path)
    };

    let wallet_conn = if let Some(ref path) = wallet_db_path {
        let path_str = path.to_string_lossy().to_string();
        match hkask_storage::Database::open(&path_str, &config.db_passphrase) {
            Ok(db) => {
                tracing::info!(
                    target: "cns.wallet",
                    path = %path_str,
                    agent = %config.agent_name,
                    "Per-agent wallet database opened"
                );
                db.conn_arc()
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.wallet",
                    path = %path_str,
                    error = %e,
                    "Failed to open per-agent wallet DB, falling back to shared connection"
                );
                Arc::clone(&f.db.conn_arc())
            }
        }
    } else {
        Arc::clone(&f.db.conn_arc())
    };
    let wallet_store = Arc::new(WalletStore::new(wallet_conn));
    let svc = WalletService::build(
        &config.wallet_config,
        Arc::clone(&wallet_store),
        Arc::clone(&f.cns_event_sink),
        Arc::clone(&l.cybernetics_loop),
    )?;
    let svc = Arc::new(
        svc.as_ref()
            .clone()
            .with_consent_manager(Arc::clone(&f.consent_manager)),
    );
    let wallet_manager = svc.manager();

    // Ensure default wallet
    let default_wallet = WalletId::default();
    wallet_manager
        .ensure_wallet(default_wallet)
        .map_err(|e| ServiceError::Wallet {
            source: Some(Box::new(e)),
            message: "Failed to ensure default wallet".into(),
        })?;

    // Bind wallets to replicants
    {
        let user_guard = f.user_store.lock().map_err(|_| ServiceError::UserStore {
            message: hkask_types::InfrastructureError::LockPoisoned.to_string(),
        })?;
        if let Ok(Some(system_identity)) = user_guard.get_replicant(&config.agent_name) {
            let user_id = system_identity.user_id;
            let replicants =
                user_guard
                    .list_replicants(&user_id)
                    .map_err(|e| ServiceError::UserStore {
                        message: e.to_string(),
                    })?;
            for identity in &replicants {
                if identity.wallet_id.is_some() {
                    continue;
                }
                let wallet_id = WalletId::from_name(&identity.replicant_name);
                if let Err(e) = wallet_manager.ensure_wallet(wallet_id) {
                    tracing::warn!(
                        target: "cns.wallet",
                        replicant = %identity.replicant_name,
                        error = %e,
                        "Failed to create wallet for replicant"
                    );
                    continue;
                }
                if let Err(e) = user_guard.set_wallet_id(&identity.replicant_name, wallet_id) {
                    tracing::warn!(
                        target: "cns.wallet",
                        replicant = %identity.replicant_name,
                        error = %e,
                        "Failed to bind wallet to replicant"
                    );
                } else {
                    tracing::info!(
                        target: "cns.wallet",
                        replicant = %identity.replicant_name,
                        wallet_id = %wallet_id,
                        "Wallet created and bound to replicant"
                    );
                }
            }
        }
    }

    // Spawn deposit monitor
    let monitor_manager = Arc::clone(wallet_manager);
    let interval_secs: u64 = std::env::var("HKASK_DEPOSIT_MONITOR_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    tokio::spawn(async move {
        tracing::info!(
            target: "cns.wallet.deposit",
            interval_secs = %interval_secs,
            "Deposit monitor started — polling every {}s",
            interval_secs
        );
        if let Err(e) = monitor_manager.start_deposit_monitor(interval_secs).await {
            tracing::error!(
                target: "cns.wallet.deposit",
                error = %e,
                "Deposit monitor loop exited with error"
            );
        }
    });

    // Spawn wallet gas calibrator (P9 feedback loop for gas→rJoule rate).
    let wallet_gas_calibrator = {
        let calibrator = Arc::new(
            hkask_cns::WalletGasCalibrator::new(
                Arc::clone(&f.gas_event_store),
                Arc::clone(wallet_manager),
            )
            .with_event_sink(Arc::clone(&f.cns_event_sink)),
        );
        calibrator
            .clone()
            .spawn_calibration(hkask_cns::DEFAULT_WALLET_CALIBRATION_INTERVAL);
        Some(calibrator)
    };

    Ok((Some(svc), Some(wallet_store), wallet_gas_calibrator))
}

/// Matrix registration retry loop — self-heals failed pod Matrix registrations.
///
/// Polls the OS keychain for pending registrations (keys matching
/// `matrix-pod-pending-*`) and retries with exponential backoff.
/// After MAX_RETRIES (10), escalates by logging a CNS error span.
async fn spawn_matrix_retry_loop(
    _pod_manager: Arc<hkask_agents::ActivePods>,
    homeserver_url: String,
) {
    const POLL_INTERVAL_SECS: u64 = 60;
    const MAX_RETRIES: u32 = 10;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let keychain = hkask_keystore::Keychain::default();
        let prefix = hkask_types::keychain_keys::KEY_MATRIX_POD_PENDING_PREFIX;

        // The keychain API doesn't support prefix queries, so we poll
        // for specific pending registrations by checking known pods.
        // For now, this is a stub — the registration is retried on
        // each pod activation attempt (in activate_pod).
        // When a persistent pod registry exists, we'll iterate it here.

        let _ = (&keychain, prefix, &homeserver_url, MAX_RETRIES);
        // TODO: iterate pod registry, check keychain for pending markers,
        // call register_pod_matrix, clear on success, escalate on max retries.
    }
}

/// Thin pod backup daemon: wake every 24h, snapshot all pod directories via gix.
/// One git repo per pod. The pod directory IS the unit.
async fn pod_backup_daemon(
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
