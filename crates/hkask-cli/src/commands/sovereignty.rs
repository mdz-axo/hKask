//! Sovereignty command handlers for `kask sovereignty`
//!
//! Implements the CLI display logic for data sovereignty management.
//!
//! The kill-zone state is process-local: `MarkAcquisition` writes to a
//! shared `UserSovereigntyState`, and `KillZone` reads from the same
//! state. This replaces the previous behavior, which derived
//! `kill_zone_active` from a string match on the user's resistance level
//! (a conflation of configuration and detection).

use crate::cli::{self, SovereigntyAction};
use crate::commands;
use hkask_types::UserSovereigntyState;
use std::sync::{LazyLock, Mutex};

/// Process-local sovereignty state, shared by `MarkAcquisition` (writer) and
/// `KillZone` (reader). The Magna Carta's "kill-zone state" lives in the
/// `UserSovereigntyState`; the CLI is one of the writers.
static SOVEREIGNTY_STATE: LazyLock<Mutex<UserSovereigntyState>> =
    LazyLock::new(|| Mutex::new(UserSovereigntyState::new()));

pub fn run(action: SovereigntyAction) {
    use hkask_types::DataCategory;

    match action {
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
        SovereigntyAction::MarkAcquisition { vc_investment } => {
            // Write the acquisition attempt and current VC investment into
            // the process-local state. The kill-zone derivation follows the
            // same rule the runtime uses (acquisition_attempt && vc < 0.5).
            let mut state = SOVEREIGNTY_STATE
                .lock()
                .expect("sovereignty state mutex poisoned");
            state.mark_acquisition_attempt();
            state.update_vc_investment(vc_investment);
            let kill_zone_active = state.is_compromised();
            println!("Acquisition attempt marked.");
            println!("  VC investment: {:.2}", vc_investment);
            println!("  Kill zone active: {}", kill_zone_active);
            if kill_zone_active {
                println!("  [ALERT] Sovereignty compromised - CNS alert triggered!");
            }
        }
        SovereigntyAction::KillZone => {
            // Read the process-local kill-zone state. This is the same state
            // that `MarkAcquisition` writes to, so the two commands are
            // coherent within a CLI process. To see the same state across
            // processes (e.g., the API server's runtime), use the
            // `/api/sovereignty/killzone` endpoint.
            let state = SOVEREIGNTY_STATE
                .lock()
                .expect("sovereignty state mutex poisoned");

            println!("Kill-Zone Detection");
            println!("===================");
            println!();
            println!("Status:");
            println!(
                "  • Kill-zone active: {}",
                state.kill_zone_state.kill_zone_active
            );
            println!("  • Kill-zone threshold: {:.2}", state.kill_zone_threshold);
            println!();
            println!("Investment:");
            println!(
                "  • VC investment level: {:.2} (threshold: {:.2})",
                state.kill_zone_state.vc_investment, state.kill_zone_threshold
            );
            println!();
            println!("State:");
            println!(
                "  • Acquisition attempt: {}",
                state.kill_zone_state.acquisition_attempt
            );
            println!();
            if state.kill_zone_state.kill_zone_active {
                println!("[ALERT] Kill-zone active — sovereignty may be compromised!");
            } else {
                println!("Sovereignty boundary intact.");
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
