//! MCP + pods: governed tool, dispatcher, pod manager, daemon handler.

use super::super::*;
use super::foundation::Foundation;
use super::loops::LoopWiring;
use hkask_cns::set_point_calibrator::SetPointCalibrator;
use hkask_services_core::ServiceError;

/// MCP + pods: governed tool, dispatcher, pod manager, daemon handler.
pub(super) struct McpPods {
    pub mcp_runtime: Arc<McpRuntime>,
    pub mcp_dispatcher: Arc<McpDispatcher>,
    pub pod_manager: Arc<ActivePods>,
    pub capability_checker: Arc<CapabilityChecker>,
    pub daemon_handler: Arc<hkask_services_runtime::ServiceDaemonHandler>,
    pub energy_estimator: Arc<hkask_cns::CalibratedEnergyEstimator>,
    /// Statistical learner shared between GovernedTool and CyberneticsLoop.
    pub tool_stats: Arc<hkask_cns::ToolStats>,
    /// Keeps the CuratorSync cancellation channel alive.
    pub _curator_cancel: tokio::sync::watch::Sender<bool>,
    pub curator_ready: tokio::sync::oneshot::Receiver<()>,
}

pub(super) async fn build_mcp_and_pods(
    config: &ServiceConfig,
    l: &LoopWiring,
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

    // Set-point auto-tuning calibrator
    let set_point_calibrator: Arc<SetPointCalibrator> = Arc::new(SetPointCalibrator::new(
        Arc::clone(&f.gas_event_store),
        hkask_cns::DEFAULT_INITIAL_LOOKBACK,
    ));
    {
        let loop_ref = Arc::clone(&l.cybernetics_loop);
        set_point_calibrator.clone().spawn_calibration(
            hkask_cns::DEFAULT_SET_POINT_CALIBRATION_INTERVAL,
            move |adjustments| {
                if let Ok(mut guard) = loop_ref.try_write() {
                    let sp = guard.set_points_mut();
                    hkask_cns::set_point_calibrator::SetPointCalibrator::apply_adjustments(
                        &adjustments,
                        &mut sp.stagnation_thresholds,
                        &mut sp.block_worsening_ratio,
                        &mut sp.substitution_after,
                    );
                }
            },
        );
    }
    let estimator: Arc<dyn EnergyEstimator> =
        Arc::clone(&energy_estimator) as Arc<dyn EnergyEstimator>;

    // Wire ToolStats into the CyberneticsLoop for reliability sensor and
    // into GovernedTool for statistical cost distribution learning.
    let tool_stats = f.cns_runtime.read().await.tool_stats().await;
    l.cybernetics_loop
        .write()
        .await
        .set_tool_stats(Arc::clone(&tool_stats));

    let governed_tool = Arc::new(
        GovernedTool::new(
            raw_tool_port,
            Arc::clone(&l.cybernetics_loop),
            Arc::clone(&f.cns_event_sink),
            estimator,
            system_webid,
        )
        .with_tool_consumption_channel(l.tool_consumption_tx.clone())
        .with_tool_stats(Arc::clone(&tool_stats)),
    );
    let mcp_runtime = Arc::new(mcp_runtime);

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
    // Replace dispatcher with one that has the signing key.
    // The original was created before the checker existed.
    let mcp_dispatcher = Arc::new(McpDispatcher::with_governed_tool_and_checker(
        (*mcp_runtime).clone(),
        governed_tool.clone(),
        Arc::clone(&capability_checker),
    ));
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
                std::path::Path::new(&config.db_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf(),
                config.db_provider,
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
    {
        let adapter = Arc::clone(&l.pod_backup_adapter);
        let pm = Arc::clone(&pod_manager);
        tokio::spawn(async move {
            super::loops::pod_backup_daemon(adapter, pm).await;
        });
    }

    // Start CuratorPod + CuratorSync (semantic aggregation loop).
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
            }
        }
    });

    // Daemon handler + listener (skip socket in test mode)
    let daemon_handler = Arc::new(hkask_services_runtime::ServiceDaemonHandler::new(
        Arc::clone(&pod_manager),
        Arc::clone(&f.user_store),
        Some(Arc::clone(&f.cns_runtime)),
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
        tool_stats,
        _curator_cancel: curator_cancel_tx,
        curator_ready: curator_ready_rx,
    })
}

pub(super) async fn wire_manifest_executor(
    loops: &LoopWiring,
    mcp_dispatcher: &Arc<McpDispatcher>,
    config: &ServiceConfig,
) -> Result<(), ServiceError> {
    if let Some(inference_port) = loops.inference_port.clone() {
        // Wire dual-model inference using the secondary classifier model.
        let model_b = hkask_inference::model_constants::classifier_model_secondary();
        let executor = Arc::new(hkask_templates::ManifestExecutor::new(
            inference_port.clone(),
            mcp_dispatcher.clone() as Arc<dyn hkask_templates::McpPort>,
            hkask_types::LLMParameters::default(),
            config.a2a_secret.clone(),
        )
        .with_dual_inference(
            Arc::new(hkask_inference::dual_model_port::DualModelPort::new(
                inference_port,
                model_b.clone(),
            )),
            model_b,
        ));
        loops.curator_context.set_manifest_executor(executor).await;
        tracing::info!(target: "hkask.startup", "ManifestExecutor wired into CuratorContext — template-driven metacognition enabled");
    }
    loops.validate().await
}
