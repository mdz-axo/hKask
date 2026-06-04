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

use hkask_mcp::runtime::McpRuntime;
use hkask_templates::{OkapiConfig, OkapiInference, SqliteRegistry};
use hkask_types::ports::InferencePort;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config as ReadlineConfig, Editor};
use std::sync::Arc;

use commands::handle_slash_command;
use helper::{KaskHelper, SessionHistory};

/// REPL state — initialized once, reused across all turns.
///
/// Holds the shared inference port and Okapi config so they aren't
/// reconstructed per chat turn or model listing. Also groups mutable
/// REPL state to keep function signatures manageable.
pub(crate) struct ReplState {
    pub(crate) inference_port: Arc<dyn InferencePort>,
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

    let mut state = ReplState {
        inference_port,
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
                    let response = rt.block_on(crate::commands::chat_with_agent(
                        input,
                        Some(&state.current_agent),
                        Some(&state.current_model),
                        Some(state.inference_port.clone()),
                        state.resolved_secrets.as_ref(),
                    ));
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
