//! Consolidation command — user-triggered episodic→semantic consolidation + semantic cleanup

use std::sync::Arc;

use hkask_memory::{ConsolidationBridge, ConsolidationService, EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_types::WebID;
use hkask_types::loops::CuratorHandle;
use hkask_types::ports::ConsolidationRequest;

pub fn run(
    agent: Option<&str>,
    limit: usize,
    confidence_floor: Option<f64>,
    max_semantic_triples: Option<usize>,
    passphrase: Option<&str>,
) {
    // Resolve agent name — defaults to "curator" for the Curator agent
    let agent_name = agent.unwrap_or("curator");

    // Resolve the agent's per-agent memory DB path and passphrase.
    // Consolidation operates on the agent's actual episodic and semantic
    // triples, which live in hkask-memory-{agent}.db — not the registry DB.
    let db_path = format!("hkask-memory-{}.db", agent_name);
    let db_passphrase = match hkask_keystore::resolve_db_passphrase() {
        Ok(pass) => String::from_utf8_lossy(&pass).to_string(),
        Err(e) => {
            eprintln!("Error: Could not resolve DB passphrase: {}", e);
            std::process::exit(1);
        }
    };
    let db = match Database::open(&db_path, &db_passphrase) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: Failed to open agent memory DB ({}): {}", db_path, e);
            std::process::exit(1);
        }
    };
    let conn = db.conn_arc();

    // Build memory infrastructure from the agent's DB
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(triple_store));
    let triple_store2 = TripleStore::new(Arc::clone(&conn));
    let embedding_store = EmbeddingStore::new(Arc::clone(&conn));
    let semantic_memory = Arc::new(SemanticMemory::new(triple_store2, embedding_store));

    // Build consolidation bridge + service
    let bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let handle = CuratorHandle::system();
    let token = handle.issue_consolidation_token();
    let service = ConsolidationService::new(bridge, semantic_memory, token);

    // Resolve perspective WebID
    let perspective = match agent {
        Some(name) => WebID::from_persona(name.as_bytes()),
        None => handle.curator_id().clone(),
    };

    // Passphrase verification using the master-passphrase → capability_key derivation chain.
    // This matches onboarding: derive_all_internal_secrets(master_passphrase) produces
    // a capability_key that is stored in the keychain as "hkask-db-passphrase" and used
    // as the DB encryption key. We verify the user-supplied master passphrase by
    // deriving the capability_key and comparing it against the resolved DB passphrase.
    if agent.is_some() {
        if let Some(provided) = passphrase {
            let expected = match hkask_keystore::resolve_db_passphrase() {
                Ok(db_pass) => String::from_utf8_lossy(&db_pass).to_string(),
                Err(_) => {
                    eprintln!(
                        "Error: Could not resolve database passphrase from keychain or environment"
                    );
                    std::process::exit(1);
                }
            };
            // Derive capability_key from the provided master passphrase
            let secrets = hkask_keystore::master_key::derive_all_internal_secrets(provided);
            if secrets.capability_key != expected {
                eprintln!("Error: Passphrase verification failed");
                std::process::exit(1);
            }
        } else {
            eprintln!("Error: --passphrase is required when specifying an agent");
            std::process::exit(1);
        }
    }

    // Report pre-consolidation state
    let candidates = service.consolidation_candidate_count(&perspective);
    let semantic_count = service.semantic_triple_count();
    let low_conf = service.semantic_low_confidence_count(0.33);
    println!("Pre-consolidation state:");
    println!("  Agent memory DB: {}", db_path);
    println!("  Consolidation candidates: {}", candidates);
    println!("  Semantic triple count: {}", semantic_count);
    println!("  Low-confidence triples (≤0.33): {}", low_conf);

    // Execute consolidation
    let request = ConsolidationRequest {
        limit,
        confidence_floor,
        max_semantic_triples,
    };

    match service.consolidate(&perspective, request) {
        Ok(outcome) => {
            println!("\nConsolidation complete:");
            println!("  Consolidated: {}", outcome.consolidated_count);
            println!("  Deleted: {}", outcome.deleted_count);
            if outcome.failed_count > 0 {
                println!("  Failed: {}", outcome.failed_count);
            }
            println!(
                "  Post-consolidation semantic count: {}",
                service.semantic_triple_count()
            );
        }
        Err(e) => {
            eprintln!("Consolidation failed: {}", e);
            std::process::exit(1);
        }
    }
}
