//! Manifest executor — deterministic multi-step orchestration
//!
//! Executes a `BundleManifest` cascade: select → populate → execute.
//! Each `BundleManifestStep` is dispatched according to its `action` field:
//!
//! - **select**: Render a selector template, call inference, parse the
//!   JSON result to choose the next step or resolve a variable.
//! - **populate**: Render a template with the accumulated context map, producing
//!   a filled prompt or data payload.
//! - **execute**: Invoke an MCP tool with parameters bound from the context map.
//! - **choice**: Evaluate a condition against context, branch by setting `_next_ordinal`.
//! - **loop**: Re-enter the cascade from `loop_target` ordinal (defaults to 0),
//!   incrementing the iteration counter. Respects matryoshka depth limit (7).
//! - **abort**: Exit the cascade with a convergence status. Emits `cns.skill.converged`.
//! - **escalate**: Exit the cascade with an escalation error. Emits `cns.skill.escalated`.
//!
//! The executor respects iterative convergence (`manifest.convergence`),
//! gas budgets (`manifest.gas.cap` — hard parent allocation with
//! per-token deduction after inference calls), timeout constraints
//! (`step.timeout_seconds` — hard, enforced via tokio::time::timeout),
//! and conditional step execution (`step.condition`).
//! The PDCA loop executes steps in ordinal order, handling `loop` actions by
//! re-entering from the target ordinal until convergence threshold is met,
//! max iterations are exhausted, or `abort`/`escalate` is triggered.
//!
//! Template rendering supports two modes:
//!
//! - **minijinja** (`step.renderer == "minijinja"`): Load template from
//!   `step.template_ref` (a file path like `curator/system_state_gather.j2`)
//!   relative to `template_base_path`, then render with full Jinja2 syntax.
//! - **inline** (no `renderer` or any other value): Render `template_ref` or
//!   `renderer` as an inline template string with simple `{{key}}` substitution.
//!
//! Architecture: hkask-templates owns the executor because it needs
//! `InferencePort` (for select/populate) and `McpPort` (for execute),
//! both of which are already dependencies of this crate.

use crate::bundle::BundleManifest;
use crate::bundle::BundleManifestStep;
use crate::ports::{McpPort, Result, TemplateError};
use hkask_capability::{DelegationAction, DelegationResource, DelegationToken};
use hkask_ports::{InferencePort, InferenceResult};
use hkask_types::WebID;
use hkask_types::template::LLMParameters;
use minijinja::UndefinedBehavior;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Error healing callback: (error_string, operation_name).
type HealCallback = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// Default base path for template files relative to the project root.
const DEFAULT_TEMPLATE_BASE_PATH: &str = "registry/templates";

/// Manifest executor — drives the select → populate → execute cascade.
///
/// Created once per session (or per manifest invocation) and wired into the
/// REPL turn loop. The executor holds references to the infrastructure
/// ports it needs:
///
/// - `InferencePort` — for rendering selector templates and populating prompts
/// - `McpPort` — for invoking MCP tools in execute steps
/// - `template_base_path` — filesystem path for resolving `template_ref` values
///   when `renderer == "minijinja"`
#[derive(Clone)]
pub struct ManifestExecutor {
    /// Inference port for select/populate actions
    inference: Arc<dyn InferencePort>,
    /// MCP port for execute actions
    mcp: Arc<dyn McpPort>,
    /// Default LLM parameters for inference calls
    default_params: LLMParameters,
    /// Secret for minting delegation tokens
    a2a_secret: Vec<u8>,
    /// Base filesystem path for resolving template_ref values.
    /// When `step.renderer == "minijinja"`, `step.template_ref` is resolved
    /// relative to this path. Defaults to `registry/templates/`.
    template_base_path: PathBuf,
    /// Optional heal callback: (error_string, operation_name).
    heal_error_cb: Option<HealCallback>,
}

impl ManifestExecutor {
    /// Create a new executor with the given infrastructure ports.
    ///
    /// expect: "The system resolves and executes template manifest cascades"
    /// \[P3\] Motivating: Generative Space — executor for template manifest cascades
    /// \[P4\] Constraining: Clear Boundaries — requires A2A secret for delegation
    /// pre:  inference and mcp are initialized, a2a_secret is non-empty
    /// post: returns ManifestExecutor with default template_base_path
    pub fn new(
        inference: Arc<dyn InferencePort>,
        mcp: Arc<dyn McpPort>,
        default_params: LLMParameters,
        a2a_secret: Vec<u8>,
    ) -> Self {
        Self {
            inference,
            mcp,
            default_params,
            a2a_secret,
            template_base_path: PathBuf::from(DEFAULT_TEMPLATE_BASE_PATH),
            heal_error_cb: None,
        }
    }

    /// Attach a self-healing callback for automatic error recovery.
    pub fn with_heal_cb(mut self, cb: HealCallback) -> Self {
        self.heal_error_cb = Some(cb);
        self
    }

    /// Execute the full manifest cascade with iterative PDCA convergence.
    ///
    /// Steps are sorted by ordinal and executed in sequence. The cascade loops
    /// when a `loop` action is encountered, re-entering from the target ordinal
    /// until the convergence threshold is met (via `abort`) or `max_iterations`
    /// is exhausted. If `convergence.max_iterations == 0`, executes once
    /// (single-pass for one-shot manifests).
    ///
    /// Returns the final context map with convergence metadata under `_convergence`.
    pub async fn execute_manifest(
        &self,
        manifest: &BundleManifest,
        initial_context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mut context = initial_context;
        let mut steps = manifest.steps.clone();
        steps.sort_by_key(|s| s.ordinal);

        let max_iterations = if manifest.convergence.max_iterations == 0 {
            1 // single-pass for one-shot manifests
        } else {
            manifest.convergence.max_iterations
        };
        let threshold = manifest.convergence.threshold;
        let field = manifest.convergence.convergence_field.clone();
        let improvement_enabled = manifest.convergence.improvement_ratio > 0.0;
        let min_iterations = manifest.convergence.min_iterations;
        let mut baseline_quality: Option<f64> = None; // captured on first pass
        let mut iteration: u32 = 0;
        let mut recursion_depth: u8 = 0;
        let matryoshka_limit: u8 = hkask_capability::SYSTEM_MAX_RECURSION;
        // Gas tracking — hard parent allocation for compute cycles
        let gas_cap = manifest.gas.cap as u64;
        let gas_cost_per_iter = manifest.gas.cost_per_iteration as u64;
        let gas_alert_threshold = manifest.gas.alert_threshold;
        let gas_hard_limit = manifest.gas.hard_limit;
        let mut gas_used: u64 = 0;
        let mut gas_alerted: bool = false;
        // rJoule tracking — hard parent allocation for inference energy
        // Cost per token is set by the inference provider/model config, not the manifest.
        let rjoule_cap = manifest.rjoule.cap as f64;
        let rjoule_alert_threshold = manifest.rjoule.alert_threshold;
        let rjoule_hard_limit = manifest.rjoule.hard_limit;
        let rjoule_enabled = rjoule_cap > 0.0;
        let mut rjoule_used: f64 = 0.0;
        let mut rjoule_alerted: bool = false;

        context.insert(
            "_convergence".to_string(),
            serde_json::json!({
                "threshold": threshold,
                "max_iterations": max_iterations,
                "field": field,
                "status": "running",
                "iterations_completed": 0,
                "exit_reason": null,
                "improvement_target": manifest.convergence.improvement_ratio,
                "baseline_quality": null,
                "gas_cap": gas_cap,
                "gas_used": 0,
                "gas_remaining": gas_cap,
                "rjoule_cap": rjoule_cap,
                "rjoule_used": 0.0,
                "rjoule_remaining": rjoule_cap,
            }),
        );

        'cascade: loop {
            iteration += 1;
            // Update live convergence context for template awareness
            context.insert(
                "_convergence".to_string(),
                serde_json::json!({
                    "threshold": threshold,
                    "max_iterations": max_iterations,
                    "field": field,
                    "status": "running",
                    "iterations_completed": iteration,
                    "exit_reason": null,
                    "improvement_target": manifest.convergence.improvement_ratio,
                    "baseline_quality": baseline_quality,
                    "gas_cap": gas_cap,
                    "gas_used": gas_used,
                    "gas_remaining": gas_cap.saturating_sub(gas_used),
                    "rjoule_cap": rjoule_cap,
                    "rjoule_used": rjoule_used,
                    "rjoule_remaining": (rjoule_cap - rjoule_used).max(0.0),
                }),
            );
            let mut step_idx: usize = 0;

            while step_idx < steps.len() {
                let step = &steps[step_idx];

                info!(
                    target: "cns.skill.cascade",
                    iteration = iteration,
                    step = step.ordinal,
                    action = %step.action,
                    description = %step.description,
                    "CNS"
                );

                // Evaluate step condition — skip if false
                if let Some(ref cond) = step.condition
                    && !evaluate_step_condition(cond, &context)
                {
                    info!(
                        target: "cns.skill.cascade",
                        iteration = iteration,
                        step = step.ordinal,
                        condition = %cond,
                        skipped = true,
                        "CNS"
                    );
                    step_idx += 1;
                    continue;
                }

                match step.action.as_str() {
                    // ── Abort: converged — exit with success ──
                    "abort" => {
                        info!(
                            target: "cns.skill.converged",
                            iteration = iteration,
                            reason = "abort action",
                            "CNS"
                        );
                        self.finalize_convergence_report(
                            &mut context,
                            "converged",
                            "quality_met",
                            iteration,
                            threshold,
                            &field,
                            baseline_quality,
                            manifest.convergence.improvement_ratio,
                        );
                        break 'cascade;
                    }

                    // ── Escalate: blocked — exit with error ──
                    "escalate" => {
                        let reason = step.description.clone();
                        info!(
                            target: "cns.skill.escalated",
                            iteration = iteration,
                            reason = %reason,
                            "CNS"
                        );
                        self.finalize_convergence_report(
                            &mut context,
                            "escalated",
                            "obstacle_blocked",
                            iteration,
                            threshold,
                            &field,
                            baseline_quality,
                            manifest.convergence.improvement_ratio,
                        );
                        return Err(TemplateError::Manifest(format!(
                            "Cascade escalated at step {}: {}",
                            step.ordinal, reason
                        )));
                    }

                    // ── Choice: evaluate condition, branch ──
                    "choice" => {
                        let target_ordinal = self.evaluate_choice(step, &context)?;
                        if let Some(target) = target_ordinal {
                            // Jump to target step
                            if let Some(pos) = steps.iter().position(|s| s.ordinal == target) {
                                step_idx = pos;
                                info!(
                                    target: "cns.skill.cascade",
                                    iteration = iteration,
                                    choice_jump = target,
                                    "CNS"
                                );
                                continue; // Re-enter loop at target step
                            }
                        }
                        // No jump — fall through to next step
                    }

                    // ── Loop: re-enter cascade from target ordinal ──
                    "loop" => {
                        recursion_depth += 1;
                        if recursion_depth > matryoshka_limit {
                            info!(
                                target: "cns.skill.escalated",
                                iteration = iteration,
                                reason = "matryoshka depth exceeded",
                                depth = recursion_depth,
                                limit = matryoshka_limit,
                                "CNS"
                            );
                            self.finalize_convergence_report(
                                &mut context,
                                "maxed_out",
                                "energy_spent",
                                iteration,
                                threshold,
                                &field,
                                baseline_quality,
                                manifest.convergence.improvement_ratio,
                            );
                            return Err(TemplateError::Manifest(format!(
                                "Matryoshka depth limit ({}) exceeded at iteration {}",
                                matryoshka_limit, iteration
                            )));
                        }

                        let loop_target = step
                            .input_mapping
                            .as_ref()
                            .and_then(|m| m.get("loop_target"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;

                        info!(
                            target: "cns.skill.cascade",
                            iteration = iteration,
                            loop_target = loop_target,
                            depth = recursion_depth,
                            "CNS"
                        );

                        // Check convergence before looping
                        if iteration >= max_iterations {
                            self.finalize_convergence_report(
                                &mut context,
                                "maxed_out",
                                "energy_spent",
                                iteration,
                                threshold,
                                &field,
                                baseline_quality,
                                manifest.convergence.improvement_ratio,
                            );
                            break 'cascade;
                        }

                        // Check threshold convergence
                        if self.check_convergence(
                            &context,
                            &manifest.convergence.convergence_field,
                            threshold,
                            manifest.convergence.improvement_ratio,
                            &manifest.convergence.improvement_gate,
                            baseline_quality,
                            iteration,
                            min_iterations,
                        ) {
                            self.finalize_convergence_report(
                                &mut context,
                                "converged",
                                "quality_met",
                                iteration,
                                threshold,
                                &field,
                                baseline_quality,
                                manifest.convergence.improvement_ratio,
                            );
                            break 'cascade;
                        }

                        // Re-enter: reset step index to loop target
                        if let Some(pos) = steps.iter().position(|s| s.ordinal == loop_target) {
                            step_idx = pos;
                            continue; // Re-enter cascade from target
                        } else {
                            step_idx = 0; // Default: restart from beginning
                            continue;
                        }
                    }

                    // ── Standard actions: select, populate, execute ──
                    "select" => {
                        context = self
                            .execute_select(
                                step,
                                context,
                                &mut gas_used,
                                gas_cap,
                                gas_cost_per_iter,
                                &mut rjoule_used,
                                rjoule_cap,
                                rjoule_enabled,
                                rjoule_hard_limit,
                            )
                            .await?;
                        // Check gas exhaustion after select
                        if gas_hard_limit && gas_used >= gas_cap {
                            info!(
                                target: "cns.skill.gas_exhausted",
                                iteration = iteration,
                                gas_used = gas_used,
                                gas_cap = gas_cap,
                                "CNS"
                            );
                            self.finalize_convergence_report(
                                &mut context,
                                "maxed_out",
                                "energy_spent",
                                iteration,
                                threshold,
                                &field,
                                baseline_quality,
                                manifest.convergence.improvement_ratio,
                            );
                            break 'cascade;
                        }
                        // Gas alert threshold
                        if !gas_alerted
                            && gas_cap > 0
                            && (gas_used as f64 / gas_cap as f64) >= gas_alert_threshold
                        {
                            gas_alerted = true;
                            info!(
                                target: "cns.skill.gas_alert",
                                gas_used = gas_used,
                                gas_cap = gas_cap,
                                pct = (gas_used as f64 / gas_cap as f64) * 100.0,
                                "CNS"
                            );
                        }
                        // Check rJoule exhaustion after select
                        if rjoule_enabled && rjoule_hard_limit && rjoule_used >= rjoule_cap {
                            info!(
                                target: "cns.skill.rjoule_exhausted",
                                iteration = iteration,
                                rjoule_used = rjoule_used,
                                rjoule_cap = rjoule_cap,
                                "CNS"
                            );
                            self.finalize_convergence_report(
                                &mut context,
                                "maxed_out",
                                "energy_spent",
                                iteration,
                                threshold,
                                &field,
                                baseline_quality,
                                manifest.convergence.improvement_ratio,
                            );
                            break 'cascade;
                        }
                        // rJoule alert threshold
                        if !rjoule_alerted
                            && rjoule_cap > 0.0
                            && (rjoule_used / rjoule_cap) >= rjoule_alert_threshold
                        {
                            rjoule_alerted = true;
                            info!(
                                target: "cns.skill.rjoule_alert",
                                rjoule_used = rjoule_used,
                                rjoule_cap = rjoule_cap,
                                pct = (rjoule_used / rjoule_cap) * 100.0,
                                "CNS"
                            );
                        }
                    }
                    "populate" => {
                        context = self.execute_populate(step, context).await?;
                    }
                    "execute" | "feedback" | "validate" | "retrieve" => {
                        context = self.execute_tool_invoke(step, context).await?;
                    }

                    other => {
                        return Err(TemplateError::Manifest(format!(
                            "Unknown manifest step action: '{}'",
                            other
                        )));
                    }
                }

                step_idx += 1;
            }

            // Check gas exhaustion at end of pass
            if gas_hard_limit && gas_used >= gas_cap {
                info!(
                    target: "cns.skill.gas_exhausted",
                    iteration = iteration,
                    gas_used = gas_used,
                    gas_cap = gas_cap,
                    "CNS"
                );
                self.finalize_convergence_report(
                    &mut context,
                    "maxed_out",
                    "energy_spent",
                    iteration,
                    threshold,
                    &field,
                    baseline_quality,
                    manifest.convergence.improvement_ratio,
                );
                break 'cascade;
            }

            // Capture baseline quality on first full pass
            if improvement_enabled && baseline_quality.is_none() {
                baseline_quality = context
                    .get(&field)
                    .and_then(|v| v.as_f64())
                    .or_else(|| resolve_dot_path(&field, &context).and_then(|v| v.as_f64()));
            }

            // Compute compound quality from nested skill reports
            if manifest.convergence.aggregation != "none"
                && !manifest.convergence.aggregation_sources.is_empty()
            {
                let compound = self.compute_compound_quality(
                    &context,
                    &manifest.convergence.aggregation,
                    &manifest.convergence.aggregation_sources,
                );
                context.insert(field.clone(), serde_json::json!(compound));
            }

            // ── End of pass: check convergence if no explicit loop/abort ──
            if iteration >= max_iterations {
                self.finalize_convergence_report(
                    &mut context,
                    "maxed_out",
                    "energy_spent",
                    iteration,
                    threshold,
                    &field,
                    baseline_quality,
                    manifest.convergence.improvement_ratio,
                );
                break 'cascade;
            }

            if self.check_convergence(
                &context,
                &manifest.convergence.convergence_field,
                threshold,
                manifest.convergence.improvement_ratio,
                &manifest.convergence.improvement_gate,
                baseline_quality,
                iteration,
                min_iterations,
            ) {
                self.finalize_convergence_report(
                    &mut context,
                    "converged",
                    "quality_met",
                    iteration,
                    threshold,
                    &field,
                    baseline_quality,
                    manifest.convergence.improvement_ratio,
                );
                break 'cascade;
            }

            // Implicit loop: re-enter from step 0
            recursion_depth += 1;
            if recursion_depth > matryoshka_limit {
                self.finalize_convergence_report(
                    &mut context,
                    "maxed_out",
                    "energy_spent",
                    iteration,
                    threshold,
                    &field,
                    baseline_quality,
                    manifest.convergence.improvement_ratio,
                );
                return Err(TemplateError::Manifest(format!(
                    "Matryoshka depth limit ({}) exceeded at iteration {}",
                    matryoshka_limit, iteration
                )));
            }
        }

        context.insert(
            "_recursion_depth".to_string(),
            Value::Number(recursion_depth.into()),
        );
        Ok(context)
    }

    /// Finalize the convergence report at cascade exit.
    /// Writes a complete report with status, reason, iterations, quality at exit, threshold, field,
    /// and improvement metadata (baseline_quality, improvement_ratio, improvement_pct).
    #[allow(clippy::too_many_arguments)]
    fn finalize_convergence_report(
        &self,
        context: &mut HashMap<String, Value>,
        status: &str,
        reason: &str,
        iteration: u32,
        threshold: f64,
        field: &str,
        baseline_quality: Option<f64>,
        improvement_target: f64,
    ) {
        let quality = context
            .get(field)
            .and_then(|v| v.as_f64())
            .or_else(|| resolve_dot_path(field, context).and_then(|v| v.as_f64()));

        let gas_used_val = context
            .get("_gas")
            .and_then(|g| g.get("used"))
            .and_then(|v| v.as_u64())
            .map(|v| v as f64)
            .unwrap_or(0.0);
        let gas_cap_val = context
            .get("_gas")
            .and_then(|g| g.get("cap"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        context.insert(
            "_convergence".to_string(),
            serde_json::json!({
                "status": status,
                "reason": reason,
                "iterations_completed": iteration,
                "quality_at_exit": quality,
                "threshold": threshold,
                "field": field,
                "improvement_achieved": baseline_quality.and_then(|b| quality.map(|q| if b > 0.0 { (b - q) / b } else { 0.0 })),
                "improvement_pct": baseline_quality.and_then(|b| quality.map(|q| if b > 0.0 { ((b - q) / b) * 100.0 } else { 0.0 })),
                "improvement_target": improvement_target,
                "baseline_quality": baseline_quality,
                "gas_used": gas_used_val,
                "gas_cap": gas_cap_val,
                "gas_remaining": (gas_cap_val - gas_used_val).max(0.0),
                "gas_pct": if gas_cap_val > 0.0 { (gas_used_val / gas_cap_val) * 100.0 } else { 0.0 },
                "rjoule_used": context.get("_rjoule").and_then(|g| g.get("used")).and_then(|v| v.as_f64()).unwrap_or(0.0),
                "rjoule_cap": context.get("_rjoule").and_then(|g| g.get("cap")).and_then(|v| v.as_f64()).unwrap_or(0.0),
            }),
        );
    }

    /// Check whether the convergence threshold has been met.
    /// Looks for the configured `convergence_field` in the context (defaults to "composite").
    /// Also enforces improvement tracking: min_iterations, improvement_ratio, and improvement_gate.
    #[allow(clippy::too_many_arguments)]
    fn check_convergence(
        &self,
        context: &HashMap<String, Value>,
        field: &str,
        threshold: f64,
        improvement_ratio: f64,
        improvement_gate: &str,
        baseline_quality: Option<f64>,
        iteration: u32,
        min_iterations: u32,
    ) -> bool {
        // Enforce minimum iterations before exit is allowed
        if iteration <= min_iterations {
            return false;
        }

        // Compute current quality and threshold check
        let current = context
            .get(field)
            .and_then(|v| v.as_f64())
            .or_else(|| resolve_dot_path(field, context).and_then(|v| v.as_f64()));

        // Fallback: also check the composite score when a specific field is configured
        let current = if current.is_none() && field != "composite" {
            context.get("composite").and_then(|v| v.as_f64())
        } else {
            current
        };

        // Check convergence metadata as additional fallback
        let current = if current.is_none() {
            context.get("_convergence_score").and_then(|v| v.as_f64())
        } else {
            current
        };

        let threshold_met = current.map(|q| q <= threshold).unwrap_or(false);

        // Compute improvement from baseline as proportional ratio
        let improvement_met = if improvement_ratio > 0.0 {
            match (baseline_quality, current) {
                (Some(b), Some(c)) if b > 0.0 => ((b - c) / b) >= improvement_ratio,
                _ => false,
            }
        } else {
            false
        };

        match improvement_gate {
            "both" => threshold_met && improvement_met,
            "either" => threshold_met || improvement_met,
            _ => threshold_met, // "threshold_only"
        }
    }

    /// Evaluate a `choice` step's condition against the context.
    /// Returns `Some(ordinal)` to jump to, or `None` to continue to next step.
    fn evaluate_choice(
        &self,
        step: &BundleManifestStep,
        context: &HashMap<String, Value>,
    ) -> Result<Option<u32>> {
        let mapping = match &step.input_mapping {
            Some(m) => m,
            None => return Ok(None),
        };

        // Branch on a JSON path comparison
        if let Some(branches) = mapping.get("branches").and_then(|b| b.as_array()) {
            for branch in branches {
                let condition = branch
                    .get("condition")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                let action = branch.get("action").and_then(|a| a.as_str()).unwrap_or("");

                let matched = match condition {
                    "default" | "else" => true,
                    _ => {
                        // Simple threshold check: "composite < 0.15"
                        if let Some((field, op, val_str)) = parse_choice_condition(condition) {
                            let current =
                                context.get(field).and_then(|v| v.as_f64()).unwrap_or(1.0);
                            let target: f64 = val_str.parse().unwrap_or(0.0);
                            match op {
                                "<" => current < target,
                                "<=" => current <= target,
                                ">" => current > target,
                                ">=" => current >= target,
                                "==" => (current - target).abs() < 0.001,
                                _ => false,
                            }
                        } else {
                            false
                        }
                    }
                };

                if matched {
                    return match action {
                        "continue" => Ok(None),
                        "abort" | "escalate" => {
                            // Handled by subsequent abort/escalate step; return None to continue
                            Ok(None)
                        }
                        _ => {
                            // Try to parse as ordinal number
                            action.parse::<u32>().ok().map(Some).ok_or_else(|| {
                                TemplateError::Manifest(format!(
                                    "Choice action '{}' is not a valid ordinal",
                                    action
                                ))
                            })
                        }
                    };
                }
            }
        }

        Ok(None)
    }

    /// **Select** — Render a selector template, call inference, parse JSON result.
    ///
    /// The selector template (from `step.template_ref` or `step.renderer`) is
    /// rendered with the current context. The rendered prompt is sent to the
    /// inference port. The response is parsed as JSON and merged into context.
    #[allow(clippy::too_many_arguments)]
    async fn execute_select(
        &self,
        step: &BundleManifestStep,
        mut context: HashMap<String, Value>,
        gas_used: &mut u64,
        gas_cap: u64,
        gas_cost_per_iter: u64,
        rjoule_used: &mut f64,
        rjoule_cap: f64,
        rjoule_enabled: bool,
        _rjoule_hard_limit: bool,
    ) -> Result<HashMap<String, Value>> {
        let prompt = self.render_step_template(step, &context)?;

        let params = self.default_params.clone();

        // Hard timeout enforcement
        let timeout_dur = std::time::Duration::from_secs(step.timeout_seconds as u64);
        let result: InferenceResult = match tokio::time::timeout(
            timeout_dur,
            self.inference.generate(&prompt, &params, None),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(TemplateError::Inference(e)),
            Err(_elapsed) => {
                return Err(TemplateError::Manifest(format!(
                    "Step {} timed out after {}s",
                    step.ordinal, step.timeout_seconds
                )));
            }
        };

        // rJoule tracking — cost per token comes from inference provider, not manifest.
        // Token count is tracked; rJoule deduction will be wired when provider reports cost.
        if rjoule_enabled {
            let _tokens = result.usage.total_tokens as f64;
            // TODO: get cost_per_token from inference provider config
        }

        // Gas tracking — deduct one iteration of compute
        *gas_used = gas_used.saturating_add(gas_cost_per_iter);

        let parsed: Value = parse_json_response(&result.text, step.ordinal)?;
        context.insert(format!("step_{}_result", step.ordinal), parsed);

        // Inject dual-budget context for template awareness
        let gas_remaining = gas_cap.saturating_sub(*gas_used);
        let rjoule_remaining = (rjoule_cap - *rjoule_used).max(0.0);
        context.insert(
            "_gas".to_string(),
            serde_json::json!({
                "used": *gas_used,
                "cap": gas_cap,
                "remaining": gas_remaining,
                "cost_per_iteration": gas_cost_per_iter,
            }),
        );
        context.insert(
            "_rjoule".to_string(),
            serde_json::json!({
                "used": *rjoule_used,
                "cap": rjoule_cap,
                "remaining": rjoule_remaining,
                "cost_per_token": rjoule_cost_per_token,
                "enabled": rjoule_enabled,
            }),
        );

        Ok(context)
    }

    /// **Populate** — Render a template with the accumulated context.
    ///
    /// If the step has `input_mapping` (bindings), those are resolved against
    /// the context and merged in before template rendering. This allows selector
    /// output fields like `step_1_result.memory_type` to be promoted to top-level
    /// template variables via `{"$ref": "step_1_result.memory_type"}` bindings.
    ///
    /// The template is rendered with the current context map. The rendered
    /// output is stored in context under `step_{ordinal}_populated`.
    async fn execute_populate(
        &self,
        step: &BundleManifestStep,
        mut context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        // Resolve bindings from input_mapping and merge into context.
        // Uses {"$ref": "dot.path"} syntax — same as execute_tool_invoke.
        if let Some(ref mapping) = step.input_mapping {
            let resolved = bind_parameters(mapping, &context);
            if let Value::Object(map) = resolved {
                for (k, v) in map {
                    context.insert(k, v);
                }
            }
        }

        let populated = self.render_step_template(step, &context)?;
        context.insert(
            format!("step_{}_populated", step.ordinal),
            Value::String(populated),
        );

        Ok(context)
    }

    /// **Execute** — Invoke an MCP tool with parameters bound from context.
    ///
    /// The MCP server/tool is specified in `step.mcp` (format: "server/tool").
    /// Parameters are bound from `step.input_mapping` or the current context.
    async fn execute_tool_invoke(
        &self,
        step: &BundleManifestStep,
        mut context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mcp_ref_raw = step.mcp.as_deref().ok_or_else(|| {
            TemplateError::Manifest(format!(
                "Execute step {} has no mcp reference",
                step.ordinal
            ))
        })?;

        // Resolve ${variable} references in the MCP reference against context
        let mcp_ref = render_inline_template(mcp_ref_raw, &context);

        let input: Value = step
            .input_mapping
            .as_ref()
            .map(|mapping| bind_parameters(mapping, &context))
            .unwrap_or_else(|| {
                Value::Object(
                    context
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
            });

        // Create a delegation token for tool invocation
        let secret_bytes: [u8; 32] = self.a2a_secret[..32]
            .try_into()
            .expect("a2a_secret must be at least 32 bytes");
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            mcp_ref.to_string(),
            DelegationAction::Execute,
            WebID::from_persona(b"manifest-executor"),
            WebID::from_persona(b"manifest-executor"),
            &signing_key,
        );

        let result = self
            .mcp
            .invoke(&mcp_ref, input, &token)
            .await
            .map_err(|e| TemplateError::Mcp(Box::new(e)))?;

        context.insert(format!("step_{}_result", step.ordinal), result);

        Ok(context)
    }
}

/// Render a template step according to its renderer mode.
///
/// Dispatches based on `step.renderer`:
/// - `"minijinja"` — Load template from `step.template_ref` (a file path
///   like `curator/system_state_gather.j2`) relative to `template_base_path`,
///   then render with full Jinja2 syntax via minijinja.
/// - Inline/absent — Render `step.template_ref` or `step.renderer` as a
///   simple template string with `{{key}}` substitution.
impl ManifestExecutor {
    fn render_step_template(
        &self,
        step: &BundleManifestStep,
        context: &HashMap<String, Value>,
    ) -> Result<String> {
        let renderer = step.renderer.as_deref().unwrap_or("");

        match renderer {
            "minijinja" => {
                // template_ref is a file path relative to template_base_path.
                // Resolve {{key}} references from context before loading.
                let template_ref_raw = step.template_ref.as_deref().ok_or_else(|| {
                    TemplateError::Manifest(format!(
                        "Step {} has renderer='minijinja' but no template_ref",
                        step.ordinal
                    ))
                })?;
                let template_ref = render_inline_template(template_ref_raw, context);

                let template_path = self.template_base_path.join(&template_ref);
                let template_content = match std::fs::read_to_string(&template_path) {
                    Ok(c) => c,
                    Err(e) => {
                        let err_msg = format!(
                            "Step {}: template file not found at {}: {}",
                            step.ordinal,
                            template_path.display(),
                            e
                        );
                        if let Some(ref cb) = self.heal_error_cb {
                            cb(&err_msg, &template_path.display().to_string());
                        }
                        return Err(TemplateError::NotFound(err_msg));
                    }
                };

                info!(
                    target: "cns.spec.executor",
                    step = step.ordinal,
                    template = %template_ref,
                    "Rendering minijinja template"
                );

                render_minijinja(&template_content, context)
            }
            _ => {
                // Inline mode: template_ref or renderer contains the template string
                let template_content = step
                    .template_ref
                    .as_deref()
                    .or(step.renderer.as_deref())
                    .ok_or_else(|| {
                        TemplateError::Manifest(format!(
                            "Step {} has no template_ref or renderer",
                            step.ordinal
                        ))
                    })?;

                Ok(render_inline_template(template_content, context))
            }
        }
    }
}

/// Render a template using minijinja (full Jinja2 syntax).
///
/// Supports `{% for %}`, `{{ var }}`, `| filter`, `{% if %}`, etc.
fn render_minijinja(template: &str, context: &HashMap<String, Value>) -> Result<String> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Lenient);

    // Register custom filters
    env.add_filter(
        "truncate",
        |state: &minijinja::State, value: String, max_len: usize| -> String {
            let _ = state;
            if value.len() <= max_len {
                value
            } else {
                let mut truncated: String = value.chars().take(max_len).collect();
                truncated.push_str("...");
                truncated
            }
        },
    );

    // Add the template to the environment
    env.add_template("step", template)
        .map_err(|e| TemplateError::Render(format!("Invalid template: {}", e)))?;

    // Convert HashMap<String, Value> to minijinja context via serde
    let context_value = serde_json::to_value(context)
        .map_err(|e| TemplateError::Render(format!("Failed to serialize context: {}", e)))?;
    let minijinja_context = minijinja::Value::from_serialize(&context_value);

    env.get_template("step")
        .and_then(|tmpl| tmpl.render(minijinja_context))
        .map_err(|e| TemplateError::Render(format!("Template render error: {}", e)))
}

/// Render an inline template using simple `{{key}}` substitution.
fn render_inline_template(template: &str, context: &HashMap<String, Value>) -> String {
    let mut result = template.to_string();
    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

/// Parse a JSON response from an inference call.
///
/// Attempts to extract JSON from the response text, handling cases where
/// the model wraps the JSON in markdown code fences.
fn parse_json_response(text: &str, step_ordinal: u32) -> Result<Value> {
    // Try direct parse first
    if let Ok(v) = serde_json::from_str(text) {
        return Ok(v);
    }

    // Try extracting JSON from markdown code fences
    let trimmed = text.trim();
    if let Some(json_start) = trimmed.find("```json") {
        let after_fence = &trimmed[json_start + 7..];
        if let Some(json_end) = after_fence.find("```") {
            let json_str = after_fence[..json_end].trim();
            return serde_json::from_str(json_str).map_err(|e| {
                TemplateError::Manifest(format!(
                    "Step {}: Failed to parse JSON response: {}",
                    step_ordinal, e
                ))
            });
        }
    }

    // Try finding JSON object boundaries
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        let json_str = &trimmed[start..=end];
        return serde_json::from_str(json_str).map_err(|e| {
            TemplateError::Manifest(format!(
                "Step {}: Failed to parse JSON response: {}",
                step_ordinal, e
            ))
        });
    }

    Err(TemplateError::Manifest(format!(
        "Step {}: No JSON found in inference response",
        step_ordinal
    )))
}

/// Bind parameters from an input mapping to values from the context.
///
/// The input mapping is a JSON object where values are either:
/// - Direct values (strings, numbers, etc.)
/// - Context references: {"$ref": "step_1_result.field"}
fn bind_parameters(mapping: &Value, context: &HashMap<String, Value>) -> Value {
    match mapping {
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                let bound = bind_single_parameter(value, context);
                result.insert(key.clone(), bound);
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

/// Bind a single parameter value from the context.
fn bind_single_parameter(value: &Value, context: &HashMap<String, Value>) -> Value {
    match value {
        Value::Object(map) => {
            // Check for context reference: {"$ref": "variable_name"}
            if let Some(Value::String(ref_path)) = map.get("$ref") {
                if let Some(context_val) = context.get(ref_path.as_str()) {
                    return context_val.clone();
                }
                // Fallback: try dot notation
                if let Some(nested) = resolve_dot_path(ref_path, context) {
                    return nested;
                }
            }
            // Not a reference — recurse
            bind_parameters(value, context)
        }
        other => other.clone(),
    }
}

/// Resolve a dot-path like "step_1_result.field" from the context.
fn resolve_dot_path(path: &str, context: &HashMap<String, Value>) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let first = context.get(parts[0])?.clone();
    let mut current = first;
    for part in &parts[1..] {
        match current {
            Value::Object(map) => {
                current = map.get(*part)?.clone();
            }
            _ => return None,
        }
    }
    Some(current)
}

impl ManifestExecutor {
    /// Compute compound quality from nested inner skill convergence reports.
    fn compute_compound_quality(
        &self,
        context: &HashMap<String, Value>,
        method: &str,
        sources: &[crate::bundle::config::AggregationSource],
    ) -> f64 {
        match method {
            "all_converged" => {
                let all_ok = sources.iter().all(|src| {
                    let key = format!("step_{}_result", src.step_ordinal);
                    context
                        .get(&key)
                        .and_then(|v| v.get("_convergence"))
                        .and_then(|c| c.get("status"))
                        .and_then(|s| s.as_str())
                        .map(|s| s == "converged")
                        .unwrap_or(false)
                });
                if all_ok { 0.0 } else { 1.0 }
            }
            "min" => sources
                .iter()
                .filter_map(|src| {
                    let key = format!("step_{}_result", src.step_ordinal);
                    context
                        .get(&key)
                        .and_then(|v| v.get("_convergence"))
                        .and_then(|c| c.get("quality_at_exit"))
                        .and_then(|v| v.as_f64())
                })
                .fold(1.0_f64, f64::min),
            "weighted_avg" => {
                let mut sum = 0.0_f64;
                let mut total = 0.0_f64;
                for src in sources {
                    let key = format!("step_{}_result", src.step_ordinal);
                    if let Some(v) = context
                        .get(&key)
                        .and_then(|v| v.get("_convergence"))
                        .and_then(|c| c.get("quality_at_exit"))
                        .and_then(|v| v.as_f64())
                    {
                        sum += v * src.weight;
                        total += src.weight;
                    }
                }
                if total > 0.0 { sum / total } else { 1.0 }
            }
            _ => 0.0,
        }
    }
}

/// Evaluate a step condition expression against the context.
/// Supported: "var_name" (truthy), "NOT var_name" (falsy),
/// "a AND b" (both truthy), "a OR b" (either truthy).
fn evaluate_step_condition(condition: &str, context: &HashMap<String, Value>) -> bool {
    let condition = condition.trim();

    // Check for boolean operators
    if let Some(pos) = condition.find(" AND ") {
        let left = &condition[..pos].trim();
        let right = &condition[pos + 5..].trim();
        return evaluate_step_condition(left, context) && evaluate_step_condition(right, context);
    }
    if let Some(pos) = condition.find(" OR ") {
        let left = &condition[..pos].trim();
        let right = &condition[pos + 4..].trim();
        return evaluate_step_condition(left, context) || evaluate_step_condition(right, context);
    }

    // Check for negation
    if let Some(inner) = condition.strip_prefix("NOT ") {
        return !evaluate_step_condition(inner.trim(), context);
    }

    // Simple variable check: is it truthy in context?
    // Also resolve dot-paths like "step_1_result.intervention_needed"
    let key = condition;
    let resolved = resolve_dot_path(key, context);
    let val: Option<&Value> = context.get(key).or(resolved.as_ref());
    match val {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Some(Value::String(s)) => !s.is_empty() && s != "false" && s != "0",
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Null) => false,
        None => false,
    }
}

/// Parse a simple choice condition string like "composite < 0.15" or "findings == 0".
/// Returns `Some((field, operator, value))` or `None` if unparseable.
fn parse_choice_condition(condition: &str) -> Option<(&str, &str, &str)> {
    let condition = condition.trim();
    for op in &["<=", ">=", "==", "<", ">"] {
        if let Some(pos) = condition.find(op) {
            let field = condition[..pos].trim();
            let value = condition[pos + op.len()..].trim();
            if !field.is_empty() && !value.is_empty() {
                return Some((field, *op, value));
            }
        }
    }
    None
}
