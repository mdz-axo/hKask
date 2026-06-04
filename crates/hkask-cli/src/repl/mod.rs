//! Interactive REPL for hKask — discoverable, self-documenting, alive.
//!
//! Design principles:
//! - Every capability is reachable from `/help`
//! - Tab completion for slash commands and agent names
//! - Fuzzy matching on slash commands (like russell's `/model`)
//! - Welcome banner with the Kask amphora logo
//! - Categorized help so the menu is scannable

mod commands;
pub(crate) mod display;
mod handlers;
mod helper;

use hkask_agents::CurationLoop;
use hkask_agents::CuratorContext;
use hkask_agents::EscalationQueue;
use hkask_agents::InferenceLoop;
use hkask_agents::LoopSystem;
use hkask_agents::adapters::MemoryLoopAdapter;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_cns::CnsRuntime;
use hkask_cns::CyberneticsLoop;
use hkask_cns::GasBudget;
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::{OkapiConfig, OkapiInference, SqliteRegistry};
use hkask_types::CuratorHandle;
use hkask_types::WebID;
use hkask_types::loops::LoopPayload;
use hkask_types::ports::InferencePort;
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
    pub(crate) cybernetics_loop: Arc<CyberneticsLoop>,
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
        InferenceLoop::new(inference_port.clone())
            .with_gas_budget(10_000, 10_000)
            .with_model(initial_model_str),
    );

    // ── Onboarding / Sign-in ──────────────────────────────────────────
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

    // Initialize persistent memory storage (episodic + semantic) using
    // encrypted SQLite. Falls back to in-memory if passphrase is unavailable.
    // This is Task 6: the memory adapters persist across REPL sessions, bound
    // to the agent's identity. Pod-mediated sessions would provide the same
    // storage via PodContext, but this gives us session persistence now.
    let (episodic_storage, semantic_storage): (
        Arc<dyn EpisodicStoragePort>,
        Arc<dyn SemanticStoragePort>,
    ) = match &onboarding_outcome.resolved_secrets {
        Some(secrets) => {
            // Use the resolved DB passphrase for encrypted persistent storage
            let db_path = format!("hkask-memory-{}.db", onboarding_outcome.signed_in_agent);
            match MemoryLoopAdapter::from_path(&db_path, &secrets.db_passphrase) {
                Ok(adapter) => {
                    let episodic: Arc<dyn EpisodicStoragePort> = Arc::new(adapter);
                    // Create a second adapter from the same path for semantic storage
                    // (they share the same underlying database)
                    let sem_adapter =
                        MemoryLoopAdapter::from_path(&db_path, &secrets.db_passphrase)
                            .expect("DB opened once, should open again");
                    let semantic: Arc<dyn SemanticStoragePort> = Arc::new(sem_adapter);
                    (episodic, semantic)
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Persistent memory init failed ({}), falling back to in-memory",
                        e
                    );
                    let epi_adapter = MemoryLoopAdapter::in_memory()
                        .expect("In-memory storage should never fail");
                    let sem_adapter = MemoryLoopAdapter::in_memory()
                        .expect("In-memory storage should never fail");
                    let episodic: Arc<dyn EpisodicStoragePort> = Arc::new(epi_adapter);
                    let semantic: Arc<dyn SemanticStoragePort> = Arc::new(sem_adapter);
                    (episodic, semantic)
                }
            }
        }
        None => {
            // No resolved secrets — use in-memory storage (ephemeral)
            let epi_adapter =
                MemoryLoopAdapter::in_memory().expect("In-memory storage should never fail");
            let sem_adapter =
                MemoryLoopAdapter::in_memory().expect("In-memory storage should never fail");
            let episodic: Arc<dyn EpisodicStoragePort> = Arc::new(epi_adapter);
            let semantic: Arc<dyn SemanticStoragePort> = Arc::new(sem_adapter);
            (episodic, semantic)
        }
    };

    // Initialize CNS runtime for variety sensing and algedonic alerts.
    // Default threshold of 100 means algedonic alerts fire when variety deficit
    // exceeds 100 in any domain. The CNS tracks prompt depth, structure, and
    // topic diversity via `decompose_prompt()` after each inference turn.
    let cns = Arc::new(RwLock::new(CnsRuntime::default()));

    // ── Loop System (7d) ────────────────────────────────────────────────
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

    // ── CurationLoop (Loop 5) ──────────────────────────────────────────
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
    let curator_context = Arc::new(CuratorContext::new(
        curator_handle,
        cns_for_curator,
        dispatch.clone(),
        escalation_queue,
    ));
    let curation_loop = CurationLoop::new(CuratorHandle::system(), curator_context);
    let curation_loop_arc: Arc<dyn hkask_types::loops::HkaskLoop> = Arc::new(curation_loop);

    // ── CyberneticsLoop (Loop 6) ───────────────────────────────────────
    // The autonomous homeostatic regulator: reads CNS variety counters and
    // gas budgets, produces regulatory actions (Throttle, AdjustGasBudget,
    // Escalate) via sense→compute→compute→act.
    //
    // Arc-wrapped so we can keep a direct reference for gas budget operations
    // (register_gas_budget, acquire_budget, can_proceed) while also
    // registering a clone with the LoopSystem for the regulation cycle.
    let cybernetics_loop = Arc::new(CyberneticsLoop::new(cns.clone(), dispatch_sender));

    // Register the agent's gas budget with the CyberneticsLoop.
    // This is the canonical budget that the homeostatic regulator tracks.
    // cap=10000, replenish_rate=1000/turn (10% of cap), alert at 80% usage,
    // hard_limit=true (block operations when exhausted).
    //
    // The InferenceLoop also tracks gas via its own AtomicU64 counter for
    // its sense() signal (inference_gas_remaining). Both trackers are
    // consumed in the REPL turn cycle — InferenceLoop's counter is the
    // operational fast-path, CyberneticsLoop's GasBudget is the regulatory
    // tracker with replenishment and homeostatic response.
    rt_handle.block_on(
        cybernetics_loop.register_gas_budget(
            agent_webid,
            GasBudget::new(10_000)
                .with_replenish_rate(1_000)
                .with_alert_threshold(0.8)
                .with_hard_limit(true),
        ),
    );
    let cybernetics_loop_arc: Arc<dyn hkask_types::loops::HkaskLoop> = cybernetics_loop.clone();

    // ── InferenceLoop (Loop 1) ────────────────────────────────────────
    // Already constructed above, wrapped in Arc for sharing with LoopSystem.
    let inference_loop_arc: Arc<dyn hkask_types::loops::HkaskLoop> = inference_loop.clone();

    // Register all loops. Authority DAG: Curation → Cybernetics → Inference.
    rt_handle.block_on(loop_system.register_loop(curation_loop_arc));
    rt_handle.block_on(loop_system.register_loop(cybernetics_loop_arc));
    rt_handle.block_on(loop_system.register_loop(inference_loop_arc));

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
    };

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
                                    println!(
                                        "\x1b[1m{}\x1b[0m (conf. {:.2}): {}\n",
                                        response.agent_webid, response.confidence, response.content
                                    );
                                    state.session_history.record(
                                        &response.agent_webid.to_string(),
                                        &response.content,
                                    );
                                }
                                if let Some(ref synthesis) = turn.curator_synthesis {
                                    println!("\x1b[1;33mCurator:\x1b[0m {}\n", synthesis);
                                    state.session_history.record("Curator", synthesis);
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
                    // ── Gas consumption ────────────────────────────────────────────
                    // Two-track gas accounting:
                    //
                    // 1. InferenceLoop (fast path): AtomicU64 counter, used for
                    //    the loop's own sense() signal and REPL display. This is
                    //    the operational tracker that the REPL reads directly.
                    //
                    // 2. CyberneticsLoop (regulatory path): GasBudget with
                    //    replenishment, alerts, and hard limits. This is the
                    //    canonical budget that the homeostatic regulator uses
                    //    to produce Throttle/AdjustGasBudget/Escalate actions.
                    //
                    // Both are consumed per turn. The InferenceLoop's atomic
                    // counter is the one displayed to the user. The
                    // CyberneticsLoop's GasBudget is what triggers regulatory
                    // actions via the sense→compute→act cycle.
                    //
                    // If the CyberneticsLoop budget is exhausted and hard_limit
                    // is set, can_proceed() returns false — the regulator is
                    // telling us to stop.
                    let turn_cost: u64 = 500;
                    let can_proceed = rt.block_on(
                        state
                            .cybernetics_loop
                            .can_proceed(&state.agent_webid, turn_cost),
                    );
                    if !can_proceed {
                        // Hard limit reached — regulator says stop.
                        // Still show the budget status but don't run inference.
                        println!(
                            "  \x1b[31m\u{2717} Gas budget exhausted (hard limit) \u{2014} turn blocked by cybernetic regulator\x1b[0m"
                        );
                        println!(
                            "  \x1b[2mUse /status to see budget details, or wait for replenishment.\x1b[0m"
                        );
                        continue;
                    }

                    // Consume from both trackers
                    state.inference_loop.consume_gas(turn_cost);
                    let _remaining = rt.block_on(
                        state
                            .cybernetics_loop
                            .acquire_budget(&state.agent_webid, turn_cost),
                    );

                    let response = rt.block_on(crate::commands::chat_with_agent(
                        input,
                        Some(&state.current_agent),
                        Some(&state.current_model),
                        Some(state.inference_port.clone()),
                        state.resolved_secrets.as_ref(),
                        Some(state.episodic_storage.clone()),
                        Some(state.semantic_storage.clone()),
                        Some(state.agent_webid),
                    ));

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
                    let analysis = hkask_cns::decompose_prompt(input);
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

                    // ── Loop System regulation cycle (7d/7e) ─────────────────
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

                    println!("{}: {}\n", state.current_agent, response);
                    state
                        .session_history
                        .record(&state.current_agent, &response);
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
