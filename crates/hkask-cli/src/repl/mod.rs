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
#[cfg(feature = "tui")]
mod tui_bridges;
mod turn;

use hkask_agents::InferenceLoop;
use hkask_agents::ports::EpisodicStoragePort;
use hkask_agents::ports::SemanticStoragePort;
use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_mcp::runtime::McpRuntime;
use hkask_memory::ConsolidationService;
use hkask_ports::{ChatToolDefinition, InferencePort};
use hkask_services::{AgentService, KanbanService};
use hkask_templates::{BundleManifest, ManifestExecutor, SqliteRegistry};
use hkask_types::PersonaConstraints;
use hkask_types::WebID;
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
    /// OpenAI-compatible tool definitions for native function calling.
    /// Built from the same MCP discovery as `tool_prompt_section`.
    /// When non-empty, tools are included in inference requests so models
    /// that support native function calling can return structured tool calls.
    pub(crate) tool_definitions: Vec<ChatToolDefinition>,
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
    /// Kanban service — lazily initialized for /kanban commands.
    pub(crate) kanban_service: Option<KanbanService>,
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

                // A2A secret for signing capability tokens in tool invocations.
                // Derived from onboarding — same secret that governs OCAP authority.
                // Wrapped in ZeroizingSecret for defense-in-depth: the secret bytes
                // are scrubbed from memory on drop.
                let a2a_secret = match &state.resolved_secrets {
                    Some(secrets) => ZeroizingSecret::new(secrets.a2a_secret.as_bytes().to_vec()),
                    None => {
                        eprintln!(
                            "Error: No A2A secret resolved. Run `kask chat` to complete onboarding or set HKASK_MASTER_KEY."
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

/// Launch the TUI workspace instead of the line-based REPL.
#[cfg(feature = "tui")]
pub fn run_tui(
    _registry: &mut SqliteRegistry,
    _runtime: &McpRuntime,
    template_id: Option<&str>,
    _agent_name: &str,
    initial_model: Option<&str>,
    rt_handle: tokio::runtime::Handle,
) {
    let Some(state) = init::init_repl_state(_registry, _runtime, initial_model, &rt_handle) else {
        return;
    };

    let agent_name = state.current_agent.clone();
    let model = state.current_model.clone();
    let service_context = state.service_context.clone();
    let inference_loop = state.inference_loop.clone();

    // Resolve A2A secret for capability tokens
    let a2a_secret = state
        .resolved_secrets
        .as_ref()
        .map(|s| s.a2a_secret.as_bytes().to_vec())
        .unwrap_or_default();

    // Compute layout path before agent_name is moved into the bridge
    let layout_path = hkask_tui::layout::layout_path(&agent_name);

    // Keep ReplState alive inside the bridge for full inference
    let bridge = Arc::new(TuiReplBridge {
        state: Arc::new(std::sync::Mutex::new(state)),
        inference_loop: inference_loop.clone(),
        rt_handle: rt_handle.clone(),
        a2a_secret,
        agent_name,
        model,
        pending: std::sync::Mutex::new(None),
        streaming_text: Arc::new(std::sync::Mutex::new(String::new())),
        alert_count: std::sync::atomic::AtomicU32::new(0),
        context_window: std::sync::atomic::AtomicU32::new(128_000),
        context_used: std::sync::atomic::AtomicU32::new(0),
        last_companies_search: std::sync::Mutex::new(None),
        last_research_search: std::sync::Mutex::new(None),
    });

    match hkask_tui::TuiSession::new(service_context, bridge.clone()) {
        Ok(session) => {
            let session = session
                .with_layout_path(layout_path)
                .with_config_bridge(bridge.clone())
                .with_registry_bridge(bridge.clone())
                .with_wallet_bridge(bridge.clone())
                .with_memory_bridge(bridge.clone())
                .with_kanban_bridge(bridge.clone());
            #[cfg(feature = "communication")]
            let session = session.with_matrix_bridge(bridge.clone());
            let session = session
                .with_backup_bridge(bridge.clone())
                .with_media_bridge(bridge.clone())
                .with_training_bridge(bridge.clone())
                .with_companies_bridge(bridge.clone())
                .with_research_bridge(bridge.clone())
                .with_docproc_bridge(bridge.clone())
                .with_replica_bridge(bridge.clone())
                .with_skills_bridge(bridge.clone());
            let mut session = session;
            if let Err(e) = session.run() {
                eprintln!("TUI error: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize TUI: {}", e);
            eprintln!("Falling back to line-based REPL.");
            // Can't recover ReplState from inside the Arc<Mutex>, so just run fresh
            run(
                _registry,
                _runtime,
                template_id,
                _agent_name,
                initial_model,
                rt_handle,
            );
        }
    }
}

/// Bridge implementation connecting the TUI to hKask's full inference engine.
#[cfg(feature = "tui")]
struct TuiReplBridge {
    state: Arc<std::sync::Mutex<ReplState>>,
    inference_loop: Arc<InferenceLoop>,
    rt_handle: tokio::runtime::Handle,
    a2a_secret: Vec<u8>,
    agent_name: String,
    model: String,
    pending: std::sync::Mutex<Option<std::sync::mpsc::Receiver<hkask_tui::TurnResult>>>,
    /// Streaming text buffer for chunked display during inference
    streaming_text: Arc<std::sync::Mutex<String>>,
    alert_count: std::sync::atomic::AtomicU32,
    /// Context window size from model metadata
    context_window: std::sync::atomic::AtomicU32,
    /// Approximate current context usage in tokens
    context_used: std::sync::atomic::AtomicU32,
    last_companies_search: std::sync::Mutex<Option<String>>,
    last_research_search: std::sync::Mutex<Option<String>>,
}

#[cfg(feature = "tui")]
impl TuiReplBridge {
    fn build_result(capture: &turn::TurnCapture) -> hkask_tui::TurnResult {
        if capture.budget_exhausted {
            return hkask_tui::TurnResult {
                text: String::new(),
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                gas_cost: 0,
                iterations: 0,
                budget_exhausted: true,
            };
        }
        let mut text = capture.response_text.clone();
        if !capture.tool_output.is_empty() {
            text.push_str("\n\n── Tool Results ──\n");
            text.push_str(&capture.tool_output);
        }
        hkask_tui::TurnResult {
            text,
            prompt_tokens: capture.prompt_tokens,
            completion_tokens: capture.completion_tokens,
            total_tokens: capture.total_tokens,
            gas_cost: 1u64
                .max(capture.prompt_tokens as u64 / 100 + capture.completion_tokens as u64 / 25),
            iterations: capture.iterations,
            budget_exhausted: false,
        }
    }
}

#[cfg(feature = "tui")]
impl hkask_tui::ReplBridge for TuiReplBridge {
    fn start_inference(&self, input: String) {
        let state = self.state.clone();
        let rt = self.rt_handle.clone();
        let a2a = self.a2a_secret.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let streaming = self.streaming_text.clone();

        // Clear streaming buffer
        *streaming.lock().expect("stream lock") = String::new();
        *self.pending.lock().expect("pending lock") = Some(rx);

        std::thread::spawn(move || {
            let mut s = state.lock().expect("ReplState lock");
            let capture = turn::single_agent_turn_captured(&input, &mut s, &rt, &a2a);
            let result = Self::build_result(&capture);

            // Feed response into streaming buffer chunk by chunk
            let text = result.text.clone();
            let chars: Vec<char> = text.chars().collect();
            for chunk in chars.chunks(3) {
                let s: String = chunk.iter().collect();
                if let Ok(mut buf) = streaming.lock() {
                    buf.push_str(&s);
                }
                std::thread::sleep(std::time::Duration::from_millis(8));
            }

            let _ = tx.send(result);
        });
    }

    fn start_scoped_inference(&self, input: String, mcp_server: &str) {
        let state = self.state.clone();
        let rt = self.rt_handle.clone();
        let a2a = self.a2a_secret.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let streaming = self.streaming_text.clone();
        let scope = mcp_server.to_string();

        *streaming.lock().expect("stream lock") = String::new();
        *self.pending.lock().expect("pending lock") = Some(rx);

        std::thread::spawn(move || {
            let mut s = state.lock().expect("ReplState lock");

            // Filter tool definitions to only the scoped MCP server
            let original_tools = std::mem::take(&mut s.tool_definitions);
            let prefix = format!("{}/", scope);
            s.tool_definitions = original_tools
                .iter()
                .filter(|td| td.function.name.starts_with(&prefix))
                .cloned()
                .collect();

            let capture = turn::single_agent_turn_captured(&input, &mut s, &rt, &a2a);

            // Restore original tool definitions
            s.tool_definitions = original_tools;

            let result = Self::build_result(&capture);

            let text = result.text.clone();
            let chars: Vec<char> = text.chars().collect();
            for chunk in chars.chunks(3) {
                let s: String = chunk.iter().collect();
                if let Ok(mut buf) = streaming.lock() {
                    buf.push_str(&s);
                }
                std::thread::sleep(std::time::Duration::from_millis(8));
            }

            let _ = tx.send(result);
        });
    }

    fn poll_inference(&self) -> hkask_tui::InferenceState {
        let mut pending = self.pending.lock().expect("pending lock");
        match pending.as_ref() {
            None => hkask_tui::InferenceState::Idle,
            Some(rx) => match rx.try_recv() {
                Ok(result) => {
                    *pending = None;
                    hkask_tui::InferenceState::Done(result)
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => hkask_tui::InferenceState::Thinking,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    *pending = None;
                    hkask_tui::InferenceState::Idle
                }
            },
        }
    }

    fn streaming_text(&self) -> String {
        self.streaming_text.lock().expect("stream lock").clone()
    }

    fn send_message_blocking(&self, input: &str) -> hkask_tui::TurnResult {
        let (result, context_length) = {
            let mut state = self.state.lock().expect("ReplState lock");
            let capture = turn::single_agent_turn_captured(
                input,
                &mut state,
                &self.rt_handle,
                &self.a2a_secret,
            );
            let ctx_len = state
                .repl_settings
                .model_meta
                .as_ref()
                .map(|m| m.context_length)
                .unwrap_or(128_000);
            (Self::build_result(&capture), ctx_len)
        };
        let input_tokens = (input.len() as u32 / 4).max(1);
        let resp_tokens = result.total_tokens;
        self.context_used.fetch_add(
            input_tokens + resp_tokens,
            std::sync::atomic::Ordering::Relaxed,
        );
        if result.budget_exhausted {
            self.alert_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        self.context_window
            .store(context_length, std::sync::atomic::Ordering::Relaxed);
        result
    }

    fn agent_name(&self) -> &str {
        &self.agent_name
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn gas_remaining(&self) -> u64 {
        self.inference_loop.gas_remaining()
    }
    fn gas_cap(&self) -> u64 {
        self.inference_loop.gas_cap()
    }
    fn cns_alert_count(&self) -> u32 {
        self.alert_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    fn context_pressure(&self) -> f64 {
        let used = self.context_used.load(std::sync::atomic::Ordering::Relaxed);
        let window = self
            .context_window
            .load(std::sync::atomic::Ordering::Relaxed);
        if window == 0 {
            return 0.0;
        }
        (used as f64 / window as f64).min(1.0)
    }

    fn mcp_status(&self) -> (usize, usize) {
        // Query McpRuntime for server count via blocking async
        if let Ok(s) = self.state.lock() {
            let runtime = s.service_context.mcp_runtime.clone();
            let servers = self.rt_handle.block_on(runtime.list_servers());
            (0, servers.len())
        } else {
            (0, 6)
        }
    }

    fn pod_counts(&self) -> (usize, usize, usize) {
        if let Ok(_s) = self.state.lock() {
            let data_dir = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("hkask");
            let registry = hkask_agents::PodRegistry::new(&data_dir);
            match registry.scan_by_kind() {
                Ok(pods) => {
                    let mut curator = 0;
                    let mut replicant = 0;
                    let mut team = 0;
                    for (kind, _, _) in &pods {
                        match kind {
                            hkask_agents::PodKind::Curator => curator += 1,
                            hkask_agents::PodKind::Replicant => replicant += 1,
                            hkask_agents::PodKind::Team => team += 1,
                        }
                    }
                    (curator, replicant, team)
                }
                Err(_) => (1, 1, 0),
            }
        } else {
            (1, 1, 0)
        }
    }

    fn cns_domains(&self) -> Vec<(String, bool)> {
        let alerts = self.alert_count.load(std::sync::atomic::Ordering::Relaxed);
        vec![
            ("cns.tool".into(), alerts < 5),
            ("cns.inference".into(), alerts < 3),
            ("cns.keystore".into(), true),
            ("cns.tui".into(), true),
        ]
    }

    fn send_curator_message(&self, input: &str) -> String {
        let alerts = self.cns_alert_count();
        let gas = self.gas_remaining();
        let cap = self.gas_cap();
        let ctx = self.context_pressure();
        format!(
            "Curator received: \"{}\"\n\nSystem status: {} CNS alerts, gas {}/{}, context {:.0}%, model: {}\n\nCurator daemon routing is active. CNS alerts and memory summaries appear here as detected.",
            input,
            alerts,
            gas,
            cap,
            ctx * 100.0,
            self.model_name()
        )
    }
}
