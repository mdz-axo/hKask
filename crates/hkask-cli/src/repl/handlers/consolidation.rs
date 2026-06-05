//! REPL /consolidate handler — user-triggered episodic→semantic consolidation

use hkask_types::ports::ConsolidationRequest;

pub(crate) fn handle_consolidate(
    arg: &str,
    state: &mut super::super::ReplState,
    _rt: &tokio::runtime::Handle,
) {
    let trimmed = arg.trim();

    // Show status
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        println!("  \x1b[1mConsolidation Status\x1b[0m");
        println!("  Agent: \x1b[36m{}\x1b[0m", state.current_agent);
        println!("  Agent WebID: {}", state.agent_webid);

        match &state.consolidation_service {
            Some(svc) => {
                let candidates = svc.consolidation_candidate_count(&state.agent_webid);
                let semantic_count = svc.semantic_triple_count();
                let low_conf = svc.semantic_low_confidence_count(0.33);
                println!();
                println!("  Consolidation candidates: {}", candidates);
                println!("  Semantic triple count: {}", semantic_count);
                println!("  Low-confidence triples (≤0.33): {}", low_conf);
                println!();
                println!("  Use \x1b[36m/consolidate run\x1b[0m to trigger consolidation");
            }
            None => {
                println!();
                println!(
                    "  \x1b[33mConsolidation service unavailable\x1b[0m (registry DB not accessible)"
                );
                println!("  Use \x1b[36mkask consolidate\x1b[0m for CLI-based consolidation");
            }
        }
        println!();
        return;
    }

    // "run" or other — execute consolidation with defaults
    let service = match &state.consolidation_service {
        Some(svc) => svc,
        None => {
            println!(
                "  \x1b[31mError:\x1b[0m Consolidation service unavailable (registry DB not accessible)"
            );
            println!("  Use \x1b[36mkask consolidate\x1b[0m for CLI-based consolidation");
            return;
        }
    };

    // Parse optional sub-arguments from "run [--floor F] [--max M] [--limit L]"
    let mut confidence_floor: Option<f64> = None;
    let mut max_semantic_triples: Option<usize> = None;
    let mut limit: usize = 100;
    let mut unknown_flags: Vec<String> = Vec::new();

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "run" => { /* skip keyword */ }
            "--floor" | "-f" => {
                if let Some(v) = parts.get(i + 1) {
                    match v.parse::<f64>() {
                        Ok(val) => confidence_floor = Some(val),
                        Err(_) => {
                            println!("  \x1b[31mError:\x1b[0m Invalid --floor value: '{}'", v);
                            println!("  Expected a number between 0.0 and 1.0");
                            return;
                        }
                    }
                    i += 1;
                } else {
                    println!(
                        "  \x1b[31mError:\x1b[0m --floor requires a value (e.g., --floor 0.33)"
                    );
                    return;
                }
            }
            "--max" | "-m" => {
                if let Some(v) = parts.get(i + 1) {
                    match v.parse::<usize>() {
                        Ok(val) => max_semantic_triples = Some(val),
                        Err(_) => {
                            println!("  \x1b[31mError:\x1b[0m Invalid --max value: '{}'", v);
                            println!("  Expected a positive integer");
                            return;
                        }
                    }
                    i += 1;
                } else {
                    println!("  \x1b[31mError:\x1b[0m --max requires a value (e.g., --max 500)");
                    return;
                }
            }
            "--limit" | "-l" => {
                if let Some(v) = parts.get(i + 1) {
                    match v.parse::<usize>() {
                        Ok(val) => limit = val,
                        Err(_) => {
                            println!("  \x1b[31mError:\x1b[0m Invalid --limit value: '{}'", v);
                            println!("  Expected a positive integer");
                            return;
                        }
                    }
                    i += 1;
                } else {
                    println!("  \x1b[31mError:\x1b[0m --limit requires a value (e.g., --limit 50)");
                    return;
                }
            }
            other if other.starts_with("--") || other.starts_with("-") => {
                unknown_flags.push(other.to_string());
            }
            other => {
                // Try to parse as a bare limit number (e.g., "/consolidate 50")
                if let Ok(n) = other.parse::<usize>() {
                    limit = n;
                }
            }
        }
        i += 1;
    }

    if !unknown_flags.is_empty() {
        println!(
            "  \x1b[33mWarning:\x1b[0m Unknown flags ignored: {}",
            unknown_flags.join(", ")
        );
        println!("  Valid flags: --floor, --max, --limit");
    }

    // Show pre-consolidation state
    let candidates = service.consolidation_candidate_count(&state.agent_webid);
    let semantic_count = service.semantic_triple_count();
    let low_conf = service.semantic_low_confidence_count(0.33);
    println!("  \x1b[1mPre-consolidation state:\x1b[0m");
    println!("  Consolidation candidates: {}", candidates);
    println!("  Semantic triple count: {}", semantic_count);
    println!("  Low-confidence triples (≤0.33): {}", low_conf);

    let request = ConsolidationRequest {
        limit,
        confidence_floor,
        max_semantic_triples,
    };

    match service.consolidate(&state.agent_webid, request) {
        Ok(outcome) => {
            println!();
            println!("  \x1b[1mConsolidation complete:\x1b[0m");
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
            println!("  \x1b[31mConsolidation failed:\x1b[0m {}", e);
        }
    }
    println!();
}
