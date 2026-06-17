//! `kask contract` — Replicant-driven contract proposal workflow (Phase B2–B4)
//!
//! Implements the behavioral contract lifecycle: agents propose contracts,
//! humans accept or reject them. Each step emits a CNS span for observability
//! and persists the proposal as a triple for curation review.
//!
//! This is the core agentic QA workflow: agents discover uncontracted functions
//! via `contract-audit.sh`, analyze behavior, propose contracts, and submit
//! them for human consent (P2).

use crate::cli::ContractAction;
use hkask_cns::{emit_contract_accepted, emit_contract_proposed, emit_contract_rejected};
use hkask_storage::{NuEventStore, Triple, TripleStore, in_memory_db};
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use std::sync::Arc;

const PROPOSAL_ENTITY: &str = "cns:contract_proposal";

/// REQ: CLI-092
pub fn run(rt: &tokio::runtime::Runtime, action: ContractAction) {
    match action {
        ContractAction::Propose {
            crate_name,
            function,
            contract_id,
            pre,
            post,
            replicant,
        } => {
            let rep = replicant.unwrap_or_else(|| "unknown-replicant".to_string());
            if let Err(e) = rt.block_on(handle_propose(
                &rep,
                &crate_name,
                &function,
                &contract_id,
                &pre,
                &post,
            )) {
                eprintln!("Proposal failed: {}", e);
                std::process::exit(1);
            }
            println!(
                "Contract proposal submitted: {} -> {}::{}",
                contract_id, crate_name, function
            );
            println!("  pre:  {}", pre);
            println!("  post: {}", post);
            println!(
                "Awaiting human review (kask contract accept|reject {}).",
                contract_id
            );
        }
        ContractAction::Accept {
            contract_id,
            reviewer,
        } => {
            let rev = reviewer.unwrap_or_else(|| "unknown-reviewer".to_string());
            if let Err(e) = rt.block_on(handle_accept(&rev, &contract_id)) {
                eprintln!("Accept failed: {}", e);
                std::process::exit(1);
            }
            println!("Contract accepted: {} by {}", contract_id, rev);
        }
        ContractAction::Reject {
            contract_id,
            reason,
            reviewer,
        } => {
            let rev = reviewer.unwrap_or_else(|| "unknown-reviewer".to_string());
            if let Err(e) = rt.block_on(handle_reject(&rev, &contract_id, &reason)) {
                eprintln!("Reject failed: {}", e);
                std::process::exit(1);
            }
            println!("Contract rejected: {} — {}", contract_id, reason);
        }
        ContractAction::List => {
            if let Err(e) = rt.block_on(handle_list()) {
                eprintln!("List failed: {}", e);
            }
        }
        ContractAction::Discover { crate_name } => {
            let workspace = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            handle_discover(crate_name, &workspace);
        }
    }
}

fn open_stores() -> Result<(Arc<dyn NuEventSink>, TripleStore), String> {
    // Contract proposals use in-memory DB — persistence comes from the daemon
    // which holds the actual DB connection across restarts.
    let db = in_memory_db();
    let conn = db.conn_arc();
    let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));
    let triple_store = TripleStore::new(conn);
    Ok((sink, triple_store))
}

async fn handle_propose(
    replicant: &str,
    crate_name: &str,
    function: &str,
    contract_id: &str,
    pre: &str,
    post: &str,
) -> Result<(), String> {
    let (sink, triple_store) = open_stores()?;
    emit_contract_proposed(&*sink, replicant, crate_name, function, contract_id);

    let value = serde_json::json!({
        "replicant": replicant, "crate": crate_name, "function": function,
        "contract_id": contract_id, "pre": pre, "post": post,
        "status": "proposed", "proposed_at": chrono::Utc::now().to_rfc3339(),
    });
    let triple = Triple::new(
        PROPOSAL_ENTITY,
        contract_id,
        value,
        WebID::from_persona(replicant.as_bytes()),
    );
    triple_store
        .insert(&triple)
        .map_err(|e| format!("Failed to persist: {e}"))?;
    Ok(())
}

async fn handle_accept(reviewer: &str, contract_id: &str) -> Result<(), String> {
    let (sink, triple_store) = open_stores()?;
    emit_contract_accepted(&*sink, reviewer, "", "", "", contract_id);

    let mut existing = triple_store
        .query_by_entity_attribute(PROPOSAL_ENTITY, contract_id)
        .map_err(|e| format!("Query failed: {e}"))?;
    if let Some(mut triple) = existing.pop() {
        let mut value = triple.value.clone();
        value["status"] = serde_json::json!("accepted");
        value["reviewer"] = serde_json::json!(reviewer);
        value["accepted_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        let update_value = value.clone();
        triple.value = value;
        triple_store
            .update(&triple.id, update_value, hkask_types::Confidence::full())
            .map_err(|e| format!("Update failed: {e}"))?;
    }
    Ok(())
}

async fn handle_reject(reviewer: &str, contract_id: &str, rationale: &str) -> Result<(), String> {
    let (sink, triple_store) = open_stores()?;
    emit_contract_rejected(&*sink, reviewer, "", "", "", contract_id, rationale);

    let mut existing = triple_store
        .query_by_entity_attribute(PROPOSAL_ENTITY, contract_id)
        .map_err(|e| format!("Query failed: {e}"))?;
    if let Some(mut triple) = existing.pop() {
        let mut value = triple.value.clone();
        value["status"] = serde_json::json!("rejected");
        value["reviewer"] = serde_json::json!(reviewer);
        value["reason"] = serde_json::json!(rationale);
        value["rejected_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        let update_value = value.clone();
        triple.value = value;
        triple_store
            .update(&triple.id, update_value, hkask_types::Confidence::full())
            .map_err(|e| format!("Update failed: {e}"))?;
    }
    Ok(())
}

async fn handle_list() -> Result<(), String> {
    let (_, triple_store) = open_stores()?;
    let proposals = triple_store
        .query_by_entity(PROPOSAL_ENTITY)
        .map_err(|e| format!("Query failed: {e}"))?;
    if proposals.is_empty() {
        println!("No contract proposals found.");
        return Ok(());
    }
    println!("Contract Proposals");
    println!("==================");
    for triple in &proposals {
        let status = triple.value["status"].as_str().unwrap_or("unknown");
        let cid = triple.value["contract_id"].as_str().unwrap_or("?");
        let fun = triple.value["function"].as_str().unwrap_or("?");
        let pre = triple.value["pre"].as_str().unwrap_or("?");
        let post = triple.value["post"].as_str().unwrap_or("?");
        println!("  [{}] {} — {}", status.to_uppercase(), cid, fun);
        println!("    pre:  {}", pre);
        println!("    post: {}", post);
    }
    Ok(())
}

fn handle_discover(crate_name: Option<String>, workspace_root: &str) {
    use hkask_test_harness::test_runner::discover_uncontracted_functions;

    let crates: Vec<String> = if let Some(c) = crate_name {
        vec![c]
    } else {
        let crates_dir = std::path::Path::new(workspace_root).join("crates");
        if let Ok(entries) = std::fs::read_dir(&crates_dir) {
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .filter(|s| s.starts_with("hkask-"))
                .collect()
        } else {
            vec![]
        }
    };

    for name in &crates {
        match discover_uncontracted_functions(name, workspace_root) {
            Some(audit) if !audit.uncontracted.is_empty() => {
                println!(
                    "{}: {}/{} contracted ({:.1}%), {} uncontracted:",
                    audit.crate_name,
                    audit.contracted,
                    audit.total_pub_fns,
                    audit.coverage_pct,
                    audit.uncontracted.len(),
                );
                for f in &audit.uncontracted {
                    println!("  {} L{} — {}", f.file, f.line, f.signature);
                }
                println!();
            }
            Some(audit) => {
                println!(
                    "{}: {}/{} contracted ({:.1}%) — all clear",
                    audit.crate_name, audit.contracted, audit.total_pub_fns, audit.coverage_pct
                );
            }
            None => {
                eprintln!("  {}: source directory not found", name);
            }
        }
    }
}
