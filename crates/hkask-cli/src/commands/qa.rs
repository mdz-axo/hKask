//! QA commands — fuzz triage with LLM classifier (Gemma 4).
//!
//! Reads bolero output from stdin, classifies each failure via
//! `classify_batch`, routes by confidence, emits CNS spans.

use crate::cli::QaAction;
use hkask_services_classify::{self, ClassifierConfig};
use hkask_test_harness::triage::{BoleroFailure, TriageReport};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

pub fn run(rt: &tokio::runtime::Runtime, action: QaAction) {
    match action {
        QaAction::Triage { input } => {
            if let Err(e) = rt.block_on(triage(input)) {
                eprintln!("QA triage error: {e}");
                std::process::exit(1);
            }
        }
    }
}

async fn triage(input_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let stdin: Box<dyn BufRead> = match &input_path {
        Some(path) => {
            let file = File::open(path).map_err(|e| format!("Cannot open {path:?}: {e}"))?;
            Box::new(BufReader::new(file))
        }
        None => Box::new(BufReader::new(io::stdin())),
    };

    let failures = hkask_test_harness::triage::parse_bolero_stdin(stdin)?;

    if failures.is_empty() {
        println!("[QA] No bolero failures detected.");
        return Ok(());
    }

    println!("[QA] {} bolero failure(s) detected", failures.len());

    // Try to load classifier config
    let registry_dir = find_registry_dir();
    let config = match hkask_services_classify::load_classifier_config("qa-triage", &registry_dir) {
        Ok(def) => {
            println!("[QA] Classifier loaded: {} via {}", def.model, def.provider);
            Some(ClassifierConfig::from_def(&def))
        }
        Err(e) => {
            eprintln!(
                "[QA] Classifier config not found at {}/classify/qa-triage.yaml: {e}",
                registry_dir.display()
            );
            eprintln!("[QA] Falling back to parse-only mode — no LLM triage.");
            None
        }
    };

    if let Some(cfg) = config {
        if cfg.api_key.is_empty() {
            println!(
                "[QA] No DEEPINFRA_API_KEY set — falling back to parse-only mode.\n\
                 [QA] Set DEEPINFRA_API_KEY for LLM-powered triage."
            );
            print_failures(&failures);
            emit_cns_spans(&failures);
            return Ok(());
        }

        // Classify each failure
        println!("[QA] Classifying {} failure(s) with LLM...", failures.len());
        let passages: Vec<String> = failures.iter().map(|f| f.to_passage()).collect();
        let results = hkask_services_classify::classify_batch(&passages, cfg).await?;

        let mut report = TriageReport::default();

        for (i, result) in results.iter().enumerate() {
            let failure = &failures[i];
            let diagnosis = parse_diagnosis(&result.category);

            match diagnosis {
                Ok(diag) => {
                    emit_cns_span_for(failure, &diag);

                    if diag.is_flake {
                        println!(
                            "  [{i}] FLAKE: {}::{} (skipped)",
                            failure.crate_name, failure.test_name
                        );
                        report.flakes += 1;
                    } else if diag.confidence >= 0.95 {
                        println!(
                            "  [{i}] HIGH confidence ({:.2}): {}::{} — {}\n       auto-repair suggested: {}",
                            diag.confidence,
                            failure.crate_name,
                            failure.test_name,
                            diag.root_cause,
                            if diag.proposed_fix.is_empty() {
                                "none"
                            } else {
                                "yes"
                            }
                        );
                        report.auto_repaired += 1;
                    } else if diag.confidence >= 0.70 {
                        println!(
                            "  [{i}] MEDIUM confidence ({:.2}): {}::{} — {}",
                            diag.confidence, failure.crate_name, failure.test_name, diag.root_cause
                        );
                        report.issues_opened += 1;
                    } else {
                        println!(
                            "  [{i}] LOW confidence ({:.2}): {}::{} — {}\n       investigation needed",
                            diag.confidence, failure.crate_name, failure.test_name, diag.root_cause
                        );
                        report.issues_opened += 1;
                    }

                    if !diag.suggested_fuzz_target.is_empty() {
                        println!(
                            "       suggested fuzz target: {}",
                            diag.suggested_fuzz_target
                        );
                    }
                }
                Err(_) => {
                    println!(
                        "  [{i}] UNPARSEABLE: {}::{} — raw: {}",
                        failure.crate_name,
                        failure.test_name,
                        &result.category.chars().take(80).collect::<String>()
                    );
                    report.unparseable += 1;
                }
            }
        }

        println!(
            "\n[QA] Triage complete: {} auto-repairs, {} issues, {} flakes, {} unparseable",
            report.auto_repaired, report.issues_opened, report.flakes, report.unparseable
        );
    } else {
        print_failures(&failures);
        emit_cns_spans(&failures);
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn find_registry_dir() -> PathBuf {
    // Try in order: env var, CWD-relative, XDG config
    if let Ok(dir) = std::env::var("HKASK_REGISTRY_DIR") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return p;
        }
    }
    let cwd_registry = Path::new("registry");
    if cwd_registry.is_dir() {
        return cwd_registry.to_path_buf();
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("hkask").join("registry")
}

#[derive(Debug, serde::Deserialize)]
struct QaDiagnosis {
    failure_type: String,
    root_cause: String,
    confidence: f64,
    #[serde(default)]
    proposed_fix: String,
    #[serde(default)]
    affected_file: String,
    #[serde(default)]
    affected_line: u32,
    #[serde(default)]
    is_flake: bool,
    #[serde(default)]
    suggested_fuzz_target: String,
}

fn parse_diagnosis(raw: &str) -> Result<QaDiagnosis, serde_json::Error> {
    // Strip markdown code fences if present
    let json = raw
        .trim()
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(raw);
    serde_json::from_str(json)
}

fn emit_cns_span_for(failure: &BoleroFailure, diagnosis: &QaDiagnosis) {
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

fn emit_cns_spans(failures: &[BoleroFailure]) {
    for f in failures {
        tracing::info!(
            target: "cns.qa.bolero_failure",
            crate_name = %f.crate_name,
            test_name = %f.test_name,
            "Bolero fuzz failure detected (parse-only mode)"
        );
    }
}

fn print_failures(failures: &[BoleroFailure]) {
    println!();
    for (i, f) in failures.iter().enumerate() {
        println!(
            "  #{i}: {crate}::{test}\n      panic: {panic}\n      input: {input}\n",
            i = i + 1,
            crate = f.crate_name,
            test = f.test_name,
            panic = f.panic_message,
            input = f.failing_input,
        );
    }
}
