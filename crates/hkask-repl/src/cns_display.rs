//! CNS algedonic alert display and loop system tick for the REPL.
//!
//! After each inference turn, the CNS is checked for critical alerts
//! and the LoopSystem is ticked. CNS variety counters are updated by
//! the service layer via CNS spans (cns.chat.request, cns.chat.response) —
//! the REPL only reads alerts, never mutates CNS state directly.
//!
//! # REQ: P4-cns-access — REPL only reads CNS alerts (read-only), never mutates CNS state
//! expect: "I can access all hKask functionality through the kask CLI"

use super::ReplState;

/// Check CNS algedonic alerts and tick the LoopSystem after each turn.
/// Only reads CNS state (alerts) and ticks loop system — no direct mutation.
/// CNS variety counters are updated by the service layer via CNS spans.
pub(super) fn update_cns_and_display(state: &ReplState, rt: &tokio::runtime::Handle) {
    // Check for CNS algedonic alerts (read-only observation)
    let cns_runtime = state.service_context.cns().runtime.clone();
    let alerts = rt.block_on(async { cns_runtime.read().await.critical_alerts().await });
    if !alerts.is_empty() {
        for alert in &alerts {
            println!(
                "  \x1b[31m\u{26a0} CNS ALERT: {} (deficit: {}/{})\x1b[0m",
                alert.message, alert.deficit, alert.threshold
            );
        }
    }

    // Tick the LoopSystem to run sense→compare→compute→act for
    // CyberneticsLoop and InferenceLoop. The CyberneticsLoop reads
    // CNS variety and energy budgets, producing regulatory actions
    // (Throttle, AdjustEnergyBudget, Escalate, Calibrate) visible
    // through tracing output (cns.cybernetics target).
    rt.block_on(async {
        state.service_context.cns().loops.tick().await;
    });
}
