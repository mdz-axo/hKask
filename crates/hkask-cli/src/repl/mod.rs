//! Interactive REPL for hKask — discoverable, self-documenting, alive.
//!
//! Design principles:
//! - Every capability is reachable from `/help`
//! - Tab completion for slash commands and agent names
//! - Fuzzy matching on slash commands (e.g. `/model`)
//! - Welcome banner with the Kask amphora logo
//! - Categorized help so the menu is scannable

mod builtin_servers;
mod cns_display;
mod commands;
pub(crate) mod display;
mod gas;
mod handlers;
mod helper;
mod hhh_loop;
mod init;
mod memory;
mod tool_augmented;
mod turn;

pub(crate) use tool_augmented::TOOL_CALL_FORMAT_INTRO;

use hkask_agents::HhhConfig;
use hkask_agents::HhhMode;
use hkask_agents::InferenceLoop;
use hkask_agents::LoopSystem;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::ports::EpisodicStoragePort;
use hkask_agents::ports::SemanticStoragePort;
use hkask_cns::{CnsRuntime, CyberneticsLoop, GovernedTool};
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::ConsolidationService;
use hkask_templates::{BundleManifest, ManifestExecutor, OkapiConfig, SqliteRegistry};
use hkask_types::PersonaConstraints;
use hkask_types::WebID;
use hkask_types::ports::InferencePort;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config as ReadlineConfig, Editor};
use std::sync::Arc;
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
    pub(crate) manifest_executor: Option<ManifestExecutor>,
    /// The resolved process manifest for the current agent.
    /// Present when the agent definition includes a process_manifest reference
    /// and the manifest was successfully loaded.
    pub(crate) process_manifest: Option<BundleManifest>,
}

pub fn run(
    _registry: &SqliteRegistry,
    _runtime: &McpRuntime,
    template_id: Option<&str>,
    _agent_name: &str,
    initial_model: Option<&str>,
    rt_handle: tokio::runtime::Handle,
) {
    let Some(mut state) = init::init_repl_state(_registry, _runtime, initial_model, &rt_handle)
    else {
        return;
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

                // ACP secret for signing capability tokens in tool invocations.
                // Derived from onboarding — same secret that governs OCAP authority.
                let acp_secret: Vec<u8> = match &state.resolved_secrets {
                    Some(secrets) => secrets.acp_secret.as_bytes().to_vec(),
                    None => {
                        eprintln!(
                            "Error: No ACP secret resolved. Run `kask chat` to complete onboarding or set HKASK_MASTER_KEY."
                        );
                        continue;
                    }
                };

                if let Some(ref session) = state.active_session.clone() {
                    turn::ensemble_turn(session, input, &mut state, &rt, &acp_secret);
                } else {
                    turn::single_agent_turn(input, &mut state, &rt, &acp_secret);
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
