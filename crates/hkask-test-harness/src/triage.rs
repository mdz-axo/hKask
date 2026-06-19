//! QA triage pipeline — parses bolero failure output, classifies via Gemma 4,
//! routes by confidence, and optionally auto-repairs.
//!
//! Called by: `kask qa triage` CLI subcommand (reads bolero output from stdin).
//!
//! # Principle grounding
//! - P9 (Homeostatic Self-Regulation): CNS spans emitted for every triage event
//! - P1 (User Sovereignty): auto-repair PRs never auto-merge
//! - P5 (Essentialism): each function earns its existence via the deletion test

use serde::Deserialize;
use std::io::BufRead;
use std::process::Command;

// ── Classifier output types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct QaDiagnosis {
    pub failure_type: String,
    pub root_cause: String,
    pub confidence: f64,
    #[serde(default)]
    pub proposed_fix: String,
    #[serde(default)]
    pub affected_file: String,
    #[serde(default)]
    pub affected_line: u32,
    #[serde(default)]
    pub is_flake: bool,
    #[serde(default)]
    pub suggested_fuzz_target: String,
}

// ── Bolero failure model ─────────────────────────────────────────────────────

#[derive(Debug)]
pub struct BoleroFailure {
    pub crate_name: String,
    pub test_name: String,
    pub panic_message: String,
    pub stack_trace: String,
    pub source_snippet: String,
    pub failing_input: String,
}

impl BoleroFailure {
    /// Format this failure as a passage for the classifier LLM.
    pub fn to_passage(&self) -> String {
        format!(
            "CRATE: {crate}\nTEST: {test}\nPANIC: {panic}\nINPUT: {input}\n\
             STACK:\n{stack}\nSOURCE:\n{source}",
            crate = self.crate_name,
            test = self.test_name,
            panic = self.panic_message,
            input = self.failing_input,
            stack = self.stack_trace,
            source = self.source_snippet,
        )
    }
}

// ── Triage report ────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct TriageReport {
    pub auto_repaired: usize,
    pub issues_opened: usize,
    pub flakes: usize,
    pub unparseable: usize,
    pub duplicates_blocked: usize,
}

impl TriageReport {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn total_actions(&self) -> usize {
        self.auto_repaired + self.issues_opened + self.flakes + self.unparseable
    }
}

// ── Bolero output parser ─────────────────────────────────────────────────────

/// Parse bolero failure output from stdin.
///
/// Bolero outputs failures in a structured format. This parser detects
/// "Test Failure" separator blocks and extracts crate name, test name,
/// panic message, location, and failing input.
///
/// Note: the exact output format needs reverse-engineering from bolero's
/// actual output. This implementation handles the common patterns.
pub fn parse_bolero_stdin(stdin: impl BufRead) -> Result<Vec<BoleroFailure>, TriageError> {
    let mut failures = Vec::new();
    let mut current: Option<BoleroFailureBuilder> = None;

    for line in stdin.lines() {
        let line = line.map_err(TriageError::Io)?;
        if line.contains("Test Failure") || line.starts_with("failures:") {
            if let Some(builder) = current.take() {
                if let Ok(f) = builder.build() {
                    failures.push(f);
                }
            }
            current = Some(BoleroFailureBuilder::new());
        } else if let Some(ref mut b) = current {
            b.feed(&line);
        }
    }
    if let Some(builder) = current.take() {
        if let Ok(f) = builder.build() {
            failures.push(f);
        }
    }

    Ok(failures)
}

struct BoleroFailureBuilder {
    crate_name: String,
    test_name: String,
    panic_message: String,
    stack_trace: String,
    source_snippet: String,
    failing_input: String,
    in_stack: bool,
}

impl BoleroFailureBuilder {
    fn new() -> Self {
        Self {
            crate_name: String::new(),
            test_name: String::new(),
            panic_message: String::new(),
            stack_trace: String::new(),
            source_snippet: String::new(),
            failing_input: String::new(),
            in_stack: false,
        }
    }

    fn feed(&mut self, line: &str) {
        if line.starts_with("thread '") {
            // "thread 'fuzz_cns_span_parse' panicked at crates/hkask-cns/..."
            if let Some(name) = line.split('\'').nth(1) {
                self.test_name = name.to_string();
            }
            if let Some(rest) = line.split("panicked at ").nth(1) {
                self.panic_message = rest.to_string();
            }
            if let Some(path) = line.split("panicked at ").nth(1) {
                if let Some(crate_path) = path.split('/').next() {
                    self.crate_name = crate_path.to_string();
                }
            }
        } else if line.contains("failing input:") {
            self.failing_input = line
                .split("failing input:")
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
        } else if self.in_stack || line.trim_start().starts_with("at ") {
            self.in_stack = true;
            if !self.stack_trace.is_empty() {
                self.stack_trace.push('\n');
            }
            self.stack_trace.push_str(line);
        } else if line.trim_start().starts_with("--> ") || line.contains(".rs:") {
            if !self.source_snippet.is_empty() {
                self.source_snippet.push('\n');
            }
            self.source_snippet.push_str(line);
        }
    }

    fn build(self) -> Result<BoleroFailure, TriageError> {
        if self.test_name.is_empty() {
            return Err(TriageError::Parse(
                "No test name found in bolero output".into(),
            ));
        }
        Ok(BoleroFailure {
            crate_name: self.crate_name,
            test_name: self.test_name,
            panic_message: self.panic_message,
            stack_trace: self.stack_trace,
            source_snippet: self.source_snippet,
            failing_input: self.failing_input,
        })
    }
}

// ── Git helpers ──────────────────────────────────────────────────────────────

/// Check if a repair branch already exists (dedup guard).
pub fn already_has_repair_branch(test_name: &str) -> Result<bool, TriageError> {
    let branch = repair_branch_name(test_name);
    let output = Command::new("git")
        .args(["branch", "--list", &branch])
        .output()
        .map_err(|e| TriageError::Git(format!("git branch --list failed: {e}")))?;
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// Attempt an auto-repair: create branch, check + apply diff, verify, push, open PR.
///
/// Returns Ok(()) if repair succeeded. On any failure, rolls back and returns Ok(())
/// (failed repair is not an error — it's a signal to escalate to human).
pub fn attempt_auto_repair(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), TriageError> {
    let branch = repair_branch_name(&failure.test_name);

    tracing::info!(
        target: "cns.qa.repair_attempted",
        crate_name = %failure.crate_name,
        test_name = %failure.test_name,
        confidence = diagnosis.confidence,
    );

    // 1. Create branch
    run_git(&["checkout", "-b", &branch])?;

    // 2. Check that diff applies cleanly
    let diff = diagnosis.proposed_fix.as_bytes();
    let mut check = Command::new("git")
        .args(["apply", "--check"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| TriageError::Git(format!("git apply --check spawn failed: {e}")))?;

    // Write diff to stdin...
    use std::io::Write;
    if let Some(ref mut stdin) = check.stdin {
        stdin.write_all(diff).map_err(|e| TriageError::Io(e))?;
    }
    let check_status = check
        .wait()
        .map_err(|e| TriageError::Git(format!("git apply --check wait failed: {e}")))?;

    if !check_status.success() {
        tracing::warn!(target: "cns.qa", "Diff does not apply cleanly — rolling back");
        rollback_repair(&branch)?;
        return Ok(());
    }

    // 3. Apply the fix
    run_git_with_stdin(&["apply"], &diagnosis.proposed_fix)?;

    // 4. Verify — run bolero tests
    let test_ok = Command::new("cargo")
        .args(["bolero", "test", "--all"])
        .status()
        .map_err(|e| TriageError::Git(format!("cargo bolero test failed: {e}")))?;

    if !test_ok.success() {
        tracing::warn!(target: "cns.qa", "Verification failed — rolling back");
        rollback_repair(&branch)?;
        return Ok(());
    }

    // 5. Commit, push, open PR
    run_git(&["add", "-A"])?;
    run_git(&[
        "commit",
        "-m",
        &format!(
            "auto-heal: {} in {} (confidence: {:.2})",
            failure.test_name, failure.crate_name, diagnosis.confidence
        ),
    ])?;
    run_git(&["push", "-u", "origin", &branch])?;

    // 6. Open PR
    open_pull_request(failure, diagnosis, &branch)?;

    tracing::info!(target: "cns.qa.repair_verified", "Repair verified and pushed");

    Ok(())
}

fn rollback_repair(branch: &str) -> Result<(), TriageError> {
    run_git(&["checkout", "--", "."])?;
    run_git(&["checkout", "-"])?; // back to previous branch
    run_git(&["branch", "-D", branch])?;
    Ok(())
}

fn repair_branch_name(test_name: &str) -> String {
    let slug: String = test_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("auto-heal/{slug}")
}

fn run_git(args: &[&str]) -> Result<(), TriageError> {
    let status = Command::new("git")
        .args(args)
        .status()
        .map_err(|e| TriageError::Git(format!("git {} failed: {e}", args.join(" "))))?;
    if !status.success() {
        return Err(TriageError::Git(format!(
            "git {} returned non-zero",
            args.join(" ")
        )));
    }
    Ok(())
}

fn run_git_with_stdin(args: &[&str], stdin_text: &str) -> Result<(), TriageError> {
    use std::io::Write;
    let mut child = Command::new("git")
        .args(args)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| TriageError::Git(format!("git {} spawn failed: {e}", args.join(" "))))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_text.as_bytes())
            .map_err(TriageError::Io)?;
    }
    let status = child
        .wait()
        .map_err(|e| TriageError::Git(format!("git {} wait failed: {e}", args.join(" "))))?;
    if !status.success() {
        return Err(TriageError::Git(format!(
            "git {} returned non-zero",
            args.join(" ")
        )));
    }
    Ok(())
}

// ── CNS span emission ────────────────────────────────────────────────────────

/// Emit CNS span for a classified bolero failure.
pub fn emit_cns_span(failure: &BoleroFailure, diagnosis: &QaDiagnosis) {
    tracing::info!(
        target: "cns.qa.bolero_failure",
        crate_name = %failure.crate_name,
        test_name = %failure.test_name,
        failure_type = %diagnosis.failure_type,
        root_cause = %diagnosis.root_cause,
        confidence = diagnosis.confidence,
        is_flake = diagnosis.is_flake,
        suggested_fuzz_target = %diagnosis.suggested_fuzz_target,
    );
}

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum TriageError {
    #[error("classifier error: {0}")]
    Classifier(String),
    #[error("git error: {0}")]
    Git(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

// ── GitHub integration (gh CLI) ──────────────────────────────────────────────

/// Open an auto-repair pull request via `gh pr create`.
pub fn open_pull_request(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
    branch: &str,
) -> Result<(), TriageError> {
    let title = format!(
        "auto-heal: {} in {} (confidence: {:.2})",
        failure.test_name, failure.crate_name, diagnosis.confidence
    );
    let body = format!(
        "## Auto-repair for bolero fuzz failure\n\n\
         **Crate:** {crate}\n\
         **Test:** {test}\n\
         **Failure type:** {ftype}\n\
         **Root cause:** {cause}\n\
         **Confidence:** {conf:.2}\n\
         **Suggested fuzz target:** {fuzz}\n\n\
         ### Proposed fix\n```diff\n{fix}\n```\n",
        crate = failure.crate_name,
        test = failure.test_name,
        ftype = diagnosis.failure_type,
        cause = diagnosis.root_cause,
        conf = diagnosis.confidence,
        fuzz = diagnosis.suggested_fuzz_target,
        fix = diagnosis.proposed_fix,
    );

    let output = Command::new("gh")
        .args([
            "pr", "create", "--title", &title, "--body", &body, "--base", "main", "--head", branch,
        ])
        .output()
        .map_err(|e| TriageError::Git(format!("gh pr create failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TriageError::Git(format!("gh pr create: {stderr}")));
    }

    let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    tracing::info!(target: "cns.qa.repair_verified", pr_url = %pr_url, "Auto-repair PR opened");
    Ok(())
}

/// Open an issue with classifier suggestion (medium confidence).
pub fn open_issue_with_suggestion(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), TriageError> {
    let title = format!(
        "[QA] Fuzz failure: {} in {}",
        failure.test_name, failure.crate_name
    );
    let body = format!(
        "## Bolero fuzz failure\n\n\
         **Crate:** {crate}\n\
         **Test:** {test}\n\
         **Failure type:** {ftype}\n\
         **Root cause (LLM):** {cause}\n\
         **Confidence:** {conf:.2}\n\
         **Failing input:** `{input}`\n\n\
         ### Suggested fix\n```diff\n{fix}\n```\n\n\
         ### Suggested fuzz target\n{fuzz}\n",
        crate = failure.crate_name,
        test = failure.test_name,
        ftype = diagnosis.failure_type,
        cause = diagnosis.root_cause,
        conf = diagnosis.confidence,
        input = failure.failing_input,
        fix = diagnosis.proposed_fix,
        fuzz = diagnosis.suggested_fuzz_target,
    );

    let output = Command::new("gh")
        .args(["issue", "create", "--title", &title, "--body", &body])
        .output()
        .map_err(|e| TriageError::Git(format!("gh issue create failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(target: "cns.qa", error = %stderr, "Failed to open issue");
    }
    Ok(())
}

/// Open an issue for human investigation (low confidence).
pub fn open_issue_for_investigation(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), TriageError> {
    let title = format!(
        "[QA] Investigate fuzz failure: {} in {}",
        failure.test_name, failure.crate_name
    );
    let body = format!(
        "## Bolero fuzz failure — needs human investigation\n\n\
         **Crate:** {crate}\n\
         **Test:** {test}\n\
         **Panic:** {panic}\n\
         **Failing input:** `{input}`\n\
         **LLM diagnosis (low confidence {conf:.2}):** {cause}\n\
         **Failure type:** {ftype}\n",
        crate = failure.crate_name,
        test = failure.test_name,
        panic = failure.panic_message,
        input = failure.failing_input,
        conf = diagnosis.confidence,
        cause = diagnosis.root_cause,
        ftype = diagnosis.failure_type,
    );

    let output = Command::new("gh")
        .args(["issue", "create", "--title", &title, "--body", &body])
        .output()
        .map_err(|e| TriageError::Git(format!("gh issue create failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(target: "cns.qa", error = %stderr, "Failed to open investigation issue");
    }
    Ok(())
}

/// Open an issue when classifier returns unparseable JSON.
pub fn open_raw_failure_issue(
    failure: &BoleroFailure,
    raw_output: &str,
) -> Result<(), TriageError> {
    let title = format!(
        "[QA] Unparseable classifier output: {} in {}",
        failure.test_name, failure.crate_name
    );
    let body = format!(
        "## Classifier returned unparseable JSON\n\n\
         **Crate:** {crate}\n\
         **Test:** {test}\n\
         **Panic:** {panic}\n\
         **Failing input:** `{input}`\n\n\
         ### Raw classifier output\n```\n{raw}\n```\n",
        crate = failure.crate_name,
        test = failure.test_name,
        panic = failure.panic_message,
        input = failure.failing_input,
        raw = raw_output,
    );

    let output = Command::new("gh")
        .args(["issue", "create", "--title", &title, "--body", &body])
        .output()
        .map_err(|e| TriageError::Git(format!("gh issue create failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(target: "cns.qa", error = %stderr, "Failed to open raw failure issue");
    }
    Ok(())
}

// ── Feedback loops ─────────────────────────────────────────────────────────

/// Feed a rejected repair back to the classifier as a correction passage.
/// Call this when a human closes an auto-repair PR without merging.
pub fn feed_rejected_repair(
    original_failure: &BoleroFailure,
    incorrect_diagnosis: &QaDiagnosis,
    correct_fix: &str,
) -> String {
    format!(
        "CORRECTION:\n\
         Original failure: {failure}\n\
         You diagnosed: {incorrect}\n\
         Correct diagnosis: {correct}\n\
         Learn from this discrepancy.",
        failure = original_failure.to_passage(),
        incorrect = incorrect_diagnosis.root_cause,
        correct = correct_fix,
    )
}

/// Format surviving mutants as passages for fuzz target suggestion.
pub fn format_mutant_for_suggestion(
    crate_name: &str,
    file: &str,
    line: u32,
    original: &str,
    mutated: &str,
) -> String {
    format!(
        "CRATE: {crate}\nFILE: {file}\nLINE: {line}\n\
         MUTATION: changed {original} to {mutated}\n\
         Suggest a fuzz target that would catch this mutant.",
        crate = crate_name,
    )
}
