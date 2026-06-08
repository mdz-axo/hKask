//! Sovereignty command handlers for `kask sovereignty`
//
//! Implements the CLI display logic for data sovereignty management.
//! Delegates consent/boundary operations to `SovereigntyService` and
//! formats results for terminal output. All context is derived from
//! `ServiceContext` via `SovereigntyContext::from(&*ctx)` — no direct
//! database access.

use hkask_services::{SovereigntyContext, SovereigntyService, parse_data_category};
use hkask_types::DataCategory;

use crate::commands;

pub fn run(action: crate::cli::SovereigntyAction) {
    match action {
        crate::cli::SovereigntyAction::Verify { .. } => commands::magna_carta::run(action),
        _ => run_sovereignty_ops(action),
    }
}

fn build_service_context() -> hkask_services::ServiceContext {
    let config =
        hkask_services::ServiceConfig::from_env().expect("Failed to resolve service config");
    let rt = tokio::runtime::Runtime::new().expect("runtime should start");
    rt.block_on(hkask_services::ServiceContext::build(config))
        .expect("Failed to build service context")
}

fn build_ctx() -> SovereigntyContext {
    SovereigntyContext::from(&build_service_context())
}

fn run_sovereignty_ops(action: crate::cli::SovereigntyAction) {
    match action {
        crate::cli::SovereigntyAction::Verify { .. } => {
            unreachable!("Verify handled by magna_carta module")
        }
        crate::cli::SovereigntyAction::Status => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let svc_ctx = build_service_context();
            let ctx = SovereigntyContext::from(&svc_ctx);
            let status = SovereigntyService::get_status(&ctx, &webid.to_string())
                .expect("Failed to get sovereignty status");

            // Use the sovereignty store from ServiceContext (replaces open_sovereignty_store)
            let store = &svc_ctx.sovereignty_boundary_store;

            println!("Sovereignty Status");
            println!("==================");
            println!();
            println!("Consent State:");
            println!("  WebID: {}", webid);

            // Per-category consent check using service
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
                match SovereigntyService::has_consent(&ctx, &webid.to_string(), cat) {
                    Ok(true) => println!("  • {}: GRANTED", label),
                    Ok(false) => println!("  • {}: DENIED", label),
                    Err(e) => println!("  • {}: ERROR ({})", label, e),
                }
            }
            println!();
            println!("Data Boundaries:");
            if status.sovereign_data.is_empty()
                && status.shared_data.is_empty()
                && status.public_data.is_empty()
            {
                println!("  • No boundary data stored yet");
            } else {
                if !status.sovereign_data.is_empty() {
                    println!("  • Sovereign: {}", status.sovereign_data.join(", "));
                }
                if !status.shared_data.is_empty() {
                    println!("  • Shared: {}", status.shared_data.join(", "));
                }
                if !status.public_data.is_empty() {
                    println!("  • Public: {}", status.public_data.join(", "));
                }
            }
            println!();
            println!("Affirmative Consent:");
            // The store may have user-customized affirmative consent settings.
            // Fall back to the service-provided default boundary.
            match store.get(&webid.to_string()) {
                Ok(Some(entry)) => {
                    println!(
                        "  • Requires Affirmative Consent: {}",
                        entry.requires_affirmative_consent
                    );
                }
                Ok(None) => println!(
                    "  • Requires Affirmative Consent: {}",
                    status.requires_affirmative_consent
                ),
                Err(_) => println!(
                    "  • Requires Affirmative Consent: {}",
                    status.requires_affirmative_consent
                ),
            }
        }
        crate::cli::SovereigntyAction::Grant { category } => {
            let webid = hkask_types::WebID::new();
            let data_category = parse_data_category(&category);
            let ctx = build_ctx();
            match SovereigntyService::grant_consent(&ctx, &webid.to_string(), &data_category) {
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
        crate::cli::SovereigntyAction::Revoke { category: _ } => {
            let webid = hkask_types::WebID::new();
            let ctx = build_ctx();
            match SovereigntyService::revoke_consent(&ctx, &webid.to_string()) {
                Ok(()) => {
                    println!("Consent revoked.");
                    println!("  Data sharing is now disabled for this category.");
                    println!("  Only public data is accessible.");
                }
                Err(e) => eprintln!("Error revoking consent: {}", e),
            }
        }
        crate::cli::SovereigntyAction::Check { category } => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let data_category = parse_data_category(&category);
            let ctx = build_ctx();

            let result = SovereigntyService::check_access(&ctx, &webid.to_string(), &data_category);

            println!("Data Access Check");
            println!("=================");
            println!("  Category: {}", category);

            match result {
                Ok(access) => {
                    println!("  Classification: {}", access.classification);
                    println!("  Access required: {}", access.access_required);
                    if access.has_consent {
                        println!("  Access: GRANTED");
                        println!("  Consent has been explicitly given for this category.");
                    } else {
                        println!("  Access: DENIED");
                        println!(
                            "  No consent for this category. Use 'kask sovereignty grant --category {}' to grant.",
                            category
                        );
                    }
                }
                Err(e) => {
                    println!("  Access: ERROR");
                    println!("  Failed to check access: {}", e);
                }
            }
        }
    }
}
