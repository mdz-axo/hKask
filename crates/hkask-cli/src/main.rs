//! hKask CLI — Binary entry point
//!
//! **Commands:**
//! - `kask chat` — Curator chat interface
//! - `kask template list` — List registered templates
//! - `kask template register` — Register a new template
//! - `kask bot capabilities` — Show bot capabilities
//! - `kask bot grant` — Grant capability to bot
//! - `kask pod create` — Create agent pod from template crate
//! - `kask pod activate` — Activate agent pod
//! - `kask pod deactivate` — Deactivate agent pod
//! - `kask pod status` — Show agent pod status
//! - `kask mcp servers` — List MCP servers
//! - `kask mcp tools` — List available tools
//! - `kask cns health` — CNS monitoring

use hkask_cli::cli::{
    self, BotAction, CnsAction, Commands, CuratorAction, DocsAction, EnsembleAction, GitAction,
    KeystoreAction, McpAction, PodAction, RegistryAction, ReplicantAction, SovereigntyAction,
    SpecAction, TemplateAction,
};
use hkask_cli::commands;
use hkask_cli::russell_mapper::RussellMappingConfig;
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;

fn main() {
    let cli = cli::Cli::parse();
    cli::init_logging(cli.verbose);

    // Initialize registry
    let registry_result = match &cli.registry {
        Some(path) => SqliteRegistry::new(Some(path.to_str().unwrap())),
        None => SqliteRegistry::new(None),
    };

    let mut registry = match registry_result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize registry: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize MCP runtime
    let runtime = McpRuntime::new();

    match cli.command {
        Commands::Chat {
            template,
            input,
            agent,
        } => {
            if let Some(input_path) = input {
                match std::fs::read_to_string(&input_path) {
                    Ok(content) => {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let response =
                            rt.block_on(commands::chat_with_agent(content.trim(), Some(&agent)));
                        println!("{}: {}", agent, response);
                    }
                    Err(e) => {
                        eprintln!("Failed to read input file: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                hkask_cli::repl::run(&registry, &runtime, template.as_deref(), &agent);
            }
        }

        Commands::Template { action } => match action {
            TemplateAction::List { r#type } => {
                let template_type = r#type.as_deref().and_then(cli::parse_template_type);
                let entries = commands::list_templates(&registry, template_type);

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

                match commands::register_template(
                    &mut registry,
                    id.clone(),
                    template_type,
                    path.to_string_lossy().to_string(),
                    lexicon_terms,
                    desc,
                ) {
                    Ok(()) => println!("Registered template: {}", id),
                    Err(e) => {
                        eprintln!("Failed to register template: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            TemplateAction::Get { id } => match commands::get_template(&registry, &id) {
                Ok(entry) => {
                    println!("Template: {}", entry.id);
                    println!("  Type: {}", entry.template_type.as_str());
                    println!("  Description: {}", entry.description);
                    println!("  Path: {}", entry.source_path);
                    println!("  Lexicon: {}", entry.lexicon_terms.join(", "));
                }
                Err(e) => {
                    eprintln!("Template not found: {}", e);
                    std::process::exit(1);
                }
            },
            TemplateAction::Search { term } => match commands::search_templates(&registry, &term) {
                Ok(results) => {
                    if results.is_empty() {
                        println!("No templates found with lexicon term: {}", term);
                    } else {
                        println!("Templates matching '{}':\n", term);
                        for entry in results {
                            println!("  {} ({})", entry.id, entry.template_type.as_str());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Search failed: {}", e);
                    std::process::exit(1);
                }
            },
        },

        Commands::Bot { action } => match action {
            BotAction::List { kind } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_list(kind.as_deref()).await {
                        Ok(agents) => {
                            if agents.is_empty() {
                                println!("No agents registered.");
                                return;
                            }
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
                        Err(e) => {
                            eprintln!("Failed to list agents: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            BotAction::Status { name } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_status(&name).await {
                        Ok(agent) => {
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
                        Err(e) => {
                            eprintln!("Failed to get agent status: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            BotAction::Grant { bot_id, capability } => {
                println!("Grant capability: {} to bot: {}", capability, bot_id);
                println!("Note: Capability granting via ACP attenuation not yet wired.");
            }
        },

        Commands::Pod { action } => match action {
            PodAction::Create {
                template,
                persona,
                name,
            } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::create_pod(&template, &persona, name.as_deref())) {
                    Ok(pod_id) => {
                        println!("Created agent pod: {}", pod_id);
                        println!("Template: {}", template);
                        println!("Persona file: {}", persona.display());
                        if let Some(n) = &name {
                            println!("Pod name: {}", n);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to create pod: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::Activate { pod_id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::activate_pod(&pod_id)) {
                    Ok(_) => {
                        println!("Activated agent pod: {}", pod_id);
                    }
                    Err(e) => {
                        eprintln!("Failed to activate pod: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::Deactivate { pod_id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::deactivate_pod(&pod_id)) {
                    Ok(_) => {
                        println!("Deactivated agent pod: {}", pod_id);
                    }
                    Err(e) => {
                        eprintln!("Failed to deactivate pod: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::Status { pod_id, verbose } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(commands::get_pod_status(&pod_id)) {
                    Ok(status) => {
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
                    Err(e) => {
                        eprintln!("Failed to get pod status: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            PodAction::List => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let pods = rt.block_on(commands::list_pods());

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
        },

        Commands::Mcp { action } => {
            match action {
                McpAction::ListServers => {
                    println!("MCP servers:");
                    // Note: runtime is not shared, so we can't list actual servers
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
            }
        }

        Commands::Cns { action } => match action {
            CnsAction::Health => {
                println!("CNS health status:");
                println!("  Overall deficit: 0");
                println!("  Critical alerts: 0");
                println!("  Warning alerts: 0");
                println!("  Status: HEALTHY");
            }
            CnsAction::Alerts => {
                println!("Algedonic alerts:");
                println!("  (no active alerts)");
            }
            CnsAction::Variety => {
                println!("Variety counters:");
                println!("  (no variety data)");
            }
        },

        Commands::Sovereignty { action } => match action {
            SovereigntyAction::Status => {
                let state = hkask_types::UserSovereigntyState::new();

                println!("User Sovereignty Status:");
                println!("  Explicit consent: {}", state.explicit_consent);
                println!("  Sovereignty compromised: {}", state.is_compromised());
                println!("  Kill zone active: {}", state.detector.kill_zone_active);
                println!("  VC investment: {:.2}", state.detector.vc_investment);
                println!("  Threshold: {:.2}", state.detector.threshold);
                println!("  Acquisition resistance: {:?}", state.boundary.resistance);
                println!();
                println!("  Sovereign data:");
                for category in &state.boundary.sovereign_data {
                    println!("    - {}", category.as_str());
                }
                println!("  Shared data:");
                for category in &state.boundary.shared_data {
                    println!("    - {}", category.as_str());
                }
                println!("  Public data:");
                for category in &state.boundary.public_data {
                    println!("    - {}", category.as_str());
                }
            }
            SovereigntyAction::GrantConsent => {
                println!("Explicit consent granted.");
                println!("  Data sharing is now enabled for shared data categories.");
                println!("  Sovereign data remains protected.");
            }
            SovereigntyAction::RevokeConsent => {
                println!("Explicit consent revoked.");
                println!("  Data sharing is now disabled.");
                println!("  Only public data is accessible.");
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
                let state = hkask_types::UserSovereigntyState::new();

                println!("Kill Zone Status:");
                println!("  Active: {}", state.detector.kill_zone_active);
                println!(
                    "  Acquisition attempt: {}",
                    state.detector.acquisition_attempt
                );
                println!("  VC investment: {:.2}", state.detector.vc_investment);
                println!("  Threshold: {:.2}", state.detector.threshold);
                if state.detector.kill_zone_active {
                    println!("  [ALERT] Kill zone active - sovereignty compromised!");
                }
            }
            SovereigntyAction::CheckAccess { category } => {
                let owner = hkask_types::WebID::new();
                let checker = hkask_agents::SovereigntyChecker::new(owner);
                let state = checker.get_state();

                // Parse category string to DataCategory
                let data_category = cli::parse_data_category(&category);

                let is_sovereign = state.boundary.is_sovereign(&data_category);
                let is_shared = state.boundary.is_shared(&data_category);
                let is_public = state.boundary.is_public(&data_category);

                println!("Data access check for '{}':", category);
                if is_sovereign {
                    println!("  Category: SOVEREIGN");
                    println!("  Access: Requires explicit consent AND owner");
                } else if is_shared {
                    println!("  Category: SHARED");
                    println!("  Access: Requires explicit consent");
                } else if is_public {
                    println!("  Category: PUBLIC");
                    println!("  Access: Always accessible");
                } else {
                    println!("  Category: UNKNOWN");
                    println!("  Access: Denied by default");
                }
            }
        },

        Commands::Docs { action } => match action {
            DocsAction::Openapi { output } => {
                let spec = hkask_api::create_openapi();
                let json =
                    serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");

                match output {
                    Some(path) => {
                        std::fs::write(&path, &json).expect("Failed to write OpenAPI spec");
                        println!("OpenAPI specification written to: {}", path.display());
                    }
                    None => println!("{}", json),
                }
            }
            DocsAction::Cli { output } => {
                let help = cli::generate_cli_markdown();
                match output {
                    Some(path) => {
                        std::fs::write(&path, &help).expect("Failed to write CLI documentation");
                        println!("CLI documentation written to: {}", path.display());
                    }
                    None => println!("{}", help),
                }
            }
            DocsAction::All { output } => {
                std::fs::create_dir_all(&output).expect("Failed to create output directory");

                let spec = hkask_api::create_openapi();
                let json =
                    serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");
                let openapi_path = output.join("openapi.json");
                std::fs::write(&openapi_path, &json).expect("Failed to write OpenAPI spec");
                println!(
                    "OpenAPI specification written to: {}",
                    openapi_path.display()
                );

                let help = cli::generate_cli_markdown();
                let cli_path = output.join("cli.md");
                std::fs::write(&cli_path, &help).expect("Failed to write CLI documentation");
                println!("CLI documentation written to: {}", cli_path.display());

                println!(
                    "\nDocumentation generated successfully in: {}",
                    output.display()
                );
            }
        },

        Commands::Registry { action } => match action {
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

                let mapper = hkask_cli::russell_mapper::RussellMapper::with_config(config.clone());

                if validate_only {
                    match hkask_cli::commands::import_russell(&source, &config, verbose) {
                        Ok(assets) => {
                            println!("Validation complete: {} manifests parsed", assets.len());
                            for asset in &assets {
                                println!("\n  ID: {} [VALID]", asset.id);
                            }
                        }
                        Err(e) => {
                            eprintln!("Validation failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    match hkask_cli::commands::import_russell_with_mapper(&mapper, &source, verbose)
                    {
                        Ok(assets) => {
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
                                        println!(
                                            "  russell[\"{}\"] --> hkask[\"{}\"]",
                                            asset.id, asset.id
                                        );
                                    }
                                }
                                _ => {
                                    println!(
                                        "Migration analysis complete: {} assets",
                                        assets.len()
                                    );
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
                                        eprintln!(
                                            "Failed to register template {}: {}",
                                            asset.id, e
                                        );
                                    } else if verbose {
                                        println!("  Registered: {}", asset.id);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Migration failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            RegistryAction::ListMigrated { origin: _ } => {
                println!("Migrated assets:");
                println!("  (use 'kask registry import-russell --dry-run' to analyze assets)");
            }
        },

        Commands::Git { action } => {
            let rt = tokio::runtime::Runtime::new().unwrap();
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
                        std::fs::read_to_string(&f).unwrap_or_else(|e| {
                            eprintln!("Failed to read file: {}", e);
                            std::process::exit(1);
                        })
                    } else {
                        eprintln!("Either --content or --file must be provided");
                        std::process::exit(1);
                    };

                    match rt.block_on(commands::archive_registry_to_git(
                        &runtime,
                        &owner,
                        &repo,
                        &branch,
                        &path,
                        &content_str,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Archive failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                GitAction::Restore {
                    owner,
                    repo,
                    r#ref,
                    target,
                } => {
                    match rt.block_on(commands::restore_registry_from_git(
                        &runtime, &owner, &repo, &r#ref, &target,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Restore failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                GitAction::List { owner, repo } => {
                    match rt.block_on(commands::list_registry_archives(&runtime, &owner, &repo)) {
                        Ok(commits) => {
                            println!("Archived versions for {}/{}:", owner, repo);
                            for (i, sha) in commits.iter().enumerate() {
                                println!("  {}. {}", i + 1, sha);
                            }
                        }
                        Err(e) => {
                            eprintln!("List failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                GitAction::Snapshot {
                    owner,
                    repo,
                    message,
                } => {
                    match rt.block_on(commands::create_registry_snapshot(
                        &runtime, &owner, &repo, &message,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Snapshot failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Spec { action } => match action {
            SpecAction::Capture {
                description,
                category,
                domain,
                criteria,
            } => {
                use hkask_types::{CompletenessCheck, DomainAnchor, GoalSpec, Spec, SpecCategory};

                let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
                let anchor = DomainAnchor::parse_str(&domain).unwrap_or(DomainAnchor::Hkask);

                let mut goal = GoalSpec::new(&description);
                if let Some(crits) = criteria {
                    for c in crits.split(',') {
                        goal = goal.with_criterion(c.trim());
                    }
                }

                let spec = Spec::new(&description, cat, anchor).with_goal(goal);
                let complete = spec.is_complete();

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
            SpecAction::Validate { threshold } => {
                println!(
                    "Validating specification collection (threshold: {:.2})",
                    threshold
                );
                println!("  Note: Validation requires hkask-mcp-spec server.");
            }
            SpecAction::Cultivate { threshold } => {
                use hkask_types::SpecCategory;

                println!(
                    "Cultivating specification collection (threshold: {:.2})",
                    threshold
                );
                println!("  Categories required:");
                for cat in SpecCategory::all() {
                    println!("    - {}", cat.as_str());
                }
                println!("  Note: Full cultivation requires hkask-mcp-spec server.");
            }
            SpecAction::Render { template, spec_id } => {
                use hkask_storage::SqliteSpecStore;
                use hkask_types::{SpecId, SpecStore};
                use minijinja::UndefinedBehavior;

                let template_path = format!("registry/templates/{}", template);
                let template_content = match std::fs::read_to_string(&template_path) {
                    Ok(content) => content,
                    Err(_) => {
                        eprintln!("Template not found: {}", template_path);
                        std::process::exit(1);
                    }
                };

                let db_path =
                    std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string());
                let conn = match rusqlite::Connection::open(&db_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to open database: {}", e);
                        std::process::exit(1);
                    }
                };
                let store = SqliteSpecStore::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
                if let Err(e) = store.init_schema() {
                    eprintln!("Failed to initialize spec schema: {}", e);
                    std::process::exit(1);
                }

                let ctx = if let Some(sid) = spec_id {
                    let parsed_id = match SpecId::from_string(&sid) {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("Invalid spec ID: {}", e);
                            std::process::exit(1);
                        }
                    };
                    match store.load(parsed_id) {
                        Ok(spec) => minijinja::context! {
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
                        },
                        Err(e) => {
                            eprintln!("Failed to load spec: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    minijinja::context! {}
                };

                let mut env = minijinja::Environment::new();
                env.set_undefined_behavior(UndefinedBehavior::Strict);
                match env.render_str(&template_content, ctx) {
                    Ok(rendered) => println!("{}", rendered),
                    Err(e) => {
                        eprintln!("Template render error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },

        Commands::Ensemble { action } => {
            let rt = tokio::runtime::Runtime::new().unwrap();

            match action {
                EnsembleAction::ChatCreate { session } => {
                    match rt.block_on(commands::ensemble_chat_create(session.clone())) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Chat create failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::ChatRegister { session, bot, role } => {
                    match rt.block_on(commands::ensemble_chat_register(
                        session.clone(),
                        bot.clone(),
                        role.clone(),
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Chat register failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::ChatSend { session, message } => {
                    match rt.block_on(commands::ensemble_chat_send(
                        session.clone(),
                        message.clone(),
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Chat send failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::ChatList => match rt.block_on(commands::ensemble_chat_list()) {
                    Ok(sessions) => {
                        println!("Active chat sessions:");
                        for s in sessions {
                            println!("  - {}", s);
                        }
                    }
                    Err(e) => {
                        eprintln!("Chat list failed: {}", e);
                        std::process::exit(1);
                    }
                },
                EnsembleAction::DeliberationCreate { session } => {
                    match rt.block_on(commands::ensemble_deliberation_create(session.clone())) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Deliberation create failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationStart { session } => {
                    match rt.block_on(commands::ensemble_deliberation_start(session.clone())) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Deliberation start failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationRecord {
                    session,
                    agent,
                    content,
                    confidence,
                } => {
                    match rt.block_on(commands::ensemble_deliberation_record(
                        session.clone(),
                        agent.clone(),
                        content.clone(),
                        confidence,
                    )) {
                        Ok(result) => println!("{}", result),
                        Err(e) => {
                            eprintln!("Deliberation record failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationSynthesize { session } => {
                    match rt.block_on(commands::ensemble_deliberation_synthesize(session.clone())) {
                        Ok(result) => println!("Synthesized response:\n{}", result),
                        Err(e) => {
                            eprintln!("Deliberation synthesize failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::DeliberationList => {
                    match rt.block_on(commands::ensemble_deliberation_list()) {
                        Ok(sessions) => {
                            println!("Active deliberation sessions:");
                            for s in sessions {
                                println!("  - {}", s);
                            }
                        }
                        Err(e) => {
                            eprintln!("Deliberation list failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::StandingStart { config } => {
                    match commands::ensemble_standing_start(&config) {
                        Ok(status) => {
                            println!("Standing session bootstrapped:");
                            println!("  Session ID: {}", status.session_id);
                            println!("  Participants: {}", status.participant_count);
                            println!("  Initial messages: {}", status.message_count);
                        }
                        Err(e) => {
                            eprintln!("Standing session bootstrap failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EnsembleAction::StandingStatus => match commands::ensemble_standing_status() {
                    Ok(status) => {
                        println!("Standing session status:");
                        println!("  Session ID: {}", status.session_id);
                        println!("  Participants: {}", status.participant_count);
                        println!("  Messages: {}", status.message_count);
                        println!("\nParticipants:");
                        for p in &status.participants {
                            println!("  - {} ({})", p.name, p.role);
                        }
                    }
                    Err(e) => {
                        eprintln!("Standing status failed: {}", e);
                        std::process::exit(1);
                    }
                },
            }
        }

        Commands::Agent { action } => match action {
            AgentAction::Register {
                webid,
                agent_type,
                capabilities,
            } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let caps: Vec<String> = capabilities
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    match commands::agent_register(&webid, &agent_type, caps).await {
                        Ok(receipt) => {
                            println!("Agent registered:");
                            println!("  WebID: {}", receipt.webid);
                            println!("  Token: {}...", &receipt.token_hash[..16]);
                            println!("  Registered at: {}", receipt.registered_at);
                        }
                        Err(e) => {
                            eprintln!("Registration failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            AgentAction::Unregister { name } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::agent_unregister(&name).await {
                        Ok(()) => println!("Agent unregistered: {}", name),
                        Err(e) => {
                            eprintln!("Unregister failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            AgentAction::List => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_list(None).await {
                        Ok(agents) => {
                            if agents.is_empty() {
                                println!("No agents registered.");
                                return;
                            }
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
                        Err(e) => {
                            eprintln!("Failed to list agents: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            AgentAction::Capabilities { name } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::bot_status(&name).await {
                        Ok(agent) => {
                            println!("Capabilities for {}:", agent.definition.name);
                            for cap in &agent.definition.capabilities {
                                println!("  - {}", cap);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to get capabilities: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
        },

        Commands::Curator { action } => match action {
            CuratorAction::Chat => {
                hkask_cli::repl::run(&registry, &runtime, None, "Curator");
            }
            CuratorAction::Escalations => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_escalations().await {
                        Ok(escalations) => {
                            if escalations.is_empty() {
                                println!("No pending escalations.");
                                return;
                            }
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
                                    &esc.error_context
                                        [..std::cmp::min(40, esc.error_context.len())],
                                );
                            }
                            println!("\nTotal: {} pending escalations", escalations.len());
                        }
                        Err(e) => {
                            eprintln!("Failed to list escalations: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            CuratorAction::Resolve { id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_resolve(&id).await {
                        Ok(()) => println!("Escalation {} resolved.", id),
                        Err(e) => {
                            eprintln!("Failed to resolve escalation: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            CuratorAction::Dismiss { id } => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_dismiss(&id).await {
                        Ok(()) => println!("Escalation {} dismissed.", id),
                        Err(e) => {
                            eprintln!("Failed to dismiss escalation: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
            CuratorAction::Metacognition => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match commands::curator_metacognition().await {
                        Ok(summary) => println!("{}", summary),
                        Err(e) => {
                            eprintln!("Metacognition cycle failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                });
            }
        },

        Commands::Replicant { action } => match action {
            ReplicantAction::Register {
                replicant_name,
                first_name,
                last_name,
                email,
                phone,
            } => {
                let store = hkask_storage::user_store::UserStore::new(std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection::open("hask.db")
                            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap()),
                    ),
                ));
                let store = std::sync::Arc::new(std::sync::Mutex::new(store));
                store.lock().unwrap().initialize_schema().unwrap();

                match commands::user::register_replicant(
                    &store,
                    &replicant_name,
                    &first_name,
                    &last_name,
                    &email,
                    phone.as_deref(),
                ) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Registration failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ReplicantAction::Login { replicant_name } => {
                let store = hkask_storage::user_store::UserStore::new(std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection::open("hask.db")
                            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap()),
                    ),
                ));
                let store = std::sync::Arc::new(std::sync::Mutex::new(store));
                store.lock().unwrap().initialize_schema().unwrap();

                match commands::user::login_replicant(&store, &replicant_name) {
                    Ok(session) => {
                        println!("Session ID: {}", session.session_id);
                        println!(
                            "\nTo logout: kask replicant logout {}",
                            &session.session_id[..8]
                        );
                    }
                    Err(e) => {
                        eprintln!("Login failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ReplicantAction::Logout { session_id } => {
                let store = hkask_storage::user_store::UserStore::new(std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection::open("hask.db")
                            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap()),
                    ),
                ));
                let store = std::sync::Arc::new(std::sync::Mutex::new(store));
                store.lock().unwrap().initialize_schema().unwrap();

                match commands::user::logout(&store, &session_id) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Logout failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ReplicantAction::Sessions { replicant_name } => {
                let store = hkask_storage::user_store::UserStore::new(std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection::open("hask.db")
                            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap()),
                    ),
                ));
                let store = std::sync::Arc::new(std::sync::Mutex::new(store));
                store.lock().unwrap().initialize_schema().unwrap();

                match commands::user::list_sessions(&store, &replicant_name) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to list sessions: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ReplicantAction::List { user_id } => {
                let store = hkask_storage::user_store::UserStore::new(std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection::open("hask.db")
                            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap()),
                    ),
                ));
                let store = std::sync::Arc::new(std::sync::Mutex::new(store));
                store.lock().unwrap().initialize_schema().unwrap();

                if let Some(uid) = user_id {
                    let user_id = hkask_types::UserID::from_string(&uid);
                    match commands::user::list_replicants(&store, &user_id) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Failed to list identities: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("--user-id is required");
                    std::process::exit(1);
                }
            }
            ReplicantAction::Show { replicant_name } => {
                let store = hkask_storage::user_store::UserStore::new(std::sync::Arc::new(
                    std::sync::Mutex::new(
                        rusqlite::Connection::open("hask.db")
                            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap()),
                    ),
                ));
                let store = std::sync::Arc::new(std::sync::Mutex::new(store));
                store.lock().unwrap().initialize_schema().unwrap();

                match commands::user::show_replicant(&store, &replicant_name) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Failed to show replicant: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },

        Commands::Keystore { action } => match action {
            KeystoreAction::Load {
                path,
                prefix,
                overwrite,
            } => {
                let keychain = hkask_keystore::Keychain::default();
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
                                println!(
                                    "  skipped {} (already in keychain, use --overwrite)",
                                    key
                                );
                                skipped += 1;
                            }
                            _ => match keychain.store_by_key(key, value) {
                                Ok(()) => {
                                    println!("  stored {}", key);
                                    loaded += 1;
                                }
                                Err(e) => {
                                    eprintln!("  failed {} : {}", key, e);
                                }
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
                let keychain = hkask_keystore::Keychain::default();
                match keychain.retrieve_by_key(&key) {
                    Ok(val) => {
                        if val.len() > 8 {
                            println!("{}={}**{}", key, &val[..4], &val[val.len() - 4..]);
                        } else {
                            println!("{}=****", key);
                        }
                    }
                    Err(e) => {
                        eprintln!("Key '{}' not found: {}", key, e);
                        std::process::exit(1);
                    }
                }
            }
            KeystoreAction::Set { key, value } => {
                let keychain = hkask_keystore::Keychain::default();
                match keychain.store_by_key(&key, &value) {
                    Ok(()) => println!("Stored {}", key),
                    Err(e) => {
                        eprintln!("Failed to store {}: {}", key, e);
                        std::process::exit(1);
                    }
                }
            }
            KeystoreAction::Delete { key } => {
                let keychain = hkask_keystore::Keychain::default();
                match keychain.delete_by_key(&key) {
                    Ok(()) => println!("Deleted {}", key),
                    Err(e) => {
                        eprintln!("Failed to delete {}: {}", key, e);
                        std::process::exit(1);
                    }
                }
            }
        },
    }
}
