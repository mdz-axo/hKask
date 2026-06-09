//! Per-turn processing for the REPL.
//!
//! Handles both ensemble and single-agent inference turns,
//! including gas governance, manifest cascade, HHH reframe,
//! tool-augmented followup, HHH gate evaluation, CNS updates,
//! and persona filtering.

use hkask_agents::HhhMode;
use hkask_agents::hhh_gate;
use hkask_types::ports::StructuredToolCall;

use super::ReplState;
use super::cns_display;
use super::energy;
use super::hhh_loop;
use super::tool_augmented;

/// Handle an ensemble (multi-agent) turn.
pub(super) fn ensemble_turn(
    session: &str,
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    acp_secret: &[u8],
) {
    match rt.block_on(crate::commands::ensemble_improv_turn(
        &state.service_context,
        session,
        input,
        Some(state.inference_port.clone()),
    )) {
        Ok(turn) => {
            if turn.responses.is_empty() {
                println!("  \x1b[2m(no agents chose to speak)\x1b[0m");
            } else {
                for response in &turn.responses {
                    // Tool-augmented processing: same function
                    // as single-agent REPL.
                    let agent_name = response.agent_webid.to_string();
                    let processed = rt.block_on(tool_augmented::process_response(
                        &response.content,
                        &agent_name,
                        &state.governed_tool,
                        &state.agent_webid,
                        acp_secret,
                        None, // ensemble responses don't carry structured tool calls yet
                    ));
                    if !processed.had_tool_calls {
                        println!(
                            "\x1b[1m{}\x1b[0m (conf. {:.2}): {}\n",
                            response.agent_webid, response.confidence, response.content
                        );
                    }
                    state.session_history.record(&agent_name, &processed.text);
                }
                if let Some(ref synthesis) = turn.curator_synthesis {
                    let processed = rt.block_on(tool_augmented::process_response(
                        synthesis,
                        "Curator",
                        &state.governed_tool,
                        &state.agent_webid,
                        acp_secret,
                        None,
                    ));
                    if !processed.had_tool_calls {
                        println!("\x1b[1;33mCurator:\x1b[0m {}\n", synthesis);
                    }
                    state.session_history.record("Curator", &processed.text);
                }
            }
            for j in &turn.judgments {
                if !j.should_speak {
                    println!(
                        "  \x1b[2m{}: silent ({:.2} — {})\x1b[0m",
                        j.agent_name, j.confidence, j.reason
                    );
                }
            }
        }
        Err(e) => println!("  \x1b[31mEnsemble error:\x1b[0m {}", e),
    }
}

/// Handle a single-agent inference turn.
///
/// Returns `false` if the turn should be skipped (energy budget exhausted).
pub(super) fn single_agent_turn(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    acp_secret: &[u8],
) -> bool {
    // Hold-settle pattern via EnergyGuard: reserve heuristic estimate
    // before inference, settle with actual token cost after.
    let Some(mut gas_guard) = energy::EnergyGuard::try_reserve(
        &state.service_context.cybernetics_loop,
        &state.inference_loop,
        &state.agent_webid,
        rt,
        500,
    ) else {
        println!(
            "  \x1b[31m\u{2717} Gas budget exhausted (hard limit) \u{2014} turn blocked by cybernetic regulator\x1b[0m"
        );
        println!("  \x1b[2mUse /status to see budget details, or wait for replenishment.\x1b[0m");
        return false;
    };

    // Execute manifest cascade if the agent has a process manifest.
    let mut manifest_context: Option<String> = None;
    if let (Some(executor), Some(manifest)) = (&state.manifest_executor, &state.process_manifest) {
        let mut initial_ctx = std::collections::HashMap::new();
        initial_ctx.insert(
            "user_input".to_string(),
            serde_json::Value::String(input.to_string()),
        );
        initial_ctx.insert(
            "agent".to_string(),
            serde_json::Value::String(state.current_agent.clone()),
        );

        match rt.block_on(executor.execute_manifest(manifest, initial_ctx)) {
            Ok(ctx) => {
                let mut context_parts: Vec<String> = Vec::new();
                for (key, value) in &ctx {
                    if key.starts_with("step_") {
                        context_parts.push(format!("{}: {}", key, value));
                    }
                }
                if !context_parts.is_empty() {
                    manifest_context = Some(context_parts.join("\n"));
                }
                tracing::info!(
                    target: "cns.spec.executor",
                    steps_completed = ctx.len(),
                    "Manifest cascade completed"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.spec.executor",
                    error = %e,
                    "Manifest cascade failed — continuing without manifest enrichment"
                );
            }
        }
    }

    // When HHH mode is active, wrap the input in a reframe template
    // and append HHH directives to the system prompt.
    let base_input: String = match &manifest_context {
        Some(ctx) => format!(
            "[Manifest Context]\n{}\n[/Manifest Context]\n\n{}",
            ctx, input
        ),
        None => input.to_string(),
    };
    let (effective_input, hhh_suffix): (String, Option<String>) =
        if state.hhh_mode == HhhMode::Active {
            let reframed = hhh_gate::hhh_reframe(&base_input);
            let suffix = hhh_gate::hhh_augment_system_prompt("");
            (reframed, Some(suffix))
        } else {
            (base_input, None)
        };

    // Stream the response incrementally
    print!("{}: ", state.current_agent);
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let chat_response = rt.block_on(crate::commands::chat_with_agent_streaming(
        &effective_input,
        Some(&state.current_agent),
        Some(&state.current_model),
        Some(state.inference_port.clone()),
        state.resolved_secrets.as_ref(),
        Some(state.episodic_storage.clone()),
        Some(state.semantic_storage.clone()),
        Some(state.agent_webid),
        hhh_suffix.as_deref(),
        Some(state.tool_prompt_section.as_str()),
    ));

    // Settle gas with actual token cost
    let actual_cost = chat_response
        .usage
        .as_ref()
        .map(|u| u.gas_cost())
        .unwrap_or(gas_guard.heuristic());
    gas_guard.settle(actual_cost);

    let response = chat_response.text;

    let structured_calls: Vec<StructuredToolCall> = if chat_response.finish_reason == "tool_calls" {
        chat_response.tool_calls
    } else {
        vec![]
    };
    let processed = rt.block_on(tool_augmented::process_response(
        &response,
        &state.current_agent,
        &state.governed_tool,
        &state.agent_webid,
        acp_secret,
        Some(&structured_calls),
    ));
    let mut final_response = processed.text;

    // If tool calls were found, feed the results back to the model.
    if processed.had_tool_calls && !processed.tool_results_formatted.is_empty() {
        let followup_prompt = format!(
            "{}\n\nThe following tool calls were executed:\n\n{}\n\nBased on these results, provide your response.",
            final_response.trim(),
            processed.tool_results_formatted
        );

        if let Some(mut followup_guard) = energy::EnergyGuard::try_reserve(
            &state.service_context.cybernetics_loop,
            &state.inference_loop,
            &state.agent_webid,
            rt,
            500,
        ) {
            let followup = rt.block_on(crate::commands::chat_with_agent(
                &followup_prompt,
                Some(&state.current_agent),
                Some(&state.current_model),
                Some(state.inference_port.clone()),
                state.resolved_secrets.as_ref(),
                Some(state.episodic_storage.clone()),
                Some(state.semantic_storage.clone()),
                Some(state.agent_webid),
                None,
                Some(state.tool_prompt_section.as_str()),
            ));

            let followup_cost = followup
                .usage
                .as_ref()
                .map(|u| u.gas_cost())
                .unwrap_or(followup_guard.heuristic());
            followup_guard.settle(followup_cost);

            if let Some(ref usage) = followup.usage {
                println!(
                    "  \x1b[2mFollowup: {} tokens ({} prompt + {} completion)\x1b[0m",
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                );
            }

            let followup_structured: Vec<StructuredToolCall> =
                if followup.finish_reason == "tool_calls" {
                    followup.tool_calls
                } else {
                    vec![]
                };
            let followup_processed = rt.block_on(tool_augmented::process_response(
                &followup.text,
                &state.current_agent,
                &state.governed_tool,
                &state.agent_webid,
                acp_secret,
                Some(&followup_structured),
            ));
            final_response = followup_processed.text;
        } else {
            println!("  \x1b[33m\u{26a0} Gas budget insufficient for followup inference\x1b[0m");
        }

        // HHH gate evaluation after tool-augmented followup
        if state.hhh_mode == HhhMode::Active {
            if let Some(ref gate_port) = state.gate_inference_port {
                hhh_loop::evaluate_hhh(input, &mut final_response, gate_port, state, rt);
            } else {
                println!("  \x1b[33m\u{26a0} HHH mode active but gate model unavailable\x1b[0m");
            }
        }
    }

    // Show token usage
    if let Some(ref usage) = chat_response.usage {
        println!(
            "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
            usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
        );
    }

    // Check energy budget and warn if low
    let gas_remaining = state.inference_loop.gas_remaining();
    let gas_cap = state.inference_loop.gas_cap();
    if gas_cap > 0 && gas_remaining > 0 && (gas_remaining as f64 / gas_cap as f64) < 0.2 {
        println!(
            "  \x1b[33m\u{26a0} Gas budget low: {}/{} ({:.0}%)\x1b[0m",
            gas_remaining,
            gas_cap,
            (gas_remaining as f64 / gas_cap as f64) * 100.0
        );
    } else if gas_cap > 0 && gas_remaining == 0 {
        println!(
            "  \x1b[31m\u{2717} Gas budget exhausted \u{2014} some operations may be throttled\x1b[0m"
        );
    }

    cns_display::update_cns_and_display(input, state, rt);

    // Persona filter (Stage 4 of alignment pipeline)
    final_response =
        hhh_gate::apply_persona_filter(&final_response, state.persona_constraints.as_ref());

    // Streaming already printed the agent label and text deltas.
    // Record the (possibly persona-filtered) response in session history.
    state
        .session_history
        .record(&state.current_agent, &final_response);

    true
}
