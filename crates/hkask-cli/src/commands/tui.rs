//! `kask tui` — Launch the interactive ratatui workspace.
//!
//! The TUI embeds the REPL as its chat window via `ReplBridge`. If ratatui
//! cannot initialize (no terminal, broken `TERM`), falls back to the
//! line-based REPL automatically.
//!
//! Non-interactive mode: `kask tui -f <file>` or `kask tui -f -` reads from
//! a file or stdin, prints the agent's response, and exits — no TUI launched.

use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;
use std::path::PathBuf;
use std::sync::Arc;

use crate::repl_host::CliHost;

/// Launch the TUI workspace or non-interactive chat.
///
/// pre:  rt is a valid tokio Runtime; registry is initialized; runtime is an McpRuntime
/// post: launches TUI (interactive) or prints one chat response (non-interactive via -f)
pub fn run_tui(
    rt: &tokio::runtime::Runtime,
    registry: &mut SqliteRegistry,
    runtime: &McpRuntime,
    handle: &tokio::runtime::Handle,
    template: Option<String>,
    input: Option<PathBuf>,
    agent: String,
    model: Option<String>,
) {
    if let Some(input_path) = input {
        let onboarding_outcome = match rt.block_on(crate::onboarding::run_onboarding()) {
            Ok(outcome) => outcome,
            Err(e) => {
                if matches!(e, crate::onboarding::OnboardingError::Cancelled) {
                    std::process::exit(0);
                }
                eprintln!("Cannot start: {}", e);
                eprintln!("Run `kask tui` first to complete onboarding interactively.");
                std::process::exit(1);
            }
        };
        let content = super::helpers::or_exit(
            std::fs::read_to_string(&input_path),
            "Failed to read input file",
        );
        print!("{}: ", agent);
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let chat_response = rt.block_on(super::chat::chat_with_agent_streaming(
            content.trim(),
            Some(&agent),
            model.as_deref(),
            None,
            onboarding_outcome.resolved_secrets.as_ref(),
            None,
            None,
            None,
            None,
        ));
        if let Some(ref usage) = chat_response.usage {
            eprintln!(
                "  {} tokens ({} prompt + {} completion)",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            );
        }
    } else {
        // Launch the TUI workspace. The TUI hosts the REPL as its chat window.
        // If the TUI feature is not built, fall back to the line-based REPL.
        #[cfg(feature = "tui")]
        {
            hkask_repl::run_tui(
                registry,
                runtime,
                template.as_deref(),
                &agent,
                model.as_deref(),
                handle.clone(),
                Arc::new(CliHost),
            );
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("TUI not built — rebuild with `cargo build --features tui`");
            hkask_repl::run(
                registry,
                runtime,
                template.as_deref(),
                &agent,
                model.as_deref(),
                handle.clone(),
                Arc::new(CliHost),
            );
        }
    }
}
