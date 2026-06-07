//! Interactive REPL for hKask — discoverable, self-documenting, alive.
//!
//! Design principles:
//! - Every capability is reachable from `/help`
//! - Tab completion for slash commands and agent names
//! - Fuzzy matching on slash commands (like russell's `/model`)
//! - Welcome banner with the Kask amphora logo
//! - Categorized help so the menu is scannable

mod builtin_servers;
mod commands;
pub(crate) mod display;
mod handlers;
mod helper;
mod tool_augmented;

pub(crate) use tool_augmented::TOOL_CALL_FORMAT_INTRO;

use hkask_agents::CurationConfidenceGate;
use hkask_agents::CurationLoop;
use hkask_agents::CuratorContext;
use hkask_agents::CyberneticsLoopHandle;
use hkask_agents::EscalationQueue;
use hkask_agents::HhhConfig;
use hkask_agents::HhhMode;
use hkask_agents::InferenceLoop;
use hkask_agents::LoopSystem;
use hkask_agents::adapters::MemoryLoopAdapter;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::hhh_gate;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::{
    CnsRuntime, CompositeGasEstimator, CyberneticsLoop, GasBudget, GasCost, GovernedTool,
};
use hkask_mcp::McpDispatcher;
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::{ConsolidationBridge, ConsolidationService, EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, TripleStore, in_memory_db};
use hkask_templates::{
    BundleManifest, ManifestExecutor, OkapiConfig, OkapiInference, SqliteRegistry,
};
use hkask_types::CuratorHandle;
use hkask_types::LLMParameters;
use hkask_types::PersonaConstraints;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::loops::LoopPayload;
use hkask_types::ports::InferencePort;
use hkask_types::ports::ToolInfo;
use hkask_types::ports::ToolPort;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config as ReadlineConfig, Editor};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;

use commands::handle_slash_command;
use helper::{KaskHelper, SessionHistory};

/// REPL state — initialized once, reused across all turns.
///
/// Holds the shared inference port, InferenceLoop (with gas budget
/// and circuit breaker), memory adapters, and Okapi config so they
/// aren't reconstructed per chat turn. Also groups mutable REPL state
/// to keep function signatures manageable.
pub(crate) struct ReplState {
    pub(crate) inference_port: Arc<dyn InferencePort>,
    /// InferenceLoop wrapping the port — provides gas budget tracking,
    /// circuit breaker state, and model selection via CNS observability.
    /// Arc-wrapped so it can be shared with the LoopSystem.
    pub(crate) inference_loop: Arc<InferenceLoop>,
    /// Episodic memory storage — private, agent-scoped
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic memory storage — shared, public knowledge
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,
    /// Agent WebID — derived from the agent name, used for memory operations
    pub(crate) agent_webid: WebID,
    /// CNS runtime for variety sensing and algedonic alerts
    pub(crate) cns: Arc<RwLock<CnsRuntime>>,
    /// CyberneticsLoop — direct reference for gas budget operations
    /// (register_gas_budget, acquire_budget, can_proceed).
    /// Also registered with LoopSystem for the sense→compare→compute→act cycle.
    /// Wrapped in RwLock so GovernedTool can share the same instance.
    pub(crate) cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    /// LoopSystem — runs the sense→compare→compute→act regulation cycle
    /// for Curation, Cybernetics, and Inference loops after each chat turn.
    /// Ticking is synchronous (after each user turn), not on a clock.
    /// The tick rate is defined by the interaction rate on the thread.
    pub(crate) loop_system: Arc<LoopSystem>,
    /// MessageDispatch — priority queue for inter-loop messages.
    /// Drained after each tick to display regulatory actions in the REPL.
    pub(crate) dispatch: Arc<MessageDispatch>,
    pub(crate) okapi_config: OkapiConfig,
    pub(crate) current_model: String,
    pub(crate) current_agent: String,
    pub(crate) session_history: SessionHistory,
    pub(crate) active_session: Option<String>,
    /// Pre-resolved secrets from onboarding, carried forward to avoid
    /// re-resolving from the OS keychain (which may use a mock backend
    /// with EntryOnly persistence on Linux).
    pub(crate) resolved_secrets: Option<crate::commands::config::ResolvedSecrets>,
    /// GovernedTool membrane — the singular governance boundary for MCP tool
    /// invocations. All tool calls route through this membrane, which enforces
    /// OCAP authority, gas budgets, and CNS observability.
    pub(crate) governed_tool: Arc<GovernedTool<RawMcpToolPort>>,
    /// HHH alignment mode — whether the Helpful/Harmless/Honest gate is active.
    pub(crate) hhh_mode: HhhMode,
    /// HHH configuration — gate model, max iterations, pass threshold.
    pub(crate) hhh_config: HhhConfig,
    /// Gate inference port — a separate InferencePort for the HHH evaluation model.
    /// Created eagerly at REPL init. None if the gate model failed to initialize.
    pub(crate) gate_inference_port: Option<Arc<dyn InferencePort>>,
    /// ConsolidationService for /consolidate command — built from the same per-agent
    /// memory DB as `episodic_storage` and `semantic_storage`. None if memory
    /// infrastructure is unavailable.
    pub(crate) consolidation_service: Option<ConsolidationService>,
    /// Persona constraints for the current agent — loaded from agent definition.
    /// When set, the persona filter strips forbidden patterns from model output.
    pub(crate) persona_constraints: Option<PersonaConstraints>,
    /// Pre-formatted tool section of the system prompt — derived from MCP
    /// runtime discovery at REPL init. The cache is intentional: `ToolPort`
    /// uses `impl Trait` returns so it is not dyn-compatible, which prevents
    /// re-deriving on demand via `Arc<dyn ToolPort>`. Re-derive it here when
    /// servers start/stop dynamically, or when making `ToolPort` dyn-compatible
    /// becomes a justified refactor.
    pub(crate) tool_prompt_section: String,
    /// Manifest executor — runs the process_manifest cascade for agents that
    /// have one defined. Created at REPL init from the agent's process_manifest
    /// reference. None if the agent has no process manifest or if loading failed.
    pub(crate) manifest_executor: Option<ManifestExecutor<McpDispatcher>>,
    /// The resolved process manifest for the current agent.
    /// Present when the agent definition includes a process_manifest reference
    /// and the manifest was successfully loaded.
    pub(crate) process_manifest: Option<BundleManifest>,
}

/// Build memory infrastructure from a Database: storage ports + ConsolidationService.
///
/// All components share the same underlying DB connection, so consolidation
/// operates on the agent's actual episodic and semantic triples.
fn build_memory_infra(
    db: Database,
) -> (
    Arc<dyn EpisodicStoragePort>,
    Arc<dyn SemanticStoragePort>,
    ConsolidationService,
) {
    let conn = db.conn_arc();

    // EpisodicMemory + SemanticMemory for ConsolidationService
    let ts1 = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(ts1));
    let ts2 = TripleStore::new(Arc::clone(&conn));
    let emb = EmbeddingStore::new(Arc::clone(&conn));
    let semantic_memory = Arc::new(SemanticMemory::new(ts2, emb));

    // ConsolidationService from the shared memories
    let bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let handle = CuratorHandle::system();
    let token = handle.issue_consolidation_token();
    let service = ConsolidationService::new(bridge, semantic_memory, token);

    // Storage ports — new EpisodicMemory/SemanticMemory from the same
    // connection (same pattern as MemoryLoopAdapter::from_database)
    let epi_adapter = Arc::new(MemoryLoopAdapter::new(
        EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))),
        SemanticMemory::new(
            TripleStore::new(Arc::clone(&conn)),
            EmbeddingStore::new(Arc::clone(&conn)),
        ),
    ));
    let sem_adapter = Arc::new(MemoryLoopAdapter::new(
        EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))),
        SemanticMemory::new(
            TripleStore::new(Arc::clone(&conn)),
            EmbeddingStore::new(Arc::clone(&conn)),
        ),
    ));

    (
        epi_adapter as Arc<dyn EpisodicStoragePort>,
        sem_adapter as Arc<dyn SemanticStoragePort>,
        service,
    )
}

pub fn run(
    _registry: &SqliteRegistry,
    _runtime: &McpRuntime,
    template_id: Option<&str>,
    _agent_name: &str,
    initial_model: Option<&str>,
    rt_handle: tokio::runtime::Handle,
) {
    let initial_model_str = initial_model.unwrap_or("deepseek-v4-pro");

    // Initialize inference port once — reused across all chat turns
    let okapi_config = OkapiConfig::local_dev();
    let inference_port: Arc<dyn InferencePort> =
        match OkapiInference::new(initial_model_str, okapi_config.clone()) {
            Ok(i) => Arc::new(i),
            Err(e) => {
                eprintln!("Failed to initialize inference port: {}", e);
                return;
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
    let onboarding_outcome = match rt_handle.block_on(crate::onboarding::run_onboarding()) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("Onboarding failed: {}", e);
            eprintln!("Run `kask chat` to set up your replicant identity.");
            return;
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
                    let (epi, sem, svc) = build_memory_infra(db);
                    (epi, sem, Some(svc))
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Persistent memory init failed ({}), falling back to in-memory",
                        e
                    );
                    let db = in_memory_db();
                    let (epi, sem, svc) = build_memory_infra(db);
                    (epi, sem, Some(svc))
                }
            }
        }
        None => {
            let db = in_memory_db();
            let (epi, sem, svc) = build_memory_infra(db);
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
    let cns_for_curator: Arc<CnsRuntime> = Arc::new(rt_handle.block_on(cns.read()).clone());
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
    rt_handle.block_on(async {
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
    rt_handle.block_on(loop_system.register_loop(curation_loop_arc));
    rt_handle.block_on(loop_system.register_loop(cybernetics_loop_arc));
    rt_handle.block_on(loop_system.register_loop(inference_loop_arc));

    // The GovernedTool membrane enforces OCAP authority, gas budgets, and
    // CNS observability for all MCP tool invocations. It shares the same
    // CyberneticsLoop as the REPL's loop system — tool invocations contribute
    // to the same gas budget and variety tracking as inference.
    let mcp_runtime = McpRuntime::new();

    // Register built-in MCP servers so /tools and /invoke work at startup.
    // Each server is spawned as a child process, tools are discovered
    // dynamically via MCP handshake — no static metadata needed.
    let server_count = rt_handle.block_on(builtin_servers::start_builtin_servers(&mcp_runtime));
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
        let tool_names = rt_handle.block_on(state.governed_tool.discover_tools());
        let mut tools: Vec<ToolInfo> = Vec::new();
        for name in &tool_names {
            if let Some(info) = rt_handle.block_on(state.governed_tool.get_tool_info(name)) {
                tools.push(info);
            }
        }
        state.tool_prompt_section = tool_augmented::format_tool_prompt_section(&tools);
    }

    // Load persona constraints for the initial agent
    state.persona_constraints = rt_handle
        .block_on(crate::commands::bot_status(&state.current_agent))
        .ok()
        .and_then(|agent| agent.definition.persona);

    // Load process manifest for the initial agent, if defined.
    // The process_manifest field on AgentDefinition holds a reference (path or ID)
    // to a BundleManifest that defines the agent's startup cascade.
    // Resolve it from the registry or filesystem, then create a ManifestExecutor
    // wired through the GovernedTool membrane for MCP tool invocations.
    let agent_definition = rt_handle
        .block_on(crate::commands::bot_status(&state.current_agent))
        .ok();

    if let Some(ref def) = agent_definition
        && let Some(ref manifest_ref) = def.definition.process_manifest
    {
        // Resolve the manifest reference to a BundleManifest.
        // Try registry first, then filesystem.
        let manifest = hkask_templates::resolve_manifest(manifest_ref, _registry);

        if let Some(bundle) = manifest {
            // Create an McpDispatcher that routes through the GovernedTool
            // membrane for OCAP authority, gas budgets, and CNS observability.
            let acp_secret: &[u8] = state
                .resolved_secrets
                .as_ref()
                .map(|s| s.acp_secret.as_bytes())
                .unwrap_or(&[]);

            let mcp_dispatcher = McpDispatcher::with_governed_tool(
                _runtime.clone(),
                acp_secret,
                state.governed_tool.clone(),
            );

            let executor = ManifestExecutor::new(
                state.inference_port.clone(),
                Arc::new(mcp_dispatcher),
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

    let helper = KaskHelper::new();

    let rl_config = ReadlineConfig::builder()
        .history_ignore_space(true)
        .history_ignore_dups(true)
        .expect("invalid readline config")
        .completion_type(CompletionType::List)
        .build();

    let mut rl = match Editor::with_config(rl_config) {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Failed to initialize readline: {}", e);
            return;
        }
    };
    rl.set_helper(Some(helper));

    if rl.load_history(&history_path()).is_err() {
        // No history file yet — that's fine
    }

    display::print_banner(&state.current_agent, template_id, &state.current_model);

    loop {
        let prompt = if let Some(ref session) = state.active_session {
            format!("\x1b[1mℏKask\x1b[0m [\x1b[33m{}\x1b[0m]> ", session)
        } else {
            format!(
                "\x1b[1mℏKask\x1b[0m [\x1b[36m{}\x1b[0m]> ",
                state.current_agent
            )
        };
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(input.to_owned());

                if input.starts_with('/') {
                    if handle_slash_command(input, template_id, &rt_handle, &mut state) {
                        let _ = rl.save_history(&history_path());
                        break;
                    }
                    continue;
                }

                if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                    println!("Goodbye!");
                    let _ = rl.save_history(&history_path());
                    break;
                }

                let rt = rt_handle.clone();

                // ACP secret for signing capability tokens in tool invocations.
                // Derived from onboarding — same secret that governs OCAP authority.
                let acp_secret: &[u8] = match &state.resolved_secrets {
                    Some(secrets) => secrets.acp_secret.as_bytes(),
                    None => {
                        eprintln!(
                            "Error: No ACP secret resolved. Run `kask chat` to complete onboarding or set HKASK_MASTER_KEY."
                        );
                        continue;
                    }
                };

                if let Some(ref session) = state.active_session {
                    match rt.block_on(crate::commands::ensemble_improv_turn(
                        session,
                        input,
                        Some(state.inference_port.clone()),
                    )) {
                        Ok(turn) => {
                            if turn.responses.is_empty() {
                                println!("  \x1b[2m(no agents chose to speak)\x1b[0m");
                            } else {
                                for response in &turn.responses {
                                    // Tool-augmented processing: same function
                                    // as single-agent REPL.
                                    let agent_name = response.agent_webid.to_string();
                                    let processed = rt.block_on(tool_augmented::process_response(
                                        &response.content,
                                        &agent_name,
                                        &state.governed_tool,
                                        &state.agent_webid,
                                        acp_secret,
                                        None, // ensemble responses don't carry structured tool calls yet
                                    ));
                                    // If no tool calls, process_response returns the
                                    // original text and we print it ourselves.
                                    // If tool calls were present, process_response
                                    // already printed everything.
                                    if !processed.had_tool_calls {
                                        println!(
                                            "\x1b[1m{}\x1b[0m (conf. {:.2}): {}\n",
                                            response.agent_webid,
                                            response.confidence,
                                            response.content
                                        );
                                    }
                                    state.session_history.record(&agent_name, &processed.text);
                                }
                                if let Some(ref synthesis) = turn.curator_synthesis {
                                    // Tool-augmented processing for curator synthesis
                                    let processed = rt.block_on(tool_augmented::process_response(
                                        synthesis,
                                        "Curator",
                                        &state.governed_tool,
                                        &state.agent_webid,
                                        acp_secret,
                                        None,
                                    ));
                                    if !processed.had_tool_calls {
                                        println!("\x1b[1;33mCurator:\x1b[0m {}\n", synthesis);
                                    }
                                    state.session_history.record("Curator", &processed.text);
                                }
                            }
                            for j in &turn.judgments {
                                if !j.should_speak {
                                    println!(
                                        "  \x1b[2m{}: silent ({:.2} — {})\x1b[0m",
                                        j.agent_name, j.confidence, j.reason
                                    );
                                }
                            }
                        }
                        Err(e) => println!("  \x1b[31mEnsemble error:\x1b[0m {}", e),
                    }
                } else {
                    // Two-track gas accounting:
                    //
                    // 1. InferenceLoop (fast path): AtomicU64 counter, used for
                    //    the loop's own sense() signal and REPL display.
                    //
                    // 2. CyberneticsLoop (regulatory path): GasBudget with
                    //    replenishment, alerts, and hard limits. This is the
                    //    canonical budget that the homeostatic regulator uses
                    //    to produce Throttle/AdjustGasBudget/Escalate actions.
                    //
                    // Hold-settle pattern: we reserve a heuristic estimate before
                    // inference, then settle with the actual token cost after.
                    // If the model doesn't report token usage, we fall back to
                    // the heuristic.
                    let heuristic_cost: u64 = 500; // pre-inference estimate
                    let can_proceed = rt.block_on(async {
                        state
                            .cybernetics_loop
                            .read()
                            .await
                            .can_proceed(&state.agent_webid, GasCost(heuristic_cost))
                            .await
                    });
                    if !can_proceed {
                        // Hard limit reached — regulator says stop.
                        println!(
                            "  \x1b[31m\u{2717} Gas budget exhausted (hard limit) \u{2014} turn blocked by cybernetic regulator\x1b[0m"
                        );
                        println!(
                            "  \x1b[2mUse /status to see budget details, or wait for replenishment.\x1b[0m"
                        );
                        continue;
                    }

                    // Reserve heuristic amount (hold-settle pattern)
                    let _reserved = rt.block_on(async {
                        state
                            .cybernetics_loop
                            .read()
                            .await
                            .reserve_gas(&state.agent_webid, GasCost(heuristic_cost))
                            .await
                    });

                    // Execute manifest cascade if the agent has a process manifest.
                    // The cascade runs select/populate/execute steps before inference,
                    // producing a context map that enriches the input with structured data
                    // from tool invocations and template rendering.
                    let mut manifest_context: Option<String> = None;
                    if let (Some(executor), Some(manifest)) =
                        (&state.manifest_executor, &state.process_manifest)
                    {
                        let mut initial_ctx = std::collections::HashMap::new();
                        initial_ctx.insert(
                            "user_input".to_string(),
                            serde_json::Value::String(input.to_string()),
                        );
                        initial_ctx.insert(
                            "agent".to_string(),
                            serde_json::Value::String(state.current_agent.clone()),
                        );

                        match rt.block_on(executor.execute_manifest(manifest, initial_ctx)) {
                            Ok(ctx) => {
                                // Format the manifest context for injection into the prompt.
                                // Use step results and populated outputs to enrich the input.
                                let mut context_parts: Vec<String> = Vec::new();
                                for (key, value) in &ctx {
                                    if key.starts_with("step_") {
                                        context_parts.push(format!("{}: {}", key, value));
                                    }
                                }
                                if !context_parts.is_empty() {
                                    manifest_context = Some(context_parts.join("\n"));
                                }
                                tracing::info!(
                                    target: "cns.spec.executor",
                                    steps_completed = ctx.len(),
                                    "Manifest cascade completed"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    target: "cns.spec.executor",
                                    error = %e,
                                    "Manifest cascade failed — continuing without manifest enrichment"
                                );
                            }
                        }
                    }

                    // When HHH mode is active, wrap the input in a reframe template
                    // and append HHH directives to the system prompt.
                    // If the manifest cascade produced context, prepend it to enrich the input.
                    let base_input: String = match &manifest_context {
                        Some(ctx) => format!(
                            "[Manifest Context]\n{}\n[/Manifest Context]\n\n{}",
                            ctx, input
                        ),
                        None => input.to_string(),
                    };
                    let (effective_input, hhh_suffix): (String, Option<String>) =
                        if state.hhh_mode == HhhMode::Active {
                            let reframed = hhh_gate::hhh_reframe(&base_input);
                            let suffix = hhh_gate::hhh_augment_system_prompt("");
                            (reframed, Some(suffix))
                        } else {
                            (base_input, None)
                        };

                    let chat_response = rt.block_on(crate::commands::chat_with_agent(
                        &effective_input,
                        Some(&state.current_agent),
                        Some(&state.current_model),
                        Some(state.inference_port.clone()),
                        state.resolved_secrets.as_ref(),
                        Some(state.episodic_storage.clone()),
                        Some(state.semantic_storage.clone()),
                        Some(state.agent_webid),
                        hhh_suffix.as_deref(),
                        Some(state.tool_prompt_section.as_str()),
                    ));

                    // Settle gas with actual token cost (7g)
                    let actual_cost = chat_response
                        .usage
                        .as_ref()
                        .map(|u| u.gas_cost())
                        .unwrap_or(heuristic_cost);

                    // Settle in CyberneticsLoop: refund difference if actual < reserved
                    let _settled = rt.block_on(async {
                        state
                            .cybernetics_loop
                            .read()
                            .await
                            .settle_gas(
                                &state.agent_webid,
                                GasCost(heuristic_cost),
                                GasCost(actual_cost),
                            )
                            .await
                    });

                    // Sync InferenceLoop's sense signal from the authoritative L6 budget
                    if let Some(status) = rt.block_on(async {
                        state
                            .cybernetics_loop
                            .read()
                            .await
                            .agent_gas_status(&state.agent_webid)
                            .await
                    }) {
                        state
                            .inference_loop
                            .sync_gas_state(status.remaining.as_raw(), status.cap.as_raw());
                    }

                    let response = chat_response.text;

                    // Both single-agent REPL and ensemble turns call the same
                    // `process_response` function. It checks for structured
                    // tool_calls first (native function calling), then falls back
                    // to parsing <<tool:...>> text directives.
                    //
                    // For single-agent, we also support a followup inference
                    // loop: if tool calls were found, we feed the results back
                    // to the model for another turn.
                    let structured_calls: Vec<hkask_types::ports::StructuredToolCall> =
                        if chat_response.finish_reason == "tool_calls" {
                            chat_response.tool_calls
                        } else {
                            vec![]
                        };
                    let processed = rt.block_on(tool_augmented::process_response(
                        &response,
                        &state.current_agent,
                        &state.governed_tool,
                        &state.agent_webid,
                        acp_secret,
                        Some(&structured_calls),
                    ));
                    let mut final_response = processed.text;

                    // If tool calls were found, feed the results back to the model
                    // for another inference turn. This gives the model a chance to
                    // synthesize the tool results into a final answer.
                    if processed.had_tool_calls && !processed.tool_results_formatted.is_empty() {
                        let followup_prompt = format!(
                            "{}\n\nThe following tool calls were executed:\n\n{}\n\nBased on these results, provide your response.",
                            final_response.trim(),
                            processed.tool_results_formatted
                        );

                        // Check gas budget before followup
                        let can_continue = rt.block_on(async {
                            state
                                .cybernetics_loop
                                .read()
                                .await
                                .can_proceed(&state.agent_webid, GasCost(heuristic_cost))
                                .await
                        });
                        if can_continue {
                            // Reserve gas for followup
                            let _reserved = rt.block_on(async {
                                state
                                    .cybernetics_loop
                                    .read()
                                    .await
                                    .reserve_gas(&state.agent_webid, GasCost(heuristic_cost))
                                    .await
                            });

                            let followup = rt.block_on(crate::commands::chat_with_agent(
                                &followup_prompt,
                                Some(&state.current_agent),
                                Some(&state.current_model),
                                Some(state.inference_port.clone()),
                                state.resolved_secrets.as_ref(),
                                Some(state.episodic_storage.clone()),
                                Some(state.semantic_storage.clone()),
                                Some(state.agent_webid),
                                None, // No HHH suffix for followup
                                Some(state.tool_prompt_section.as_str()),
                            ));

                            // Settle followup gas
                            let followup_cost = followup
                                .usage
                                .as_ref()
                                .map(|u| u.gas_cost())
                                .unwrap_or(heuristic_cost);
                            let _settled = rt.block_on(async {
                                state
                                    .cybernetics_loop
                                    .read()
                                    .await
                                    .settle_gas(
                                        &state.agent_webid,
                                        GasCost(heuristic_cost),
                                        GasCost(followup_cost),
                                    )
                                    .await
                            });

                            // Sync InferenceLoop's sense signal from the authoritative L6 budget
                            if let Some(status) = rt.block_on(async {
                                state
                                    .cybernetics_loop
                                    .read()
                                    .await
                                    .agent_gas_status(&state.agent_webid)
                                    .await
                            }) {
                                state
                                    .inference_loop
                                    .sync_gas_state(status.remaining.as_raw(), status.cap.as_raw());
                            }

                            if let Some(ref usage) = followup.usage {
                                println!(
                                    "  \x1b[2mFollowup: {} tokens ({} prompt + {} completion)\x1b[0m",
                                    usage.total_tokens,
                                    usage.prompt_tokens,
                                    usage.completion_tokens
                                );
                            }

                            // Process the followup for tool calls too
                            // Followup may also have structured tool calls from native function calling
                            let followup_structured: Vec<hkask_types::ports::StructuredToolCall> =
                                if followup.finish_reason == "tool_calls" {
                                    followup.tool_calls
                                } else {
                                    vec![]
                                };
                            let followup_processed = rt.block_on(tool_augmented::process_response(
                                &followup.text,
                                &state.current_agent,
                                &state.governed_tool,
                                &state.agent_webid,
                                acp_secret,
                                Some(&followup_structured),
                            ));
                            final_response = followup_processed.text;
                        } else {
                            println!(
                                "  \x1b[33m\u{26a0} Gas budget insufficient for followup inference\x1b[0m"
                            );
                        }

                        // When HHH mode is active, evaluate the final response through
                        // the gate model. If it fails, loop with correction prompts.
                        if state.hhh_mode == HhhMode::Active {
                            if let Some(ref gate_port) = state.gate_inference_port {
                                println!(
                                    "  \x1b[2m[HHH] Evaluating response for HHH compliance...\x1b[0m"
                                );

                                let mut hhh_iteration: u32 = 0;
                                let max_iterations = state.hhh_config.max_iterations;
                                let mut current_response = final_response.clone();

                                loop {
                                    // Gas check for gate evaluation
                                    let gate_heuristic: u64 = 500;
                                    let gate_can_proceed = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .can_proceed(
                                                &state.agent_webid,
                                                GasCost(gate_heuristic),
                                            )
                                            .await
                                    });
                                    if !gate_can_proceed {
                                        println!(
                                            "  \x1b[33m\u{26a0} HHH gate skipped: gas budget exhausted\x1b[0m"
                                        );
                                        tracing::warn!(
                                            target: "cns.hhh.gas_exhausted",
                                            "HHH gate evaluation skipped — gas budget exhausted"
                                        );
                                        break;
                                    }

                                    // Reserve gas for gate evaluation
                                    let _gate_reserved = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .reserve_gas(
                                                &state.agent_webid,
                                                GasCost(gate_heuristic),
                                            )
                                            .await
                                    });

                                    // Evaluate through the gate
                                    let evaluation = rt.block_on(hhh_gate::hhh_evaluate(
                                        input,
                                        &current_response,
                                        gate_port,
                                    ));

                                    // Settle gate gas
                                    let _gate_settled = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .settle_gas(
                                                &state.agent_webid,
                                                GasCost(gate_heuristic),
                                                GasCost(gate_heuristic),
                                            )
                                            .await
                                    });

                                    // Sync InferenceLoop's sense signal from L6 budget
                                    if let Some(status) = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .agent_gas_status(&state.agent_webid)
                                            .await
                                    }) {
                                        state.inference_loop.sync_gas_state(
                                            status.remaining.as_raw(),
                                            status.cap.as_raw(),
                                        );
                                    }

                                    if evaluation.overall_pass {
                                        println!(
                                            "  \x1b[32m[HHH] \u{2713} Passed (iteration {})\x1b[0m",
                                            hhh_iteration + 1
                                        );
                                        final_response = current_response;
                                        break;
                                    }

                                    if hhh_iteration >= max_iterations {
                                        // Max iterations reached — deliver with uncertainty marker
                                        final_response = format!(
                                            "{}\n\n\u{26a0}\u{fe0f} This response may not fully meet HHH standards.",
                                            current_response
                                        );
                                        println!(
                                            "  \x1b[33m[HHH] Max iterations reached, delivering with uncertainty marker\x1b[0m"
                                        );
                                        tracing::warn!(
                                            target: "cns.hhh.gate",
                                            iterations = hhh_iteration,
                                            "HHH gate exhausted — delivering with uncertainty marker"
                                        );
                                        break;
                                    }

                                    // Gate failed — print diagnostic and prepare correction
                                    let failures = evaluation.failures.join(", ");
                                    println!(
                                        "  \x1b[31m[HHH] \u{2717} Failed: {}\x1b[0m",
                                        failures
                                    );
                                    println!(
                                        "  \x1b[33m[HHH] Correcting (iteration {})...\x1b[0m",
                                        hhh_iteration + 2
                                    );

                                    let correction_input = hhh_gate::hhh_correction_prompt(
                                        input,
                                        &current_response,
                                        &evaluation,
                                    );

                                    // Gas check for correction inference
                                    let correction_can_proceed = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .can_proceed(
                                                &state.agent_webid,
                                                GasCost(heuristic_cost),
                                            )
                                            .await
                                    });
                                    if !correction_can_proceed {
                                        final_response = format!(
                                            "{}\n\n\u{26a0}\u{fe0f} HHH correction skipped: gas budget exhausted",
                                            current_response
                                        );
                                        println!(
                                            "  \x1b[33m\u{26a0} HHH correction skipped: gas budget exhausted\x1b[0m"
                                        );
                                        tracing::warn!(
                                            target: "cns.hhh.gas_exhausted",
                                            "HHH correction skipped — gas budget exhausted"
                                        );
                                        break;
                                    }

                                    // Reserve gas for correction inference
                                    let _correction_reserved = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .reserve_gas(
                                                &state.agent_webid,
                                                GasCost(heuristic_cost),
                                            )
                                            .await
                                    });

                                    let correction_suffix = hhh_gate::hhh_augment_system_prompt("");
                                    let correction_response =
                                        rt.block_on(crate::commands::chat_with_agent(
                                            &correction_input,
                                            Some(&state.current_agent),
                                            Some(&state.current_model),
                                            Some(state.inference_port.clone()),
                                            state.resolved_secrets.as_ref(),
                                            Some(state.episodic_storage.clone()),
                                            Some(state.semantic_storage.clone()),
                                            Some(state.agent_webid),
                                            Some(&correction_suffix),
                                            Some(state.tool_prompt_section.as_str()),
                                        ));

                                    // Settle correction gas
                                    let correction_cost = correction_response
                                        .usage
                                        .as_ref()
                                        .map(|u| u.gas_cost())
                                        .unwrap_or(heuristic_cost);
                                    let _correction_settled = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .settle_gas(
                                                &state.agent_webid,
                                                GasCost(heuristic_cost),
                                                GasCost(correction_cost),
                                            )
                                            .await
                                    });

                                    // Sync InferenceLoop's sense signal from L6 budget
                                    if let Some(status) = rt.block_on(async {
                                        state
                                            .cybernetics_loop
                                            .read()
                                            .await
                                            .agent_gas_status(&state.agent_webid)
                                            .await
                                    }) {
                                        state.inference_loop.sync_gas_state(
                                            status.remaining.as_raw(),
                                            status.cap.as_raw(),
                                        );
                                    }

                                    current_response = correction_response.text;
                                    hhh_iteration += 1;
                                }
                            } else {
                                println!(
                                    "  \x1b[33m\u{26a0} HHH mode active but gate model unavailable\x1b[0m"
                                );
                            }
                        }
                        // End of tool-augmented followup block
                    }

                    // Show token usage (7g)
                    if let Some(ref usage) = chat_response.usage {
                        println!(
                            "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                            usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                        );
                    }

                    // Check gas budget and warn if low
                    let gas_remaining = state.inference_loop.gas_remaining();
                    let gas_cap = state.inference_loop.gas_cap();
                    if gas_cap > 0
                        && gas_remaining > 0
                        && (gas_remaining as f64 / gas_cap as f64) < 0.2
                    {
                        println!(
                            "  \x1b[33m\u{26a0} Gas budget low: {}/{} ({:.0}%)\x1b[0m",
                            gas_remaining,
                            gas_cap,
                            (gas_remaining as f64 / gas_cap as f64) * 100.0
                        );
                    } else if gas_cap > 0 && gas_remaining == 0 {
                        println!(
                            "  \x1b[31m\u{2717} Gas budget exhausted \u{2014} some operations may be throttled\x1b[0m"
                        );
                    }

                    // CNS variety sensing: decompose the prompt and increment
                    // variety counters for depth, structure, and topic domains.
                    let analysis = hkask_agents::decompose_prompt(input);
                    {
                        let cns_guard = rt.block_on(state.cns.read());
                        // Prompt depth bucket (shallow/medium/deep)
                        rt.block_on(cns_guard.increment_variety(
                            "cns.inference.prompt_depth",
                            analysis.depth_bucket,
                        ));
                        // Prompt structure (question/imperative/declarative/conditional)
                        if analysis.question_count > 0 {
                            rt.block_on(
                                cns_guard.increment_variety(
                                    "cns.inference.prompt_structure",
                                    "question",
                                ),
                            );
                        }
                        if analysis.imperative_count > 0 {
                            rt.block_on(
                                cns_guard.increment_variety(
                                    "cns.inference.prompt_structure",
                                    "imperative",
                                ),
                            );
                        }
                        if analysis.sentence_count
                            > analysis.question_count + analysis.imperative_count
                        {
                            rt.block_on(cns_guard.increment_variety(
                                "cns.inference.prompt_structure",
                                "declarative",
                            ));
                        }
                        if analysis.conditional_count > 0 {
                            rt.block_on(cns_guard.increment_variety(
                                "cns.inference.prompt_structure",
                                "conditional",
                            ));
                        }
                        // Prompt topic domains (each unique keyword is a new variety state)
                        for keyword in &analysis.topic_keywords {
                            rt.block_on(
                                cns_guard.increment_variety("cns.inference.prompt_domain", keyword),
                            );
                        }
                    }

                    // Check for CNS algedonic alerts
                    let alerts =
                        rt.block_on(async { state.cns.read().await.critical_alerts().await });
                    if !alerts.is_empty() {
                        for alert in &alerts {
                            println!(
                                "  \x1b[31m\u{26a0} CNS ALERT: {} (deficit: {}/{})\x1b[0m",
                                alert.message, alert.deficit, alert.threshold
                            );
                        }
                    }

                    // Tick the LoopSystem to run sense→compare→compute→act for
                    // CyberneticsLoop and InferenceLoop. The CyberneticsLoop reads
                    // CNS variety and gas budgets, producing regulatory actions
                    // (Throttle, AdjustGasBudget, Escalate, Calibrate).
                    rt.block_on(state.loop_system.tick());

                    // Drain the MessageDispatch for regulatory actions produced
                    // by the loop cycle. Surface Throttle, Calibrate, Escalate,
                    // AdjustGasBudget, and CircuitBreak actions as REPL notices.
                    loop {
                        let msg = rt.block_on(state.dispatch.receive());
                        match msg {
                            Some(msg) => match &msg.payload {
                                LoopPayload::CyberneticsRegulation {
                                    regulation_type,
                                    parameters,
                                    ..
                                } => match regulation_type.as_str() {
                                    "throttle" => {
                                        let reason = parameters
                                            .get("reason")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        println!(
                                            "  \x1b[33m\u{26a0} CNS: Throttle — {}\x1b[0m",
                                            reason
                                        );
                                    }
                                    "adjust_gas_budget" => {
                                        let ratio = parameters
                                            .get("remaining_ratio")
                                            .and_then(|v| v.as_f64())
                                            .map(|r| format!("{:.0}%", r * 100.0))
                                            .unwrap_or_else(|| "?".to_string());
                                        println!(
                                            "  \x1b[33m\u{26a0} CNS: Gas budget adjusted — remaining {}\x1b[0m",
                                            ratio
                                        );
                                    }
                                    "circuit_break" => {
                                        println!(
                                            "  \x1b[31m\u{2717} CNS: Circuit breaker opened\x1b[0m"
                                        );
                                    }
                                    "calibrate" => {
                                        let reason = parameters
                                            .get("reason")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        println!(
                                            "  \x1b[36m\u{21bb} CNS: Calibrate — {}\x1b[0m",
                                            reason
                                        );
                                    }
                                    other => {
                                        println!("  \x1b[2mCNS: {}\x1b[0m", other);
                                    }
                                },
                                LoopPayload::AlgedonicAlert {
                                    current, threshold, ..
                                } => {
                                    println!(
                                        "  \x1b[31m\u{26a0} CNS: Algedonic escalation (deficit: {}/{})\x1b[0m",
                                        current, threshold
                                    );
                                }
                                _ => { /* other payload types — not displayed */ }
                            },
                            None => break,
                        }
                    }

                    // Persona filter (Stage 4 of alignment pipeline):
                    // strip forbidden patterns from the final Curator output.
                    final_response = hhh_gate::apply_persona_filter(
                        &final_response,
                        state.persona_constraints.as_ref(),
                    );

                    println!("{}: {}\n", state.current_agent, final_response);
                    state
                        .session_history
                        .record(&state.current_agent, &final_response);
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("(Ctrl+C — type /quit to exit)");
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                let _ = rl.save_history(&history_path());
                break;
            }
            Err(err) => {
                eprintln!("Readline error: {}", err);
                let _ = rl.save_history(&history_path());
                break;
            }
        }
    }
}

fn history_path() -> std::path::PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("hkask");
    let _ = std::fs::create_dir_all(&path);
    path.push("kask_history.txt");
    path
}
