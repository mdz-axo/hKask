//! REPL dependency injection — wires CNS, loops, dispatch, gas budgets,
//! GovernedTool, and builds the initial ReplState.
//!
//! Uses `ServiceContext::build()` for shared infrastructure (CNS, loop system,
//! curation, governed tool, pod manager) and adds CLI-specific concerns
//! on top (inference, per-agent memory, HHH gate, onboarding).

use std::sync::Arc;

use hkask_agents::HhhConfig;
use hkask_agents::HhhMode;
use hkask_agents::InferenceLoop;
use hkask_agents::hhh_gate;
use hkask_cns::{CompositeGasEstimator, GasBudget, GasCost, GovernedTool};
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_memory::ConsolidationService;
use hkask_services::{InferenceContext, InferenceService};
use hkask_storage::Database;
use hkask_templates::{ManifestExecutor, McpPort};
use hkask_types::LLMParameters;
use hkask_types::WebID;
use hkask_types::ports::{InferencePort, ToolInfo, ToolPort};

use super::ReplState;
use super::helper::SessionHistory;
use super::memory;
use super::tool_augmented;

/// Initialize all REPL dependencies and return a fully-wired ReplState.
///
/// Returns `None` if a critical dependency fails to initialize
/// (inference port, onboarding). Error messages are printed to stderr.
///
/// Uses `ServiceContext::build()` for shared infrastructure (CNS, loop system,
/// curation loop, pod manager, registry, MCP runtime) and adds CLI-specific
/// concerns on top (inference, per-agent memory, GovernedTool for tool
/// discovery, HHH gate, onboarding state).
pub(super) fn init_repl_state(
    registry: &hkask_templates::SqliteRegistry,
    _runtime: &hkask_mcp::runtime::McpRuntime,
    initial_model: Option<&str>,
    rt: &tokio::runtime::Handle,
) -> Option<ReplState> {
    let initial_model_str = initial_model.unwrap_or("deepseek-v4-pro");

    // Resolve okapi_base_url from env for InferenceService calls.
    // This is used before onboarding to create the initial inference port.
    let okapi_base_url = std::env::var("OKAPI_BASE_URL")
        .unwrap_or_else(|_| hkask_services::DEFAULT_OKAPI_BASE_URL.to_string());

    // Initialize inference port once — reused across all chat turns.
    // Route through InferenceService so all surfaces share the same logic.
    let inference_ctx = InferenceContext::from_parts(
        None, // No shared port yet — we're creating it now
        initial_model_str,
        &okapi_base_url,
    );
    let inference_port: Arc<dyn InferencePort> =
        match InferenceService::resolve_port(&inference_ctx, initial_model_str) {
            Ok(port) => port,
            Err(e) => {
                eprintln!("Failed to initialize inference port: {}", e);
                return None;
            }
        };

    // Wrap the inference port in an InferenceLoop for CNS observability.
    let inference_loop = Arc::new(
        InferenceLoop::new()
            .with_gas_budget(10_000, 10_000)
            .with_model(initial_model_str),
    );

    // Created eagerly to avoid cold-start latency when /hhh on is first called.
    let gate_inference_port: Option<Arc<dyn InferencePort>> = {
        let gate_ctx =
            InferenceContext::from_parts(None, hhh_gate::HHH_DEFAULT_GATE_MODEL, &okapi_base_url);
        match InferenceService::resolve_port(&gate_ctx, hhh_gate::HHH_DEFAULT_GATE_MODEL) {
            Ok(port) => Some(port),
            Err(e) => {
                tracing::warn!(
                    target: "cns.hhh.gate",
                    error = %e,
                    "Gate model initialization failed — HHH mode unavailable until /hhh model is used"
                );
                None
            }
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

    // Build a ServiceConfig from onboarding outcome for ServiceContext::build().
    let service_config = match &onboarding_outcome.resolved_secrets {
        Some(secrets) => hkask_services::ServiceConfig::from_secrets(
            secrets.acp_secret.clone(),
            secrets.db_passphrase.clone(),
            crate::commands::config::resolve_mcp_secret()
                .unwrap_or_else(|_| "hkask-mcp-default".to_string()),
            onboarding_outcome.signed_in_agent.clone(),
        ),
        None => hkask_services::ServiceConfig::from_env().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to resolve service config from env: {}", e);
            hkask_services::ServiceConfig::in_memory()
        }),
    };

    // Derive the agent's WebID from the agent name (deterministic)
    let agent_webid = WebID::from_persona_with_namespace(
        onboarding_outcome.signed_in_agent.as_bytes(),
        "replicant",
    );

    // Build shared infrastructure via ServiceContext::build().
    // This creates: CNS, loop system (cybernetics, episodic, semantic, curation loops),
    // governed tool membrane, MCP runtime + dispatcher, pod manager, registry, etc.
    let ctx = match rt.block_on(hkask_services::ServiceContext::build(
        service_config.clone(),
    )) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Failed to build service context: {}", e);
            return None;
        }
    };

    // Register the CLI's inference loop on the shared loop system.
    rt.block_on(ctx.loop_system.register_loop(inference_loop.clone()));

    // Start built-in MCP servers on the ServiceContext's MCP runtime.
    let mcp_runtime = ctx.mcp_runtime.clone();
    let server_count = rt.block_on(super::builtin_servers::start_builtin_servers(&mcp_runtime));
    if server_count > 0 {
        tracing::info!(target: "hkask.repl", servers = server_count, "MCP servers started");
    }

    // Create the GovernedTool membrane for CLI tool discovery.
    // This wraps ServiceContext's MCP runtime with gas governance and CNS observability,
    // sharing the same cybernetics loop as the loop system.
    let raw_tool_port = Arc::new(RawMcpToolPort::new((*mcp_runtime).clone()));
    let estimator: Arc<dyn hkask_cns::GasEstimator> = Arc::new(CompositeGasEstimator::new());
    let governed_tool = Arc::new(GovernedTool::new(
        raw_tool_port,
        ctx.cybernetics_loop.clone(),
        ctx.event_sink.clone(),
        estimator,
        agent_webid,
        ctx.loop_system.dispatch_sender(),
    ));

    // Register the agent's gas budget with the CyberneticsLoop.
    // cap=10000, replenish_rate=1000/turn (10% of cap), alert at 80% usage,
    // hard_limit=true (block operations when exhausted).
    rt.block_on(async {
        ctx.cybernetics_loop
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

    // Build EpisodicMemory and SemanticMemory from the agent's per-agent DB
    // (hkask-memory-{agent}.db). Both the storage ports and the
    // ConsolidationService share the same underlying DB connection.
    let (episodic_storage, semantic_storage, consolidation_service): (
        Arc<dyn hkask_agents::ports::EpisodicStoragePort>,
        Arc<dyn hkask_agents::ports::SemanticStoragePort>,
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
                    let db = hkask_storage::in_memory_db();
                    let (epi, sem, svc) = memory::build_memory_infra(db);
                    (epi, sem, Some(svc))
                }
            }
        }
        None => {
            let db = hkask_storage::in_memory_db();
            let (epi, sem, svc) = memory::build_memory_infra(db);
            (epi, sem, Some(svc))
        }
    };

    let mut state = ReplState {
        inference_port,
        inference_loop,
        // Per-agent memory (NOT ServiceContext's shared memory)
        episodic_storage,
        semantic_storage,
        agent_webid,
        // Shared CNS from ServiceContext
        cns: ctx.cns_runtime.clone(),
        cybernetics_loop: ctx.cybernetics_loop.clone(),
        loop_system: ctx.loop_system.clone(),
        dispatch: ctx.dispatch.clone(),
        service_config,
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
    let agent_definition = rt
        .block_on(crate::commands::bot_status(&state.current_agent))
        .ok();

    if let Some(ref def) = agent_definition
        && let Some(ref manifest_ref) = def.definition.process_manifest
    {
        let manifest = hkask_templates::resolve_manifest(manifest_ref, registry);

        if let Some(bundle) = manifest {
            let acp_secret: &[u8] = state
                .resolved_secrets
                .as_ref()
                .map(|s| s.acp_secret.as_bytes())
                .unwrap_or(&[]);

            let mcp_dispatcher = hkask_mcp::McpDispatcher::with_governed_tool(
                (*mcp_runtime).clone(),
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
