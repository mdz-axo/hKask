//! Sovereignty command handlers — call consent manager directly.

use hkask_types::DataCategory;
use hkask_types::sovereignty::DataSovereigntyBoundary;

use crate::commands;

pub fn run(action: crate::cli::SovereigntyAction) {
    match action {
        crate::cli::SovereigntyAction::Verify { .. } => commands::magna_carta::run(action),
        _ => run_sovereignty_ops(action),
    }
}

fn parse_data_category(s: &str) -> DataCategory {
    DataCategory::parse(s)
}

fn build_consent() -> (
    hkask_services::AgentService,
    hkask_services::SovereigntyService,
) {
    let config = hkask_services::ServiceConfig::from_env().unwrap_or_else(|e| {
        eprintln!("Config env: {e}");
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        eprintln!("runtime: {e}");
        std::process::exit(1);
    });
    let svc = rt
        .block_on(hkask_services::AgentService::build(config))
        .unwrap_or_else(|e| {
            eprintln!("build svc: {e}");
            std::process::exit(1);
        });
    let cm = svc.sovereignty();
    (svc, cm)
}

fn run_sovereignty_ops(action: crate::cli::SovereigntyAction) {
    match action {
        crate::cli::SovereigntyAction::Verify { .. } => unreachable!(),
        crate::cli::SovereigntyAction::Status => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let (svc_ctx, cm) = build_consent();
            let boundary = DataSovereigntyBoundary::hkask_default();
            let _granted: Vec<String> = cm
                .get_granted_categories(&webid.to_string())
                .unwrap_or_default();

            println!("Sovereignty Status");
            println!("==================");
            println!();
            println!("Consent State:");
            println!("  WebID: {}", webid);

            let categories = [
                ("episodic_memory", &DataCategory::EpisodicMemory),
                ("semantic_memory", &DataCategory::SemanticMemory),
                ("personal_context", &DataCategory::PersonalContext),
                ("capability_tokens", &DataCategory::CapabilityTokens),
                ("ocap_boundaries", &DataCategory::OcapBoundaries),
                ("template_invocations", &DataCategory::TemplateInvocations),
                ("hlexicon_terms", &DataCategory::HLexiconTerms),
                ("template_registry", &DataCategory::TemplateRegistry),
            ];
            for (label, cat) in &categories {
                match cm.has_consent(&webid.to_string(), cat) {
                    true => println!("  • {label}: GRANTED"),
                    false => println!("  • {label}: DENIED"),
                }
            }
            println!();
            println!("Data Boundaries:");
            if boundary.sovereign_data.is_empty()
                && boundary.shared_data.is_empty()
                && boundary.public_data.is_empty()
            {
                println!("  • No boundary data stored yet");
            } else {
                if !boundary.sovereign_data.is_empty() {
                    println!(
                        "  • Sovereign: {}",
                        boundary
                            .sovereign_data
                            .iter()
                            .map(|c| c.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if !boundary.shared_data.is_empty() {
                    println!(
                        "  • Shared: {}",
                        boundary
                            .shared_data
                            .iter()
                            .map(|c| c.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if !boundary.public_data.is_empty() {
                    println!(
                        "  • Public: {}",
                        boundary
                            .public_data
                            .iter()
                            .map(|c| c.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
            println!();
            println!("Affirmative Consent:");
            let store = &svc_ctx.sovereignty_boundary_store();
            match store.get(&webid.to_string()) {
                Ok(Some(entry)) => println!(
                    "  • Requires Affirmative Consent: {}",
                    entry.requires_affirmative_consent
                ),
                Ok(None) => println!(
                    "  • Requires Affirmative Consent: {}",
                    boundary.requires_affirmative_consent()
                ),
                Err(_) => println!(
                    "  • Requires Affirmative Consent: {}",
                    boundary.requires_affirmative_consent()
                ),
            }
        }
        crate::cli::SovereigntyAction::Grant { category } => {
            let webid = hkask_types::WebID::new();
            let (_svc, cm) = build_consent();
            let cat = parse_data_category(&category);
            match cm.grant_consent(&webid.to_string(), &cat) {
                Ok(()) => {
                    println!("Consent granted for category: {category}");
                    println!("  Data sharing is now enabled for this category.");
                    if cat.is_typically_sovereign() {
                        println!("  Note: Sovereign data still requires owner verification.");
                    }
                }
                Err(e) => eprintln!("Error granting consent: {e}"),
            }
        }
        crate::cli::SovereigntyAction::Revoke { category: _ } => {
            let webid = hkask_types::WebID::new();
            let (_svc, cm) = build_consent();
            match cm.revoke_consent(&webid.to_string()) {
                Ok(()) => {
                    println!("Consent revoked.");
                    println!("  Data sharing is now disabled for this category.");
                    println!("  Only public data is accessible.");
                }
                Err(e) => eprintln!("Error revoking consent: {e}"),
            }
        }
        crate::cli::SovereigntyAction::Check { category } => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let cat = parse_data_category(&category);
            let (_svc, cm) = build_consent();
            let boundary = DataSovereigntyBoundary::hkask_default();

            let class = boundary.classify(&cat);
            let classification = class.label();
            let access_required = class.access_required();
            let has_consent = if classification == "PUBLIC" {
                true
            } else {
                cm.has_consent(&webid.to_string(), &cat)
            };

            println!("Data Access Check");
            println!("=================");
            println!("  Category: {category}");
            println!("  Classification: {classification}");
            println!("  Access required: {access_required}");
            if has_consent {
                println!("  Access: GRANTED");
            } else {
                println!("  Access: DENIED");
                println!("  Use 'kask sovereignty grant --category {category}' to grant.");
            }
        }
    }
}
