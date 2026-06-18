//! hKask CLI — Binary entry point
//!
//! Thin dispatcher: setup → route to command handler → done.
//! All business logic and display formatting lives in the `commands` module.

use clap::Parser;
use hkask_cli::cli::Commands;
use hkask_cli::commands;
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;
use std::time::Instant;

fn main() {
    // Load .env from current directory (silently skip if absent)
    dotenvy::dotenv().ok();

    let cli = hkask_cli::cli::Cli::parse();
    hkask_cli::cli::init_logging(cli.verbose);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let handle = rt.handle().clone();

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

    // REQ: P9-CNS-SRF-001 pre: valid command parsed post: cns.cli span emitted
// expect: "I can access all hKask functionality through the kask CLI" [P3]
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

        Commands::Contract { action } => commands::contract::run(&rt, action),

        Commands::Kata { action } => commands::kata::run(action, &registry),

        Commands::Models => commands::models::run(&rt),

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
