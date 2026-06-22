//! hKask CLI — Binary entry point
//!
//! Thin dispatcher: setup → route to command handler → done.
//! All business logic and display formatting lives in the `commands` module.

use clap::Parser;
use hkask_cli::cli::Commands;
use hkask_cli::commands;
use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;
use std::time::Instant;

/// Check fusion model configuration at startup.
///
/// When `HKASK_FUSION_MODEL` is set, verifies the fusion group exists on
/// OpenRouter. If the group is not found (or OpenRouter is unreachable), asks
/// the user whether to proceed with the default model or wait for the fusion
/// group to be set up. This prevents accidentally hitting OpenRouter's default
/// behavior (sending to ALL models, which can be very expensive).
fn check_fusion_startup(rt: &tokio::runtime::Runtime) {
    let config = InferenceConfig::from_env();
    let fusion = match &config.fusion_model {
        Some(f) => f.clone(),
        None => return,
    };

    eprintln!(
        "\n  \x1b[1;33m⚡ Fusion mode active\x1b[0m — model: \x1b[36m{}\x1b[0m",
        fusion
    );

    let router = InferenceRouter::new(config);
    match rt.block_on(router.verify_fusion_model()) {
        Ok(true) => {
            // Fusion verified — proceed silently. The user deliberately configured
            // this and the group exists. No need for a notice.
        }
        Ok(false) => {
            eprintln!(
                "  \x1b[1;31m✗\x1b[0m Fusion group \x1b[1mNOT FOUND\x1b[0m on OpenRouter.\n  Create a fusion group named \x1b[1m'kask'\x1b[0m at \x1b[34mhttps://openrouter.ai/fusion\x1b[0m\n"
            );
            eprintln!(
                "  OpenRouter's default sends to \x1b[1;31mALL models\x1b[0m — this can be \x1b[1;31mvery expensive\x1b[0m."
            );
            prompt_proceed_or_wait(&fusion, false);
        }
        Err(e) => {
            eprintln!("  \x1b[33m⚠\x1b[0m Could not verify fusion group: {}", e);
            prompt_proceed_or_wait(&fusion, true);
        }
    }
}

/// Ask the user whether to proceed with the default model or wait for fusion setup.
///
/// `connection_error`: if true, OpenRouter was unreachable (not just missing group).
fn prompt_proceed_or_wait(fusion: &str, connection_error: bool) {
    use std::io::{self, Write};

    let default = hkask_inference::model_constants::DEFAULT_FALLBACK_MODEL;
    let wait_msg = if connection_error {
        "Wait while I fix the OpenRouter connection and set up the 'kask' fusion group"
    } else {
        "Wait while I set up the 'kask' fusion group"
    };
    eprintln!();
    eprintln!(
        "  [\x1b[1;36m1\x1b[0m] Proceed with default model (\x1b[36m{}\x1b[0m)",
        default
    );
    eprintln!("  [\x1b[1;36m2\x1b[0m] {wait_msg}");
    eprintln!();
    eprint!("  Choose [1/2]: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        eprintln!("\n  Could not read input — proceeding with default model.\n");
        // SAFETY: called in main() before any threads or async tasks are spawned.
        unsafe { std::env::remove_var("HKASK_FUSION_MODEL") };
        return;
    }

    match input.trim() {
        "2" => {
            eprintln!();
            eprintln!("  Create your fusion group at \x1b[34mhttps://openrouter.ai/fusion\x1b[0m");
            eprintln!("  Name it \x1b[1m'kask'\x1b[0m, then re-run your command with:\n");
            eprintln!("    \x1b[1mexport HKASK_FUSION_MODEL={}\x1b[0m\n", fusion);
            eprintln!("  Run \x1b[1mkask doctor\x1b[0m to verify everything is wired correctly.\n");
            std::process::exit(0);
        }
        _ => {
            eprintln!(
                "\n  Proceeding with default model (\x1b[36m{}\x1b[0m).\n",
                default
            );
            // SAFETY: called in main() before any threads or async tasks are spawned.
            unsafe { std::env::remove_var("HKASK_FUSION_MODEL") };
        }
    }
}

fn main() {
    // Load .env from current directory (silently skip if absent)
    dotenvy::dotenv().ok();

    let cli = hkask_cli::cli::Cli::parse();
    hkask_cli::cli::init_logging(cli.verbose);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let handle = rt.handle().clone();

    // Verify fusion model if HKASK_FUSION_MODEL is set (P9: proactive cost-safety)
    check_fusion_startup(&rt);

    let mut registry = commands::helpers::or_exit(
        match &cli.registry {
            Some(path) => SqliteRegistry::new(Some(&path.to_string_lossy())),
            None => SqliteRegistry::new(None),
        },
        "Failed to initialize registry",
    );

    // Shared MCP runtime for chat and curator commands.
    // CLI commands that need MCP servers (mcp, models, web-search, serve)
    // create their own runtimes with servers started via start_server().
    let runtime = McpRuntime::new();

    // P9: CNS span
    let cns_start = Instant::now();
    tracing::info!(target: "cns.cli", operation = "command_invoked", command = ?cli.command, "CNS");
    tracing::info!(target: "cns.cli", operation = "command_dispatched", command = ?cli.command, "CNS");

    match cli.command {
        Commands::Chat {
            template,
            input,
            agent,
            model,
        } => commands::chat::run_chat(
            &rt,
            &mut registry,
            &runtime,
            &handle,
            template,
            input,
            agent,
            model,
        ),

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
            commands::curator::run_curator(&rt, &mut registry, &runtime, &handle, action)
        }

        Commands::Replicant { action } => commands::user::run_replicant(action),

        Commands::Keystore { action } => commands::keystore::run(action),

        Commands::Bundle { action } => commands::bundle::run_bundle(&rt, action),

        Commands::Skill { action } => commands::skill::run_skill(action),

        Commands::Style { action } => commands::style::run(&rt, action),

        Commands::Kanban { action } => {
            let webid = crate::commands::helpers::resolve_user_webid(); // P12: every action has author: every action has author
            commands::kanban::run_cli(action, webid, None);
        }
        Commands::Adapter { action } => commands::adapter::run(action),

        Commands::Qa { action } => commands::qa::run(&rt, action),

        Commands::Kata { action } => commands::kata::run(action, &registry),

        Commands::Models => commands::models::run(&rt),

        Commands::Doctor => commands::doctor::run_doctor_cmd(&rt),

        Commands::Onboard => commands::onboard::run(&rt),

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
        } => commands::test::run(&rt, crate_name, &format, watch),

        Commands::WebSearch { query, max_results } => {
            commands::web_search::run(&rt, query, max_results)
        }

        Commands::Serve {
            port,
            host,
            json_logs,
        } => {
            if let Err(e) = rt.block_on(commands::serve::run_server(port, &host, json_logs)) {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Init => {
            if let Err(e) = commands::init::run_init() {
                eprintln!("Init error: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Wallet { action } => commands::wallet::run(action),

        Commands::List {
            registry: list_target,
        } => commands::registry::run_list(&rt, &registry, list_target),

        Commands::Rm {
            target,
            db,
            passphrase,
        } => commands::registry::run_rm(&rt, &mut registry, target, db, passphrase),

        Commands::Transcript { path } => {
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

        Commands::Matrix { action } => commands::matrix::run(action),
    }

    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "command_completed", latency_ms = cns_start.elapsed().as_millis(), "CNS");
}
