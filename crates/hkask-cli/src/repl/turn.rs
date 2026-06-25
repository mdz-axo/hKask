//! Per-turn processing for the REPL.
//!
//! Handles single-agent inference turns,
//! including gas governance, tool-augmented followup,
//! CNS updates, and persona filtering.
//!
//! The per-turn pipeline delegates to `ChatService::execute_turn()` for
//! manifest cascade, history suffix, inference, and persona
//! filtering. The CLI layer handles gas governance, streaming display,
//! tool execution (via GovernedTool), and CNS updates.

use hkask_services::{ChatService, TurnRequest};

use super::ReplState;
use super::cns_display;
use super::energy;
use super::handlers::speak_response;
use super::handlers::to_llm_params;
use super::tool_augmented;

fn build_turn_request(
    state: &ReplState,
    current_input: &str,
    iteration: usize,
    tool_results: Option<String>,
    agent_override: Option<&str>,
) -> TurnRequest {
    let settings = &state.repl_settings;
    TurnRequest {
        input: current_input.to_string(),
        agent_name: agent_override.unwrap_or(&state.current_agent).to_string(),
        model: state.current_model.clone(),
        inference_port: state.inference_port.clone(),
        episodic_storage: state.episodic_storage.clone(),
        semantic_storage: state.semantic_storage.clone(),
        agent_webid: state.agent_webid,
        persona_constraints: state.persona_constraints.clone(),
        tool_section: state.tool_prompt_section.clone(),
        llm_params: to_llm_params(settings),
        capability_checker: state.service_context.capability_checker.clone(),
        system_webid: *state.service_context.identity().0,
        iteration,
        tool_results,
        auto_condense: settings.auto_condense,
        context_window: settings.model_meta.as_ref().map(|m| m.context_length),
        condenser_model: Some(
            state
                .current_model
                .strip_prefix("OM/")
                .unwrap_or(&state.current_model)
                .to_string(),
        ),
        condense_pressure_threshold: settings.condense_pressure_threshold,
        condense_saliency_window: settings.condense_saliency_window,
        // Thread history injection: only on cold starts (session restart or
        // thread switch). After the first turn, episodic recall provides
        // conversation context. This avoids redundant injection when the
        // conversation is already in episodic memory.
        thread_history: if state.thread_registry.seeded {
            None
        } else {
            state
                .thread_registry
                .thread_history(Some(settings.condense_saliency_window))
        },
        improv_mode: state.improv_mode.clone(),
        source: None,
        tools: if state.tool_definitions.is_empty() {
            None
        } else {
            Some(state.tool_definitions.clone())
        },
    }
}

/// Handle a single-agent inference turn.
///
/// Returns `false` if the turn should be skipped (energy budget exhausted).
///
/// The turn follows an agentic tool-use loop:
/// 1. Delegate manifest cascade, history suffix, inference,
///    and persona filtering to `ChatService::execute_turn()`.
/// 2. Execute any tool calls returned by the model via GovernedTool.
/// 3. Feed tool results back as input for the next iteration.
/// 4. Repeat until model stops requesting tools or tool_loop_limit reached.
pub(super) fn single_agent_turn(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
    agent_override: Option<&str>,
) -> bool {
    let settings = state.repl_settings.clone();
    let max_loops = settings.tool_loop_limit;

    let mut current_input: String = input.to_string();
    let mut tool_results: Option<String> = None;
    let mut iteration: usize = 0;
    let mut total_usage: Option<hkask_services::TokenUsage> = None;
    let mut final_response: Option<String> = None;

    // CNS: turn lifecycle — emit start span for observability.
    tracing::info!(
        target: "cns",
        cns_domain = "cns.chat.turn",
        operation = "started",
        agent = %agent_override.unwrap_or(&state.current_agent),
        input_len = input.len(),
        "CNS"
    );

    loop {
        iteration += 1;
        if iteration > max_loops {
            println!(
                "  \x1b[33m\u{26a0} Tool-use loop max iterations ({}) reached \u{2014} yielding current response\x1b[0m",
                max_loops
            );
            break;
        }

        // Hold-settle pattern via EnergyGuard.
        let Some(gas_guard) = energy::EnergyGuard::try_reserve(
            state.service_context.cybernetics_loop(),
            &state.inference_loop,
            &state.agent_webid,
            rt,
            settings.gas_heuristic,
        ) else {
            println!(
                "  \x1b[31m\u{2717} Gas budget exhausted (hard limit) \u{2014} turn blocked by cybernetic regulator\x1b[0m"
            );
            println!(
                "  \x1b[2mUse /status to see budget details, or wait for replenishment.\x1b[0m"
            );
            return false;
        };

        // Build TurnRequest for this iteration.
        let turn_req = build_turn_request(
            state,
            &current_input,
            iteration,
            tool_results.take(),
            agent_override,
        );

        let chat_result = rt.block_on(ChatService::execute_turn(
            &state.service_context,
            &turn_req,
            state.manifest_executor.as_ref(),
            state.process_manifest.as_ref(),
        ));
        let chat_response = match chat_result {
            Ok(r) => r,
            Err(e) => {
                println!("  \x1b[31mInference error:\x1b[0m {}", e);
                return false;
            }
        };

        // Accumulate usage.
        let usage = chat_response.usage;
        if let Some(ref mut total) = total_usage {
            total.prompt_tokens += usage.prompt_tokens;
            total.completion_tokens += usage.completion_tokens;
            total.total_tokens += usage.total_tokens;
        } else {
            total_usage = Some(usage);
        }

        // Settle gas.
        let actual_cost = total_usage
            .as_ref()
            .map(|u| u.gas_cost())
            .unwrap_or(gas_guard.heuristic());
        gas_guard.settle(actual_cost);

        let response = chat_response.text;
        let structured_calls = chat_response.structured_tool_calls;

        // Display the response on first iteration.
        let display_name = agent_override.unwrap_or(&state.current_agent);
        if iteration == 1 && !structured_calls.is_empty() {
            // Tool calls requested — display raw text before tool execution.
            if !response.is_empty() {
                println!("{}: {}", display_name, response);
            }
        }

        // Execute tool calls through GovernedTool.
        let processed = rt.block_on(tool_augmented::process_response(
            &response,
            display_name,
            &state.governed_tool,
            &state.agent_webid,
            a2a_secret,
            if structured_calls.is_empty() {
                None
            } else {
                Some(&structured_calls)
            },
        ));

        // If no tool calls, this is the final response.
        if !processed.had_tool_calls {
            if iteration == 1 {
                println!("{}: {}", display_name, processed.text);
            }
            final_response = Some(processed.text.clone());
            // Talk mode: summarize and speak the response aloud
            if state.talk_enabled {
                speak_response(&processed.text, state, rt);
            }
            break;
        }

        // Tool calls found — build the next iteration's input with results.
        current_input = response;
        tool_results = Some(processed.tool_results_formatted);
    }

    // Show token usage.
    if let Some(ref usage) = total_usage {
        if iteration > 1 {
            println!(
                "  \x1b[2m{} tokens ({} prompt + {} completion) across {} iterations\x1b[0m",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens, iteration
            );
        } else {
            println!(
                "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            );
        }
    }

    // Check energy budget and warn if low.
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

    // Append this exchange to the active thread's short-term memory stream.
    // This is independent of long-term episodic memory — threads are the
    // agent's immediate context; episodic/semantic processing runs in parallel.
    if let Some(ref resp) = final_response {
        state
            .thread_registry
            .append_turn(&state.current_agent, input, resp);
    }

    // Mark thread as seeded — subsequent turns won't re-inject thread
    // history; episodic recall handles conversation context from here.
    state.thread_registry.mark_seeded();

    // CNS: turn lifecycle — emit completion span.
    if let Some(ref resp) = final_response {
        tracing::info!(
            target: "cns",
            cns_domain = "cns.chat.turn",
            operation = "completed",
            agent = %agent_override.unwrap_or(&state.current_agent),
            response_len = resp.len(),
            iterations = iteration,
            "CNS"
        );
    }

    cns_display::update_cns_and_display(state, rt);

    true
}

/// Captured result of a single-agent inference turn.
/// Returns structured output instead of printing to stdout.
#[cfg(feature = "tui")]
pub(crate) struct TurnCapture {
    pub response_text: String,
    pub tool_output: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub iterations: usize,
    pub budget_exhausted: bool,
}

/// Handle a single-agent inference turn, capturing all output.
///
/// Same logic as single_agent_turn but returns structured data
/// instead of printing to stdout. Used by the TUI bridge.
#[cfg(feature = "tui")]
pub(crate) fn single_agent_turn_captured(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
) -> TurnCapture {
    let settings = state.repl_settings.clone();
    let max_loops = settings.tool_loop_limit;

    let mut current_input: String = input.to_string();
    let mut tool_results: Option<String> = None;
    let mut iteration: usize = 0;
    let mut total_usage: Option<hkask_services::TokenUsage> = None;
    let mut captured_text = String::new();
    let mut tool_text = String::new();

    loop {
        iteration += 1;
        if iteration > max_loops {
            use std::fmt::Write;
            let _ = writeln!(
                captured_text,
                "  \u{26a0} Tool-use loop max iterations ({}) reached \u{2014} yielding current response",
                max_loops
            );
            break;
        }

        let Some(gas_guard) = energy::EnergyGuard::try_reserve(
            state.service_context.cybernetics_loop(),
            &state.inference_loop,
            &state.agent_webid,
            rt,
            settings.gas_heuristic,
        ) else {
            return TurnCapture {
                response_text: String::new(),
                tool_output: String::new(),
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                iterations: 0,
                budget_exhausted: true,
            };
        };

        let turn_req =
            build_turn_request(state, &current_input, iteration, tool_results.take(), None);

        let chat_result = rt.block_on(ChatService::execute_turn(
            &state.service_context,
            &turn_req,
            state.manifest_executor.as_ref(),
            state.process_manifest.as_ref(),
        ));
        let chat_response = match chat_result {
            Ok(r) => r,
            Err(e) => {
                use std::fmt::Write;
                let _ = writeln!(captured_text, "  Inference error: {}", e);
                break;
            }
        };

        let usage = chat_response.usage;
        if let Some(ref mut total) = total_usage {
            total.prompt_tokens += usage.prompt_tokens;
            total.completion_tokens += usage.completion_tokens;
            total.total_tokens += usage.total_tokens;
        } else {
            total_usage = Some(usage);
        }

        let actual_cost = total_usage
            .as_ref()
            .map(|u| u.gas_cost())
            .unwrap_or(gas_guard.heuristic());
        gas_guard.settle(actual_cost);

        let response = chat_response.text;
        let structured_calls = chat_response.structured_tool_calls;

        let processed = rt.block_on(tool_augmented::process_response(
            &response,
            &state.current_agent,
            &state.governed_tool,
            &state.agent_webid,
            a2a_secret,
            if structured_calls.is_empty() {
                None
            } else {
                Some(&structured_calls)
            },
        ));

        if !processed.had_tool_calls {
            use std::fmt::Write;
            let _ = writeln!(captured_text, "{}", processed.text);
            if state.talk_enabled {
                speak_response(&processed.text, state, rt);
            }
            break;
        }

        use std::fmt::Write;
        let _ = writeln!(tool_text, "{}", processed.tool_results_formatted);
        current_input = response;
        tool_results = Some(processed.tool_results_formatted);
    }

    cns_display::update_cns_and_display(state, rt);

    let usage = total_usage.unwrap_or(hkask_services::TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    });

    TurnCapture {
        response_text: captured_text.trim().to_string(),
        tool_output: tool_text,
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
        iterations: iteration,
        budget_exhausted: false,
    }
}

#[cfg(test)]
mod tests {
    // skips when estimated tokens are below that threshold.
    // The threshold calculation in ChatService::execute_turn() is:
    //   threshold = (context_window as f64 * 0.875) as u32
    //   approx_tokens = approx_token_count(input_with_context) as u32
    //   if approx_tokens > threshold → trigger condensation

    #[test]
    fn compaction_triggers_above_87_5_percent() {
        let context_length: u32 = 4096;
        let threshold = (context_length as f64 * 0.875) as u64;
        // Simulate candidate text at 90% of window (above threshold)
        let candidate_len = (context_length as f64 * 0.90 * 4.0) as usize;
        let estimated_tokens = (candidate_len as u64) / 4;
        assert!(
            estimated_tokens > threshold,
            "estimated {} should exceed threshold {} at 90%",
            estimated_tokens,
            threshold
        );
    }

    #[test]
    fn compaction_skips_below_87_5_percent() {
        let context_length: u32 = 4096;
        let threshold = (context_length as f64 * 0.875) as u64;
        // Simulate candidate text at 80% of window (below threshold)
        let candidate_len = (context_length as f64 * 0.80 * 4.0) as usize;
        let estimated_tokens = (candidate_len as u64) / 4;
        assert!(
            estimated_tokens <= threshold,
            "estimated {} should be ≤ threshold {} at 80%",
            estimated_tokens,
            threshold
        );
    }

    #[test]
    fn compaction_threshold_matches_87_5_percent_formula() {
        // Verify the 87.5% threshold for common context window sizes
        let cases = [(2048, 1792), (4096, 3584), (8192, 7168), (32768, 28672)];
        for (window, expected_threshold) in &cases {
            let computed = (*window as f64 * 0.875) as u64;
            assert_eq!(
                computed, *expected_threshold,
                "window={}: expected threshold={}, got {}",
                window, expected_threshold, computed
            );
        }
    }
}
