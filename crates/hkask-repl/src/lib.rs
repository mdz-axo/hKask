//! Interactive REPL for hKask — discoverable, self-documenting, alive.
//!
//! Design principles:
//! - Every capability is reachable from `/help`
//! - Tab completion for slash commands and agent names
//! - Fuzzy matching on slash commands (e.g. `/model`)
//! - Welcome banner with the Kask amphora logo
//! - Categorized help so the menu is scannable

mod builtin_servers;

#[cfg(feature = "tui")]
use hkask_tui::SystemBridge;
mod cns_display;
mod commands;
pub mod deps;
pub mod display;
mod energy;
pub mod handlers;
mod helper;
pub mod host;
#[cfg(feature = "tui")]
pub use host::{OnboardingError, OnboardingOutcome, ReplHost};
mod init;
mod threads;
mod tool_augmented;
#[cfg(feature = "tui")]
mod tui_bridges;
mod turn;

use hkask_mcp::runtime::McpRuntime;
use hkask_ports::ChatToolDefinition;
use hkask_services_context::AgentService;
use hkask_services_kata_kanban::KanbanService;
use hkask_templates::{BundleManifest, ManifestExecutor, SqliteRegistry};
use hkask_types::PersonaConstraints;
use hkask_types::WebID;
use hkask_types::secret::ZeroizingSecret;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config as ReadlineConfig, Editor};
use std::sync::Arc;

// Dependencies used via #[derive] or in submodules not directly importable.
use async_trait as _;
use hkask_memory as _;

use commands::handle_slash_command;
use handlers::ReplSettings;
use helper::KaskHelper;

/// Talk configuration — paired voice design and enabled state.
///
/// Set together via `/talk on|off|voice`. Checked in the turn pipeline
/// to decide whether to summarize and speak responses aloud.
#[derive(Debug, Clone)]
pub struct TalkConfig {
    /// Whether spoken summaries are emitted. Uses an enum (not `bool`)
    /// so the invalid state "off but has voice_design" is representable
    /// but the on/off decision is explicit at the type level.
    pub mode: TalkMode,
    /// Voice design JSON for TTS (None = default "Rachel" voice).
    pub voice_design: Option<String>,
}

/// Talk mode — whether the REPL speaks responses aloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalkMode {
    /// Spoken summaries are emitted after each turn.
    On,
    /// No spoken output.
    Off,
}

/// Manifest cascade — process manifest paired with its executor.
///
/// The two fields are always present together: the manifest defines the
/// steps, the executor runs them. Wrapping in a single `Option` enforces
/// the "both Some or both None" invariant at the type level — the invalid
/// state `Some(manifest) + None(executor)` is unrepresentable.
///
/// Cannot derive `Debug` because `ManifestExecutor` wraps non-Debug types
/// (trait objects, secrets).
#[allow(missing_debug_implementations)]
pub struct ManifestCascade {
    pub manifest: BundleManifest,
    pub executor: ManifestExecutor,
}

/// Manifest state — `Some` when the agent has a process manifest cascade
/// defined, `None` otherwise. The invariant (both fields together) is
/// enforced by the single `Option<ManifestCascade>` wrapper.
pub type ManifestState = Option<ManifestCascade>;

/// Tool prompt cache — the pre-formatted tool section and definitions.
///
/// Both are refreshed together during MCP tool discovery (REPL init
/// and after server start/stop). Cached because `ToolPort` uses
/// `impl Trait` returns, making re-derivation via `Arc<dyn ToolPort>`
/// infeasible without changing the port trait.
#[derive(Debug, Clone)]
pub struct ToolPrompt {
    /// Pre-formatted tool section of the system prompt.
    pub section: String,
    /// OpenAI-compatible tool definitions for native function calling.
    /// When non-empty, tools are included in inference requests.
    pub definitions: Vec<ChatToolDefinition>,
}

/// REPL state — surface-specific presentation fields.
/// All infrastructure (inference, memory, tool dispatch, gas tracking)
/// is accessed through `service_context: Arc<AgentService>`.
pub struct ReplState {
    /// Agent WebID — derived from the agent name, used for memory operations
    pub agent_webid: WebID,
    pub current_model: String,
    pub current_agent: String,
    pub active_session: Option<String>,
    /// Pre-resolved secrets from onboarding, carried forward to avoid
    /// re-resolving from the OS keychain (which may use a mock backend
    /// with EntryOnly persistence on Linux).
    pub resolved_secrets: Option<hkask_services_onboarding::ResolvedSecrets>,
    /// Persona constraints for the current agent — loaded from agent definition.
    /// When set, the persona filter strips forbidden patterns from model output.
    persona_constraints: Option<PersonaConstraints>,
    /// Tool prompt cache — derived from MCP runtime discovery. The cache is
    /// intentional: `ToolPort` uses `impl Trait` returns so it is not
    /// dyn-compatible, which prevents re-deriving on demand via
    /// `Arc<dyn ToolPort>`. Re-derive here when servers start/stop
    /// dynamically, or when making `ToolPort` dyn-compatible becomes
    /// a justified refactor.
    pub tool_prompt: ToolPrompt,
    /// Manifest state — `Some` when the agent has a process manifest
    /// cascade defined, `None` otherwise. The cascade bundles manifest +
    /// executor together so the "both present" invariant is enforced by
    /// construction.
    pub manifest_state: ManifestState,
    /// Shared service context — the canonical assembly point for all
    /// infrastructure.
    pub service_context: Arc<AgentService>,
    /// REPL settings — user-configurable inference parameters.
    /// Exposed via /repl command. Magna Carta P3 (Generative Space).
    pub repl_settings: ReplSettings,
    /// Whether this session started from a first-run onboarding (true)
    /// or a returning session (false). Controls First Steps display.
    pub is_first_run: bool,
    /// Talk configuration — voice design and enabled state.
    /// Set via /talk command; checked in the turn pipeline.
    pub talk_config: TalkConfig,
    /// Active improv mode — set via /improv command.
    /// None means no improv posture is active (default agent behavior).
    pub improv_mode: Option<hkask_improv::ImprovMode>,
    /// Kanban service — lazily initialized for /kanban commands.
    pub kanban_service: Option<KanbanService>,
    /// MCP servers that failed to auto-start (server_id → error message).
    /// Populated during REPL init; surfaced in the session banner so the user
    /// knows which capabilities are degraded before their first prompt.
    pub degraded_servers: Vec<(String, String)>,
    /// Chat thread registry — persists conversation threads across sessions.
    /// Loaded from `agents/{name}/threads.json` on REPL init. Supports
    /// thread listing, switching, creation, and archival with configurable
    /// short-term memory lifespan.
    pub thread_registry: threads::ThreadRegistry,
    /// Host trait — provides WebID resolution, onboarding, template
    /// listing, and transcript viewing to the REPL subsystem.
    pub host: Arc<dyn host::ReplHost>,
}

impl std::fmt::Debug for ReplState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplState")
            .field("agent_webid", &self.agent_webid)
            .field("current_model", &self.current_model)
            .field("current_agent", &self.current_agent)
            .field("active_session", &self.active_session)
            // Redacted: carries master key and DB passphrase.
            .field("resolved_secrets", &"<redacted>")
            .field("persona_constraints", &self.persona_constraints)
            .field("tool_prompt", &self.tool_prompt)
            // ManifestCascade wraps non-Debug trait objects.
            .field(
                "manifest_state",
                &if self.manifest_state.is_some() {
                    "Some(<ManifestCascade>)"
                } else {
                    "None"
                },
            )
            .field("service_context", &"<AgentService>")
            .field("repl_settings", &self.repl_settings)
            .field("is_first_run", &self.is_first_run)
            .field("talk_config", &self.talk_config)
            .field("improv_mode", &self.improv_mode)
            .field("kanban_service", &self.kanban_service.is_some())
            .field("degraded_servers", &self.degraded_servers)
            .field("thread_registry", &self.thread_registry)
            .field("host", &"<ReplHost>")
            .finish()
    }
}

pub fn run(
    _registry: &mut SqliteRegistry,
    _runtime: &McpRuntime,
    template_id: Option<&str>,
    _agent_name: &str,
    initial_model: Option<&str>,
    rt_handle: tokio::runtime::Handle,
    host: Arc<dyn host::ReplHost>,
) {
    let Some(mut state) =
        init::init_repl_state(_registry, _runtime, initial_model, &rt_handle, host)
    else {
        return;
    };

    let helper = KaskHelper::new(state.thread_registry.clone());

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
        &state.degraded_servers,
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
                    turn::single_agent_turn(input, &mut state, &rt, &a2a_secret, None);
                } else {
                    turn::single_agent_turn(input, &mut state, &rt, &a2a_secret, None);
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
    host: Arc<dyn host::ReplHost>,
) {
    let Some(state) = init::init_repl_state(
        _registry,
        _runtime,
        initial_model,
        &rt_handle,
        Arc::clone(&host),
    ) else {
        return;
    };

    let agent_name = state.current_agent.clone();
    let model = state.current_model.clone();

    // Resolve A2A secret for capability tokens. Wrapped in ZeroizingSecret
    // so the bytes are scrubbed from memory when the bridge is dropped.
    let a2a_secret = state
        .resolved_secrets
        .as_ref()
        .map(|s| hkask_types::secret::ZeroizingSecret::new(s.a2a_secret.as_bytes().to_vec()))
        .unwrap_or_else(|| hkask_types::secret::ZeroizingSecret::new(Vec::new()));

    // Compute layout path before agent_name is moved into the bridge
    let layout_path = hkask_tui::layout::layout_path(&agent_name);

    // Keep ReplState alive inside the bridge for full inference
    let bridge = Arc::new(TuiReplBridge {
        state: Arc::new(std::sync::Mutex::new(state)),
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

    match hkask_tui::TuiSession::new(
        bridge.clone() as Arc<dyn hkask_tui::SystemBridge>,
        bridge.clone(),
    ) {
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
                .with_skills_bridge(bridge.clone())
                .with_scenarios_bridge(bridge.clone());
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
                host,
            );
        }
    }
}

/// Bridge implementation connecting the TUI to hKask's full inference engine.
#[cfg(feature = "tui")]
struct TuiReplBridge {
    state: Arc<std::sync::Mutex<ReplState>>,
    rt_handle: tokio::runtime::Handle,
    a2a_secret: hkask_types::secret::ZeroizingSecret,
    agent_name: String,
    model: String,
    pending: std::sync::Mutex<Option<std::sync::mpsc::Receiver<hkask_tui::TuiTurnResult>>>,
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
    fn build_result(capture: &turn::TurnCapture) -> hkask_tui::TuiTurnResult {
        if capture.budget_exhausted {
            return hkask_tui::TuiTurnResult {
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
        hkask_tui::TuiTurnResult {
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
        *streaming.lock().unwrap_or_else(|e| e.into_inner()) = String::new();
        *self.pending.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);

        std::thread::spawn(move || {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
            let capture = turn::single_agent_turn_captured(&input, &mut s, &rt, a2a.as_bytes());
            let result = Self::build_result(&capture);

            // Publish the full response text to the streaming buffer so the
            // TUI can render it immediately on the next poll. The previous
            // implementation faked streaming by chunking the text 3 chars at
            // a time with an 8ms sleep — this added latency (a 1000-char
            // response took ~2.7s of artificial delay on top of the actual
            // inference time) and blocked a thread doing nothing. Real
            // streaming should be wired through InferencePort::generate_stream
            // if progressive display is desired.
            if let Ok(mut buf) = streaming.lock() {
                buf.push_str(&result.text);
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

        *streaming.lock().unwrap_or_else(|e| e.into_inner()) = String::new();
        *self.pending.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);

        std::thread::spawn(move || {
            let mut s = state.lock().unwrap_or_else(|e| e.into_inner());

            // Scope tool definitions to the requested MCP server. We save
            // and restore the originals around the turn. If the turn panics,
            // the definitions would be left scoped — but the thread is
            // isolated and the next /mcp start refreshes tool_prompt anyway.
            // A panic-safe guard would require passing scoped tools as a
            // parameter to single_agent_turn_captured (a larger refactor).
            let original_tools = std::mem::take(&mut s.tool_prompt.definitions);
            let prefix = format!("{}/", scope);
            s.tool_prompt.definitions = original_tools
                .iter()
                .filter(|td| td.function.name.starts_with(&prefix))
                .cloned()
                .collect();

            let capture = turn::single_agent_turn_captured(&input, &mut s, &rt, a2a.as_bytes());

            // Restore original tool definitions.
            s.tool_prompt.definitions = original_tools;

            let result = Self::build_result(&capture);

            // Publish the full response text to the streaming buffer immediately.
            // See start_inference for why we don't fake-stream chunk by chunk.
            if let Ok(mut buf) = streaming.lock() {
                buf.push_str(&result.text);
            }

            let _ = tx.send(result);
        });
    }

    fn poll_inference(&self) -> hkask_tui::InferenceState {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
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
        self.streaming_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn send_message_blocking(&self, input: &str) -> hkask_tui::TuiTurnResult {
        let (result, context_length) = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let capture = turn::single_agent_turn_captured(
                input,
                &mut state,
                &self.rt_handle,
                self.a2a_secret.as_bytes(),
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

#[cfg(feature = "tui")]
impl hkask_tui::SystemBridge for TuiReplBridge {
    fn agent_name(&self) -> &str {
        &self.agent_name
    }
    fn model_name(&self) -> &str {
        &self.model
    }
    fn gas_remaining(&self) -> u64 {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.service_context.gas_remaining())
            .unwrap_or(0)
    }
    fn gas_cap(&self) -> u64 {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.service_context.gas_cap())
            .unwrap_or(0)
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
            0.0
        } else {
            (used as f64 / window as f64).min(1.0)
        }
    }
    fn mcp_status(&self) -> (usize, usize) {
        if let Ok(s) = self.state.lock() {
            let runtime = s.service_context.infra().mcp.clone().clone();
            let loaded = self.rt_handle.block_on(runtime.list_servers()).len();
            let total = hkask_mcp::BUILTIN_SERVERS.len();
            (loaded, total)
        } else {
            (0, hkask_mcp::BUILTIN_SERVERS.len())
        }
    }
    fn pod_counts(&self) -> (usize, usize, usize) {
        if self.state.lock().is_ok() {
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
            ("cns.mcp.media.face".into(), true),
            ("cns.storage".into(), true),
            ("cns.keystore".into(), true),
            ("cns.tui".into(), true),
        ]
    }
}
