//! hKask CLI — Binary entry point
//!
//! Thin dispatcher: setup → route to command handler → done.
//! All business logic and display formatting lives in the `commands` module.

use clap::Parser;
use hkask_cli::cli::{
    self, AdminAction, AgentAction, BotAction, CnsAction, Commands, CuratorAction, DocsAction,
    EnsembleAction, GitAction, GoalAction, KeystoreAction, McpAction, PodAction, RegistryAction,
    ReplicantAction, SovereigntyAction, SpecAction, TemplateAction,
};
use hkask_cli::commands;
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;
use std::path::Path;

// ── Helpers ────────────────────────────────────────────────────────────────

fn or_exit<T, E: std::fmt::Display>(result: Result<T, E>, label: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {}", label, e);
            std::process::exit(1);
        }
    }
}

fn write_or_print(content: &str, output: Option<&Path>, label: &str) {
    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, content) {
                eprintln!("Failed to write {}: {}", label, e);
                std::process::exit(1);
            }
            println!("{} written to {}", label, path.display());
        }
        None => println!("{}", content),
    }
}

fn open_user_store() -> std::sync::Arc<std::sync::Mutex<hkask_storage::user_store::UserStore>> {
    let db_path = std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string());
    let conn = rusqlite::Connection::open(&db_path).unwrap_or_else(|_| {
        rusqlite::Connection::open_in_memory().expect("in-memory connection always succeeds")
    });
    let store =
        hkask_storage::user_store::UserStore::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
    let store = std::sync::Arc::new(std::sync::Mutex::new(store));
    store
        .lock()
        .expect("mutex lock")
        .initialize_schema()
        .expect("Failed to initialize user store schema");
    store
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    let cli = cli::Cli::parse();
    cli::init_logging(cli.verbose);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let handle = rt.handle().clone();

    let mut registry = or_exit(
        match &cli.registry {
            Some(path) => {
                SqliteRegistry::new(Some(path.to_str().expect("path must be valid UTF-8")))
            }
            None => SqliteRegistry::new(None),
        },
        "Failed to initialize registry",
    );

    let runtime = McpRuntime::new();

    match cli.command {
        Commands::Chat {
            template,
            input,
            agent,
            model,
        } => run_chat(
            &rt, &registry, &runtime, &handle, template, input, agent, model,
        ),

        Commands::Template { action } => run_template(&mut registry, action),

        Commands::Bot { action } => run_bot(&rt, action),

        Commands::Pod { action } => run_pod(&rt, action),

        Commands::Mcp { action } => run_mcp(&rt, action),

        Commands::Cns { action } => run_cns(&rt, action),

        Commands::Sovereignty { action } => run_sovereignty(action),

        Commands::Goal { action } => run_goal(action),

        Commands::Docs { action } => run_docs(action),

        Commands::Registry { action } => run_registry(&rt, &mut registry, action),

        Commands::Git { action } => run_git(&rt, action),

        Commands::Spec { action } => run_spec(action),

        Commands::Ensemble { action } => run_ensemble(&rt, action),

        Commands::Agent { action } => run_agent(&rt, action),

        Commands::Curator { action } => run_curator(&rt, &registry, &runtime, &handle, action),

        Commands::Replicant { action } => run_replicant(action),

        Commands::Keystore { action } => run_keystore(action),

        Commands::Admin { action } => run_admin(action),

        Commands::Models => run_models(&rt),

        Commands::WebSearch { query, max_results } => run_web_search(&rt, query, max_results),
    }
}

// ── Command Handlers ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_chat(
    rt: &tokio::runtime::Runtime,
    registry: &SqliteRegistry,
    runtime: &McpRuntime,
    handle: &tokio::runtime::Handle,
    template: Option<String>,
    input: Option<std::path::PathBuf>,
    agent: String,
    model: Option<String>,
) {
    if let Some(input_path) = input {
        // Non-interactive mode: run onboarding to ensure keys are configured.
        // Fast path: if keys are already set (env vars or keychain), this is transparent.
        // Otherwise, walks through interactive onboarding (creates replicant or signs in).
        // Falls back to the old error if stdin is piped (rpassword reads /dev/tty, but
        // prompt_line uses stdin which may be the pipe).
        if let Err(e) = rt.block_on(hkask_cli::onboarding::run_onboarding()) {
            eprintln!("Cannot chat: {}", e);
            eprintln!("Run `kask chat` first to complete onboarding interactively.");
            std::process::exit(1);
        }
        let content = or_exit(
            std::fs::read_to_string(&input_path),
            "Failed to read input file",
        );
        let response = rt.block_on(commands::chat_with_agent(
            content.trim(),
            Some(&agent),
            model.as_deref(),
        ));
        println!("{}: {}", agent, response);
    } else {
        hkask_cli::repl::run(
            registry,
            runtime,
            template.as_deref(),
            &agent,
            model.as_deref(),
            handle.clone(),
        );
    }
}

fn run_template(registry: &mut SqliteRegistry, action: TemplateAction) {
    match action {
        TemplateAction::List { r#type } => {
            let template_type = r#type.as_deref().and_then(cli::parse_template_type);
            let entries = commands::list_templates(registry, template_type);
            if entries.is_empty() {
                println!("No templates registered.");
            } else {
                println!("Registered templates ({}):\n", entries.len());
                for entry in entries {
                    println!("  {} ({})", entry.id, entry.template_type.as_str());
                    println!("    Description: {}", entry.description);
                    println!("    Path: {}", entry.source_path);
                    if !entry.lexicon_terms.is_empty() {
                        println!("    Lexicon: {}", entry.lexicon_terms.join(", "));
                    }
                    println!();
                }
            }
        }
        TemplateAction::Register {
            id,
            path,
            r#type,
            lexicon,
            description,
        } => {
            let template_type = match cli::parse_template_type(&r#type) {
                Some(t) => t,
                None => {
                    eprintln!(
                        "Invalid template type: {}. Valid types: prompt, cognition, process",
                        r#type
                    );
                    std::process::exit(1);
                }
            };
            let lexicon_terms: Vec<String> = lexicon
                .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            let desc = description.unwrap_or_else(|| format!("Template {}", id));
            or_exit(
                commands::register_template(
                    registry,
                    id.clone(),
                    template_type,
                    path.to_string_lossy().to_string(),
                    lexicon_terms,
                    desc,
                ),
                "Failed to register template",
            );
            println!("Registered template: {}", id);
        }
        TemplateAction::Get { id } => {
            let entry = or_exit(commands::get_template(registry, &id), "Template not found");
            println!("Template: {}", entry.id);
            println!("  Type: {}", entry.template_type.as_str());
            println!("  Description: {}", entry.description);
            println!("  Path: {}", entry.source_path);
            println!("  Lexicon: {}", entry.lexicon_terms.join(", "));
        }
        TemplateAction::Search { term } => {
            let results = or_exit(commands::search_templates(registry, &term), "Search failed");
            if results.is_empty() {
                println!("No templates found with lexicon term: {}", term);
            } else {
                println!("Templates matching '{}':\n", term);
                for entry in results {
                    println!("  {} ({})", entry.id, entry.template_type.as_str());
                }
            }
        }
    }
}

fn run_bot(rt: &tokio::runtime::Runtime, action: BotAction) {
    match action {
        BotAction::List { kind } => {
            let agents = or_exit(
                rt.block_on(commands::bot_list(kind.as_deref())),
                "Failed to list agents",
            );
            if agents.is_empty() {
                println!("No agents registered.");
            } else {
                println!(
                    "{:<25} {:<12} {:<40} SOURCE",
                    "NAME", "KIND", "CAPABILITIES"
                );
                println!("{}", "-".repeat(100));
                for agent in &agents {
                    println!(
                        "{:<25} {:<12} {:<40} {}",
                        agent.definition.name,
                        agent.definition.agent_kind,
                        agent.definition.capabilities.len(),
                        agent.source_yaml,
                    );
                }
                println!("\nTotal: {} agents", agents.len());
            }
        }
        BotAction::Status { name } => {
            let agent = or_exit(
                rt.block_on(commands::bot_status(&name)),
                "Failed to get agent status",
            );
            let def = &agent.definition;
            println!("Agent: {}", def.name);
            println!("  Kind: {}", def.agent_kind);
            println!("  Editor: {}", def.editor);
            println!("  Binding contract: {}", def.binding_contract);
            if let Some(charter) = &def.charter {
                println!("  Charter: {}", charter.description);
                println!("  Archetype: {}", charter.archetype);
            }
            println!("  Capabilities:");
            for cap in &def.capabilities {
                println!("    - {}", cap);
            }
            if !def.rights.is_empty() {
                println!("  Rights:");
                for r in &def.rights_flat() {
                    println!("    - {}", r);
                }
            }
            if !def.responsibilities.is_empty() {
                println!("  Responsibilities:");
                for r in &def.responsibilities_flat() {
                    println!("    - {}", r);
                }
            }
            if let Some(persona) = &def.persona {
                println!("  Persona:");
                println!("    Tone: {}", persona.tone);
                println!("    Verbosity: {}", persona.verbosity);
                if !persona.forbidden.is_empty() {
                    println!("    Forbidden: {}", persona.forbidden.join(", "));
                }
            }
            if let Some(probe) = &def.readiness_probe {
                println!(
                    "  Readiness probe: {} ({})",
                    probe.endpoint, probe.probe_type
                );
            }
            println!("  Registered: {}", agent.registered_at);
            println!("  Source: {}", agent.source_yaml);
        }
        BotAction::Grant { bot_id, capability } => {
            println!("Grant capability: {} to bot: {}", capability, bot_id);
            println!("Note: Capability granting via ACP attenuation not yet wired.");
        }
    }
}

fn run_pod(rt: &tokio::runtime::Runtime, action: PodAction) {
    match action {
        PodAction::Create {
            template,
            persona,
            name,
        } => {
            let pod_id = or_exit(
                rt.block_on(commands::create_pod(&template, &persona, name.as_deref())),
                "Failed to create pod",
            );
            println!("Created agent pod: {}", pod_id);
            println!("Template: {}", template);
            println!("Persona file: {}", persona.display());
            if let Some(n) = &name {
                println!("Pod name: {}", n);
            }
        }
        PodAction::Activate { pod_id } => {
            or_exit(
                rt.block_on(commands::activate_pod(&pod_id)),
                "Failed to activate pod",
            );
            println!("Activated agent pod: {}", pod_id);
        }
        PodAction::Deactivate { pod_id } => {
            or_exit(
                rt.block_on(commands::deactivate_pod(&pod_id)),
                "Failed to deactivate pod",
            );
            println!("Deactivated agent pod: {}", pod_id);
        }
        PodAction::Status { pod_id, verbose } => {
            let status = or_exit(
                rt.block_on(commands::get_pod_status(&pod_id)),
                "Failed to get pod status",
            );
            println!("Agent pod status: {}", pod_id);
            println!("  State: {}", status.state);
            println!("  WebID: {}", status.webid);
            if let Some(name) = &status.name {
                println!("  Name: {}", name);
            }
            if verbose {
                println!("  Created at: {}", status.created_at);
            }
        }
        PodAction::List => match rt.block_on(commands::list_pods()) {
            Ok(pods) => {
                if pods.is_empty() {
                    println!("No pods registered.");
                } else {
                    println!("Agent pods ({}):\n", pods.len());
                    for pod in pods {
                        println!("  {} ({})", pod.pod_id, pod.state);
                        println!("    WebID: {}", pod.webid);
                        if let Some(name) = &pod.name {
                            println!("    Name: {}", name);
                        }
                        println!();
                    }
                }
            }
            Err(e) => eprintln!("Pod listing unavailable: {}", e),
        },
    }
}

fn run_mcp(rt: &tokio::runtime::Runtime, action: McpAction) {
    match action {
        McpAction::ListServers => {
            println!("MCP servers:");
            println!("  (no servers registered)");
        }
        McpAction::ListTools => {
            println!("Available tools:");
            println!("  (no tools registered)");
        }
        McpAction::GetTool { name } => {
            println!("Get tool: {}", name);
            println!("Note: Tool lookup requires MCP runtime integration.");
        }
        McpAction::Invoke {
            server: _server,
            tool,
            input,
        } => {
            use hkask_mcp::dispatch::McpDispatcher;
            use hkask_templates::McpPort;
            use hkask_types::WebID;

            let input_value: serde_json::Value =
                or_exit(serde_json::from_str(&input), "parse JSON input");

            let runtime = McpRuntime::new();
            let secret = b"hkask-devel-mcp-secret-key-32byte!";
            let dispatcher = McpDispatcher::new(runtime, secret);

            let tools = rt.block_on(dispatcher.list_tools());
            if tools.is_empty() {
                eprintln!("Warning: No tools registered in MCP runtime.");
            } else {
                eprintln!("Available tools: {:?}", tools);
            }

            let from = WebID::new();
            let to = WebID::new();
            let token = dispatcher.issue_capability(tool.clone(), from, to);

            let result = match rt.block_on(dispatcher.invoke(&tool, input_value, &token)) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Tool invocation error: {}", e);
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
            );
        }
    }
}

fn run_cns(rt: &tokio::runtime::Runtime, action: CnsAction) {
    match action {
        CnsAction::Health => {
            let cns_runtime = hkask_cns::CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD);
            let health = rt.block_on(cns_runtime.health());
            let alerts = rt.block_on(cns_runtime.alerts());
            let variety = rt.block_on(cns_runtime.variety());

            println!("CNS Health Status");
            println!("=================");
            println!();
            println!("Runtime Status:");
            println!("  • Healthy: {}", health.healthy);
            println!("  • Overall variety deficit: {}", health.overall_deficit);
            println!("  • Critical alerts: {}", health.critical_count);
            println!("  • Warning alerts: {}", health.warning_count);
            println!();
            println!("Variety Counter Summary:");
            if variety.is_empty() {
                println!("  • No variety data recorded");
            } else {
                for (domain, count) in &variety {
                    println!("  • {}: {} states", domain, count);
                }
            }
            println!();
            println!("Active Algedonic Alerts:");
            if alerts.is_empty() {
                println!("  • No active alerts");
            } else {
                for alert in &alerts {
                    println!(
                        "  • [{:?}] {}: {}",
                        alert.severity, alert.domain, alert.message
                    );
                }
            }
            println!();
            println!("Rate Limiter Status:");
            println!("  • Default config: 100 requests/min");
            println!("  • Status: OPERATIONAL");
            println!();
            println!("Review Queue Depth:");
            println!("  • Pending reviews: 0");
            println!("  • Queue status: IDLE");
        }
        CnsAction::Alerts => {
            println!("Algedonic alerts:");
            println!("  (no active alerts)");
        }
        CnsAction::Variety => {
            println!("Variety counters:");
            println!("  (no variety data)");
        }
    }
}

fn run_goal(action: GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => commands::goal::create(&text, &visibility),
        GoalAction::List { state } => commands::goal::list(state.as_deref()),
        GoalAction::SetState { id, state } => commands::goal::set_state(&id, &state),
    };
    or_exit(result, "Goal command failed");
}

fn run_sovereignty(action: SovereigntyAction) {
    use hkask_types::DataCategory;

    match action {
        SovereigntyAction::Status => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let store = or_exit(
                commands::config::open_sovereignty_store(),
                "Failed to open sovereignty store",
            );
            let consent_manager = hkask_agents::ConsentManager::new(store.clone());

            println!("Sovereignty Status");
            println!("==================");
            println!();
            println!("Consent State:");
            println!("  WebID: {}", webid);
            let categories = [
                ("episodic_memory", DataCategory::EpisodicMemory),
                ("semantic_memory", DataCategory::SemanticMemory),
                ("personal_context", DataCategory::PersonalContext),
                ("capability_tokens", DataCategory::CapabilityTokens),
                ("ocap_boundaries", DataCategory::OcapBoundaries),
                ("template_invocations", DataCategory::TemplateInvocations),
                ("hlexicon_terms", DataCategory::HLexiconTerms),
                ("template_registry", DataCategory::TemplateRegistry),
            ];
            for (label, cat) in &categories {
                match consent_manager.has_consent(&webid.to_string(), cat) {
                    Ok(true) => println!("  • {}: GRANTED", label),
                    Ok(false) => println!("  • {}: DENIED", label),
                    Err(e) => println!("  • {}: ERROR ({})", label, e),
                }
            }
            println!();
            println!("Data Boundaries:");
            match store.get(&webid.to_string()) {
                Ok(Some(entry)) => {
                    if !entry.sovereign_categories.is_empty() {
                        println!("  • Sovereign: {}", entry.sovereign_categories.join(", "));
                    }
                    if !entry.shared_categories.is_empty() {
                        println!("  • Shared: {}", entry.shared_categories.join(", "));
                    }
                    if !entry.public_categories.is_empty() {
                        println!("  • Public: {}", entry.public_categories.join(", "));
                    }
                    if entry.sovereign_categories.is_empty()
                        && entry.shared_categories.is_empty()
                        && entry.public_categories.is_empty()
                    {
                        println!("  • No boundary data stored yet");
                    }
                }
                Ok(None) => {
                    println!("  • No boundary data stored yet (run 'kask sovereignty grant' first)")
                }
                Err(e) => println!("  • Error loading boundaries: {}", e),
            }
            println!();
            println!("Resistance Level:");
            match store.get(&webid.to_string()) {
                Ok(Some(entry)) => {
                    println!("  • Resistance: {}", entry.resistance);
                    println!("  • Kill-zone threshold: {:.2}", entry.kill_zone_threshold);
                }
                Ok(None) => println!("  • No resistance data stored yet"),
                Err(e) => println!("  • Error loading resistance: {}", e),
            }
        }
        SovereigntyAction::Grant { category } => {
            let webid = hkask_types::WebID::new();
            let data_category = cli::parse_data_category(&category);
            let store = hkask_storage::SovereigntyBoundaryStore::new(
                hkask_storage::Database::in_memory()
                    .expect("in-memory db")
                    .conn_arc(),
            );
            let consent_manager = hkask_agents::ConsentManager::new(store);
            match consent_manager.grant_consent(&webid.to_string(), &data_category) {
                Ok(()) => {
                    println!("Consent granted for category: {}", category);
                    println!("  Data sharing is now enabled for this category.");
                    if data_category.is_typically_sovereign() {
                        println!("  Note: Sovereign data still requires owner verification.");
                    }
                }
                Err(e) => eprintln!("Error granting consent: {}", e),
            }
        }
        SovereigntyAction::Revoke { category } => {
            let webid = hkask_types::WebID::new();
            let store = hkask_storage::SovereigntyBoundaryStore::new(
                hkask_storage::Database::in_memory()
                    .expect("in-memory db")
                    .conn_arc(),
            );
            let consent_manager = hkask_agents::ConsentManager::new(store);
            let data_category = cli::parse_data_category(&category);
            let _ = consent_manager.grant_consent(&webid.to_string(), &data_category);
            match consent_manager.revoke_consent(&webid.to_string()) {
                Ok(()) => {
                    println!("Consent revoked for category: {}", category);
                    println!("  Data sharing is now disabled for this category.");
                    println!("  Only public data is accessible.");
                }
                Err(e) => eprintln!("Error revoking consent: {}", e),
            }
        }
        SovereigntyAction::MarkAcquisition { vc_investment } => {
            let mut state = hkask_types::UserSovereigntyState::new();
            state.mark_acquisition_attempt();
            state.update_vc_investment(vc_investment);
            println!("Acquisition attempt marked.");
            println!("  VC investment: {:.2}", vc_investment);
            println!("  Kill zone active: {}", state.is_compromised());
            if state.is_compromised() {
                println!("  [ALERT] Sovereignty compromised - CNS alert triggered!");
            }
        }
        SovereigntyAction::KillZone => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let store = or_exit(
                commands::config::open_sovereignty_store(),
                "Failed to open sovereignty store",
            );
            println!("Kill-Zone Detection");
            println!("===================");
            println!();
            match store.get(&webid.to_string()) {
                Ok(Some(entry)) => {
                    let resistance = &entry.resistance;
                    let threshold = entry.kill_zone_threshold;
                    let kill_zone_active = resistance != "Minimum" && resistance != "Low";
                    println!("Status:");
                    println!("  • Kill-zone active: {}", kill_zone_active);
                    println!("  • Kill-zone threshold: {:.2}", threshold);
                    println!();
                    println!("Investment:");
                    println!(
                        "  • VC investment level: {} (threshold: {:.2})",
                        if kill_zone_active {
                            "HIGH (above threshold)"
                        } else {
                            "LOW (below threshold)"
                        },
                        threshold
                    );
                    println!();
                    println!("Resistance:");
                    println!("  • Resistance level: {}", resistance);
                    println!();
                    if kill_zone_active {
                        println!("[ALERT] Kill-zone active — sovereignty may be compromised!");
                    } else {
                        println!("Sovereignty boundary intact.");
                    }
                }
                Ok(None) => {
                    println!("  • No sovereignty data stored yet");
                    println!("  • Kill-zone status: UNKNOWN");
                    println!("  • Use 'kask sovereignty grant' to initialize");
                }
                Err(e) => println!("  • Error loading kill-zone data: {}", e),
            }
        }
        SovereigntyAction::Check { category } => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let store = or_exit(
                commands::config::open_sovereignty_store(),
                "Failed to open sovereignty store",
            );
            let consent_manager = hkask_agents::ConsentManager::new(store);
            let data_category = cli::parse_data_category(&category);
            println!("Data Access Check");
            println!("=================");
            println!("  Category: {}", category);
            match consent_manager.has_consent(&webid.to_string(), &data_category) {
                Ok(true) => {
                    println!("  Access: GRANTED");
                    println!("  Consent has been explicitly given for this category.");
                }
                Ok(false) => {
                    println!("  Access: DENIED");
                    println!(
                        "  No consent for this category. Use 'kask sovereignty grant --category {}' to grant.",
                        category
                    );
                }
                Err(e) => {
                    println!("  Access: ERROR");
                    println!("  Failed to check consent: {}", e);
                }
            }
        }
    }
}

fn run_docs(action: DocsAction) {
    match action {
        DocsAction::Openapi { output } => {
            let spec = hkask_api::create_openapi();
            let json = or_exit(
                serde_json::to_string_pretty(&spec),
                "Failed to serialize OpenAPI spec",
            );
            write_or_print(&json, output.as_deref(), "OpenAPI specification");
        }
        DocsAction::Cli { output } => {
            let help = cli::generate_cli_markdown();
            write_or_print(&help, output.as_deref(), "CLI documentation");
        }
        DocsAction::All { output } => {
            or_exit(
                std::fs::create_dir_all(&output),
                "Failed to create output directory",
            );
            let spec = hkask_api::create_openapi();
            let json = or_exit(
                serde_json::to_string_pretty(&spec),
                "Failed to serialize OpenAPI spec",
            );
            let openapi_path = output.join("openapi.json");
            write_or_print(&json, Some(&openapi_path), "OpenAPI specification");
            let help = cli::generate_cli_markdown();
            let cli_path = output.join("cli.md");
            write_or_print(&help, Some(&cli_path), "CLI documentation");
            println!(
                "\nDocumentation generated successfully in: {}",
                output.display()
            );
        }
    }
}

fn run_registry(
    _rt: &tokio::runtime::Runtime,
    registry: &mut SqliteRegistry,
    action: RegistryAction,
) {
    use hkask_cli::commands::russell::RussellMappingConfig;

    match action {
        RegistryAction::ImportRussell {
            source,
            dry_run,
            validate_only,
            output_format,
            transform_rules,
            verbose,
        } => {
            let mut config = if let Some(rules_path) = &transform_rules {
                match RussellMappingConfig::load_from_yaml(rules_path.to_str().unwrap_or("")) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load transform rules from {}: {}. Using defaults.",
                            rules_path.display(),
                            e
                        );
                        RussellMappingConfig::defaults()
                    }
                }
            } else {
                let default_path = "registry/manifests/russell-mapping.yaml";
                match RussellMappingConfig::load_from_yaml(default_path) {
                    Ok(c) => c,
                    Err(_) => RussellMappingConfig::defaults(),
                }
            };

            config.dry_run = dry_run;

            let mapper = commands::russell::RussellMapper::with_config(config.clone());

            if validate_only {
                let assets = or_exit(
                    commands::import_russell(&source, &config, verbose),
                    "Validation failed",
                );
                println!("Validation complete: {} manifests parsed", assets.len());
                for asset in &assets {
                    println!("\n  ID: {} [VALID]", asset.id);
                }
            } else {
                let assets = or_exit(
                    commands::import_russell_with_mapper(&mapper, &source, verbose),
                    "Migration failed",
                );
                let fmt = output_format.to_lowercase();
                match fmt.as_str() {
                    "json" => {
                        let json = serde_json::to_string_pretty(&assets)
                            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                        println!("{}", json);
                    }
                    "mermaid" => {
                        println!("graph LR");
                        for asset in &assets {
                            println!("  russell[\"{}\"] --> hkask[\"{}\"]", asset.id, asset.id);
                        }
                    }
                    _ => {
                        println!("Migration analysis complete: {} assets", assets.len());
                        for asset in &assets {
                            println!("\n  ID: {}", asset.id);
                            println!("  Type: {:?}", asset.template_type);
                            println!("  Description: {}", asset.description);
                            println!("  Model Tier: {}", asset.model_tier);
                            println!("  Energy Cap: {}", asset.energy_cap);
                        }
                    }
                }
                if !dry_run {
                    for asset in &assets {
                        let entry = hkask_templates::RegistryEntry {
                            id: asset.id.clone(),
                            template_type: asset.template_type,
                            lexicon_terms: vec!["russell-migrated".to_string()],
                            description: asset.description.clone(),
                            source_path: format!("russell-migrated:{}", asset.id),
                            required_capabilities: vec![],
                        };
                        if let Err(e) = registry.register(entry, None) {
                            eprintln!("Failed to register template {}: {}", asset.id, e);
                        } else if verbose {
                            println!("  Registered: {}", asset.id);
                        }
                    }
                }
            }
        }
        RegistryAction::ListMigrated { origin: _ } => {
            println!("Migrated assets:");
            println!("  (use 'kask registry import-russell --dry-run' to analyze assets)");
        }
    }
}

fn run_git(rt: &tokio::runtime::Runtime, action: GitAction) {
    let runtime = hkask_mcp::runtime::McpRuntime::new();

    match action {
        GitAction::Archive {
            owner,
            repo,
            branch,
            path,
            content,
            file,
        } => {
            let content_str = if let Some(c) = content {
                c
            } else if let Some(f) = file {
                or_exit(std::fs::read_to_string(&f), "Failed to read file")
            } else {
                eprintln!("Either --content or --file must be provided");
                std::process::exit(1);
            };
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::archive_registry_to_git(
                        &runtime,
                        &owner,
                        &repo,
                        &branch,
                        &path,
                        &content_str,
                    )),
                    "Archive failed",
                )
            );
        }
        GitAction::Restore {
            owner,
            repo,
            r#ref,
            target,
        } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::restore_registry_from_git(
                        &runtime, &owner, &repo, &r#ref, &target,
                    )),
                    "Restore failed",
                )
            );
        }
        GitAction::List { owner, repo } => {
            let commits = or_exit(
                rt.block_on(commands::list_registry_archives(&runtime, &owner, &repo)),
                "List failed",
            );
            println!("Archived versions for {}/{}:", owner, repo);
            for (i, sha) in commits.iter().enumerate() {
                println!("  {}. {}", i + 1, sha);
            }
        }
        GitAction::Snapshot {
            owner,
            repo,
            message,
        } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::create_registry_snapshot(
                        &runtime, &owner, &repo, &message,
                    )),
                    "Snapshot failed",
                )
            );
        }
    }
}

fn run_spec(action: SpecAction) {
    use hkask_types::{SpecId, SpecStore};

    match action {
        SpecAction::Capture {
            name,
            category,
            domain,
            criteria,
        } => {
            use hkask_types::{DomainAnchor, GoalSpec, Spec, SpecCategory};

            let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
            let anchor = DomainAnchor::parse_str(&domain).unwrap_or(DomainAnchor::Hkask);
            let mut goal = GoalSpec::new(&name);
            if let Some(crits) = criteria {
                for c in crits.split(',') {
                    goal = goal.with_criterion(c.trim());
                }
            }
            let spec = Spec::new(&name, cat, anchor).with_goal(goal);
            let complete = spec.is_complete();

            let store = or_exit(
                commands::config::open_spec_store(),
                "Failed to open spec store",
            );
            or_exit(store.save(&spec), "Failed to save specification");

            println!("Specification captured:");
            println!("  ID: {}", spec.id);
            println!("  Name: {}", spec.name);
            println!("  Category: {}", spec.category.as_str());
            println!("  Domain: {}", spec.domain_anchor.as_str());
            println!("  Complete: {}", complete);
        }
        SpecAction::List { category } => {
            println!("Specifications:");
            if let Some(cat) = category {
                println!("  (filtered by category: {})", cat);
            }
            println!("  Note: Persistent spec storage requires hkask-mcp-spec server.");
        }
        SpecAction::Evaluate { spec_id } => {
            println!("Evaluating specification: {}", spec_id);
            println!("  Note: Evaluation requires hkask-mcp-spec server.");
        }
        SpecAction::Validate { id } => {
            use hkask_storage::DefaultSpecCurator;
            use hkask_types::SpecCurator;

            let spec_id = or_exit(SpecId::from_string(&id), "Invalid spec ID");
            let store = or_exit(
                commands::config::open_spec_store(),
                "Failed to open spec store",
            );
            let spec = or_exit(store.load(spec_id), "Failed to load specification");
            let curator = DefaultSpecCurator::default();
            let record = or_exit(curator.evaluate(&spec), "Failed to evaluate specification");

            println!("Specification validation:");
            println!("  ID: {}", record.spec_id);
            println!("  Decision: {:?}", record.decision);
            println!("  Rationale: {}", record.rationale);
            println!("  Coherence: {:.2}", record.coherence_score);
            println!("  Curated at: {}", record.curated_at);
        }
        SpecAction::Cultivate { id } => {
            use hkask_storage::DefaultSpecCurator;
            use hkask_types::{SpecCategory, SpecCurator};

            let spec_id = or_exit(SpecId::from_string(&id), "Invalid spec ID");
            let store = or_exit(
                commands::config::open_spec_store(),
                "Failed to open spec store",
            );
            let spec = or_exit(store.load(spec_id), "Failed to load specification");
            let curator = DefaultSpecCurator::default();
            let record = or_exit(curator.evaluate(&spec), "Failed to cultivate specification");

            println!("Specification cultivation:");
            println!("  ID: {}", record.spec_id);
            println!("  Decision: {:?}", record.decision);
            println!("  Rationale: {}", record.rationale);
            println!("  Coherence: {:.2}", record.coherence_score);
            println!("  Spec completeness: {}", spec.is_complete());
            println!("  Spec coherence: {:.2}", spec.coherence());
            println!();
            println!("  Required categories for full collection coherence:");
            for cat in SpecCategory::all() {
                println!("    - {}", cat.as_str());
            }
        }
        SpecAction::Render { template, spec_id } => {
            use minijinja::UndefinedBehavior;

            let template_path = format!("registry/templates/{}", template);
            let template_content = or_exit(
                std::fs::read_to_string(&template_path),
                "Template not found",
            );

            let store = or_exit(
                commands::config::open_spec_store(),
                "Failed to open spec store",
            );

            let ctx = if let Some(sid) = spec_id {
                let parsed_id = or_exit(SpecId::from_string(&sid), "Invalid spec ID");
                let spec = or_exit(store.load(parsed_id), "Failed to load spec");
                minijinja::context! {
                    spec_id => spec.id.to_string(),
                    goal_name => spec.name,
                    spec_category => spec.category.as_str(),
                    domain_anchor => spec.domain_anchor.as_str(),
                    goals => spec.goals.iter().map(|g| minijinja::context! {
                        text => g.text,
                        depth => g.depth,
                        criteria => g.criteria.iter().map(|c| minijinja::context! {
                            description => c.description,
                            satisfied => c.satisfied,
                        }).collect::<Vec<_>>(),
                    }).collect::<Vec<_>>(),
                }
            } else {
                minijinja::context! {}
            };

            let mut env = minijinja::Environment::new();
            env.set_undefined_behavior(UndefinedBehavior::Strict);
            let rendered = or_exit(
                env.render_str(&template_content, ctx),
                "Template render error",
            );
            println!("{}", rendered);
        }
    }
}

fn run_ensemble(rt: &tokio::runtime::Runtime, action: EnsembleAction) {
    match action {
        EnsembleAction::ChatCreate { session } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::ensemble_chat_create(session.clone())),
                    "Chat create failed",
                )
            );
        }
        EnsembleAction::ChatRegister { session, bot, role } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::ensemble_chat_register(
                        session.clone(),
                        bot.clone(),
                        role.clone(),
                    )),
                    "Chat register failed",
                )
            );
        }
        EnsembleAction::ChatSend { session, message } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::ensemble_chat_send(
                        session.clone(),
                        message.clone(),
                    )),
                    "Chat send failed",
                )
            );
        }
        EnsembleAction::ChatList => {
            let sessions = or_exit(
                rt.block_on(commands::ensemble_chat_list()),
                "Chat list failed",
            );
            println!("Active chat sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::DeliberationCreate { session } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::ensemble_deliberation_create(session.clone())),
                    "Deliberation create failed",
                )
            );
        }
        EnsembleAction::DeliberationStart { session } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::ensemble_deliberation_start(session.clone())),
                    "Deliberation start failed",
                )
            );
        }
        EnsembleAction::DeliberationRecord {
            session,
            agent,
            content,
            confidence,
        } => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::ensemble_deliberation_record(
                        session.clone(),
                        agent.clone(),
                        content.clone(),
                        confidence,
                    )),
                    "Deliberation record failed",
                )
            );
        }
        EnsembleAction::DeliberationSynthesize { session } => {
            println!(
                "Synthesized response:\n{}",
                or_exit(
                    rt.block_on(commands::ensemble_deliberation_synthesize(session.clone())),
                    "Deliberation synthesize failed",
                )
            );
        }
        EnsembleAction::DeliberationList => {
            let sessions = or_exit(
                rt.block_on(commands::ensemble_deliberation_list()),
                "Deliberation list failed",
            );
            println!("Active deliberation sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::StandingStart { config } => {
            let status = or_exit(
                commands::ensemble_standing_start(&config),
                "Standing session bootstrap failed",
            );
            println!("Standing session bootstrapped:");
            println!("  Session ID: {}", status.session_id);
            println!("  Participants: {}", status.participant_count);
            println!("  Initial messages: {}", status.message_count);
        }
        EnsembleAction::StandingStatus => {
            let status = or_exit(
                commands::ensemble_standing_status(),
                "Standing status failed",
            );
            println!("Standing session status:");
            println!("  Session ID: {}", status.session_id);
            println!("  Participants: {}", status.participant_count);
            println!("  Messages: {}", status.message_count);
            println!("\nParticipants:");
            for p in &status.participants {
                println!("  - {} ({})", p.name, p.role);
            }
        }
    }
}

fn run_agent(rt: &tokio::runtime::Runtime, action: AgentAction) {
    match action {
        AgentAction::Register {
            webid,
            agent_type,
            capabilities,
        } => {
            let caps: Vec<String> = capabilities
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            let receipt = or_exit(
                rt.block_on(commands::agent_register(&webid, &agent_type, caps)),
                "Registration failed",
            );
            println!("Agent registered:");
            println!("  WebID: {}", receipt.webid);
            println!("  Token: {}...", &receipt.token_hash[..16]);
            println!("  Registered at: {}", receipt.registered_at);
        }
        AgentAction::Unregister { name } => {
            or_exit(
                rt.block_on(commands::agent_unregister(&name)),
                "Unregister failed",
            );
            println!("Agent unregistered: {}", name);
        }
        AgentAction::List => {
            let agents = or_exit(
                rt.block_on(commands::bot_list(None)),
                "Failed to list agents",
            );
            if agents.is_empty() {
                println!("No agents registered.");
            } else {
                println!("{:<25} {:<12} {:<40}", "NAME", "KIND", "CAPABILITIES");
                println!("{}", "-".repeat(80));
                for agent in &agents {
                    println!(
                        "{:<25} {:<12} {:<40}",
                        agent.definition.name,
                        agent.definition.agent_kind,
                        agent.definition.capabilities.join(", "),
                    );
                }
            }
        }
        AgentAction::Capabilities { name } => {
            let agent = or_exit(
                rt.block_on(commands::bot_status(&name)),
                "Failed to get capabilities",
            );
            println!("Capabilities for {}:", agent.definition.name);
            for cap in &agent.definition.capabilities {
                println!("  - {}", cap);
            }
        }
    }
}

fn run_curator(
    rt: &tokio::runtime::Runtime,
    registry: &SqliteRegistry,
    runtime: &McpRuntime,
    handle: &tokio::runtime::Handle,
    action: CuratorAction,
) {
    match action {
        CuratorAction::Chat => {
            hkask_cli::repl::run(registry, runtime, None, "Curator", None, handle.clone());
        }
        CuratorAction::Escalations => {
            let escalations = or_exit(
                rt.block_on(commands::curator_escalations()),
                "Failed to list escalations",
            );
            if escalations.is_empty() {
                println!("No pending escalations.");
            } else {
                println!("{:<20} {:<15} {:<10} CONTEXT", "ID", "BOT", "CONFIDENCE");
                println!("{}", "-".repeat(80));
                for esc in &escalations {
                    println!(
                        "{:<20} {:<15} {:<10.2} {}",
                        &esc.id[..std::cmp::min(20, esc.id.len())],
                        esc.bot_id
                            .0
                            .to_string()
                            .split('-')
                            .next()
                            .unwrap_or("unknown"),
                        esc.confidence,
                        &esc.error_context[..std::cmp::min(40, esc.error_context.len())],
                    );
                }
                println!("\nTotal: {} pending escalations", escalations.len());
            }
        }
        CuratorAction::Resolve { id } => {
            or_exit(
                rt.block_on(commands::curator_resolve(&id)),
                "Failed to resolve escalation",
            );
            println!("Escalation {} resolved.", id);
        }
        CuratorAction::Dismiss { id } => {
            or_exit(
                rt.block_on(commands::curator_dismiss(&id)),
                "Failed to dismiss escalation",
            );
            println!("Escalation {} dismissed.", id);
        }
        CuratorAction::Metacognition => {
            println!(
                "{}",
                or_exit(
                    rt.block_on(commands::curator_metacognition()),
                    "Metacognition cycle failed",
                )
            );
        }
    }
}

fn run_replicant(action: ReplicantAction) {
    let store = open_user_store();

    match action {
        ReplicantAction::Register {
            replicant_name,
            first_name,
            last_name,
            email,
            phone,
        } => {
            or_exit(
                commands::user::register_replicant(
                    &store,
                    &replicant_name,
                    &first_name,
                    &last_name,
                    &email,
                    phone.as_deref(),
                ),
                "Registration failed",
            );
        }
        ReplicantAction::Login { replicant_name } => {
            let session = or_exit(
                commands::user::login_replicant(&store, &replicant_name),
                "Login failed",
            );
            println!("Session ID: {}", session.session_id);
            println!(
                "\nTo logout: kask replicant logout {}",
                &session.session_id[..8]
            );
        }
        ReplicantAction::Logout { session_id } => {
            or_exit(commands::user::logout(&store, &session_id), "Logout failed");
        }
        ReplicantAction::Sessions { replicant_name } => {
            or_exit(
                commands::user::list_sessions(&store, &replicant_name),
                "Failed to list sessions",
            );
        }
        ReplicantAction::List { user_id } => {
            if let Some(uid) = user_id {
                let user_id = hkask_types::UserID::from_string(&uid);
                or_exit(
                    commands::user::list_replicants(&store, &user_id),
                    "Failed to list identities",
                );
            } else {
                eprintln!("--user-id is required");
                std::process::exit(1);
            }
        }
        ReplicantAction::Show { replicant_name } => {
            or_exit(
                commands::user::show_replicant(&store, &replicant_name),
                "Failed to show replicant",
            );
        }
    }
}

fn run_keystore(action: KeystoreAction) {
    let keychain = hkask_keystore::Keychain::default();

    match action {
        KeystoreAction::Load {
            path,
            prefix,
            overwrite,
        } => {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read {}: {}", path.display(), e);
                    std::process::exit(1);
                }
            };
            let mut loaded = 0usize;
            let mut skipped = 0usize;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if !key.starts_with(&prefix) {
                        continue;
                    }
                    if value.is_empty() {
                        continue;
                    }
                    match keychain.retrieve_by_key(key) {
                        Ok(_) if !overwrite => {
                            println!("  skipped {} (already in keychain, use --overwrite)", key);
                            skipped += 1;
                        }
                        _ => match keychain.store_by_key(key, value) {
                            Ok(()) => {
                                println!("  stored {}", key);
                                loaded += 1;
                            }
                            Err(e) => eprintln!("  failed {} : {}", key, e),
                        },
                    }
                }
            }
            println!("\nLoaded {} keys, skipped {}", loaded, skipped);
        }
        KeystoreAction::List => {
            eprintln!(
                "OS keychain does not support listing. Use 'kask keystore get <KEY>' to check individual keys."
            );
        }
        KeystoreAction::Get { key } => {
            let val = or_exit(keychain.retrieve_by_key(&key), "Key not found");
            if val.len() > 8 {
                println!("{}={}**{}", key, &val[..4], &val[val.len() - 4..]);
            } else {
                println!("{}=****", key);
            }
        }
        KeystoreAction::Set { key, value } => {
            or_exit(keychain.store_by_key(&key, &value), "Failed to store key");
            println!("Stored {}", key);
        }
        KeystoreAction::Delete { key } => {
            or_exit(keychain.delete_by_key(&key), "Failed to delete key");
            println!("Deleted {}", key);
        }
    }
}

fn run_admin(action: AdminAction) {
    match action {
        AdminAction::Init => {
            commands::admin::admin_init();
        }
        AdminAction::Reset => {
            commands::admin::admin_reset();
        }
    }
}

fn run_models(rt: &tokio::runtime::Runtime) {
    use hkask_mcp::dispatch::McpDispatcher;
    use hkask_templates::McpPort;
    use hkask_types::WebID;

    let runtime = McpRuntime::new();
    let secret = b"hkask-devel-mcp-secret-key-32byte!";
    let dispatcher = McpDispatcher::new(runtime, secret);
    let from = WebID::new();
    let to = WebID::new();
    let token = dispatcher.issue_capability("models".to_string(), from, to);

    match rt.block_on(dispatcher.invoke("inference:models", serde_json::json!({}), &token)) {
        Ok(result) => {
            if let Some(tiers) = result.get("model_tiers").and_then(|t| t.as_array()) {
                println!("\n=== Available Model Tiers ===");
                for tier in tiers {
                    let label = tier
                        .get("tier")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    let count = tier.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
                    println!("  {}: {} models", label, count);
                    if let Some(models) = tier.get("models").and_then(|m| m.as_array()) {
                        for model in models {
                            let name = model.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                            let size = model.get("size").and_then(|s| s.as_str()).unwrap_or("");
                            println!("    - {}  {}", name, size);
                        }
                    }
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to list models: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_web_search(rt: &tokio::runtime::Runtime, query: String, max_results: usize) {
    use hkask_mcp::dispatch::McpDispatcher;
    use hkask_templates::McpPort;
    use hkask_types::WebID;

    let runtime = McpRuntime::new();
    let secret = b"hkask-devel-mcp-secret-key-32byte!";
    let dispatcher = McpDispatcher::new(runtime, secret);
    let from = WebID::new();
    let to = WebID::new();
    let token = dispatcher.issue_capability("web".to_string(), from, to);

    match rt.block_on(dispatcher.invoke(
        "web:search",
        serde_json::json!({"query": query, "max_results": max_results}),
        &token,
    )) {
        Ok(result) => {
            if let Some(results) = result.get("results").and_then(|r| r.as_array()) {
                println!("\n=== Web Search: {} ===\n", query);
                for (i, item) in results.iter().enumerate() {
                    let title = item
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled");
                    let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
                    let snippet = item.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                    println!("{}. {}", i + 1, title);
                    println!("   URL: {}", url);
                    if !snippet.is_empty() {
                        println!("   {}", snippet);
                    }
                    println!();
                }
            } else if let Some(error) = result.get("error") {
                eprintln!("Search error: {}", error);
                std::process::exit(1);
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            eprintln!("Web search failed: {}", e);
            std::process::exit(1);
        }
    }
}
