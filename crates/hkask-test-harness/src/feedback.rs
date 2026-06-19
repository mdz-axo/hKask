//! QA feedback loops — correction passages and mutant-to-fuzz suggestions.
//!
//! Two feedback paths make the QA system improve over time:
//!
//! Path A — Human-rejected repairs feed back to classifier.
//! When a human closes an auto-repair PR without merging, the rejection
//! reason + correct fix are formatted as a "correction passage" and sent
//! through the `qa-feedback` classifier. This improves future classifications
//! via in-context learning without fine-tuning.
//!
//! Path B — Surviving mutants suggest new fuzz targets.
//! When `cargo mutants` reports uncaught mutants, each surviving mutant's
//! location and mutation are formatted as a passage. The classifier suggests
//! a fuzz target that would catch it. The suggestion is appended to the
//! crate's fuzz file.
//!
//! # Principle grounding
//! - P9 (Homeostatic Self-Regulation): feedback improves system accuracy
//! - P5 (Essentialism): two focused functions, no framework

use crate::triage::{BoleroFailure, QaDiagnosis};

/// Feed a rejected repair back to the classifier.
///
/// Returns a correction passage suitable for `classify_batch` with the
/// `qa-feedback` classifier config.
pub fn correction_passage(
    failure: &BoleroFailure,
    incorrect_diagnosis: &QaDiagnosis,
    correct_diagnosis: &str,
    rejection_reason: &str,
) -> String {
    format!(
        "CORRECTION:\n\
         Original failure: {failure_passage}\n\
         You diagnosed: {incorrect}\n\
         Correct diagnosis: {correct}\n\
         Rejection reason: {reason}\n\
         Learn from this discrepancy.",
        failure_passage = failure.to_passage(),
        incorrect = incorrect_diagnosis.root_cause,
        correct = correct_diagnosis,
        reason = rejection_reason,
    )
}

/// Format a surviving mutant as a fuzz target suggestion passage.
pub fn mutant_passage(
    crate_name: &str,
    file: &str,
    line: u32,
    original_code: &str,
    mutated_code: &str,
) -> String {
    format!(
        "MUTANT:\n\
         CRATE: {crate}\n\
         FILE: {file}\n\
         LINE: {line}\n\
         CHANGED: {original} → {mutated}\n\
         This mutant survived — the test suite didn't catch it.\n\
         Suggest a fuzz target that would catch this mutant.",
        crate = crate_name,
        original = original_code,
        mutated = mutated_code,
    )
}

/// Parse cargo-mutants output into structured surviving mutant records.
///
/// cargo-mutants outputs lines like:
/// `Uncaught mutants in hkask-cns: src/algedonic.rs:42 (changed > to >=)`
pub fn parse_mutant_line(line: &str) -> Option<SurvivingMutant> {
    // "Uncaught mutants in crate_name: path/to/file.rs:NN (changed X to Y)"
    let rest = line.strip_prefix("Uncaught mutants in ")?;
    let (crate_name, rest) = rest.split_once(':')?;
    let crate_name = crate_name.trim().to_string();
    let rest = rest.trim();

    // "path/to/file.rs:NN (changed X to Y)"
    let (file_path, rest) = rest.split_once(':')?;
    let file_path = file_path.trim().to_string();
    let rest = rest.trim();

    // "NN (changed X to Y)"
    let (line_str, rest) = rest.split_once(' ')?;
    let line: u32 = line_str.parse().ok()?;
    let rest = rest.trim();

    // "(changed X to Y)"
    let rest = rest.strip_prefix("(changed ")?.strip_suffix(')')?;
    let (original, mutated) = rest.split_once(" to ")?;

    Some(SurvivingMutant {
        crate_name,
        file: file_path,
        line,
        original: original.trim().to_string(),
        mutated: mutated.trim().to_string(),
    })
}

#[derive(Debug, Clone)]
pub struct SurvivingMutant {
    pub crate_name: String,
    pub file: String,
    pub line: u32,
    pub original: String,
    pub mutated: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mutant_line_valid() {
        let line = "Uncaught mutants in hkask-cns: src/algedonic.rs:42 (changed > to >=)";
        let m = parse_mutant_line(line).unwrap();
        assert_eq!(m.crate_name, "hkask-cns");
        assert_eq!(m.file, "src/algedonic.rs");
        assert_eq!(m.line, 42);
        assert_eq!(m.original, ">");
        assert_eq!(m.mutated, ">=");
    }

    #[test]
    fn parse_mutant_line_invalid() {
        assert!(parse_mutant_line("").is_none());
        assert!(parse_mutant_line("not a mutant line").is_none());
    }

    #[test]
    fn correction_passage_contains_context() {
        let failure = BoleroFailure {
            crate_name: "hkask-types".into(),
            test_name: "fuzz_test".into(),
            panic_message: "panic!".into(),
            stack_trace: "stack".into(),
            source_snippet: "src".into(),
            failing_input: "input".into(),
        };
        let diagnosis = QaDiagnosis {
            failure_type: "Panic".into(),
            root_cause: "wrong cause".into(),
            confidence: 0.95,
            proposed_fix: "diff".into(),
            affected_file: "file.rs".into(),
            affected_line: 1,
            is_flake: false,
            suggested_fuzz_target: "".into(),
        };
        let passage = correction_passage(&failure, &diagnosis, "right cause", "bad fix");
        assert!(passage.contains("wrong cause"));
        assert!(passage.contains("right cause"));
        assert!(passage.contains("bad fix"));
    }

    #[test]
    fn mutant_passage_contains_details() {
        let passage = mutant_passage("test-crate", "src/lib.rs", 42, ">", ">=");
        assert!(passage.contains("test-crate"));
        assert!(passage.contains("src/lib.rs"));
        assert!(passage.contains("42"));
        assert!(passage.contains(">"));
        assert!(passage.contains(">="));
    }
}
