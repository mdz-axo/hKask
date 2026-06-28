//! hKask CLI — Binary entry point
//!
//! Thin dispatcher: setup → route to command handler → done.
//! All business logic and display formatting lives in the `commands` module.

use clap::Parser;
use hkask_cli::cli::Commands;
use hkask_cli::commands;
use hkask_mcp::runtime::McpRuntime;
use hkask_services::InferenceConfig;
use hkask_templates::SqliteRegistry;
use std::time::Instant;

/// Check fusion model configuration at startup.
///
/// Fusion is opt-in — only active when HKASK_FUSION_JUDGE,
/// HKASK_FUSION_KILO_TIER, or legacy vars are explicitly set.
fn check_fusion_startup() {
    let config = InferenceConfig::from_env();
    let fusion = match &config.fusion {
        Some(f) => f,
        None => return,
    };

    let (model, desc) = (fusion.model_id(), fusion.description());
    eprintln!(
        "\n  \x1b[1;33m⚡ Fusion mode active\x1b[0m — model: \x1b[36m{model}\x1b[0m\n     {desc}"
    );
}

/// ── Main ─────────────────────────────────────────────────────────────────
fn main() {
    // Load .env from current directory (silently skip if absent)
    dotenvy::dotenv().ok();

    let cli = hkask_cli::cli::Cli::parse();
    hkask_cli::cli::init_logging(cli.verbose, cli.json_logs);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let handle = rt.handle().clone();

    // Verify fusion model if configured (P9: proactive cost-safety)
    check_fusion_startup();

    let mut registry = commands::helpers::or_exit(
        match &cli.registry {
            Some(path) => SqliteRegistry::new(Some(&path.to_string_lossy())),
            None => SqliteRegistry::new(None),
        },
        "Failed to initialize registry",
    );

    // P9: CNS span
    let cns_start = Instant::now();
    tracing::info!(
        target: "cns.cli",
        operation = "command_dispatched",
        command = %cli.command.label(),
        "CNS"
    );

    match cli.command {
        Commands::Chat {
            template,
            input,
            agent,
            model,
            tui,
        } => {
            // McpRuntime created only when Chat needs it (P5: avoid waste)
            let runtime = McpRuntime::new();
            commands::chat::run_chat(
                &rt,
                &mut registry,
                &runtime,
                &handle,
                template,
                input,
                agent,
                model,
                tui,
            );
        }

        Commands::Template { action } => commands::template::run_template(&mut registry, action),

        Commands::Bot { action } => commands::agent::run_bot(&rt, action),

        Commands::Pod { action } => commands::pod::run_pod(&rt, action),

        Commands::Mcp { action } => commands::mcp::run(&rt, action),

        Commands::Cns { action } => commands::cns::run(&rt, action),

        Commands::Sovereignty { action } => commands::sovereignty::run(action),

        Commands::Goal { action } => commands::goal::run_goal(action),

        Commands::Docs { action } => commands::docs::run(action),

        Commands::Git { action } => commands::git_cmd::run(&rt, action),

        Commands::Backup { action } => commands::backup_cmd::run(&rt, action),

        Commands::Spec { action } => commands::spec::run(action),

        Commands::Agent { action } => commands::agent::run_agent(&rt, action),

        Commands::Curator { action } => {
            // McpRuntime created only when Curator needs it (P5: avoid waste)
            let runtime = McpRuntime::new();
            commands::curator::run_curator(&rt, &mut registry, &runtime, &handle, action)
        }

        Commands::Federation { action } => commands::federation::run_federation(&rt, action),

        Commands::Token { action } => commands::token::run_token(&rt, action),

        Commands::Replicant { action } => commands::user::run_replicant(&rt, action),

        Commands::Keystore { action } => commands::keystore::run(action),

        Commands::Bundle { action } => commands::bundle::run_bundle(&rt, action),

        Commands::Skill { action } => commands::skill::run_skill(action),

        Commands::Style { action } => commands::style::run(&rt, action),

        Commands::Kanban { action } => {
            let webid = crate::commands::helpers::resolve_user_webid(); // P12: every action has author: every action has author
            commands::kanban::run_cli(action, webid, None);
        }
        Commands::Adapter { action } => commands::adapter::run(action),

        Commands::Kata { action } => commands::kata::run(&rt, action, &registry),

        Commands::Models => commands::models::run(&rt),

        Commands::Doctor => commands::doctor::run_doctor_cmd(&rt),

        Commands::Onboard => match rt.block_on(hkask_cli::onboarding::run_add_replicant()) {
            Ok(()) => {}
            Err(e) => {
                if matches!(e, hkask_cli::onboarding::OnboardingError::Cancelled) {
                    std::process::exit(0);
                }
                eprintln!("Onboarding failed: {}", e);
                std::process::exit(1);
            }
        },

        Commands::Settings { action } => commands::settings::run(action),

        Commands::Consolidate {
            agent,
            limit,
            confidence_floor,
            max_semantic_triples,
            passphrase,
        } => commands::consolidation::run(
            agent.as_deref(),
            limit,
            confidence_floor,
            max_semantic_triples,
            passphrase.as_deref(),
        ),

        Commands::Loops => commands::loops::run(&rt),

        Commands::Daemon { action } => commands::daemon::run(&rt, action),

        Commands::Test {
            crate_name,
            format,
            watch,
        } => commands::test::run(crate_name, &format, watch),

        Commands::WebSearch { query, max_results } => {
            commands::web_search::run(&rt, query, max_results)
        }

        Commands::Serve {
            port: _port,
            host: _host,
        } => {
            #[cfg(feature = "api")]
            {
                if let Err(e) = rt.block_on(commands::serve::run_server(_port, &_host)) {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                }
            }
            #[cfg(not(feature = "api"))]
            {
                eprintln!("HTTP API server not built — rebuild with `cargo build --features api`");
                std::process::exit(1);
            }
        }

        Commands::Init => {
            if let Err(e) = commands::init::run_init() {
                eprintln!("Init error: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Export { action } => {
            commands::export_cmd::run(&rt, action);
        }

        Commands::Wallet { action } => commands::wallet::run(&rt, action),

        Commands::List {
            registry: list_target,
        } => commands::registry::run_list(&registry, list_target),

        Commands::Rm {
            target,
            db,
            passphrase,
        } => commands::registry::run_rm(&mut registry, target, db, passphrase),

        Commands::Transcript { path } => {
            #[cfg(feature = "tui")]
            {
                let mut viewer = hkask_cli::transcript_viewer::TranscriptViewer::from_file(&path)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to load transcript: {}", e);
                        std::process::exit(1);
                    });
                if let Err(e) = viewer.run() {
                    eprintln!("Transcript viewer error: {}", e);
                    std::process::exit(1);
                }
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!(
                    "Transcript viewer not built — rebuild with `cargo build --features tui`"
                );
                std::process::exit(1);
            }
        }

        Commands::Matrix { action } => commands::matrix::run(action),

        Commands::Repair { dry_run, force } => commands::repair::run(dry_run, force),
    }

    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "command_completed", latency_ms = cns_start.elapsed().as_millis(), "CNS");
}
