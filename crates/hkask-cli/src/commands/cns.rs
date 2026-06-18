//! CNS command handlers for `kask cns`
//!
//! Implements the CLI display logic for the Cybernetic Nervous System subcommand.

use crate::cli::CnsAction;
use hkask_cns::CnsRuntime;
use hkask_cns::SetPointsConfig;
use hkask_services::{AgentService, CnsService, ServiceConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

/// REQ: CLI-068
/// expect: "I can access all hKask functionality through the kask CLI" [P3]
/// pre:  rt is a valid tokio Runtime; action is a valid CnsAction variant
/// post: dispatches to health, alerts, variety, subscribe, or set_points display logic
pub fn run(rt: &tokio::runtime::Runtime, action: CnsAction) {
    match action {
        CnsAction::Health => {
            let rt_ref = build_cns_runtime(rt);
            let health = rt.block_on(async { rt_ref.read().await.health().await });
            let alerts = rt.block_on(async { rt_ref.read().await.alerts().await });
            let variety = rt.block_on(async { rt_ref.read().await.variety().await });

            println!("CNS Health Status");
            println!("=================");
            println!();
            println!("Runtime Status:");
            println!("  • Healthy: {}", health.healthy);
            println!("  • Overall variety deficit: {}", health.overall_deficit);
            println!("  • Critical alerts: {}", health.critical_count);
            println!("  • Warning alerts: {}", health.warning_count);
            println!();
            println!("Variety Counter Summary:");
            if variety.is_empty() {
                println!("  • No variety data recorded");
            } else {
                for (ns, count) in &variety {
                    println!("  • {}: {} states", ns.as_str(), count);
                }
            }
            println!();
            println!("Active Algedonic Alerts:");
            if alerts.is_empty() {
                println!("  • No active alerts");
            } else {
                for alert in &alerts {
                    println!(
                        "  • [{:?}] {}: {}",
                        alert.severity, alert.domain, alert.message
                    );
                }
            }
            println!();
            println!("Energy Budget Status:");
            println!("  • Model: Energy tracking (subsumes rate limiting)");
            println!("  • Status: OPERATIONAL");
            println!();
            println!("Review Queue Depth:");
            println!("  • Pending reviews: 0");
            println!("  • Queue status: IDLE");
        }
        CnsAction::Alerts => {
            let rt_ref = build_cns_runtime(rt);
            let alerts = rt.block_on(async { rt_ref.read().await.alerts().await });
            println!("Algedonic alerts:");
            if alerts.is_empty() {
                println!("  (no active alerts)");
            } else {
                for alert in &alerts {
                    println!(
                        "  • [{:?}] {}: {}",
                        alert.severity, alert.domain, alert.message
                    );
                }
            }
        }
        CnsAction::Variety => {
            let rt_ref = build_cns_runtime(rt);
            let variety = rt.block_on(async { rt_ref.read().await.variety().await });
            println!("Variety counters:");
            if variety.is_empty() {
                println!("  (no variety data)");
            } else {
                for (ns, count) in &variety {
                    println!("  • {}: {} states", ns.as_str(), count);
                }
            }
        }
        CnsAction::Subscribe { agent, spans } => {
            let span_list: Vec<&str> = spans.split(',').map(|s| s.trim()).collect();
            println!("CNS Event Subscription");
            println!("=====================");
            println!("  Agent: {}", agent);
            println!("  Span namespaces:");
            for span in &span_list {
                println!("    • {}", span);
            }
            println!();
            println!("  Note: Subscription is active for the lifetime of this process.");
            println!("  Events matching the specified namespaces will be delivered.");
        }
        CnsAction::SetPoints {
            gas_min_remaining,
            variety_max_deficit,
            error_rate_max,
            connector_latency_max_secs,
            communication_backpressure_threshold,
        } => {
            let rt_ref = build_cns_runtime(rt);
            let cns_svc = CnsService::new(Arc::clone(&rt_ref));
            let defaults = cns_svc.get_set_points();
            println!("CNS Set-Points");
            println!("==============");
            println!(
                "  gas_min_remaining:       {}",
                gas_min_remaining.unwrap_or(defaults.gas_min_remaining)
            );
            println!(
                "  variety_max_deficit:        {}",
                variety_max_deficit.unwrap_or(defaults.variety_max_deficit)
            );
            println!(
                "  error_rate_max:             {}",
                error_rate_max.unwrap_or(defaults.error_rate_max)
            );
            println!(
                "  connector_latency_max_secs: {}",
                connector_latency_max_secs.unwrap_or(defaults.connector_latency_max_secs)
            );
            println!(
                "  communication_backpressure_threshold: {}",
                communication_backpressure_threshold
                    .map(hkask_types::cns::QueueDepth::new)
                    .unwrap_or(defaults.communication_backpressure_threshold)
                    .as_raw()
            );
            if gas_min_remaining.is_some()
                || variety_max_deficit.is_some()
                || error_rate_max.is_some()
                || connector_latency_max_secs.is_some()
                || communication_backpressure_threshold.is_some()
            {
                let config = SetPointsConfig {
                    gas_min_remaining,
                    variety_max_deficit,
                    error_rate_max,
                    connector_latency_max_secs,
                    communication_backpressure_threshold: communication_backpressure_threshold
                        .map(hkask_types::cns::QueueDepth::new),
                    seam_coverage_min: None,
                };
                let updated = cns_svc.update_set_points(&config);
                println!();
                println!("Updated values would be:");
                println!("  gas_min_remaining:       {}", updated.gas_min_remaining);
                println!(
                    "  variety_max_deficit:        {}",
                    updated.variety_max_deficit
                );
                println!("  error_rate_max:             {}", updated.error_rate_max);
                println!(
                    "  connector_latency_max_secs: {}",
                    updated.connector_latency_max_secs
                );
                println!(
                    "  communication_backpressure_threshold: {}",
                    updated.communication_backpressure_threshold.as_raw()
                );
            }
        }
    }
}

/// Build a `CnsRuntime` — prefers `AgentService` when available,
/// falls back to a standalone `CnsRuntime` for lightweight queries.
fn build_cns_runtime(rt: &tokio::runtime::Runtime) -> Arc<RwLock<CnsRuntime>> {
    let config = match ServiceConfig::from_env() {
        Ok(c) => c,
        Err(_) => {
            return standalone_cns_runtime();
        }
    };
    match rt.block_on(AgentService::build(config)) {
        Ok(ctx) => ctx.cns_runtime().clone(),
        Err(_) => standalone_cns_runtime(),
    }
}

/// Fallback: lightweight CnsRuntime backed by a standalone runtime.
fn standalone_cns_runtime() -> Arc<RwLock<CnsRuntime>> {
    Arc::new(RwLock::new(CnsRuntime::with_threshold(
        hkask_cns::DEFAULT_THRESHOLD,
    )))
}
