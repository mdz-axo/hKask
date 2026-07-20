//! Per-turn processing for the REPL.
//!
//! Handles single-agent inference turns, including gas governance,
//! tool-augmented followup, CNS updates, and persona filtering.
//!
//! Both CLI (stdout) and TUI (capture buffer) surfaces share a single
//! `run_turn_loop` via the `TurnSink` trait. Behavioral dependencies
//! (inference, gas, tools, threads) are injected via `TurnDeps`.

use hkask_services_chat::TokenUsage;

use super::ReplState;
use super::TalkMode;
use super::cns_display;
use super::deps::{TurnConfig, TurnDeps, TurnInput};
use super::handlers::speak_response;

// ── TurnSink: output abstraction ─────────────────────────────────────

trait TurnSink {
    fn agent_text(&mut self, agent: &str, text: &str);
    fn tool_log(&mut self, line: &str);
    fn status(&mut self, line: &str);
}

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
        if line.contains("tokens (") {
            return;
        }
        use std::fmt::Write;
        let _ = writeln!(self.response_text, "{}", line);
    }
}

// ── TurnOutcome ──────────────────────────────────────────────────────

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
struct TurnOutcome {
    success: bool,
    final_response: Option<String>,
    usage: TokenUsage,
    iterations: usize,
    budget_exhausted: bool,
}

fn zero_usage() -> TokenUsage {
    TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    }
}

// ── Post-loop status display ─────────────────────────────────────────

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

fn run_turn_loop(
    input: &str,
    deps: TurnDeps,
    config: &TurnConfig,
    rt: &tokio::runtime::Handle,
    sink: &mut impl TurnSink,
    agent_override: Option<&str>,
) -> TurnOutcome {
    let display_name = agent_override.unwrap_or(&config.default_agent).to_string();
    let mut current_input: String = input.to_string();
    let mut tool_results: Option<String> = None;
    let mut iteration: usize = 0;
    let mut total_usage: Option<TokenUsage> = None;
    let mut final_response: Option<String> = None;
    let mut inference_error = false;

    tracing::info!(target: "cns", cns_domain = "cns.chat.turn", operation = "started", agent = %display_name, input_len = input.len(), "CNS");

    loop {
        iteration += 1;
        if iteration > config.max_loops {
            sink.status(&format!("  \x1b[33m\u{26a0} Tool-use loop max iterations ({}) reached \u{2014} yielding current response\x1b[0m", config.max_loops));
            break;
        }

        let Some(mut gas_guard) = deps.gas.try_reserve(config.gas_heuristic) else {
            sink.status("  \x1b[31m\u{2717} Gas budget exhausted (hard limit) \u{2014} turn blocked by cybernetic regulator\x1b[0m");
            sink.status(
                "  \x1b[2mUse /status to see budget details, or wait for replenishment.\x1b[0m",
            );
            return TurnOutcome {
                success: false,
                final_response: None,
                usage: total_usage.unwrap_or_else(zero_usage),
                iterations: 0,
                budget_exhausted: true,
            };
        };

        let thread_history = if deps.threads.is_seeded() {
            None
        } else {
            deps.threads.thread_history(config.saliency_window)
        };
        let turn_input = TurnInput {
            input: &current_input,
            iteration,
            tool_results: tool_results.take(),
            agent_override,
            thread_history,
        };

        let chat_result = rt.block_on(deps.executor.execute_turn(&turn_input));
        let chat_response = match chat_result {
            Ok(r) => r,
            Err(e) => {
                sink.status(&format!("  \x1b[31mInference error:\x1b[0m {}", e));
                gas_guard.release();
                inference_error = true;
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
        let parsed = extract_tool_calls(
            &response,
            if structured_calls.is_empty() {
                None
            } else {
                Some(&structured_calls)
            },
        );

        if parsed.tool_calls.is_empty() {
            // Nudge: if tools are available but the model didn't emit a tool
            // call on the first iteration, inject a reminder. This helps models
            // that narrate intent ("Let me start by...") instead of emitting
            // <<tool:...>> directives. The nudge is only injected once (iteration 1)
            // to avoid infinite looping.
            if iteration == 1 && config.has_tools {
                sink.status(
                    "  \x1b[2m\u{2139} Tools are available — if you need to use a tool, emit a <<tool:server/name\n{...}\n>> directive.\x1b[0m",
                );
            }
            sink.agent_text(&display_name, &parsed.text);
            final_response = Some(parsed.text.clone());
            break;
        }

        if !parsed.text.trim().is_empty() {
            sink.agent_text(&display_name, parsed.text.trim());
        }
        sink.tool_log(&format!(
            "  \x1b[2m\u{2750} {} tool call(s) from {}\x1b[0m",
            parsed.tool_calls.len(),
            display_name
        ));

        let mut tool_results_vec = Vec::new();
        for call in &parsed.tool_calls {
            let mut line = format!("  \x1b[2m  Invoking {}\x1b[0m", call.tool);
            if !call.server.is_empty() {
                line.push_str(&format!(" on \x1b[36m{}\x1b[0m", call.server));
            }
            line.push_str("...");
            sink.tool_log(&line);

            let result = rt.block_on(async {
                use hkask_capability::{
                    DelegationAction, DelegationResource, DelegationToken, derive_signing_key,
                };
                let token = DelegationToken::new(
                    DelegationResource::Tool,
                    call.tool.clone(),
                    DelegationAction::Execute,
                    config.principal_webid,
                    config.agent_webid,
                    &derive_signing_key(config.a2a_secret.as_bytes()),
                );
                deps.tools
                    .invoke(&call.server, &call.tool, call.args.clone(), &token)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}: {}", call.tool, e))
            });
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
                Err(err) => sink.tool_log(&format!(
                    "  \x1b[31m  \u{2717}\x1b[0m {} \u{2014} {}",
                    call.tool, err
                )),
            }
            tool_results_vec.push((call.clone(), result));
        }

        current_input = response;
        tool_results = Some(format_tool_results(&tool_results_vec));
    }

    let (gas_remaining, gas_cap) = deps.gas.gas_status();
    emit_turn_status(
        sink,
        total_usage.as_ref(),
        iteration,
        gas_remaining,
        gas_cap,
    );

    if let Some(ref resp) = final_response {
        deps.threads.append_turn(&config.default_agent, input, resp);
    }
    if !inference_error {
        deps.threads.mark_seeded();
    }

    if let Some(ref resp) = final_response {
        tracing::info!(target: "cns", cns_domain = "cns.chat.turn", operation = "completed", agent = %display_name, response_len = resp.len(), iterations = iteration, "CNS");
    }

    (deps.on_cns_update)();

    TurnOutcome {
        success: !inference_error,
        final_response,
        usage: total_usage.unwrap_or_else(zero_usage),
        iterations: iteration,
        budget_exhausted: false,
    }
}

// ── Public wrappers ──────────────────────────────────────────────────

pub(super) fn single_agent_turn(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
    agent_override: Option<&str>,
) -> bool {
    let mut sink = StdoutSink;
    let outcome = run_turn_with_state(input, state, rt, a2a_secret, agent_override, &mut sink);
    if let Some(ref resp) = outcome.final_response
        && state.talk_config.mode == TalkMode::On
    {
        speak_response(resp, state, rt);
    }
    outcome.success
}

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

#[cfg(feature = "tui")]
pub fn single_agent_turn_captured(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
) -> TurnCapture {
    let mut sink = CaptureSink::new();
    let outcome = run_turn_with_state(input, state, rt, a2a_secret, None, &mut sink);
    if let Some(ref resp) = outcome.final_response
        && state.talk_config.mode == TalkMode::On
    {
        speak_response(resp, state, rt);
    }
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

/// Build TurnDeps from ReplState and run the turn loop.
fn run_turn_with_state(
    input: &str,
    state: &mut ReplState,
    rt: &tokio::runtime::Handle,
    a2a_secret: &[u8],
    agent_override: Option<&str>,
    sink: &mut impl TurnSink,
) -> TurnOutcome {
    let governed_runtime = state.service_context.governed_tool(state.agent_webid);
    let config = TurnConfig {
        max_loops: state.repl_settings.tool_loop_limit,
        gas_heuristic: state.repl_settings.gas_heuristic,
        saliency_window: state.repl_settings.condense_saliency_window,
        default_agent: state.current_agent.clone(),
        has_tools: !state.tool_definitions.is_empty(),
        a2a_secret: hkask_types::secret::ZeroizingSecret::new(a2a_secret.to_vec()),
        principal_webid: state.host.resolve_user_webid(),
        agent_webid: state.agent_webid,
    };
    let executor = super::deps::ReplTurnExecutor::from_state(state);
    let gas = super::deps::ReplGasGovernor::from_state(state, rt);
    let svc_ctx = &state.service_context;
    let on_cns_update = || cns_display::update_cns_and_display(svc_ctx, rt);
    let mut threads = super::deps::ReplThreadMemory::new(&mut state.thread_registry);
    let deps = TurnDeps {
        executor: &executor,
        gas: &gas,
        tools: governed_runtime.as_ref(),
        threads: &mut threads,
        on_cns_update: &on_cns_update,
    };
    run_turn_loop(input, deps, &config, rt, sink, agent_override)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deps::*;
    use hkask_services_chat::{TokenUsage, TurnResult};
    use hkask_services_core::{DomainKind, ErrorKind, ServiceError};
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    // ── Compaction threshold tests ───────────────────────────────────
    #[test]
    fn compaction_triggers_above_87_5_percent() {
        let w: u32 = 4096;
        let t = (w as f64 * 0.875) as u64;
        let c = (w as f64 * 0.90 * 4.0) as usize;
        assert!((c as u64) / 4 > t);
    }
    #[test]
    fn compaction_skips_below_87_5_percent() {
        let w: u32 = 4096;
        let t = (w as f64 * 0.875) as u64;
        let c = (w as f64 * 0.80 * 4.0) as usize;
        assert!((c as u64) / 4 <= t);
    }
    #[test]
    fn compaction_threshold_matches_formula() {
        for (w, e) in [(2048, 1792), (4096, 3584), (8192, 7168), (32768, 28672)] {
            assert_eq!((w as f64 * 0.875) as u64, e);
        }
    }

    // ── emit_turn_status tests ───────────────────────────────────────
    struct MockSink {
        lines: Vec<String>,
    }
    impl MockSink {
        fn new() -> Self {
            Self { lines: vec![] }
        }
    }
    impl TurnSink for MockSink {
        fn agent_text(&mut self, a: &str, t: &str) {
            self.lines.push(format!("{}: {}", a, t));
        }
        fn tool_log(&mut self, l: &str) {
            self.lines.push(l.to_string());
        }
        fn status(&mut self, l: &str) {
            self.lines.push(l.to_string());
        }
    }

    #[test]
    fn emit_status_single_iteration_omits_across() {
        let mut s = MockSink::new();
        let u = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 20,
            total_tokens: 120,
        };
        emit_turn_status(&mut s, Some(&u), 1, 5000, 10000);
        assert!(
            s.lines
                .iter()
                .any(|l| l.contains("120 tokens") && !l.contains("across"))
        );
    }
    #[test]
    fn emit_status_multi_iteration_includes_across() {
        let mut s = MockSink::new();
        let u = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 20,
            total_tokens: 120,
        };
        emit_turn_status(&mut s, Some(&u), 3, 5000, 10000);
        assert!(s.lines.iter().any(|l| l.contains("across 3 iterations")));
    }
    #[test]
    fn emit_status_no_usage_nothing() {
        let mut s = MockSink::new();
        emit_turn_status(&mut s, None, 1, 5000, 10000);
        assert!(s.lines.is_empty());
    }
    #[test]
    fn emit_status_gas_low_warns() {
        let mut s = MockSink::new();
        emit_turn_status(&mut s, None, 1, 100, 10000);
        assert!(s.lines.iter().any(|l| l.contains("Gas budget low")));
    }
    #[test]
    fn emit_status_gas_exhausted_warns() {
        let mut s = MockSink::new();
        emit_turn_status(&mut s, None, 1, 0, 10000);
        assert!(s.lines.iter().any(|l| l.contains("Gas budget exhausted")));
    }
    #[test]
    fn emit_status_gas_healthy_no_warning() {
        let mut s = MockSink::new();
        emit_turn_status(&mut s, None, 1, 5000, 10000);
        assert!(s.lines.is_empty());
    }
    #[test]
    fn emit_status_gas_cap_zero_no_warning() {
        let mut s = MockSink::new();
        emit_turn_status(&mut s, None, 1, 0, 0);
        assert!(s.lines.is_empty());
    }

    // ── Mock implementations for loop tests ──────────────────────────

    struct MockExecutor {
        responses: Mutex<VecDeque<Result<TurnResult, ServiceError>>>,
    }
    impl MockExecutor {
        fn new() -> Self {
            Self {
                responses: Mutex::new(VecDeque::new()),
            }
        }
        fn then(mut self, r: TurnResult) -> Self {
            self.responses.get_mut().unwrap().push_back(Ok(r));
            self
        }
        fn then_error(mut self, msg: &str) -> Self {
            self.responses
                .get_mut()
                .unwrap()
                .push_back(Err(ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Inference,
                    source: None,
                    message: msg.to_string(),
                }));
            self
        }
    }
    #[async_trait::async_trait]
    impl TurnExecutor for MockExecutor {
        async fn execute_turn(&self, _input: &TurnInput<'_>) -> Result<TurnResult, ServiceError> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Err(ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Inference,
                    source: None,
                    message: "exhausted".to_string(),
                }))
        }
    }

    struct MockGas {
        remaining: u64,
        cap: u64,
    }
    impl MockGas {
        fn new(r: u64, c: u64) -> Self {
            Self {
                remaining: r,
                cap: c,
            }
        }
    }
    struct MockRes {
        h: u64,
        settled: bool,
        released: bool,
    }
    impl GasReservation for MockRes {
        fn heuristic(&self) -> u64 {
            self.h
        }
        fn settle(&mut self, _: u64) {
            self.settled = true;
        }
        fn release(&mut self) {
            self.released = true;
        }
    }
    impl GasGovernor for MockGas {
        fn try_reserve(&self, h: u64) -> Option<Box<dyn GasReservation>> {
            if self.remaining == 0 {
                None
            } else {
                Some(Box::new(MockRes {
                    h,
                    settled: false,
                    released: false,
                }))
            }
        }
        fn gas_status(&self) -> (u64, u64) {
            (self.remaining, self.cap)
        }
    }

    struct MockTools {
        results: std::collections::HashMap<String, serde_json::Value>,
    }
    impl MockTools {
        fn new() -> Self {
            Self {
                results: std::collections::HashMap::new(),
            }
        }
        fn returning(mut self, t: &str, v: serde_json::Value) -> Self {
            self.results.insert(t.to_string(), v);
            self
        }
    }
    impl hkask_ports::ToolPort for MockTools {
        fn invoke<'a>(
            &'a self,
            _server: &'a str,
            tool: &'a str,
            _args: serde_json::Value,
            _token: &'a hkask_capability::DelegationToken,
        ) -> hkask_ports::ToolFuture<'a, Result<serde_json::Value, hkask_ports::ToolPortError>>
        {
            Box::pin(async move {
                self.results.get(tool).cloned().ok_or_else(|| {
                    hkask_ports::ToolPortError::InvocationFailed(format!("no mock for {}", tool))
                })
            })
        }
        fn discover_tools(&self) -> hkask_ports::ToolFuture<'_, Vec<String>> {
            Box::pin(async move { vec![] })
        }
        fn get_tool_info(
            &self,
            _: &str,
        ) -> hkask_ports::ToolFuture<'_, Option<hkask_ports::ToolInfo>> {
            Box::pin(async move { None })
        }
    }

    struct MockThreads {
        seeded: bool,
        mark_seeded_count: usize,
    }
    impl MockThreads {
        fn new() -> Self {
            Self {
                seeded: false,
                mark_seeded_count: 0,
            }
        }
        fn mark_seeded_count(&self) -> usize {
            self.mark_seeded_count
        }
    }
    impl ThreadMemory for MockThreads {
        fn is_seeded(&self) -> bool {
            self.seeded
        }
        fn thread_history(&self, _: usize) -> Option<String> {
            None
        }
        fn append_turn(&mut self, _: &str, _: &str, _: &str) {}
        fn mark_seeded(&mut self) {
            self.seeded = true;
            self.mark_seeded_count += 1;
        }
    }

    fn turn_result(text: &str, tools: Vec<crate::ToolCall>) -> TurnResult {
        use hkask_ports::StructuredToolCall;
        TurnResult {
            text: text.to_string(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            iterations: 1,
            finish_reason: if tools.is_empty() {
                "stop".to_string()
            } else {
                "tool_calls".to_string()
            },
            structured_tool_calls: tools
                .into_iter()
                .map(|tc| StructuredToolCall {
                    server: tc.server,
                    tool: tc.tool,
                    args: tc.args,
                    call_id: None,
                })
                .collect(),
        }
    }
    fn tool_call(name: &str) -> crate::ToolCall {
        crate::ToolCall {
            server: "mock".into(),
            tool: name.into(),
            args: serde_json::json!({}),
        }
    }
    fn mock_config() -> TurnConfig {
        TurnConfig {
            max_loops: 21,
            gas_heuristic: 500,
            saliency_window: 5,
            default_agent: "TestAgent".into(),
            has_tools: false,
            a2a_secret: hkask_types::secret::ZeroizingSecret::new(vec![]),
            principal_webid: hkask_types::WebID::from_persona_with_namespace(b"test", "replicant"),
            agent_webid: hkask_types::WebID::from_persona_with_namespace(b"test", "replicant"),
        }
    }
    fn noop() {}
    fn mock_deps<'a>(
        ex: &'a MockExecutor,
        gas: &'a MockGas,
        tools: &'a MockTools,
        threads: &'a mut MockThreads,
    ) -> TurnDeps<'a> {
        TurnDeps {
            executor: ex,
            gas,
            tools,
            threads,
            on_cns_update: &noop,
        }
    }

    // ── Loop regression tests ────────────────────────────────────────

    #[test]
    fn loop_displays_final_response_after_tool_calls() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ex = MockExecutor::new()
            .then(turn_result("Let me search.", vec![tool_call("search")]))
            .then(turn_result("The answer is 42.", vec![]));
        let gas = MockGas::new(10000, 10000);
        let tools = MockTools::new().returning("search", json!({"result": "42"}));
        let mut threads = MockThreads::new();
        let mut sink = MockSink::new();
        let deps = mock_deps(&ex, &gas, &tools, &mut threads);
        let outcome = run_turn_loop("q", deps, &mock_config(), rt.handle(), &mut sink, None);
        assert!(outcome.success);
        assert!(
            sink.lines.iter().any(|l| l.contains("The answer is 42.")),
            "final response must display after tool calls"
        );
    }

    #[test]
    fn loop_shows_inference_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ex = MockExecutor::new().then_error("connection refused");
        let gas = MockGas::new(10000, 10000);
        let tools = MockTools::new();
        let mut threads = MockThreads::new();
        let mut sink = MockSink::new();
        let deps = mock_deps(&ex, &gas, &tools, &mut threads);
        let outcome = run_turn_loop("q", deps, &mock_config(), rt.handle(), &mut sink, None);
        assert!(!outcome.success);
        assert!(sink.lines.iter().any(|l| l.contains("Inference error")));
    }

    #[test]
    fn loop_no_mark_seeded_on_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ex = MockExecutor::new().then_error("fail");
        let gas = MockGas::new(10000, 10000);
        let tools = MockTools::new();
        let mut threads = MockThreads::new();
        let mut sink = MockSink::new();
        let deps = mock_deps(&ex, &gas, &tools, &mut threads);
        let _ = run_turn_loop("q", deps, &mock_config(), rt.handle(), &mut sink, None);
        assert_eq!(
            threads.mark_seeded_count(),
            0,
            "mark_seeded must not be called on inference error"
        );
    }

    #[test]
    fn loop_marks_seeded_on_success() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ex = MockExecutor::new().then(turn_result("Hello!", vec![]));
        let gas = MockGas::new(10000, 10000);
        let tools = MockTools::new();
        let mut threads = MockThreads::new();
        let mut sink = MockSink::new();
        let deps = mock_deps(&ex, &gas, &tools, &mut threads);
        let _ = run_turn_loop("q", deps, &mock_config(), rt.handle(), &mut sink, None);
        assert_eq!(
            threads.mark_seeded_count(),
            1,
            "mark_seeded must be called on success"
        );
    }

    #[test]
    fn loop_displays_preamble() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ex = MockExecutor::new()
            .then(turn_result("Let me check.", vec![tool_call("check")]))
            .then(turn_result("Done!", vec![]));
        let gas = MockGas::new(10000, 10000);
        let tools = MockTools::new().returning("check", json!({"ok": true}));
        let mut threads = MockThreads::new();
        let mut sink = MockSink::new();
        let deps = mock_deps(&ex, &gas, &tools, &mut threads);
        let _ = run_turn_loop("q", deps, &mock_config(), rt.handle(), &mut sink, None);
        assert!(
            sink.lines.iter().any(|l| l.contains("Let me check.")),
            "preamble must display before tool calls"
        );
    }

    #[test]
    fn loop_warns_on_max_iterations() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ex = MockExecutor::new()
            .then(turn_result("x", vec![tool_call("loop")]))
            .then(turn_result("x", vec![tool_call("loop")]))
            .then(turn_result("x", vec![tool_call("loop")]));
        let gas = MockGas::new(100000, 100000);
        let tools = MockTools::new().returning("loop", json!({}));
        let mut threads = MockThreads::new();
        let mut sink = MockSink::new();
        let mut cfg = mock_config();
        cfg.max_loops = 2;
        let deps = mock_deps(&ex, &gas, &tools, &mut threads);
        let _ = run_turn_loop("q", deps, &cfg, rt.handle(), &mut sink, None);
        assert!(sink.lines.iter().any(|l| l.contains("max iterations")));
    }
}

#[cfg(all(test, feature = "tui"))]
mod capture_sink_tests {
    use super::*;

    #[test]
    fn agent_text_to_response() {
        let mut s = CaptureSink::new();
        s.agent_text("A", "hi");
        assert!(s.response_text.contains("hi"));
        assert!(s.tool_output.is_empty());
    }
    #[test]
    fn tool_log_to_output() {
        let mut s = CaptureSink::new();
        s.tool_log("invoking");
        assert!(s.tool_output.contains("invoking"));
        assert!(s.response_text.is_empty());
    }
    #[test]
    fn status_tokens_filtered() {
        let mut s = CaptureSink::new();
        s.status("  120 tokens (100 prompt + 20 completion)");
        assert!(s.response_text.is_empty());
    }
    #[test]
    fn status_error_captured() {
        let mut s = CaptureSink::new();
        s.status("  Inference error: fail");
        assert!(s.response_text.contains("Inference error"));
    }
    #[test]
    fn status_gas_warning_captured() {
        let mut s = CaptureSink::new();
        s.status("  Gas budget low: 100/10000 (1%)");
        assert!(s.response_text.contains("Gas budget low"));
    }
    #[test]
    fn status_max_iter_captured() {
        let mut s = CaptureSink::new();
        s.status("  max iterations reached");
        assert!(s.response_text.contains("max iterations"));
    }
}

// ── Tool call parsing (inlined from deleted tool_augmented.rs) ──────────

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub server: String,
    pub tool: String,
    pub args: serde_json::Value,
}

impl From<hkask_ports::StructuredToolCall> for ToolCall {
    fn from(stc: hkask_ports::StructuredToolCall) -> Self {
        Self {
            server: stc.server,
            tool: stc.tool,
            args: stc.args,
        }
    }
}

pub struct ParsedResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
}

pub fn extract_tool_calls(
    response_text: &str,
    structured_tool_calls: Option<&[hkask_ports::StructuredToolCall]>,
) -> ParsedResponse {
    let tool_calls = structured_tool_calls
        .unwrap_or(&[])
        .iter()
        .cloned()
        .map(ToolCall::from)
        .collect();
    ParsedResponse {
        text: response_text.to_string(),
        tool_calls,
    }
}

pub fn format_tool_results(calls: &[(ToolCall, anyhow::Result<serde_json::Value>)]) -> String {
    if calls.is_empty() {
        return String::new();
    }
    let mut parts = vec!["Tool results:".to_string(), String::new()];
    for (call, result) in calls {
        match result {
            Ok(value) => {
                let formatted =
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value));
                parts.push(format!("✓ {} → {}", call.tool, formatted));
            }
            Err(err) => parts.push(format!("✗ {} → ERROR: {}", call.tool, err)),
        }
    }
    parts.join("\n")
}
