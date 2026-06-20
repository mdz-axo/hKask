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

// ── Manifest types ──────────────────────────────────────────────────────────────

/// Top-level QA script manifest deserialized from YAML.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QaScriptManifest {
    pub manifest: ManifestMeta,
    #[serde(default)]
    pub gas: GasConfig,
    #[serde(default)]
    pub inputs: Vec<ManifestInput>,
    pub steps: Vec<QaScriptStep>,
    #[serde(default)]
    pub error_handling: ErrorHandling,
    #[serde(default)]
    pub cns: CnsConfig,
    #[serde(default)]
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub editor: String,
    #[serde(default)]
    pub visibility: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GasConfig {
    #[serde(default = "default_gas_cap")]
    pub cap: u64,
    #[serde(default = "default_cost_per_token")]
    pub cost_per_token: f64,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
    #[serde(default = "default_true")]
    pub hard_limit: bool,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            cap: default_gas_cap(),
            cost_per_token: default_cost_per_token(),
            alert_threshold: default_alert_threshold(),
            hard_limit: default_true(),
        }
    }
}

fn default_gas_cap() -> u64 {
    15000
}
fn default_cost_per_token() -> f64 {
    0.25
}
fn default_alert_threshold() -> f64 {
    0.7
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ManifestInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: String,
}

/// A single step in a QA autonomous script.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QaScriptStep {
    pub ordinal: u32,
    pub action: String,
    pub description: String,
    /// Classifier config name (e.g., "qa-triage") — used when action is "classify"
    #[serde(default)]
    pub classifier: Option<String>,
    /// Shell command to run — used when action is "run_command"
    #[serde(default)]
    pub command: Option<String>,
    /// Path to fuzz output file (relative to workspace) — used when action is "fuzz"
    #[serde(default)]
    pub fuzz_output: Option<String>,
    /// Branching table: maps condition → target ordinal
    /// Conditions: "high_confidence" (≥0.95), "medium_confidence" (≥0.70),
    /// "low_confidence", "flake", "unparseable", "success", "failure"
    #[serde(default)]
    pub branching: HashMap<String, u32>,
    /// Default next step if no branch condition matches
    #[serde(default)]
    pub default_next: Option<u32>,
    /// Max iterations for loop actions
    #[serde(default)]
    pub max_iterations: Option<u32>,
    /// Delay between iterations in seconds
    #[serde(default)]
    pub iteration_delay_secs: Option<u64>,
    /// CNS span target for this step
    #[serde(default)]
    pub cns_span: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorHandling {
    #[serde(default = "default_on_gas")]
    pub on_gas_exceeded: String,
    #[serde(default = "default_on_timeout")]
    pub on_timeout: String,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff_seconds: u64,
}

impl Default for ErrorHandling {
    fn default() -> Self {
        Self {
            on_gas_exceeded: default_on_gas(),
            on_timeout: default_on_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_seconds: default_retry_backoff(),
        }
    }
}

fn default_on_gas() -> String {
    "abort".into()
}
fn default_on_timeout() -> String {
    "retry".into()
}
fn default_max_retries() -> u32 {
    2
}
fn default_retry_backoff() -> u64 {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CnsConfig {
    #[serde(default = "default_true")]
    pub emit_spans: bool,
    #[serde(default)]
    pub span_namespace: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuditConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub log_level: Option<String>,
}

// ── Runtime types ───────────────────────────────────────────────────────────────

/// Result of a single classification (returned by the caller's classify function).
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    pub category: String,
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
}

/// Report summarizing a full script execution.
#[derive(Debug, Clone)]
pub struct QaScriptReport {
    pub manifest_id: String,
    pub steps_executed: Vec<StepResult>,
    pub total_steps: usize,
    pub terminal_outcome: String,
    pub exceeded_gas: bool,
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
    MaxIterationsExhausted {
        ordinal: u32,
        iterations: u32,
    },
    StepNotFound {
        ordinal: u32,
    },
    NoClassifierConfig {
        ordinal: u32,
    },
    EmptyScript,
}

impl fmt::Display for QaScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(s) => write!(f, "Parse error: {s}"),
            Self::GasExceeded { cap } => write!(f, "Gas exceeded (cap: {cap})"),
            Self::CommandFailed {
                ordinal,
                command,
                stderr,
            } => {
                write!(f, "Step {ordinal}: command `{command}` failed: {stderr}")
            }
            Self::ClassifyFailed { ordinal, reason } => {
                write!(f, "Step {ordinal}: classification failed: {reason}")
            }
            Self::MaxIterationsExhausted {
                ordinal,
                iterations,
            } => {
                write!(f, "Step {ordinal}: max iterations ({iterations}) exhausted")
            }
            Self::StepNotFound { ordinal } => write!(f, "Step {ordinal} not found in manifest"),
            Self::NoClassifierConfig { ordinal } => {
                write!(
                    f,
                    "Step {ordinal}: action 'classify' requires a classifier config name"
                )
            }
            Self::EmptyScript => write!(f, "Manifest has no steps"),
        }
    }
}

impl std::error::Error for QaScriptError {}

impl From<std::io::Error> for QaScriptError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

// ── Runner ──────────────────────────────────────────────────────────────────────

/// Type alias for the classify function the caller provides.
/// Takes (classifier_config_name, passages) and returns classify results.
pub type ClassifyFn = dyn Fn(&str, &[String]) -> Result<Vec<ClassifyResult>, String> + Send + Sync;

/// Executes a QA script manifest autonomously.
pub struct QaScriptRunner {
    manifest: QaScriptManifest,
    /// Caller-provided classify function
    classify: Box<ClassifyFn>,
}

impl QaScriptRunner {
    /// Create a new runner from a parsed manifest.
    ///
    /// pre:  manifest must have at least one step
    /// post: returns runner with classify function wired
    pub fn new(manifest: QaScriptManifest, classify: Box<ClassifyFn>) -> Self {
        Self { manifest, classify }
    }

    /// Access the parsed manifest metadata.
    pub fn manifest(&self) -> &ManifestMeta {
        &self.manifest.manifest
    }

    /// Number of steps in the manifest.
    pub fn step_count(&self) -> usize {
        self.manifest.steps.len()
    }

    /// Run the script to completion.
    ///
    /// Executes steps in order, branching on classify outcomes.
    /// Loop actions repeat up to `max_iterations`.
    pub fn run(&self) -> Result<QaScriptReport, QaScriptError> {
        let steps = &self.manifest.steps;
        if steps.is_empty() {
            return Err(QaScriptError::EmptyScript);
        }

        let mut results: Vec<StepResult> = Vec::new();
        let mut gas_used: u64 = 0;
        let gas_cap = self.manifest.gas.cap;
        let mut current_idx: usize = 0;

        while current_idx < steps.len() {
            let step = &steps[current_idx];
            let start = std::time::Instant::now();

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
                "classify" => self.execute_classify(step, &mut gas_used, gas_cap)?,
                "run_command" => self.execute_command(step)?,
                "loop" => self.execute_loop(current_idx, steps, &mut gas_used, gas_cap)?,
                _ => StepResult {
                    ordinal: step.ordinal,
                    action: step.action.clone(),
                    outcome: "success".into(),
                    classify_category: None,
                    retries: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            };

            let duration_ms = start.elapsed().as_millis() as u64;
            let mut result = outcome;
            result.duration_ms = duration_ms;

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

        Ok(QaScriptReport {
            manifest_id: self.manifest.manifest.id.clone(),
            total_steps: results.len(),
            steps_executed: results,
            terminal_outcome,
            exceeded_gas: gas_used >= gas_cap,
        })
    }

    fn execute_classify(
        &self,
        step: &QaScriptStep,
        gas_used: &mut u64,
        gas_cap: u64,
    ) -> Result<StepResult, QaScriptError> {
        let classifier_name =
            step.classifier
                .as_deref()
                .ok_or_else(|| QaScriptError::NoClassifierConfig {
                    ordinal: step.ordinal,
                })?;

        if *gas_used >= gas_cap {
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

        // Estimate gas: ~1 token per character for prompt + response
        *gas_used += step.description.len() as u64;

        let category = result
            .first()
            .map(|r| r.category.clone())
            .unwrap_or_default();

        // Parse QaDiagnosis from category string to extract confidence/root_cause/flake
        let diagnosis = parse_diagnosis_from_category(&category);

        let outcome = if diagnosis.is_flake {
            "flake"
        } else if diagnosis.confidence >= 0.95 {
            "high_confidence"
        } else if diagnosis.confidence >= 0.70 {
            "medium_confidence"
        } else if diagnosis.confidence > 0.0 {
            "low_confidence"
        } else {
            "unparseable"
        };

        Ok(StepResult {
            ordinal: step.ordinal,
            action: "classify".into(),
            outcome: outcome.into(),
            classify_category: Some(category),
            retries: 0,
            duration_ms: 0,
        })
    }

    fn execute_command(&self, step: &QaScriptStep) -> Result<StepResult, QaScriptError> {
        let cmd = step.command.as_deref().unwrap_or("true");

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| QaScriptError::CommandFailed {
                ordinal: step.ordinal,
                command: cmd.into(),
                stderr: e.to_string(),
            })?;

        let outcome = if output.status.success() {
            "success"
        } else {
            "failure"
        };

        Ok(StepResult {
            ordinal: step.ordinal,
            action: "run_command".into(),
            outcome: outcome.into(),
            classify_category: None,
            retries: 0,
            duration_ms: 0,
        })
    }

    fn execute_loop(
        &self,
        current_idx: usize,
        steps: &[QaScriptStep],
        gas_used: &mut u64,
        gas_cap: u64,
    ) -> Result<StepResult, QaScriptError> {
        let step = &steps[current_idx];
        let max_iters = step.max_iterations.unwrap_or(5);
        let delay = std::time::Duration::from_secs(step.iteration_delay_secs.unwrap_or(1));

        for iter in 0..max_iters {
            if *gas_used >= gas_cap {
                return Err(QaScriptError::GasExceeded { cap: gas_cap });
            }

            // Execute the action specified in the loop step
            let inner_result = match step.command.as_deref() {
                Some(cmd) => {
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .output()
                        .map_err(|e| QaScriptError::CommandFailed {
                            ordinal: step.ordinal,
                            command: cmd.into(),
                            stderr: e.to_string(),
                        })?;

                    if output.status.success() {
                        "success".to_string()
                    } else {
                        String::from_utf8_lossy(&output.stderr).to_string()
                    }
                }
                None => {
                    // No command — treat as classifier loop
                    if let Some(classifier_name) = &step.classifier {
                        let passages = vec![step.description.clone()];
                        let result = (self.classify)(classifier_name, &passages).map_err(|e| {
                            QaScriptError::ClassifyFailed {
                                ordinal: step.ordinal,
                                reason: e,
                            }
                        })?;
                        *gas_used += step.description.len() as u64;
                        result
                            .first()
                            .map(|r| r.category.clone())
                            .unwrap_or_default()
                    } else {
                        "no_command_or_classifier".into()
                    }
                }
            };

            // Check branch conditions
            for (condition, target) in &step.branching {
                if *condition == inner_result || inner_result.contains(condition) {
                    // Return result indicating loop branched — the outer run loop
                    // will advance to target_idx, but we can't do that from here.
                    // Instead, we return a result that instructs the caller.
                    let _valid = find_step_index(steps, *target)
                        .ok_or(QaScriptError::StepNotFound { ordinal: *target })?;
                    return Ok(StepResult {
                        ordinal: step.ordinal,
                        action: format!("loop_branch_to_{target}"),
                        outcome: condition.clone(),
                        classify_category: None,
                        retries: iter + 1,
                        duration_ms: 0,
                    });
                }
            }

            if iter < max_iters - 1 {
                std::thread::sleep(delay);
            }
        }

        Ok(StepResult {
            ordinal: step.ordinal,
            action: "loop".into(),
            outcome: "loop_exhausted".into(),
            classify_category: None,
            retries: max_iters,
            duration_ms: 0,
        })
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

fn find_step_index(steps: &[QaScriptStep], ordinal: u32) -> Option<usize> {
    steps.iter().position(|s| s.ordinal == ordinal)
}

/// Lightweight diagnosis structure parsed from classifier output.
/// Mirrors `QaDiagnosis` fields used for branching decisions.
#[derive(Debug, Clone, Default)]
struct DiagnosisFields {
    confidence: f64,
    is_flake: bool,
}

/// Parse a classify result category string into diagnosis fields.
///
/// The category is expected to be JSON conforming to the QaDiagnosis schema,
/// but may be wrapped in markdown code fences. Non-JSON is treated as
/// unparseable (confidence = 0.0).
fn parse_diagnosis_from_category(raw: &str) -> DiagnosisFields {
    let json = raw
        .trim()
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(raw);

    #[derive(Deserialize)]
    struct RawDiag {
        #[serde(default)]
        confidence: f64,
        #[serde(default)]
        is_flake: bool,
    }

    serde_json::from_str::<RawDiag>(json)
        .map(|d| DiagnosisFields {
            confidence: d.confidence,
            is_flake: d.is_flake,
        })
        .unwrap_or_default()
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
        let fields = parse_diagnosis_from_category(
            r#"{"confidence":0.55,"is_flake":true,"root_cause":"race condition"}"#,
        );
        assert!(fields.is_flake);
    }

    #[test]
    fn parse_diagnosis_unparseable() {
        let fields = parse_diagnosis_from_category("not json at all");
        assert!((fields.confidence - 0.0).abs() < 0.001);
        assert!(!fields.is_flake);
    }

    #[test]
    fn parse_diagnosis_markdown_fenced() {
        let fields =
            parse_diagnosis_from_category("```json\n{\"confidence\":0.85,\"is_flake\":false}\n```");
        assert!((fields.confidence - 0.85).abs() < 0.001);
    }

    #[test]
    fn report_counts_classify_steps() {
        let report = QaScriptReport {
            manifest_id: "test".into(),
            steps_executed: vec![
                StepResult {
                    ordinal: 1,
                    action: "classify".into(),
                    outcome: "high_confidence".into(),
                    classify_category: None,
                    retries: 0,
                    duration_ms: 100,
                },
                StepResult {
                    ordinal: 2,
                    action: "run_command".into(),
                    outcome: "success".into(),
                    classify_category: None,
                    retries: 0,
                    duration_ms: 50,
                },
                StepResult {
                    ordinal: 3,
                    action: "classify".into(),
                    outcome: "medium_confidence".into(),
                    classify_category: None,
                    retries: 1,
                    duration_ms: 200,
                },
            ],
            total_steps: 3,
            terminal_outcome: "medium_confidence".into(),
            exceeded_gas: false,
        };
        assert_eq!(report.classify_steps(), 2);
        assert_eq!(report.total_retries(), 1);
    }

    #[test]
    fn empty_manifest_rejected() {
        let manifest = QaScriptManifest {
            manifest: ManifestMeta {
                id: "empty".into(),
                name: "empty".into(),
                description: "empty".into(),
                editor: "test".into(),
                visibility: "public".into(),
            },
            gas: GasConfig {
                cap: 1000,
                cost_per_token: 0.25,
                alert_threshold: 0.7,
                hard_limit: true,
            },
            inputs: vec![],
            steps: vec![],
            error_handling: ErrorHandling {
                on_gas_exceeded: "abort".into(),
                on_timeout: "retry".into(),
                max_retries: 2,
                retry_backoff_seconds: 1,
            },
            cns: CnsConfig {
                emit_spans: false,
                span_namespace: "cns.qa.test".into(),
            },
            audit: AuditConfig {
                enabled: false,
                log_level: None,
            },
        };
        let classify: Box<ClassifyFn> = Box::new(|_, _| Ok(vec![]));
        let runner = QaScriptRunner::new(manifest, classify);
        let result = runner.run();
        assert!(result.is_err());
    }

    #[test]
    fn linear_success_script_runs() {
        let yaml = r#"
manifest:
  id: "linear-test"
  name: "Linear Test"
  description: "Runs two commands sequentially"
steps:
  - ordinal: 1
    action: "run_command"
    command: "true"
    description: "First pass"
  - ordinal: 2
    action: "run_command"
    command: "true"
    description: "Second pass"
cns:
  emit_spans: false
  span_namespace: "cns.qa.test"
"#;
        let manifest: QaScriptManifest = serde_yaml_neo::from_str(yaml).unwrap();
        let classify: Box<ClassifyFn> = Box::new(|_, _| Ok(vec![]));
        let runner = QaScriptRunner::new(manifest, classify);
        let report = runner.run().unwrap();
        assert_eq!(report.steps_executed.len(), 2);
        assert_eq!(report.steps_executed[0].outcome, "success");
        assert_eq!(report.steps_executed[1].outcome, "success");
    }

    #[test]
    fn classify_with_mock_branches() {
        let yaml = r#"
manifest:
  id: "classify-branch"
  name: "Classify Branch"
  description: "Classify and branch on confidence"
steps:
  - ordinal: 1
    action: "classify"
    classifier: "qa-triage"
    description: "Test fuzz failure: off-by-one in index calculation"
    branching:
      high_confidence: 3
      medium_confidence: 2
      low_confidence: 2
  - ordinal: 2
    action: "run_command"
    command: "echo 'escalate'"
    description: "Escalate to human"
  - ordinal: 3
    action: "run_command"
    command: "echo 'auto-repair'"
    description: "Auto-repair"
cns:
  emit_spans: false
  span_namespace: "cns.qa.test"
"#;
        let manifest: QaScriptManifest = serde_yaml_neo::from_str(yaml).unwrap();
        // Mock classify returns high confidence
        let classify: Box<ClassifyFn> = Box::new(|_name, _passages| {
            Ok(vec![ClassifyResult {
                category: r#"{"confidence":0.96,"is_flake":false,"root_cause":"off-by-one"}"#
                    .into(),
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
}
