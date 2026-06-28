//! Consolidation command — user-triggered episodic→semantic consolidation

use hkask_ports::ConsolidationRequest;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  agent is an optional agent name (defaults to curator); limit > 0; passphrase required when agent specified
/// post: executes episodic-to-semantic consolidation; prints consolidated/deleted/failed counts
pub fn run(
    agent: Option<&str>,
    limit: usize,
    confidence_floor: Option<f64>,
    max_semantic_triples: Option<usize>,
    passphrase: Option<&str>,
) {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "consolidation", agent = ?agent, limit = limit, "CNS");
    // Resolve agent name — defaults to "curator" for the Curator agent
    let agent_name = agent.unwrap_or("curator");

    let agent_service = super::helpers::build_service_context();

    // Passphrase verification using ConsolidationService.
    // Required when targeting a non-Curator agent as an additional auth gate.
    if agent.is_some() {
        if let Some(provided) = passphrase {
            match hkask_memory::consolidation_ops::verify_passphrase(provided) {
                Ok(_) => {}
                Err(_) => {
                    eprintln!("Error: Passphrase verification failed");
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Error: --passphrase is required when specifying an agent");
            std::process::exit(1);
        }
    }

    // Execute consolidation via AgentService (consent + per-agent DB)
    let request = ConsolidationRequest {
        limit,
        confidence_floor,
        max_semantic_triples,
    };

    match agent_service.consolidate_agent_memory(agent_name, request) {
        Ok(outcome) => {
            println!("Consolidation complete:");
            println!("  Consolidated: {}", outcome.consolidated_count);
            println!("  Deleted: {}", outcome.deleted_count);
            if outcome.failed_count > 0 {
                println!("  Failed: {}", outcome.failed_count);
            }
        }
        Err(hkask_services::ServiceError::ConsentDenied { message }) => {
            eprintln!("Consent required: {}", message);
            eprintln!("Grant consent with: kask sovereignty grant --category episodic_memory");
            eprintln!(
                "                              kask sovereignty grant --category semantic_memory"
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Consolidation failed: {}", e);
            std::process::exit(1);
        }
    }
}
