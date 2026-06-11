//! REPL dependency injection — wires CNS, loops, dispatch, energy budgets,
//! GovernedTool, and builds the initial ReplState.
//!
//! Uses `AgentService::build()` for shared infrastructure (CNS, loop system,
//! curation, governed tool, pod manager) and adds CLI-specific concerns
//! on top (inference, per-agent memory, HHH gate, onboarding).

use std::sync::Arc;

use hkask_agents::HhhConfig;
use hkask_agents::HhhMode;
use hkask_agents::InferenceLoop;
use hkask_agents::hhh_gate;
use hkask_cns::{CompositeEnergyEstimator, EnergyBudget, EnergyCost, GovernedTool};
use hkask_mcp::RawMcpToolPort;
use hkask_services::{AgentService, InferenceContext, InferenceService};
use hkask_storage::Database;
use hkask_templates::{ManifestExecutor, McpPort};
use hkask_types::LLMParameters;
use hkask_types::WebID;
use hkask_types::ports::{InferencePort, ToolInfo, ToolPort};

use super::ReplState;
use super::tool_augmented;

/// Initialize all REPL dependencies and return a fully-wired ReplState.
///
/// Returns `None` if a critical dependency fails to initialize
/// (inference port, onboarding). Error messages are printed to stderr.
///
/// Uses `AgentService::build()` for shared infrastructure (CNS, loop system,
/// curation loop, pod manager, registry, MCP runtime) and adds CLI-specific
/// concerns on top (inference, per-agent memory, GovernedTool for tool
/// discovery, HHH gate, onboarding state).
pub(super) fn init_repl_state(
    registry: &mut hkask_templates::SqliteRegistry,
    _runtime: &hkask_mcp::runtime::McpRuntime,
    initial_model: Option<&str>,
    rt: &tokio::runtime::Handle,
) -> Option<ReplState> {
    let initial_model_str = initial_model.unwrap_or("deepseek-v4-pro");

    // Default REPL settings — used to initialize energy budget before
    // ReplState is fully constructed. Loads from ~/.config/hkask/settings.json
    // if available; falls back to ReplSettings::default().
    // Mutable here so the user can override via /repl during the session.
    let repl_settings = crate::commands::settings::load_settings();

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
            .with_energy_budget(repl_settings.gas_cap, repl_settings.gas_cap)
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

    // Build a ServiceConfig from onboarding outcome for AgentService::build().
    let service_config = match &onboarding_outcome.resolved_secrets {
        Some(secrets) => {
            // Onboarding provides ACP + DB secrets. MCP secret is resolved
            // separately since ResolvedSecrets doesn't carry it.
            let mcp_secret = hkask_keystore::resolve_mcp_secret()
                .map(|s| String::from_utf8_lossy(&s).to_string())
                .unwrap_or_else(|_| "hkask-mcp-default".to_string());
            hkask_services::ServiceConfig::from_secrets(
                secrets.acp_secret.clone(),
                secrets.db_passphrase.clone(),
                mcp_secret,
                onboarding_outcome.signed_in_agent.clone(),
            )
        }
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

    // Load skills from .agents/skills/ and skills/ into the registry before
    // building AgentService. This populates registry.skills() for bundle
    // composition, skill listing, and process_manifest resolution.
    // # REQ: P11 (Digital Public/Private Sphere) — load skills from both zones
    {
        let project_root =
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let loader = hkask_templates::SkillLoader::new(&project_root);
        let result = loader.load_into(registry);
        if !result.loaded.is_empty() {
            tracing::info!(
                target: "hkask.repl",
                skills_loaded = result.loaded.len(),
                warnings = result.warnings.len(),
                "Skills loaded from disk"
            );
        }
        for warning in &result.warnings {
            tracing::warn!(target: "hkask.repl", warning = %warning, "Skill load warning");
        }
    }

    // Build shared infrastructure via AgentService::build().
    // This creates: CNS, loop system (cybernetics, episodic, semantic, curation loops),
    // governed tool membrane, MCP runtime + dispatcher, pod manager, registry, etc.
    let ctx = match rt.block_on(hkask_services::AgentService::build(service_config.clone())) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Failed to build service context: {}", e);
            return None;
        }
    };

    // Register the CLI's inference loop on the shared loop system.
    rt.block_on(ctx.cns().2.register_loop(inference_loop.clone()));

    // Start built-in MCP servers on the AgentService's MCP runtime.
    let mcp_runtime = ctx.coordination().1.clone();
    let server_count = rt.block_on(super::builtin_servers::start_builtin_servers(&mcp_runtime));
    if server_count > 0 {
        tracing::info!(target: "hkask.repl", servers = server_count, "MCP servers started");
    }

    // Create the GovernedTool membrane for CLI tool discovery.
    // This wraps AgentService's MCP runtime with gas governance and CNS observability,
    // sharing the same cybernetics loop as the loop system.
    let raw_tool_port = Arc::new(RawMcpToolPort::new((*mcp_runtime).clone()));
    let estimator: Arc<dyn hkask_cns::EnergyEstimator> = Arc::new(CompositeEnergyEstimator::new());
    let governed_tool = Arc::new(GovernedTool::new(
        raw_tool_port,
        ctx.cns().1.clone(),
        ctx.cns().3.clone(),
        estimator,
        agent_webid,
    ));

    // Register the agent's energy budget with the CyberneticsLoop.
    // Uses repl_settings.gas_cap (default 10_000), replenish_rate=10% of cap,
    // alert at 80% usage, hard_limit=true (block operations when exhausted).
    rt.block_on(async {
        ctx.cns()
            .1
            .read()
            .await
            .register_energy_budget(
                agent_webid,
                EnergyBudget::new(EnergyCost(repl_settings.gas_cap))
                    .with_replenish_rate(EnergyCost(repl_settings.gas_cap / 10))
                    .with_alert_threshold(0.8)
                    .with_hard_limit(true),
            )
            .await
    });

    // Build per-agent memory via the service layer (NOT direct domain-crate
    // construction). AgentService::build_per_agent_memory constructs storage
    // ports and ConsolidationService from an agent-scoped Database, respecting
    // the hkask-cli → hkask-services → domain dependency rule.
    let (episodic_storage, semantic_storage, consolidation_service): (
        Arc<dyn hkask_agents::ports::EpisodicStoragePort>,
        Arc<dyn hkask_agents::ports::SemanticStoragePort>,
        Option<hkask_memory::ConsolidationService>,
    ) = {
        let db = match &onboarding_outcome.resolved_secrets {
            Some(secrets) => {
                let db_path = format!("hkask-memory-{}.db", onboarding_outcome.signed_in_agent);
                match Database::open(&db_path, &secrets.db_passphrase) {
                    Ok(db) => db,
                    Err(e) => {
                        eprintln!(
                            "Warning: Persistent memory init failed ({}), falling back to in-memory",
                            e
                        );
                        hkask_storage::in_memory_db()
                    }
                }
            }
            None => hkask_storage::in_memory_db(),
        };
        let mem = AgentService::build_per_agent_memory(db);
        (
            mem.episodic_storage,
            mem.semantic_storage,
            Some(mem.consolidation_service),
        )
    };

    let ctx = Arc::new(ctx);

    let mut state = ReplState {
        inference_port,
        inference_loop,
        // Per-agent memory (NOT AgentService's shared memory)
        episodic_storage,
        semantic_storage,
        agent_webid,
        current_model: initial_model_str.to_string(),
        current_agent: onboarding_outcome.signed_in_agent,
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
        service_context: ctx.clone(),
        repl_settings,
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

    // Populate model metadata (context_length, thinking support) from
    // Ollama on REPL init, so it's available immediately without waiting
    // for the user to switch models via /model.
    super::handlers::model::populate_model_meta(&mut state, rt);

    Some(state)
}
