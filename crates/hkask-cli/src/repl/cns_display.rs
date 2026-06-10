//! CNS variety sensing, algedonic alerts, and loop system tick for the REPL.
//!
//! After each inference turn, the CNS is updated with prompt variety data,
//! algedonic alerts are surfaced, and the LoopSystem is ticked.
//! Regulatory actions are visible via `tracing` output (cns.cybernetics target).

use super::ReplState;

/// Update CNS variety counters, check algedonic alerts, tick the LoopSystem,
/// and display regulatory actions from tracing output.
pub(super) fn update_cns_and_display(input: &str, state: &ReplState, rt: &tokio::runtime::Handle) {
    // CNS variety sensing: decompose the prompt and increment
    // variety counters for depth, structure, and topic domains.
    let analysis = hkask_agents::decompose_prompt(input);
    {
        let cns_guard = rt.block_on(state.service_context.cns_runtime().read());
        // Prompt depth bucket (shallow/medium/deep)
        rt.block_on(
            cns_guard.increment_variety("cns.inference.prompt_depth", analysis.depth_bucket),
        );
        // Prompt structure (question/imperative/declarative/conditional)
        if analysis.question_count > 0 {
            rt.block_on(cns_guard.increment_variety("cns.inference.prompt_structure", "question"));
        }
        if analysis.imperative_count > 0 {
            rt.block_on(
                cns_guard.increment_variety("cns.inference.prompt_structure", "imperative"),
            );
        }
        if analysis.sentence_count > analysis.question_count + analysis.imperative_count {
            rt.block_on(
                cns_guard.increment_variety("cns.inference.prompt_structure", "declarative"),
            );
        }
        if analysis.conditional_count > 0 {
            rt.block_on(
                cns_guard.increment_variety("cns.inference.prompt_structure", "conditional"),
            );
        }
        // Prompt topic domains (each unique keyword is a new variety state)
        for keyword in &analysis.topic_keywords {
            rt.block_on(cns_guard.increment_variety("cns.inference.prompt_domain", keyword));
        }
    }

    // Check for CNS algedonic alerts
    let alerts = rt.block_on(async {
        state
            .service_context
            .cns_runtime
            .read()
            .await
            .critical_alerts()
            .await
    });
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
    rt.block_on(state.service_context.loop_system().tick());
}
