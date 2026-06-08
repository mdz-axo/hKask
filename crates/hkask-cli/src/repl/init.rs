//! REPL dependency injection — wires CNS, loops, dispatch, gas budgets,
//! GovernedTool, and builds the initial ReplState.
//!
//! Extracted from `repl/mod.rs` to keep `run()` focused on the interactive loop.

use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;

use hkask_agents::CurationConfidenceGate;
use hkask_agents::CurationLoop;
use hkask_agents::CuratorContext;
use hkask_agents::CyberneticsLoopHandle;
use hkask_agents::EscalationQueue;
use hkask_agents::HhhConfig;
use hkask_agents::HhhMode;
use hkask_agents::InferenceLoop;
use hkask_agents::LoopSystem;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::hhh_gate;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{
    CnsRuntime, CompositeGasEstimator, CyberneticsLoop, GasBudget, GasCost, GovernedTool,
};
use hkask_mcp::McpDispatcher;
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::ConsolidationService;
use hkask_storage::{Database, in_memory_db};
use hkask_templates::{ManifestExecutor, McpPort, OkapiConfig, OkapiInference, SqliteRegistry};
use hkask_types::CuratorHandle;
use hkask_types::LLMParameters;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::ports::{InferencePort, ToolInfo, ToolPort};

use super::ReplState;
use super::helper::SessionHistory;
use super::memory;
use super::tool_augmented;

/// Initialize all REPL dependencies and return a fully-wired ReplState.
///
/// Returns `None` if a critical dependency fails to initialize
/// (inference port, onboarding). Error messages are printed to stderr.
pub(super) fn init_repl_state(
    registry: &SqliteRegistry,
    runtime: &McpRuntime,
    initial_model: Option<&str>,
    rt: &tokio::runtime::Handle,
) -> Option<ReplState> {
    let initial_model_str = initial_model.unwrap_or("deepseek-v4-pro");

    // Initialize inference port once — reused across all chat turns
    let okapi_config = OkapiConfig::local_dev();
    let inference_port: Arc<dyn InferencePort> =
        match OkapiInference::new(initial_model_str, okapi_config.clone()) {
            Ok(i) => Arc::new(i),
            Err(e) => {
                eprintln!("Failed to initialize inference port: {}", e);
                return None;
            }
        };

    // Wrap the inference port in an InferenceLoop for CNS observability.
    // The loop provides gas budget tracking, circuit breaker integration,
    // and model selection via the sense/compute/act cycle.
    // Gas budget: 10000 units per session (roughly 20 turns at ~500 tokens each)
    // Arc-wrapped so it can be shared with the LoopSystem.
    let inference_loop = Arc::new(
        InferenceLoop::new()
            .with_gas_budget(10_000, 10_000)
            .with_model(initial_model_str),
    );

    // Created eagerly to avoid cold-start latency when /hhh on is first called.
    // If the gate model is unavailable, the port is None and HHH mode
    // auto-disables — the user can configure a different model with /hhh model.
    let gate_inference_port: Option<Arc<dyn InferencePort>> = match OkapiInference::new(
        hhh_gate::HHH_DEFAULT_GATE_MODEL,
        okapi_config.clone(),
    ) {
        Ok(port) => Some(Arc::new(port)),
        Err(e) => {
            tracing::warn!(
                target: "cns.hhh.gate",
                error = %e,
                "Gate model initialization failed — HHH mode unavailable until /hhh model is used"
            );
            None
        }
    };

    // Runs before the interactive loop. If keys are already configured,
    // this is transparent. Otherwise, walks the user through creating or
    // signing into a replicant.
    let onboarding_outcome = match rt.block_on(crate::onboarding::run_onboarding()) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("Onboarding failed: {}", e);
            eprintln!("Run `kask chat` to set up your replicant identity.");
            return None;
        }
    };

    // Derive the agent's WebID from the agent name (deterministic)
    let agent_webid = WebID::from_persona_with_namespace(
        onboarding_outcome.signed_in_agent.as_bytes(),
        "replicant",
    );

    // Build EpisodicMemory and SemanticMemory from the agent's per-agent DB
    // (hkask-memory-{agent}.db). Both the storage ports (inference/recall)
    // and the ConsolidationService (/consolidate) share the same underlying
    // DB connection, so consolidation operates on the agent's actual memory.
    //
    // Previously the ConsolidationService was incorrectly built from the
    // registry DB (hkask.db), which meant /consolidate operated on the wrong
    // triples — the agent's working memory lives in the per-agent DB.
    let (episodic_storage, semantic_storage, consolidation_service): (
        Arc<dyn EpisodicStoragePort>,
        Arc<dyn SemanticStoragePort>,
        Option<ConsolidationService>,
    ) = match &onboarding_outcome.resolved_secrets {
        Some(secrets) => {
            let db_path = format!("hkask-memory-{}.db", onboarding_outcome.signed_in_agent);
            match Database::open(&db_path, &secrets.db_passphrase) {
                Ok(db) => {
                    let (epi, sem, svc) = memory::build_memory_infra(db);
                    (epi, sem, Some(svc))
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Persistent memory init failed ({}), falling back to in-memory",
                        e
                    );
                    let db = in_memory_db();
                    let (epi, sem, svc) = memory::build_memory_infra(db);
                    (epi, sem, Some(svc))
                }
            }
        }
        None => {
            let db = in_memory_db();
            let (epi, sem, svc) = memory::build_memory_infra(db);
            (epi, sem, Some(svc))
        }
    };

    // Initialize CNS runtime for variety sensing and algedonic alerts.
    // Default threshold of 100 means algedonic alerts fire when variety deficit
    // exceeds 100 in any domain. The CNS tracks prompt depth, structure, and
    // topic diversity via `decompose_prompt()` after each inference turn.
    let cns = Arc::new(RwLock::new(CnsRuntime::default()));

    // Construct the cybernetic loop system: MessageDispatch → LoopSystem
    // → register Curation + Cybernetics + Inference → tick after each turn.
    //
    // Ticking is synchronous after each user turn, not on a clock.
    // The tick rate is defined by the interaction rate on the thread.
    //
    // Authority DAG: Curation → Cybernetics → {Inference, ...}
    // Curation is the metacognitive and human-in-the-loop meta-regulator.
    // Cybernetics is the autonomous homeostatic regulator.
    let dispatch = Arc::new(MessageDispatch::new());
    let loop_system = Arc::new(LoopSystem::new(dispatch.clone()));

    // Register loops with the LoopSystem. The dispatch_sender is obtained
    // after construction because LoopSystem owns the channel internally.
    let dispatch_sender = loop_system.dispatch_sender();

    // The metacognitive observer: reviews algedonic events, processes
    // escalations, and issues CuratorDirectives. It is the ONLY loop that
    // can override Cybernetics (authority DAG: Curation → Cybernetics).
    //
    // CuratorContext needs Arc<CnsRuntime> (not RwLock-wrapped). Since
    // CnsRuntime is cheaply clonable (all internals are Arc<>), we clone
    // and wrap differently: REPL/Cybernetics use Arc<RwLock<CnsRuntime>>
    // for async mutation, Curation uses Arc<CnsRuntime> for read-only access.
    // Both clones share the same inner state.
    let escalation_queue = {
        let conn = rusqlite::Connection::open_in_memory()
            .expect("in-memory escalation DB should never fail");
        Arc::new(
            EscalationQueue::new(Arc::new(Mutex::new(conn)))
                .expect("escalation queue init should never fail"),
        )
    };
    let curator_handle = CuratorHandle::system();
    let cns_for_curator: Arc<CnsRuntime> = Arc::new(rt.block_on(cns.read()).clone());
    let curator_context = Arc::new(
        CuratorContext::new(
            curator_handle,
            cns_for_curator,
            dispatch.clone(),
            escalation_queue,
        )
        .with_loop_dispatch_tx(loop_system.dispatch_sender()),
    );
    let curation_loop = CurationLoop::new(CuratorHandle::system(), curator_context)
        .with_confidence_gate(CurationConfidenceGate::new(vec![]));
    let curation_loop_arc: Arc<dyn hkask_types::loops::HkaskLoop> = Arc::new(curation_loop);

    // The autonomous homeostatic regulator: reads CNS variety counters and
    // gas budgets, produces regulatory actions (Throttle, AdjustGasBudget,
    // Escalate) via sense→compute→compute→act.
    //
    // Wrapped in RwLock so GovernedTool can share the same instance for
    // gas budget checks during tool invocations. Direct method access
    // (register_gas_budget, can_proceed, reserve_gas, settle_gas) goes
    // through .read() on the RwLock — all CyberneticsLoop methods use
    // interior mutability via Arc<RwLock<HashMap>> for their data.
    let cybernetics_loop = Arc::new(RwLock::new(
        CyberneticsLoop::new(cns.clone(), dispatch_sender.clone()).with_event_sink(Arc::new(
            hkask_storage::NuEventStore::new(in_memory_db().conn_arc()),
        )),
    ));

    // Register the agent's gas budget with the CyberneticsLoop.
    // This is the canonical budget that the homeostatic regulator tracks.
    // cap=10000, replenish_rate=1000/turn (10% of cap), alert at 80% usage,
    // hard_limit=true (block operations when exhausted).
    //
    // InferenceLoop's gas counter is a read-only mirror of this budget,
    // synced via `sync_gas_state()` after each CyberneticsLoop operation.
    // The L6 budget is the authoritative regulator; the L1 counter is a sense signal.
    rt.block_on(async {
        cybernetics_loop
            .read()
            .await
            .register_gas_budget(
                agent_webid,
                GasBudget::new(GasCost(10_000))
                    .with_replenish_rate(GasCost(1_000))
                    .with_alert_threshold(0.8)
                    .with_hard_limit(true),
            )
            .await
    });

    let cybernetics_loop_arc: Arc<dyn hkask_types::loops::HkaskLoop> =
        Arc::new(CyberneticsLoopHandle(cybernetics_loop.clone()));

    // Already constructed above, wrapped in Arc for sharing with LoopSystem.
    let inference_loop_arc: Arc<dyn hkask_types::loops::HkaskLoop> = inference_loop.clone();

    // Register all loops. Authority DAG: Curation → Cybernetics → Inference.
    rt.block_on(loop_system.register_loop(curation_loop_arc));
    rt.block_on(loop_system.register_loop(cybernetics_loop_arc));
    rt.block_on(loop_system.register_loop(inference_loop_arc));

    // The GovernedTool membrane enforces OCAP authority, gas budgets, and
    // CNS observability for all MCP tool invocations. It shares the same
    // CyberneticsLoop as the REPL's loop system — tool invocations contribute
    // to the same gas budget and variety tracking as inference.
    let mcp_runtime = McpRuntime::new();

    // Register built-in MCP servers so /tools and /invoke work at startup.
    // Each server is spawned as a child process, tools are discovered
    // dynamically via MCP handshake — no static metadata needed.
    let server_count = rt.block_on(super::builtin_servers::start_builtin_servers(&mcp_runtime));
    if server_count > 0 {
        tracing::info!(target: "hkask.repl", servers = server_count, "MCP servers started");
    }

    let raw_tool_port = Arc::new(RawMcpToolPort::new(mcp_runtime.clone()));
    let cns_event_sink: Arc<dyn NuEventSink> =
        Arc::new(hkask_storage::NuEventStore::new(in_memory_db().conn_arc()));
    let gas_estimator: Arc<dyn hkask_cns::GasEstimator> = Arc::new(CompositeGasEstimator::new());

    let governed_tool = Arc::new(GovernedTool::new(
        raw_tool_port,
        cybernetics_loop.clone(), // Arc<RwLock<CyberneticsLoop>> — shared with LoopSystem
        cns_event_sink,
        gas_estimator,
        agent_webid,
        dispatch_sender, // LoopSystem's dispatch channel
    ));

    let mut state = ReplState {
        inference_port,
        inference_loop,
        episodic_storage,
        semantic_storage,
        agent_webid,
        cns,
        cybernetics_loop,
        loop_system,
        dispatch,
        okapi_config,
        current_model: initial_model_str.to_string(),
        current_agent: onboarding_outcome.signed_in_agent,
        session_history: SessionHistory::new(),
        active_session: None,
        resolved_secrets: onboarding_outcome.resolved_secrets,
        governed_tool,
        hhh_mode: HhhMode::Inactive,
        hhh_config: HhhConfig::default(),
        gate_inference_port,
        consolidation_service,
        persona_constraints: None,
        tool_prompt_section: String::new(), // populated below
        manifest_executor: None,            // populated below
        process_manifest: None,             // populated below
    };

    // Discover available MCP tools and format the system prompt section.
    // This replaces the hardcoded tool format string — the LLM sees only
    // tools that are actually running. GovernedTool enforces authorization.
    {
        let tool_names = rt.block_on(state.governed_tool.discover_tools());
        let mut tools: Vec<ToolInfo> = Vec::new();
        for name in &tool_names {
            if let Some(info) = rt.block_on(state.governed_tool.get_tool_info(name)) {
                tools.push(info);
            }
        }
        state.tool_prompt_section = tool_augmented::format_tool_prompt_section(&tools);
    }

    // Load persona constraints for the initial agent
    state.persona_constraints = rt
        .block_on(crate::commands::bot_status(&state.current_agent))
        .ok()
        .and_then(|agent| agent.definition.persona);

    // Load process manifest for the initial agent, if defined.
    // The process_manifest field on AgentDefinition holds a reference (path or ID)
    // to a BundleManifest that defines the agent's startup cascade.
    // Resolve it from the registry or filesystem, then create a ManifestExecutor
    // wired through the GovernedTool membrane for MCP tool invocations.
    let agent_definition = rt
        .block_on(crate::commands::bot_status(&state.current_agent))
        .ok();

    if let Some(ref def) = agent_definition
        && let Some(ref manifest_ref) = def.definition.process_manifest
    {
        // Resolve the manifest reference to a BundleManifest.
        // Try registry first, then filesystem.
        let manifest = hkask_templates::resolve_manifest(manifest_ref, registry);

        if let Some(bundle) = manifest {
            // Create an McpDispatcher that routes through the GovernedTool
            // membrane for OCAP authority, gas budgets, and CNS observability.
            let acp_secret: &[u8] = state
                .resolved_secrets
                .as_ref()
                .map(|s| s.acp_secret.as_bytes())
                .unwrap_or(&[]);

            let mcp_dispatcher = McpDispatcher::with_governed_tool(
                runtime.clone(),
                acp_secret,
                state.governed_tool.clone(),
            );

            let executor = ManifestExecutor::new(
                state.inference_port.clone(),
                Arc::new(mcp_dispatcher) as Arc<dyn McpPort>,
                LLMParameters::default(),
                acp_secret.to_vec(),
            );

            tracing::info!(
                target: "hkask.repl",
                manifest_id = %bundle.id,
                steps = bundle.steps.len(),
                "Loaded process manifest for agent"
            );

            state.process_manifest = Some(bundle);
            state.manifest_executor = Some(executor);
        } else {
            tracing::warn!(
                target: "hkask.repl",
                manifest_ref = %manifest_ref,
                agent = %state.current_agent,
                "Failed to resolve process manifest — agent will run without manifest cascade"
            );
        }
    }

    Some(state)
}
