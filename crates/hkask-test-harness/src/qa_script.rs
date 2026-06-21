//! QA Script Runner — classifier-driven autonomous interactive test scripts.
//!
//! Reads a YAML manifest describing an autonomous QA pipeline (fuzz → classify →
//! branch → repair or escalate), executes steps sequentially, branching on
//! `classify_batch` confidence levels. Each step emits a CNS span.
//!
//! The runner is classifier-agnostic: the caller provides a `ClassifyFn` closure.
//! The CLI wires in `hkask_services_classify::classify_batch`.
//!
//! # Principle grounding
//! - P8 (Semantic Grounding): every step maps to a CNS namespace
//! - P9 (Homeostatic Self-Regulation): autonomous branching adapts to classifier output
//! - P5 (Essentialism): one runner, one manifest, no framework

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use hkask_ledger::{Ledger, LedgerTransaction, Posting};

// ── Manifest types ──────────────────────────────────────────────────────────────

/// Top-level QA script manifest parsed from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct QaScriptManifest {
    pub manifest: ManifestMeta,
    pub gas: GasConfig,
    pub cns: CnsConfig,
    pub steps: Vec<QaScriptStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestMeta {
    pub id: String,
    pub description: String,
}

/// Gas / energy budget configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GasConfig {
    pub cap: u64,
    #[serde(default = "default_gas_per_function")]
    pub gas_per_function: u64,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
    #[serde(default = "default_hard_limit")]
    pub hard_limit: bool,
    #[serde(default)]
    pub monthly_subscriptions_urj: u64,
}

fn default_gas_per_function() -> u64 {
    100
}
fn default_alert_threshold() -> f64 {
    0.7
}
fn default_hard_limit() -> bool {
    true
}

/// CNS (Cybernetic Nervous System) configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CnsConfig {
    #[serde(default = "default_true")]
    pub emit_spans: bool,
    pub alert: Option<String>,
}

fn default_true() -> bool {
    true
}

/// One step in the QA pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct QaScriptStep {
    pub ordinal: u32,
    pub action: String,
    pub classifier: Option<String>,
    pub description: String,
    pub command: Option<String>,
    pub retries: u32,
    #[serde(default)]
    pub branching: HashMap<String, u32>,
    pub default_next: Option<u32>,
    #[serde(default)]
    pub gas_multiplier: u32,
    pub training_cost_urj: Option<u64>,
    pub max_iterations: Option<u32>,
}

/// Result of a classify operation, passed back from the classify closure.
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    pub category: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_urj: u64,
    pub failed: bool,
}

/// Parse diagnosis fields from a JSON category string.
fn parse_diagnosis_from_category(category: &str) -> DiagnosisFields {
    #[derive(Deserialize)]
    struct Raw {
        confidence: Option<f64>,
        is_flake: Option<bool>,
        root_cause: Option<String>,
        proposed_fix: Option<String>,
    }
    match serde_json::from_str::<Raw>(category) {
        Ok(raw) => DiagnosisFields {
            confidence: raw.confidence.unwrap_or(0.0),
            is_flake: raw.is_flake.unwrap_or(false),
            root_cause: raw.root_cause,
            proposed_fix: raw.proposed_fix,
        },
        Err(_) => DiagnosisFields::default(),
    }
}

#[derive(Debug, Clone, Default)]
struct DiagnosisFields {
    confidence: f64,
    is_flake: bool,
    root_cause: Option<String>,
    proposed_fix: Option<String>,
}

// ── Outcome normalization ───────────────────────────────────────────────────────

/// Normalize a classify result into a branching outcome tag.
fn classify_outcome(fields: &DiagnosisFields) -> String {
    if fields.is_flake {
        "flake".into()
    } else if fields.confidence >= 0.85 {
        "high_confidence".into()
    } else if fields.confidence >= 0.5 {
        "medium_confidence".into()
    } else if fields.root_cause.is_some() || fields.confidence > 0.0 {
        "low_confidence".into()
    } else {
        "unparseable".into()
    }
}

fn find_step_index(steps: &[QaScriptStep], ordinal: u32) -> Option<usize> {
    steps.iter().position(|s| s.ordinal == ordinal)
}

// ── Cost Tracking ───────────────────────────────────────────────────────────────

/// Tracks all costs across the lifetime of a script run.
/// All values are in micro-rJoules (µrJ) — integer for transferability.
/// 1 µrJ = 0.000001 rJ = $0.000001 USD. 1 gas = 4 µrJ (250,000 gas = 1 rJ).
#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    pub gas_used: u64,
    pub api_token_urj: u64,
    pub failed_api_cost_urj: u64,
    pub training_urj: u64,
    pub classify_calls: u64,
}

impl CostTracker {
    pub fn total_urj(&self) -> u64 {
        (self.gas_used * 4) + self.api_token_urj + self.failed_api_cost_urj + self.training_urj
    }
    pub fn rjoule_cap_urj(&self, gas_cap: u64) -> u64 {
        gas_cap * 4
    }
    /// Compute the cost delta since a snapshot — used for per-step cost breakdown.
    pub fn step_cost_since(&self, snapshot: &CostSnapshot) -> StepCost {
        StepCost {
            gas_urj: (self.gas_used - snapshot.gas_used) * 4,
            api_token_urj: self.api_token_urj - snapshot.api_token_urj,
            failed_api_urj: self.failed_api_cost_urj - snapshot.failed_api_cost_urj,
        }
    }
    pub fn snapshot(&self) -> CostSnapshot {
        CostSnapshot {
            gas_used: self.gas_used,
            api_token_urj: self.api_token_urj,
            failed_api_cost_urj: self.failed_api_cost_urj,
        }
    }
}

/// Snapshot of CostTracker for computing per-step deltas.
#[derive(Debug, Clone)]
pub struct CostSnapshot {
    pub gas_used: u64,
    pub api_token_urj: u64,
    pub failed_api_cost_urj: u64,
}

/// Result of a single script step execution.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub ordinal: u32,
    pub action: String,
    /// Outcome tag: "high_confidence", "medium_confidence", "low_confidence",
    /// "flake", "unparseable", "success", "failure", "loop_continue", "loop_exhausted"
    pub outcome: String,
    /// If action was "classify", the raw category string from the LLM
    #[allow(dead_code)]
    pub classify_category: Option<String>,
    /// Number of retries consumed
    pub retries: u32,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Cost breakdown for this step in µrJ
    pub cost: StepCost,
}

/// Cost breakdown for a single step.
#[derive(Debug, Clone, Default)]
pub struct StepCost {
    /// Gas charged for this step in µrJ.
    pub gas_urj: u64,
    /// API token cost from this step's classify call in µrJ (successful calls).
    pub api_token_urj: u64,
    /// API cost recovered from failed calls in µrJ.
    pub failed_api_urj: u64,
}

/// Report summarizing a full script execution.
#[derive(Debug, Clone)]
pub struct QaScriptReport {
    pub manifest_id: String,
    pub steps_executed: Vec<StepResult>,
    pub total_steps: usize,
    pub terminal_outcome: String,
    pub exceeded_gas: bool,
    pub cost: CostSummary,
}

/// Cost summary for a completed script run, in micro-rJoules (µrJ).
#[derive(Debug, Clone, Default)]
pub struct CostSummary {
    /// Total gas consumed.
    pub gas_used: u64,
    /// Gas-derived µrJ (gas_used × 4).
    pub gas_urj: u64,
    /// API token costs from successful calls in µrJ.
    pub api_token_urj: u64,
    /// API costs recovered from failed calls in µrJ.
    pub failed_api_cost_urj: u64,
    /// Training costs in µrJ.
    pub training_urj: u64,
    /// Run total in µrJ.
    pub total_urj: u64,
    /// Budget cap in µrJ.
    pub cap_urj: u64,
    /// Number of classify calls made.
    pub classify_calls: u64,
    /// Monthly recurring costs in µrJ (informational, not included in run total).
    pub monthly_subscriptions_urj: u64,
    /// Whether costs were committed to a ledger.
    pub ledger_committed: bool,
}

impl QaScriptReport {
    pub fn total_retries(&self) -> u32 {
        self.steps_executed.iter().map(|s| s.retries).sum()
    }

    pub fn classify_steps(&self) -> usize {
        self.steps_executed
            .iter()
            .filter(|s| s.action == "classify")
            .count()
    }
}

// ── Error type ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum QaScriptError {
    Io(std::io::Error),
    Parse(String),
    GasExceeded {
        cap: u64,
    },
    CommandFailed {
        ordinal: u32,
        command: String,
        stderr: String,
    },
    ClassifyFailed {
        ordinal: u32,
        reason: String,
    },
    NoClassifierConfig {
        ordinal: u32,
    },
    StepNotFound {
        ordinal: u32,
    },
    EmptyScript,
    LoopExhausted {
        ordinal: u32,
        iterations: u32,
    },
}

impl fmt::Display for QaScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QaScriptError::Io(e) => write!(f, "I/O error: {}", e),
            QaScriptError::Parse(s) => write!(f, "Parse error: {}", s),
            QaScriptError::GasExceeded { cap } => {
                write!(f, "Gas budget exceeded (cap: {} µrJ)", cap)
            }
            QaScriptError::CommandFailed {
                ordinal,
                command,
                stderr,
            } => {
                write!(
                    f,
                    "Command failed at step {}: {} — {}",
                    ordinal, command, stderr
                )
            }
            QaScriptError::ClassifyFailed { ordinal, reason } => {
                write!(f, "Classify failed at step {}: {}", ordinal, reason)
            }
            QaScriptError::NoClassifierConfig { ordinal } => {
                write!(f, "No classifier configured for step {}", ordinal)
            }
            QaScriptError::StepNotFound { ordinal } => {
                write!(f, "Branch target step {} not found", ordinal)
            }
            QaScriptError::EmptyScript => {
                write!(f, "Script has no steps")
            }
            QaScriptError::LoopExhausted {
                ordinal,
                iterations,
            } => {
                write!(
                    f,
                    "Loop at step {} exhausted after {} iterations",
                    ordinal, iterations
                )
            }
        }
    }
}

// ── Runner ──────────────────────────────────────────────────────────────────────

/// Closure type for the classify function.
pub type ClassifyFn = dyn Fn(&str, &[String]) -> Result<Vec<ClassifyResult>, String> + Send + Sync;

/// Executes a QA script manifest autonomously.
pub struct QaScriptRunner {
    manifest: QaScriptManifest,
    /// Caller-provided classify function
    classify: Box<ClassifyFn>,
    /// Optional path to cost ledger database
    ledger_path: Option<PathBuf>,
}

impl QaScriptRunner {
    /// Create a new runner from a parsed manifest.
    ///
    /// pre:  manifest must have at least one step
    /// post: returns runner with classify function wired
    pub fn new(manifest: QaScriptManifest, classify: Box<ClassifyFn>) -> Self {
        Self {
            manifest,
            classify,
            ledger_path: None,
        }
    }

    /// Attach a cost ledger for immutable accounting. Costs are committed
    /// to the ledger on run completion n.
    pub fn with_ledger_path(mut self, path: PathBuf) -> Self {
        self.ledger_path = Some(path);
        self
    }

    /// Access the parsed manifest metadata.
    pub fn manifest(&self) -> &ManifestMeta {
        &self.manifest.manifest
    }

    /// Count of steps in the manifest.
    pub fn step_count(&self) -> usize {
        self.manifest.steps.len()
    }

    /// Executes steps in order, branching on classify outcomes.
    /// Loop actions repeat up to `max_iterations`.
    pub fn run(&self) -> Result<QaScriptReport, QaScriptError> {
        let steps = &self.manifest.steps;
        if steps.is_empty() {
            return Err(QaScriptError::EmptyScript);
        }

        let mut results: Vec<StepResult> = Vec::new();
        let mut cost = CostTracker::default();
        let gas_cap = self.manifest.gas.cap;
        let gas_per_fn = self.manifest.gas.gas_per_function;
        let mut current_idx: usize = 0;

        while current_idx < steps.len() {
            let step = &steps[current_idx];
            let step_gas = gas_per_fn * step.gas_multiplier as u64;

            // Track declared training cost if present on the step
            if let Some(train_cost) = step.training_cost_urj {
                cost.training_urj += train_cost;
            }

            let start = std::time::Instant::now();
            let pre_snapshot = cost.snapshot();

            // CNS span
            if self.manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.qa.script",
                    manifest = %self.manifest.manifest.id,
                    ordinal = %step.ordinal,
                    action = %step.action,
                    "CNS"
                );
            }

            let outcome = match step.action.as_str() {
                "classify" => self.execute_classify(step, &mut cost, gas_cap, step_gas)?,
                "run_command" => {
                    cost.gas_used += step_gas;
                    self.execute_command(step)?
                }
                "loop" => self.execute_loop(current_idx, steps, &mut cost, gas_cap, step_gas)?,
                _ => {
                    cost.gas_used += step_gas;
                    StepResult {
                        ordinal: step.ordinal,
                        action: step.action.clone(),
                        outcome: "success".into(),
                        classify_category: None,
                        retries: 0,
                        duration_ms: start.elapsed().as_millis() as u64,
                        cost: StepCost::default(),
                    }
                }
            };

            let duration_ms = start.elapsed().as_millis() as u64;
            let mut result = outcome;
            result.duration_ms = duration_ms;
            result.cost = cost.step_cost_since(&pre_snapshot);

            if self.manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.qa.script",
                    manifest = %self.manifest.manifest.id,
                    ordinal = %step.ordinal,
                    outcome = %result.outcome,
                    duration_ms = %duration_ms,
                    "CNS"
                );
            }

            results.push(result.clone());

            // Verify: step gas tracked (emit if step didn't increment gas counter)
            if cost.gas_used == pre_snapshot.gas_used {
                tracing::warn!(
                    target: "cns.qa.cost.step_untracked",
                    manifest = %self.manifest.manifest.id,
                    ordinal = %step.ordinal,
                    action = %step.action,
                    "Step executed but gas counter was not incremented"
                );
            }

            // Determine next step via branching
            if let Some(&target) = step.branching.get(&result.outcome) {
                match find_step_index(steps, target) {
                    Some(idx) => {
                        current_idx = idx;
                        continue;
                    }
                    None => {
                        return Err(QaScriptError::StepNotFound { ordinal: target });
                    }
                }
            }

            // Use default_next if no branch matched
            if let Some(target) = step.default_next {
                match find_step_index(steps, target) {
                    Some(idx) => {
                        current_idx = idx;
                        continue;
                    }
                    None => {
                        return Err(QaScriptError::StepNotFound { ordinal: target });
                    }
                }
            }

            // No branching, advance linearly
            current_idx += 1;
        }

        let terminal_outcome = results
            .last()
            .map(|r| r.outcome.clone())
            .unwrap_or_else(|| "completed".into());

        let total_urj = cost.total_urj();
        let cap_urj = cost.rjoule_cap_urj(gas_cap);

        // Verification: gas_used must be >= minimum expected (each step charges at least gas_per_fn)
        let min_expected_gas = results.len() as u64 * gas_per_fn;
        if cost.gas_used < min_expected_gas {
            tracing::warn!(
                target: "cns.qa.cost.gas_mismatch",
                expected = min_expected_gas,
                actual = cost.gas_used,
                "Gas tracking mismatch"
            );
        }

        // Alert threshold check
        if self.manifest.gas.alert_threshold > 0.0 && cap_urj > 0 {
            let fraction = total_urj as f64 / cap_urj as f64;
            if fraction >= self.manifest.gas.alert_threshold {
                tracing::warn!(
                    target: "cns.qa.cost.threshold_warning",
                    total_urj = total_urj,
                    cap_urj = cap_urj,
                    fraction = %format!("{:.1}%", fraction * 100.0),
                    "Cost threshold reached"
                );
            }
        }

        let exceeded = total_urj >= cap_urj && self.manifest.gas.hard_limit;
        if exceeded {
            tracing::error!(
                target: "cns.qa.cost.cap_exceeded",
                manifest = %self.manifest.manifest.id,
                total_urj = total_urj,
                cap_urj = cap_urj,
                "rJoule budget cap exceeded"
            );
        }

        // Commit costs to ledger if path configured
        let ledger_committed = if let Some(ref path) = self.ledger_path {
            self.commit_to_ledger(path, &cost).is_ok()
        } else {
            false
        };

        Ok(QaScriptReport {
            manifest_id: self.manifest.manifest.id.clone(),
            total_steps: results.len(),
            steps_executed: results,
            terminal_outcome,
            exceeded_gas: exceeded,
            cost: CostSummary {
                gas_used: cost.gas_used,
                gas_urj: cost.gas_used * 4,
                api_token_urj: cost.api_token_urj,
                failed_api_cost_urj: cost.failed_api_cost_urj,
                training_urj: cost.training_urj,
                total_urj,
                cap_urj,
                classify_calls: cost.classify_calls,
                monthly_subscriptions_urj: self.manifest.gas.monthly_subscriptions_urj,
                ledger_committed,
            },
        })
    }

    /// Commit cost transactions to the ledger at the given path.
    fn commit_to_ledger(
        &self,
        path: &std::path::Path,
        cost: &CostTracker,
    ) -> Result<(), hkask_ledger::LedgerError> {
        let ledger = Ledger::open(path)?;
        let manifest_id = &self.manifest.manifest.id;
        let now = chrono::Utc::now().to_rfc3339();
        let ref_prefix = format!("qa-run:{}", manifest_id);
        let gas_ref = format!("{}/gas", ref_prefix);
        let api_ref = format!("{}/api", ref_prefix);

        // Ensure accounts exist (idempotent)
        let _ = ledger.ensure_account("cost:qa/run", "cost");
        let _ = ledger.ensure_account("cost:gas/functions", "cost");
        let _ = ledger.ensure_account("cost:api/all", "cost");

        // Gas posting: qa/run → gas/functions
        let gas_urj = (cost.gas_used * 4) as i64;
        if gas_urj > 0 {
            let tx = LedgerTransaction {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: now.clone(),
                reference: gas_ref,
                postings: vec![Posting {
                    source: "cost:qa/run".into(),
                    destination: "cost:gas/functions".into(),
                    asset: "rJ".into(),
                    amount: gas_urj,
                }],
                metadata: serde_json::json!({"manifest_id": manifest_id, "type": "gas"}),
            };
            ledger.commit(&tx)?;
        }

        // API posting: qa/run → api/all
        let api_urj = (cost.api_token_urj + cost.failed_api_cost_urj) as i64;
        if api_urj > 0 {
            let tx = LedgerTransaction {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: now,
                reference: api_ref,
                postings: vec![Posting {
                    source: "cost:qa/run".into(),
                    destination: "cost:api/all".into(),
                    asset: "rJ".into(),
                    amount: api_urj,
                }],
                metadata: serde_json::json!({"manifest_id": manifest_id, "type": "api"}),
            };
            ledger.commit(&tx)?;
        }

        Ok(())
    }

    fn execute_classify(
        &self,
        step: &QaScriptStep,
        cost: &mut CostTracker,
        gas_cap: u64,
        gas_per_fn: u64,
    ) -> Result<StepResult, QaScriptError> {
        let classifier_name =
            step.classifier
                .as_deref()
                .ok_or(QaScriptError::NoClassifierConfig {
                    ordinal: step.ordinal,
                })?;

        if cost.total_urj() >= cost.rjoule_cap_urj(gas_cap) && self.manifest.gas.hard_limit {
            return Err(QaScriptError::GasExceeded { cap: gas_cap });
        }

        // Build passage from step description — in a real interactive scenario,
        // this would come from piped input or accumulated context
        let passage = step.description.clone();
        let passages = vec![passage];

        let result = (self.classify)(classifier_name, &passages).map_err(|e| {
            QaScriptError::ClassifyFailed {
                ordinal: step.ordinal,
                reason: e,
            }
        })?;

        // Track gas: one software function call
        cost.gas_used += gas_per_fn;
        cost.classify_calls += 1;

        if result.is_empty() {
            return Err(QaScriptError::ClassifyFailed {
                ordinal: step.ordinal,
                reason: "classifier returned no results".into(),
            });
        }

        let classify_result = &result[0];

        // Track API token costs
        if classify_result.failed {
            cost.failed_api_cost_urj += classify_result.cost_urj;
        } else {
            cost.api_token_urj += classify_result.cost_urj;
        }

        let diagnosis = parse_diagnosis_from_category(&classify_result.category);
        let outcome = classify_outcome(&diagnosis);

        Ok(StepResult {
            ordinal: step.ordinal,
            action: "classify".into(),
            outcome,
            classify_category: Some(classify_result.category.clone()),
            retries: 0,
            duration_ms: 0,            // filled by caller
            cost: StepCost::default(), // filled by caller
        })
    }

    fn execute_command(&self, step: &QaScriptStep) -> Result<StepResult, QaScriptError> {
        let cmd = step
            .command
            .as_deref()
            .ok_or(QaScriptError::CommandFailed {
                ordinal: step.ordinal,
                command: "(none)".into(),
                stderr: "no command configured".into(),
            })?;

        use std::process::Command;
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| QaScriptError::CommandFailed {
                ordinal: step.ordinal,
                command: cmd.into(),
                stderr: e.to_string(),
            })?;

        if output.status.success() {
            Ok(StepResult {
                ordinal: step.ordinal,
                action: "run_command".into(),
                outcome: "success".into(),
                classify_category: None,
                retries: 0,
                duration_ms: 0,
                cost: StepCost::default(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(StepResult {
                ordinal: step.ordinal,
                action: "run_command".into(),
                outcome: "failure".into(),
                classify_category: None,
                retries: 0,
                duration_ms: 0,
                cost: StepCost::default(),
            })
        }
    }

    fn execute_loop(
        &self,
        start_idx: usize,
        steps: &[QaScriptStep],
        cost: &mut CostTracker,
        gas_cap: u64,
        step_gas: u64,
    ) -> Result<StepResult, QaScriptError> {
        let step = &steps[start_idx];
        let max_iter = step.max_iterations.unwrap_or(10);
        let loop_start = step.ordinal + 1;

        for i in 0..max_iter {
            cost.gas_used += step_gas;

            // Execute the loop body (steps ordinal 2..N until we hit a non-loop step)
            let mut loop_idx = start_idx + 1;
            while loop_idx < steps.len() {
                let body_step = &steps[loop_idx];
                if body_step.action == "loop" {
                    break; // nested loop — stop
                }
                // Execute body step inline (simplified — no branching in loop body for now)
                cost.gas_used += step_gas;
                loop_idx += 1;
            }

            if i + 1 < max_iter {
                return Ok(StepResult {
                    ordinal: step.ordinal,
                    action: "loop".into(),
                    outcome: "loop_continue".into(),
                    classify_category: None,
                    retries: 0,
                    duration_ms: 0,
                    cost: StepCost::default(),
                });
            }
        }

        Ok(StepResult {
            ordinal: step.ordinal,
            action: "loop".into(),
            outcome: "loop_exhausted".into(),
            classify_category: None,
            retries: 0,
            duration_ms: 0,
            cost: StepCost::default(),
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_diagnosis_high_confidence() {
        let fields = parse_diagnosis_from_category(
            r#"{"confidence":0.98,"is_flake":false,"root_cause":"off-by-one","proposed_fix":"fix index"}"#,
        );
        assert!((fields.confidence - 0.98).abs() < 0.001);
        assert!(!fields.is_flake);
    }

    #[test]
    fn parse_diagnosis_flake() {
        let fields = parse_diagnosis_from_category(r#"{"confidence":0.3,"is_flake":true}"#);
        assert!(fields.is_flake);
    }

    #[test]
    fn parse_diagnosis_unparseable() {
        let fields = parse_diagnosis_from_category("not json");
        assert!(!fields.is_flake);
        assert_eq!(fields.confidence, 0.0);
    }

    #[test]
    fn parse_diagnosis_markdown_fenced() {
        let fields = parse_diagnosis_from_category(
            r#"```json
{"confidence":0.92,"is_flake":false}
```"#,
        );
        // The raw category still won't parse, but the runner handles markdown
        // fencing in the classify closure. Here we just test the raw parser.
        assert_eq!(fields.confidence, 0.0);
    }

    #[test]
    fn empty_manifest_rejected() {
        let manifest = QaScriptManifest {
            manifest: ManifestMeta {
                id: "test".into(),
                description: "".into(),
            },
            gas: GasConfig {
                cap: 1000,
                gas_per_function: 100,
                alert_threshold: 1.0,
                hard_limit: false,
                monthly_subscriptions_urj: 0,
            },
            cns: CnsConfig {
                emit_spans: false,
                alert: None,
            },
            steps: vec![],
        };
        let classify: Box<ClassifyFn> = Box::new(|_name, _passages| Ok(vec![]));
        let runner = QaScriptRunner::new(manifest, classify);
        assert!(runner.run().is_err());
    }

    #[test]
    fn linear_success_script_runs() {
        let yaml = r#"
manifest:
  id: test-script
  description: "Test script"
gas:
  cap: 10000
  gas_per_function: 100
  alert_threshold: 1.0
  hard_limit: false
  monthly_subscriptions_urj: 0
steps:
  - ordinal: 1
    action: run_command
    command: echo hello
    branching: {}
    retries: 1
  - ordinal: 2
    action: run_command
    command: echo world
    branching: {}
    retries: 1
"#;
        let manifest: QaScriptManifest = serde_yaml_neo::from_str(yaml).unwrap();
        let classify: Box<ClassifyFn> = Box::new(|_name, _passages| Ok(vec![]));
        let runner = QaScriptRunner::new(manifest, classify);
        let report = runner.run().unwrap();
        assert_eq!(report.steps_executed.len(), 2);
        assert_eq!(report.total_steps, 2);
        assert!(!report.exceeded_gas);
    }

    #[test]
    fn report_counts_classify_steps() {
        let yaml = r#"
manifest:
  id: test-script
  description: "Test"
gas:
  cap: 1000
  gas_per_function: 100
  alert_threshold: 1.0
  hard_limit: false
  monthly_subscriptions_urj: 0
steps:
  - ordinal: 1
    action: classify
    classifier: test
    description: "Test passage"
    branching: {}
    retries: 1
"#;
        let manifest: QaScriptManifest = serde_yaml_neo::from_str(yaml).unwrap();
        let classify: Box<ClassifyFn> = Box::new(|_name, _passages| {
            Ok(vec![ClassifyResult {
                category: r#"{"confidence":0.96,"is_flake":false}"#.into(),
                prompt_tokens: 400,
                completion_tokens: 300,
                cost_urj: 30,
                failed: false,
            }])
        });
        let runner = QaScriptRunner::new(manifest, classify);
        let report = runner.run().unwrap();
        assert_eq!(report.classify_steps(), 1);
    }

    #[test]
    fn classify_with_mock_branches() {
        let yaml = r#"
manifest:
  id: branch-script
  description: "Branching test"
gas:
  cap: 1000
  gas_per_function: 100
  alert_threshold: 1.0
  hard_limit: false
  monthly_subscriptions_urj: 0
steps:
  - ordinal: 1
    action: classify
    classifier: test
    description: "Check"
    branching:
      high_confidence: 3
    retries: 1
  - ordinal: 2
    action: run_command
    command: echo skipped
    branching: {}
    retries: 1
  - ordinal: 3
    action: run_command
    command: echo auto-repair
    branching: {}
    retries: 1
"#;
        let manifest: QaScriptManifest = serde_yaml_neo::from_str(yaml).unwrap();
        // Mock classify returns high confidence
        let classify: Box<ClassifyFn> = Box::new(|_name, _passages| {
            Ok(vec![ClassifyResult {
                category: r#"{"confidence":0.96,"is_flake":false,"root_cause":"off-by-one"}"#
                    .into(),
                prompt_tokens: 400,
                completion_tokens: 300,
                cost_urj: 30,
                failed: false,
            }])
        });
        let runner = QaScriptRunner::new(manifest, classify);
        let report = runner.run().unwrap();

        // Should branch to step 3 (auto-repair)
        assert_eq!(report.steps_executed.len(), 2);
        assert_eq!(report.steps_executed[0].ordinal, 1);
        assert_eq!(report.steps_executed[0].outcome, "high_confidence");
        assert_eq!(report.steps_executed[1].ordinal, 3);
        assert_eq!(report.steps_executed[1].action, "run_command");
    }

    // REQ: P8-ledger-cost-tracker — costs committed to ledger on run completion
    #[test]
    fn cost_tracker_commits_to_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("ledger.db");

        let yaml = r#"
manifest:
  id: ledger-test-script
  description: "Ledger integration test"
gas:
  cap: 10000
  gas_per_function: 100
  alert_threshold: 1.0
  hard_limit: false
  monthly_subscriptions_urj: 0
steps:
  - ordinal: 1
    action: classify
    classifier: ledger-test
    description: "Test classify"
    branching: {}
    retries: 1
  - ordinal: 2
    action: run_command
    command: echo ok
    branching: {}
    retries: 1
"#;
        let manifest: QaScriptManifest = serde_yaml_neo::from_str(yaml).unwrap();

        let classify: Box<ClassifyFn> = Box::new(|_name, _passages| {
            Ok(vec![ClassifyResult {
                category: r#"{"confidence":0.9,"is_flake":false}"#.into(),
                prompt_tokens: 400,
                completion_tokens: 300,
                cost_urj: 30,
                failed: false,
            }])
        });

        let runner = QaScriptRunner::new(manifest, classify).with_ledger_path(ledger_path.clone());
        let report = runner.run().unwrap();

        // Verify ledger was committed
        assert!(report.cost.ledger_committed);

        // Re-open ledger to verify balances
        let ledger = Ledger::open(&ledger_path).unwrap();

        // Verify gas cost account
        let gas_balance = ledger.balance("cost:gas/functions", Some("rJ")).unwrap();
        assert!(
            gas_balance > 0,
            "gas/functions should have positive balance"
        );

        // Verify API cost account
        let api_balance = ledger.balance("cost:api/all", Some("rJ")).unwrap();
        assert!(api_balance > 0, "api/all should have positive balance");

        // qa/run should be net-negative (cost sink)
        let qa_balance = ledger.balance("cost:qa/run", Some("rJ")).unwrap();
        assert_eq!(
            qa_balance + gas_balance + api_balance,
            0,
            "conservation invariant"
        );
    }
}
