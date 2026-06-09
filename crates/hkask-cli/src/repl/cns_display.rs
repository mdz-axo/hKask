//! CNS variety sensing, algedonic alerts, and dispatch drain for the REPL.
//!
//! After each inference turn, the CNS is updated with prompt variety data,
//! algedonic alerts are surfaced, the LoopSystem is ticked, and regulatory
//! actions from the dispatch queue are displayed.

use hkask_types::loops::LoopPayload;

use super::ReplState;

/// Update CNS variety counters, check algedonic alerts, tick the LoopSystem,
/// and drain regulatory actions for display.
pub(super) fn update_cns_and_display(input: &str, state: &ReplState, rt: &tokio::runtime::Handle) {
    // CNS variety sensing: decompose the prompt and increment
    // variety counters for depth, structure, and topic domains.
    let analysis = hkask_agents::decompose_prompt(input);
    {
        let cns_guard = rt.block_on(state.service_context.cns_runtime.read());
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
    // (Throttle, AdjustEnergyBudget, Escalate, Calibrate).
    rt.block_on(state.service_context.loop_system.tick());

    // Drain the MessageDispatch for regulatory actions produced
    // by the loop cycle. Surface Throttle, Calibrate, Escalate,
    // AdjustEnergyBudget, and CircuitBreak actions as REPL notices.
    loop {
        let msg = rt.block_on(state.service_context.dispatch.receive());
        match msg {
            Some(msg) => match &msg.payload {
                LoopPayload::CyberneticsRegulation {
                    regulation_type,
                    parameters,
                    ..
                } => match regulation_type.as_str() {
                    "throttle" => {
                        let reason = parameters
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        println!("  \x1b[33m\u{26a0} CNS: Throttle — {}\x1b[0m", reason);
                    }
                    "adjust_energy_budget" => {
                        let ratio = parameters
                            .get("remaining_ratio")
                            .and_then(|v| v.as_f64())
                            .map(|r| format!("{:.0}%", r * 100.0))
                            .unwrap_or_else(|| "?".to_string());
                        println!(
                            "  \x1b[33m\u{26a0} CNS: Gas budget adjusted — remaining {}\x1b[0m",
                            ratio
                        );
                    }
                    "circuit_break" => {
                        println!("  \x1b[31m\u{2717} CNS: Circuit breaker opened\x1b[0m");
                    }
                    "calibrate" => {
                        let reason = parameters
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        println!("  \x1b[36m\u{21bb} CNS: Calibrate — {}\x1b[0m", reason);
                    }
                    other => {
                        println!("  \x1b[2mCNS: {}\x1b[0m", other);
                    }
                },
                LoopPayload::AlgedonicAlert {
                    current, threshold, ..
                } => {
                    println!(
                        "  \x1b[31m\u{26a0} CNS: Algedonic escalation (deficit: {}/{})\x1b[0m",
                        current, threshold
                    );
                }
                _ => { /* other payload types — not displayed */ }
            },
            None => break,
        }
    }
}
