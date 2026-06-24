//! Magna Carta command handlers for `kask sovereignty verify`
//!
//! Implements CLI display logic for sovereignty verification reports.

use crate::cli::SovereigntyAction;
use hkask_services::VerificationService;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is SovereigntyAction::Verify with optional principle filter and json flag
/// post: runs Magna Carta verification; prints pass/fail/gap report with findings and recommendations
pub fn run(action: SovereigntyAction) {
    match action {
        SovereigntyAction::Verify { principle, json } => {
            if json {
                let result = VerificationService::verify_json(principle.as_deref());
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}).to_string()));
            } else {
                run_verify(principle.as_deref());
            }
        }
        _ => unreachable!("sovereignty::run dispatched wrong variant"),
    }
}

fn run_verify(filter: Option<&str>) {
    let report = VerificationService::verify(filter);
    if report.principles.is_empty() {
        eprintln!(
            "No Magna Carta manifests found. Expected manifests in .agents/skills/magna-carta-verifier/manifests/"
        );
        std::process::exit(1);
    }

    println!("Magna Carta Verification Report");
    println!("==============================");
    println!();

    for pr in &report.principles {
        let mut principle_pass = 0usize;
        let mut principle_fail = 0usize;
        let mut principle_gap = 0usize;

        println!("## {}", pr.display_name);
        println!();

        for result in &pr.assertion_results {
            let status_icon = match result.status.as_str() {
                "pass" => "✓",
                "fail" => "✗",
                "gap" => "△",
                "skip" => "—",
                _ => "?",
            };

            println!(
                "  {status_icon} {id} {name}: {status}",
                id = result.id,
                name = result.name,
                status = result.status
            );

            for finding in &result.findings {
                println!("    → {finding}");
            }
            for rec in &result.recommendations {
                println!("    ⚑ {rec}");
            }

            match result.status.as_str() {
                "pass" => principle_pass += 1,
                "fail" => principle_fail += 1,
                "gap" => principle_gap += 1,
                _ => {}
            }
        }

        println!();
        println!(
            "  Principle summary: {pass} pass, {fail} fail, {gap} gap",
            pass = principle_pass,
            fail = principle_fail,
            gap = principle_gap,
        );
        println!();
    }

    println!("---");
    println!(
        "Total: {} assertions — {} pass, {} fail, {} gap, {} skip",
        report.total_assertions,
        report.total_pass,
        report.total_fail,
        report.total_gap,
        report.total_skip
    );

    if report.total_fail > 0 || report.total_gap > 0 {
        println!();
        println!(
            "⚠ {} assertion(s) failed and {} have gaps.",
            report.total_fail, report.total_gap
        );
        println!("  Escalate to Curator for review with human user or replicant.");
    }
}
