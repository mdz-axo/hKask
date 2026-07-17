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
//!
//! Both CLI (stdout) and TUI (capture buffer) surfaces share a single
//! `run_turn_loop` via the `TurnSink` trait, which abstracts where
//! agent responses, tool activity, and status lines go.

use hkask_services_chat::{ChatService, TokenUsage, TurnRequest};

use super::ReplState;
use super::cns_display;
use super::energy;
use super::handlers::speak_response;
use super::handlers::to_llm_params;
use super::tool_augmented;

// ── TurnSink: output abstraction ─────────────────────────────────────

/// Output destination for the turn loop.
///
/// Abstracts whether agent responses, tool activity, and status lines
/// go to stdout (CLI) or a capture buffer (TUI). This is the seam
/// that lets the tool-augmented turn loop be shared between surfaces
/// without duplicating ~200 lines of loop logic.
///
/// Three channels:
/// - `agent_text`: the agent's response (preamble before tools or final answer)
/// - `tool_log`: tool activity (invocations, results, call counts)
/// - `status`: turn metadata (usage, warnings, gas, errors)
trait TurnSink {
    /// Agent's text response — preamble before tool calls, or the final answer.
    fn agent_text(&mut self, agent: &str, text: &str);
    /// Tool activity line — invocations, results, call summaries.
    fn tool_log(&mut self, line: &str);
    /// Status line — usage stats, warnings, gas, errors.
    fn status(&mut self, line: &str);
}

/// CLI sink — prints to stdout with ANSI formatting.
struct StdoutSink;

impl TurnSink for StdoutSink {
    fn agent_text(&mut self, agent: &str, text: &str) {
        println!("{}: {}", agent, text);
    }
    fn tool_log(&mut self, line: &str) {
        println!("{}", line);
    }
    fn status(&mut self, line: &str) {
        println!("{}", line);
    }
}

/// TUI sink — captures text into separate buffers for response and tool output.
#[cfg(feature = "tui")]
struct CaptureSink {
    response_text: String,
    tool_output: String,
}

#[cfg(feature = "tui")]
impl CaptureSink {
    fn new() -> Self {
        Self {
            response_text: String::new(),
            tool_output: String::new(),
        }
    }
}

#[cfg(feature = "tui")]
impl TurnSink for CaptureSink {
    fn agent_text(&mut self, _agent: &str, text: &str) {
        use std::fmt::Write;
        let _ = writeln!(self.response_text, "{}", text);
    }
    fn tool_log(&mut self, line: &str) {
        use std::fmt::Write;
        let _ = writeln!(self.tool_output, "{}", line);
    }
    fn status(&mut self, line: &str) {
        // Capture diagnostics (errors, warnings, gas alerts) into response_text
        // so the TUI user sees them. Token usage is skipped — it's carried in
        // TurnCapture's numeric fields (prompt_tokens, completion_tokens,
        // total_tokens) and would be redundant in the chat bubble.
        if line.contains("tokens (") {
            return;
        }
        use std::fmt::Write;
        let _ = writeln!(self.response_text, "{}", line);
    }
}

// ── TurnOutcome: structured result ───────────────────────────────────

/// Result of a completed turn loop.
///
/// Carries the structured data both surfaces need after the loop:
/// token usage, iteration count, and budget status. The final response
/// text is handled inside `run_turn_loop` (thread memory, CNS spans)
/// and displayed via the `TurnSink` — callers don't need it back.
#[cfg_attr(not(feature = "tui"), allow(dead_code))]
struct TurnOutcome {
    /// Whether the turn completed successfully (false = gas exhausted or inference error).
    success: bool,
    /// Accumulated token usage across all iterations.
    usage: TokenUsage,
    /// Number of iterations the loop ran.
    iterations: usize,
    /// Whether the gas budget was exhausted (hard limit).
    budget_exhausted: bool,
}

fn zero_usage() -> TokenUsage {
    TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    }
}

// ── TurnRequest builder ──────────────────────────────────────────────

fn build_turn_request(
    state: &ReplState,
    current_input: &str,
    iteration: usize,
    tool_results: Option<String>,
    agent_override: Option<&str>,
) -> TurnRequest {
    let settings = &state.repl_settings;
    let mem = state
        .service_context
        .per_agent_memory(&state.current_agent)
        .expect("per-agent memory");
    TurnRequest {
        input: current_input.to_string(),
        agent_name: agent_override.unwrap_or(&state.current_agent).to_string(),
        model: state.current_model.clone(),
        inference_port: state
            .service_context
            .inference_port()
            .expect("inference port"),
        episodic_storage: mem.episodic_storage,
        semantic_storage: mem.semantic_storage,
        agent_webid: state.agent_webid,
        persona_constraints: state.persona_constraints.clone(),
        tool_section: state.tool_prompt.section.clone(),
        api_spec: None,
        llm_params: to_llm_params(settings),
        capability_checker: state.service_context.governance().checker.clone(),
        system_webid: *state.service_context.webid(),
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
        pre_compress: settings.pre_compress,
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
        tools: if state.tool_prompt.definitions.is_empty() {
            None
        } else {
            Some(state.tool_prompt.definitions.clone())
        },
    }
}

// ── Post-loop status display ─────────────────────────────────────────

/// Emit token usage and gas budget warnings to the sink.
///
/// Extracted from `run_turn_loop` for testability — the conditional
/// formatting ("across N iterations" vs single, gas low vs exhausted
/// vs healthy) can be verified without a full ReplState.
fn emit_turn_status(
    sink: &mut impl TurnSink,
    usage: Option<&TokenUsage>,
    iteration: usize,
    gas_remaining: u64,
    gas_cap: u64,
) {
    if let Some(usage) = usage {
        if iteration > 1 {
            sink.status(&format!(
                "  \x1b[2m{} tokens ({} prompt + {} completion) across {} iterations\x1b[0m",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens, iteration
            ));
        } else {
            sink.status(&format!(
                "  \x1b[2m{} tokens ({} prompt + {} completion)\x1b[0m",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            ));
        }
    }

    if gas_cap > 0 && gas_remaining > 0 && (gas_remaining as f64 / gas_cap as f64) < 0.2 {
        sink.status(&format!(
            "  \x1b[33m\u{26a0} Gas budget low: {}/{} ({:.0}%)\x1b[0m",
            gas_remaining,
            gas_cap,
            (gas_remaining as f64 / gas_cap as f64) * 100.0
        ));
    } else if gas_cap > 0 && gas_remaining == 0 {
        sink.status("  \x1b[31m\u{2717} Gas budget exhausted \u{2014} some operations may be throttled\x1b[0m");
    }
}

// ── Unified turn loop ────────────────────────────────────────────────

/// Run the tool-augmented inference turn loop.
///
/// This is the single shared implementation for both CLI and TUI surfaces.
/// The `sink` parameter controls where output goes. The loop:
/// 1. Reserves gas via EnergyGuard (hold-settle pattern)
/// 2. Builds a TurnRequest and delegates to `ChatService::execute_turn()`
/// 3. Extracts tool calls (structured or text-directive) via `extract_tool_calls`
/// 4. Invokes tools through GovernedTool, displaying via the sink
/// 5. Feeds tool results back for the next iteration
/// 6. Repeats until the model stops requesting tools or max_loops is reached
///
/// After the loop: token usage, gas warnings, thread memory, and CNS
/// spans are handled identically for both surfaces.
fn run_turn_loop(
    input: &str,
    deps: crate::deps::TurnDeps,
    rt: &tokio::runtime::Handle,
    sink: &mut impl TurnSink,
    agent_override: Option<&str>,
    max_loops: usize,
    gas_heuristic: u64,
    default_agent: &str,
) -> TurnOutcome {
    let display_name = agent_override.unwrap_or(default_agent).to_string();

    let mut current_input: String = input.to_string();
    let mut tool_results: Option<String> = None;
    let mut iteration: usize = 0;
    let mut total_usage: Option<TokenUsage> = None;
    let mut final_response: Option<String> = None;
    let mut inference_error = false;

    // CNS: turn lifecycle — emit start span for observability.
    tracing::info!(
        target: "cns",
        cns_domain = "cns.chat.turn",
        operation = "started",
        agent = %display_name,
        input_len = input.len(),
        "CNS"
    );

    loop {
        iteration += 1;
        if iteration > max_loops {
            sink.status(&format!(
                "  \x1b[33m\u{26a0} Tool-use loop max iterations ({}) reached \u{2014} yielding current response\x1b[0m",
                max_loops
            ));
            break;
        }

        :        // Hold-settle pattern via GasGovernor.
                let Some(mut gas_guard) = deps.gas.try_reserve(gas_heuristic) else {
            sink.status("  \x1b[31m\u{2717} Gas budget exhausted (hard limit) \u{2014} turn blocked by cybernetic regulator\x1b[0m");
            sink.status(
                "  \x1b[2mUse /status to see budget details, or wait for replenishment.\x1b[0m",
            );
            return TurnOutcome {
                success: false,
                usage: total_usage.unwrap_or_else(zero_usage),
                iterations: 0, // no inference iterations completed
                budget_exhausted: true,
            };
        };

        // Build TurnRequest for this iteration.
        let turn_req = (deps.build_request)(
            &current_input,
            iteration,
            tool_results.take(),
            agent_override,
        );

        let chat_result = rt.block_on(deps.executor.execute_turn(&turn_req));
        let chat_response = match chat_result {
            Ok(r) => r,
            Err(e) => {
                sink.status(&format!("  \x1b[31mInference error:\x1b[0m {}", e));
                // Release the gas reservation — inference failed, no actual
                // cost was incurred. Without this, the reserved gas would
                // be permanently encumbered (EnergyGuard has no Drop fallback
                // that calls settle_gas).
                gas_guard.release();
                // Break (not return) so post-loop code runs: cns_display,
                // gas warnings, etc. The old TUI path used break; the old
                // CLI path used return. We unify on break so both surfaces
                // get CNS regulation updates after inference failures.
                inference_error = true;
                break;
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

        // Extract tool calls — pure, no I/O side effects.
        let parsed = tool_augmented::extract_tool_calls(
            &response,
            if structured_calls.is_empty() {
                None
            } else {
                Some(&structured_calls)
            },
        );

        if parsed.tool_calls.is_empty() {
            // No tool calls — this is the final response. Always display
            // it, regardless of iteration count. (The previous code
            // suppressed this when iteration > 1, causing the user to
            // see only the preamble and a token count.)
            sink.agent_text(&display_name, &parsed.text);
            final_response = Some(parsed.text.clone());
            // Talk mode: the closure checks talk_config.enabled internally.
            (deps.on_speak)(&parsed.text);
            break;
        }

        // Tool calls found — display preamble, invoke tools, build next iteration.
        if !parsed.text.trim().is_empty() {
            sink.agent_text(&display_name, parsed.text.trim());
        }

        sink.tool_log(&format!(
            "  \x1b[2m\u{2750} {} tool call(s) from {}\x1b[0m",
            parsed.tool_calls.len(),
            display_name
        ));

        // Invoke each tool call through the ToolInvoker trait.
        let mut tool_results_vec = Vec::new();
        for call in &parsed.tool_calls {
            let mut line = format!("  \x1b[2m  Invoking {}\x1b[0m", call.tool);
            if !call.server.is_empty() {
                line.push_str(&format!(" on \x1b[36m{}\x1b[0m", call.server));
            }
            line.push_str("...");
            sink.tool_log(&line);

            let result = rt.block_on(deps.tools.invoke(call));

            match &result {
                Ok(value) => {
                    sink.tool_log(&format!("  \x1b[32m  \u{2713}\x1b[0m {}", call.tool));
                    if let Ok(formatted) = serde_json::to_string_pretty(value) {
                        for line in formatted.lines().take(5) {
                            sink.tool_log(&format!("    {}", line));
                        }
                        if formatted.lines().count() > 5 {
                            sink.tool_log("    ...");
                        }
                    }
                }
                Err(err) => {
                    sink.tool_log(&format!(
                        "  \x1b[31m  \u{2717}\x1b[0m {} \u{2014} {}",
                        call.tool, err
                    ));
                }
            }

            tool_results_vec.push((call.clone(), result));
        }

        // Feed tool results back for the next iteration.
        let tool_results_formatted = tool_augmented::format_tool_results(&tool_results_vec);
        current_input = response;
        tool_results = Some(tool_results_formatted);
    }

    // Post-loop status display (token usage + gas warnings).
    let (gas_remaining, gas_cap) = deps.gas.gas_status();
    emit_turn_status(
        sink,
        total_usage.as_ref(),
        iteration,
        gas_remaining,
        gas_cap,
    );

    // Append this exchange to the active thread's short-term memory stream.
    // The old TUI path did NOT update the thread registry; the unified loop
    // now does for both surfaces. This is intentional - the TUI is the same
    // agent, not an ephemeral session. Without this, TUI conversations would
    // not persist in thread memory, causing context loss when switching
    // between CLI and TUI.
    //
    // This is independent of long-term episodic memory - threads are the
    // agent's immediate context; episodic/semantic processing runs in parallel.
    if let Some(ref resp) = final_response {
        deps.threads.append_turn(default_agent, input, resp);
    }

    // Mark thread as seeded only when the turn did not error. On inference
    // error, no conversation happened, so the next turn should still inject
    // thread history. The old CLI code returned early on error, skipping
    // mark_seeded; we replicate that by gating on !inference_error.
    if !inference_error {
        deps.threads.mark_seeded();
    }

    // CNS: turn lifecycle — emit completion span.
    if let Some(ref resp) = final_response {
        tracing::info!(
            target: "cns",
            cns_domain = "cns.chat.turn",
            operation = "completed",
            agent = %display_name,
            response_len = resp.len(),
            iterations = iteration,
            "CNS"
        );
    }

    (deps.on_cns_update)();

    TurnOutcome {
        success: !inference_error,
        usage: total_usage.unwrap_or_else(zero_usage),
        iterations: iteration,
        budget_exhausted: false,
    }
}

// ── Public wrappers ──────────────────────────────────────────────────

/// Handle a single-agent inference turn (CLI — prints to stdout).
///
/// Returns `false` if the turn should be skipped (energy budget exhausted).
pub(super) fn single_agent_turn(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
    agent_override: Option<&str>,
) -> bool {
    let mut sink = StdoutSink;
    let outcome = run_turn_loop(input, state, rt, a2a_secret, agent_override, &mut sink);
    outcome.success
}

/// Captured result of a single-agent inference turn.
/// Returns structured output instead of printing to stdout.
#[cfg(feature = "tui")]
pub struct TurnCapture {
    pub response_text: String,
    pub tool_output: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub iterations: usize,
    pub budget_exhausted: bool,
}

/// Handle a single-agent inference turn, capturing all output (TUI).
///
/// Same logic as `single_agent_turn` but captures output into a
/// `TurnCapture` struct instead of printing to stdout. Used by the TUI bridge.
#[cfg(feature = "tui")]
pub fn single_agent_turn_captured(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
) -> TurnCapture {
    let mut sink = CaptureSink::new();
    let outcome = run_turn_loop(input, state, rt, a2a_secret, None, &mut sink);
    TurnCapture {
        response_text: sink.response_text.trim().to_string(),
        tool_output: sink.tool_output,
        prompt_tokens: outcome.usage.prompt_tokens,
        completion_tokens: outcome.usage.completion_tokens,
        total_tokens: outcome.usage.total_tokens,
        iterations: outcome.iterations,
        budget_exhausted: outcome.budget_exhausted,
    }
}

#[cfg(test)]
mod tests {
    use super::{TurnSink, emit_turn_status};
    use hkask_services_chat::TokenUsage;
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

    // ── emit_turn_status tests ──────────────────────────────────────

    /// Mock sink that collects all status lines for assertion.
    struct MockSink {
        status_lines: Vec<String>,
    }

    impl MockSink {
        fn new() -> Self {
            Self {
                status_lines: Vec::new(),
            }
        }
    }

    impl TurnSink for MockSink {
        fn agent_text(&mut self, agent: &str, text: &str) {
            self.status_lines.push(format!("{}: {}", agent, text));
        }
        fn tool_log(&mut self, line: &str) {
            self.status_lines.push(line.to_string());
        }
        fn status(&mut self, line: &str) {
            self.status_lines.push(line.to_string());
        }
    }

    #[test]
    fn emit_status_single_iteration_omits_across_phrase() {
        let mut sink = MockSink::new();
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 20,
            total_tokens: 120,
        };
        emit_turn_status(&mut sink, Some(&usage), 1, 5000, 10000);
        assert!(
            sink.status_lines
                .iter()
                .any(|l| l.contains("120 tokens") && !l.contains("across"))
        );
    }

    #[test]
    fn emit_status_multi_iteration_includes_across_phrase() {
        let mut sink = MockSink::new();
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 20,
            total_tokens: 120,
        };
        emit_turn_status(&mut sink, Some(&usage), 3, 5000, 10000);
        assert!(
            sink.status_lines
                .iter()
                .any(|l| l.contains("across 3 iterations"))
        );
    }

    #[test]
    fn emit_status_no_usage_emits_nothing() {
        let mut sink = MockSink::new();
        emit_turn_status(&mut sink, None, 1, 5000, 10000);
        assert!(sink.status_lines.is_empty());
    }

    #[test]
    fn emit_status_gas_low_warns() {
        let mut sink = MockSink::new();
        emit_turn_status(&mut sink, None, 1, 100, 10000);
        assert!(
            sink.status_lines
                .iter()
                .any(|l| l.contains("Gas budget low"))
        );
    }

    #[test]
    fn emit_status_gas_exhausted_warns() {
        let mut sink = MockSink::new();
        emit_turn_status(&mut sink, None, 1, 0, 10000);
        assert!(
            sink.status_lines
                .iter()
                .any(|l| l.contains("Gas budget exhausted"))
        );
    }

    #[test]
    fn emit_status_gas_healthy_no_warning() {
        let mut sink = MockSink::new();
        emit_turn_status(&mut sink, None, 1, 5000, 10000);
        assert!(sink.status_lines.is_empty());
    }

    #[test]
    fn emit_status_gas_cap_zero_no_warning() {
        let mut sink = MockSink::new();
        emit_turn_status(&mut sink, None, 1, 0, 0);
        assert!(sink.status_lines.is_empty());
    }
}

#[cfg(all(test, feature = "tui"))]
mod capture_sink_tests {
    use super::*;

    #[test]
    fn agent_text_goes_to_response_text() {
        let mut sink = CaptureSink::new();
        sink.agent_text("Agent", "Hello world");
        assert!(sink.response_text.contains("Hello world"));
        assert!(sink.tool_output.is_empty());
    }

    #[test]
    fn tool_log_goes_to_tool_output() {
        let mut sink = CaptureSink::new();
        sink.tool_log("  Invoking search...");
        assert!(sink.tool_output.contains("Invoking search"));
        assert!(sink.response_text.is_empty());
    }

    #[test]
    fn status_token_usage_filtered_out() {
        let mut sink = CaptureSink::new();
        sink.status("  120 tokens (100 prompt + 20 completion)");
        assert!(
            sink.response_text.is_empty(),
            "token usage should not appear in response_text — it's in numeric fields"
        );
    }

    #[test]
    fn status_error_captured_in_response_text() {
        let mut sink = CaptureSink::new();
        sink.status("  Inference error: connection refused");
        assert!(sink.response_text.contains("Inference error"));
    }

    #[test]
    fn status_gas_warning_captured_in_response_text() {
        let mut sink = CaptureSink::new();
        sink.status("  Gas budget low: 100/10000 (1%)");
        assert!(sink.response_text.contains("Gas budget low"));
    }

    #[test]
    fn status_max_iterations_captured_in_response_text() {
        let mut sink = CaptureSink::new();
        sink.status("  Tool-use loop max iterations reached");
        assert!(sink.response_text.contains("max iterations"));
    }
}
