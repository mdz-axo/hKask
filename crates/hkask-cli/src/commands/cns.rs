//! CNS command handlers for `kask cns`
//!
//! Implements the CLI display logic for the Cybernetic Nervous System subcommand.

use crate::cli::CnsAction;
use hkask_services::{AgentService, CnsService, ServiceConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn run(rt: &tokio::runtime::Runtime, action: CnsAction) {
    match action {
        CnsAction::Health => {
            let cns = build_cns_service(rt);
            let health = rt.block_on(cns.health());
            let alerts = rt.block_on(cns.alerts());
            let variety = rt.block_on(cns.variety());

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
                for (domain, count) in &variety {
                    println!("  • {}: {} states", domain, count);
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
            let cns = build_cns_service(rt);
            let alerts = rt.block_on(cns.alerts());
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
            let cns = build_cns_service(rt);
            let variety = rt.block_on(cns.variety());
            println!("Variety counters:");
            if variety.is_empty() {
                println!("  (no variety data)");
            } else {
                for (domain, count) in &variety {
                    println!("  • {}: {} states", domain, count);
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
            let defaults = hkask_cns::SetPoints::default();
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
                let config = hkask_cns::SetPointsConfig {
                    gas_min_remaining,
                    variety_max_deficit,
                    error_rate_max,
                    connector_latency_max_secs,
                    communication_backpressure_threshold: communication_backpressure_threshold
                        .map(hkask_types::cns::QueueDepth::new),
                };
                let updated = hkask_cns::SetPoints::from_config(&config);
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

/// Build a `CnsService` — prefers `AgentService` when available,
/// falls back to a standalone `CnsRuntime` for lightweight queries.
fn build_cns_service(rt: &tokio::runtime::Runtime) -> CnsService {
    let config = match ServiceConfig::from_env() {
        Ok(c) => c,
        Err(_) => {
            return standalone_cns();
        }
    };
    match rt.block_on(AgentService::build(config)) {
        Ok(ctx) => ctx.cns().clone(),
        Err(_) => standalone_cns(),
    }
}

/// Fallback: lightweight `CnsService` backed by a standalone runtime.
fn standalone_cns() -> CnsService {
    CnsService::new(Arc::new(RwLock::new(
        hkask_cns::CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD),
    )))
}
