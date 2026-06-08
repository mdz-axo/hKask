//! HHH gate evaluation loop for the REPL.
//!
//! When HHH mode is active, the final response is evaluated through a
//! gate model. If it fails, correction prompts loop until the response
//! passes or max iterations are reached.

use std::sync::Arc;

use hkask_agents::hhh_gate;
use hkask_types::ports::InferencePort;

use super::ReplState;
use super::gas;

/// Evaluate a response through the HHH gate model, looping with
/// corrections until it passes or max iterations are reached.
///
/// Modifies `final_response` in place if correction is needed.
/// The caller must ensure `state.hhh_mode == HhhMode::Active` and
/// `gate_port` is available.
pub(super) fn evaluate_hhh(
    input: &str,
    final_response: &mut String,
    gate_port: &Arc<dyn InferencePort>,
    state: &ReplState,
    rt: &tokio::runtime::Handle,
) {
    println!("  \x1b[2m[HHH] Evaluating response for HHH compliance...\x1b[0m");

    let mut hhh_iteration: u32 = 0;
    let max_iterations = state.hhh_config.max_iterations;
    let mut current_response = final_response.clone();

    loop {
        let Some(mut gate_guard) = gas::GasGuard::try_reserve(
            &state.service_context.cybernetics_loop,
            &state.inference_loop,
            &state.agent_webid,
            rt,
            500,
        ) else {
            println!("  \x1b[33m\u{26a0} HHH gate skipped: gas budget exhausted\x1b[0m");
            tracing::warn!(
                target: "cns.hhh.gas_exhausted",
                "HHH gate evaluation skipped — gas budget exhausted"
            );
            break;
        };

        // Evaluate through the gate
        let evaluation = rt.block_on(hhh_gate::hhh_evaluate(input, &current_response, gate_port));

        // Settle gate gas (heuristic == actual for gate evaluations)
        gate_guard.settle(gate_guard.heuristic());

        if evaluation.overall_pass {
            println!(
                "  \x1b[32m[HHH] \u{2713} Passed (iteration {})\x1b[0m",
                hhh_iteration + 1
            );
            *final_response = current_response;
            break;
        }

        if hhh_iteration >= max_iterations {
            // Max iterations reached — deliver with uncertainty marker
            *final_response = format!(
                "{}\n\n\u{26a0}\u{fe0f} This response may not fully meet HHH standards.",
                current_response
            );
            println!(
                "  \x1b[33m[HHH] Max iterations reached, delivering with uncertainty marker\x1b[0m"
            );
            tracing::warn!(
                target: "cns.hhh.gate",
                iterations = hhh_iteration,
                "HHH gate exhausted — delivering with uncertainty marker"
            );
            break;
        }

        // Gate failed — print diagnostic and prepare correction
        let failures = evaluation.failures.join(", ");
        println!("  \x1b[31m[HHH] \u{2717} Failed: {}\x1b[0m", failures);
        println!(
            "  \x1b[33m[HHH] Correcting (iteration {})...\x1b[0m",
            hhh_iteration + 2
        );

        let correction_input =
            hhh_gate::hhh_correction_prompt(input, &current_response, &evaluation);

        let Some(mut correction_guard) = gas::GasGuard::try_reserve(
            &state.service_context.cybernetics_loop,
            &state.inference_loop,
            &state.agent_webid,
            rt,
            500,
        ) else {
            *final_response = format!(
                "{}\n\n\u{26a0}\u{fe0f} HHH correction skipped: gas budget exhausted",
                current_response
            );
            println!("  \x1b[33m\u{26a0} HHH correction skipped: gas budget exhausted\x1b[0m");
            tracing::warn!(
                target: "cns.hhh.gas_exhausted",
                "HHH correction skipped — gas budget exhausted"
            );
            break;
        };

        let correction_suffix = hhh_gate::hhh_augment_system_prompt("");
        let correction_response = rt.block_on(crate::commands::chat_with_agent(
            &correction_input,
            Some(&state.current_agent),
            Some(&state.current_model),
            Some(state.inference_port.clone()),
            state.resolved_secrets.as_ref(),
            Some(state.episodic_storage.clone()),
            Some(state.semantic_storage.clone()),
            Some(state.agent_webid),
            Some(&correction_suffix),
            Some(state.tool_prompt_section.as_str()),
        ));

        // Settle correction gas
        let correction_cost = correction_response
            .usage
            .as_ref()
            .map(|u| u.gas_cost())
            .unwrap_or(correction_guard.heuristic());
        correction_guard.settle(correction_cost);

        current_response = correction_response.text;
        hhh_iteration += 1;
    }
}
