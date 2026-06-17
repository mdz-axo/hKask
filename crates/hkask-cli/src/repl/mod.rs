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
mod energy;
pub(crate) mod handlers;
mod helper;
mod init;
mod tool_augmented;
mod turn;

use hkask_agents::InferenceLoop;
use hkask_agents::ports::EpisodicStoragePort;
use hkask_agents::ports::SemanticStoragePort;
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::ConsolidationService;
use hkask_services::AgentService;
use hkask_templates::{BundleManifest, ManifestExecutor, SqliteRegistry};
use hkask_types::PersonaConstraints;
use hkask_types::WebID;
use hkask_types::ports::InferencePort;
use hkask_types::secret::ZeroizingSecret;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config as ReadlineConfig, Editor};
use std::sync::Arc;

use commands::handle_slash_command;
use handlers::ReplSettings;
use helper::KaskHelper;

/// REPL state — initialized once, reused across all turns.
///
/// Holds the shared inference port, InferenceLoop (with energy budget
/// and circuit breaker), memory adapters, and inference config so they
/// aren't reconstructed per chat turn. Also groups mutable REPL state
/// to keep function signatures manageable.
pub(crate) struct ReplState {
    pub(crate) inference_port: Arc<dyn InferencePort>,
    /// InferenceLoop wrapping the port — provides energy budget tracking,
    /// circuit breaker state, and model selection via CNS observability.
    /// Arc-wrapped so it can be shared with the LoopSystem.
    pub(crate) inference_loop: Arc<InferenceLoop>,
    /// Episodic memory storage — private, agent-scoped
    pub(crate) episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic memory storage — shared, public knowledge
    pub(crate) semantic_storage: Arc<dyn SemanticStoragePort>,
    /// Agent WebID — derived from the agent name, used for memory operations
    pub(crate) agent_webid: WebID,
    pub(crate) current_model: String,
    pub(crate) current_agent: String,
    pub(crate) active_session: Option<String>,
    /// Pre-resolved secrets from onboarding, carried forward to avoid
    /// re-resolving from the OS keychain (which may use a mock backend
    /// with EntryOnly persistence on Linux).
    pub(crate) resolved_secrets: Option<hkask_services::ResolvedSecrets>,
    /// GovernedTool membrane — the singular governance boundary for MCP tool
    /// invocations. All tool calls route through this membrane, which enforces
    /// OCAP authority, energy budgets, and CNS observability.
    pub(crate) governed_tool: Arc<GovernedTool<RawMcpToolPort>>,
    /// ConsolidationService for /consolidate command — built from the same per-agent
    /// memory DB as `episodic_storage` and `semantic_storage`. None if memory
    /// infrastructure is unavailable.
    consolidation_service: Option<ConsolidationService>,
    /// Persona constraints for the current agent — loaded from agent definition.
    /// When set, the persona filter strips forbidden patterns from model output.
    persona_constraints: Option<PersonaConstraints>,
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
    manifest_executor: Option<ManifestExecutor>,
    /// The resolved process manifest for the current agent.
    /// Present when the agent definition includes a process_manifest reference
    /// and the manifest was successfully loaded.
    process_manifest: Option<BundleManifest>,
    /// Shared service context — the canonical assembly point for all
    /// infrastructure.
    pub(crate) service_context: Arc<AgentService>,
    /// REPL settings — user-configurable inference parameters.
    /// Exposed via /repl command. Magna Carta P3 (Generative Space).
    pub(crate) repl_settings: ReplSettings,
    /// Whether this session started from a first-run onboarding (true)
    /// or a returning session (false). Controls First Steps display.
    pub(crate) is_first_run: bool,
    /// Talk mode — when enabled, agent responses are summarized and spoken aloud.
    pub(crate) talk_enabled: bool,
    /// Voice design JSON for TTS (None = default "Rachel" voice).
    pub(crate) voice_design: Option<String>,
    /// Active improv mode — set via /improv command.
    /// None means no improv posture is active (default agent behavior).
    pub(crate) improv_mode: Option<hkask_improv::ImprovMode>,
}

pub fn run(
    _registry: &mut SqliteRegistry,
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

    display::print_banner(
        &state.current_agent,
        template_id,
        &state.current_model,
        state.is_first_run,
    );

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
                // Wrapped in ZeroizingSecret for defense-in-depth: the secret bytes
                // are scrubbed from memory on drop.
                let a2a_secret = match &state.resolved_secrets {
                    Some(secrets) => ZeroizingSecret::new(secrets.a2a_secret.as_bytes().to_vec()),
                    None => {
                        eprintln!(
                            "Error: No ACP secret resolved. Run `kask chat` to complete onboarding or set HKASK_MASTER_KEY."
                        );
                        continue;
                    }
                };

                if let Some(ref _session) = state.active_session.clone() {
                    // Dual-presence active. Fall back to single-agent mode.
                    state.active_session = None;
                    turn::single_agent_turn(input, &mut state, &rt, &a2a_secret);
                } else {
                    turn::single_agent_turn(input, &mut state, &rt, &a2a_secret);
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
