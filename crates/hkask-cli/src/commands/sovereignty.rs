//! Sovereignty command handlers — consent manager, boundary enforcement, and
//! Magna Carta verification.

use std::sync::Arc;

use hkask_agents::consent::ConsentManager;
use hkask_types::DataCategory;
use hkask_types::curation::DataSovereigntyBoundary;

use crate::cli::SovereigntyAction;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is a valid SovereigntyAction variant
/// post: dispatches to verify or sovereignty ops (status, grant, revoke, check)
pub fn run(action: SovereigntyAction) {
    match action {
        SovereigntyAction::Verify { .. } => run_verify(action),
        _ => run_sovereignty_ops(action),
    }
}

// ── Magna Carta verification ────────────────────────────────────────────

fn run_verify(action: SovereigntyAction) {
    let (principle, json) = match action {
        SovereigntyAction::Verify { principle, json } => (principle, json),
        _ => unreachable!(),
    };
    if json {
        let result =
            hkask_services_verification::VerificationService::verify_json(principle.as_deref());
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| { serde_json::json!({"error": e.to_string()}).to_string() })
        );
        return;
    }
    let report = hkask_services_verification::VerificationService::verify(principle.as_deref());
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
                "  {status_icon} {} {}: {}",
                result.id, result.name, result.status
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
            "  Principle summary: {principle_pass} pass, {principle_fail} fail, {principle_gap} gap"
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

fn build_consent() -> (hkask_services_context::AgentService, Arc<ConsentManager>) {
    let svc = super::helpers::build_agent_service();
    let cm = svc.governance().consent.clone();
    (svc, cm)
}

fn run_sovereignty_ops(action: SovereigntyAction) {
    match action {
        SovereigntyAction::Verify { .. } => unreachable!(),
        SovereigntyAction::Status => {
            let webid = super::helpers::resolve_user_webid();
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

            let categories = DataCategory::all_known();
            for cat in categories {
                match cm.has_consent(&webid.to_string(), cat) {
                    Ok(true) => println!("  • {}: GRANTED", cat.as_str()),
                    _ => println!("  • {}: DENIED", cat.as_str()),
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
            let store = &svc_ctx.storage().sovereignty.clone();
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
        SovereigntyAction::Grant { category, agent } => {
            let webid = if let Some(name) = agent {
                hkask_types::WebID::for_agent_name(&name)
            } else {
                super::helpers::resolve_user_webid()
            };
            let (_svc, cm) = build_consent();
            let cat = hkask_types::DataCategory::parse(&category);
            match cm.grant_consent(&webid.to_string(), &cat) {
                Ok(()) => {
                    println!("Consent granted for category: {category}");
                    println!("  WebID: {}", webid);
                    println!("  Data sharing is now enabled for this category.");
                    if cat.is_typically_sovereign() {
                        println!("  Note: Sovereign data still requires owner verification.");
                    }
                }
                Err(e) => eprintln!("Error granting consent: {e}"),
            }
        }
        SovereigntyAction::Revoke => {
            let webid = super::helpers::resolve_user_webid();
            let (_svc, cm) = build_consent();
            match cm.revoke_consent(&webid.to_string()) {
                Ok(()) => {
                    println!("Consent revoked for all categories.");
                    println!("  All data sharing is now disabled.");
                    println!("  Only public data is accessible.");
                }
                Err(e) => eprintln!("Error revoking consent: {e}"),
            }
        }
        SovereigntyAction::Check { category } => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            let cat = hkask_types::DataCategory::parse(&category);
            let (_svc, cm) = build_consent();
            let boundary = DataSovereigntyBoundary::hkask_default();

            let class = boundary.classify(&cat);
            let classification = class.label();
            let access_required = class.access_required();
            let has_consent = if classification == "PUBLIC" {
                true
            } else {
                cm.has_consent(&webid.to_string(), &cat).unwrap_or(false)
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
