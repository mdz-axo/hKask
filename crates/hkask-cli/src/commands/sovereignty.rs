//! Sovereignty command handlers for `kask sovereignty`
//!
//! Implements the CLI display logic for data sovereignty management.

use crate::cli::{self, SovereigntyAction};
use crate::commands;

pub fn run(action: SovereigntyAction) {
    match action {
        SovereigntyAction::Verify { .. } => commands::magna_carta::run(action),
        _ => run_sovereignty_ops(action),
    }
}

fn run_sovereignty_ops(action: SovereigntyAction) {
    use hkask_types::DataCategory;

    match action {
        SovereigntyAction::Verify { .. } => unreachable!("Verify handled by magna_carta module"),
        SovereigntyAction::Status => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let store = super::helpers::or_exit(
                commands::config::open_sovereignty_store(),
                "Failed to open sovereignty store",
            );
            let consent_store = super::helpers::or_exit(
                commands::config::open_consent_store(),
                "Failed to open consent store",
            );
            let consent_manager = hkask_agents::ConsentManager::new(consent_store);

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
            println!("Affirmative Consent:");
            match store.get(&webid.to_string()) {
                Ok(Some(entry)) => {
                    println!(
                        "  • Requires Affirmative Consent: {}",
                        entry.requires_affirmative_consent
                    );
                }
                Ok(None) => println!("  • No affirmative consent data stored yet"),
                Err(e) => println!("  • Error loading affirmative consent: {}", e),
            }
        }
        SovereigntyAction::Grant { category } => {
            let webid = hkask_types::WebID::new();
            let data_category = cli::parse_data_category(&category);
            let consent_store = super::helpers::or_exit(
                commands::config::open_consent_store(),
                "Failed to open consent store",
            );
            let consent_manager = hkask_agents::ConsentManager::new(consent_store);
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
            let consent_store = super::helpers::or_exit(
                commands::config::open_consent_store(),
                "Failed to open consent store",
            );
            let consent_manager = hkask_agents::ConsentManager::new(consent_store);
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

        SovereigntyAction::Check { category } => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let consent_store = super::helpers::or_exit(
                commands::config::open_consent_store(),
                "Failed to open consent store",
            );
            let consent_manager = hkask_agents::ConsentManager::new(consent_store);
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
