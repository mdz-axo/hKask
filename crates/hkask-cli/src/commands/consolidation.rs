//! Consolidation command — user-triggered episodic→semantic consolidation


use hkask_services::consolidation;
use hkask_types::WebID;
use hkask_types::loops::CuratorHandle;
use hkask_types::ports::ConsolidationRequest;

#[contract(
    id = "P9-CNS-SURF-007 pre: valid consolidation params post: cns.cli span emitted",
    principle = "P9"
)]
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

    // Build AgentService to get config with DB passphrase
    let config = super::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        eprintln!("Runtime error: {e}");
        std::process::exit(1)
    });
    let _ctx = super::helpers::or_exit(
        rt.block_on(hkask_services::AgentService::build(config.clone())),
        "Failed to build AgentService",
    );

    // Resolve the agent's per-agent memory DB path and passphrase.
    let db_path = format!("hkask-memory-{}.db", agent_name);
    let db_passphrase = config.db_passphrase.clone();

    // Resolve perspective WebID
    let handle = CuratorHandle::system();
    let perspective = match agent {
        Some(name) => WebID::from_persona(name.as_bytes()),
        None => *handle.curator_id(),
    };

    // Passphrase verification using ConsolidationService.
    if agent.is_some() {
        if let Some(provided) = passphrase {
            match consolidation::verify_passphrase(provided) {
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

    // Execute consolidation via ConsolidationService
    let request = ConsolidationRequest {
        limit,
        confidence_floor,
        max_semantic_triples,
    };

    match consolidation::consolidate(&perspective, &db_passphrase, &db_path, request) {
        Ok(outcome) => {
            println!("Consolidation complete:");
            println!("  Consolidated: {}", outcome.consolidated_count);
            println!("  Deleted: {}", outcome.deleted_count);
            if outcome.failed_count > 0 {
                println!("  Failed: {}", outcome.failed_count);
            }
        }
        Err(e) => {
            eprintln!("Consolidation failed: {}", e);
            std::process::exit(1);
        }
    }
}
