//! QA commands — fuzz triage
//!
//! Reads bolero output from stdin (piped from `cargo bolero test`) and
//! produces a triage report. Classifier integration (Gemma 4 via
//! `classify_batch`) is planned for future — the plumbing is in
//! `hkask-test-harness::triage`.

use crate::cli::QaAction;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub fn run(action: QaAction) {
    match action {
        QaAction::Triage { input } => {
            if let Err(e) = triage(input) {
                eprintln!("QA triage error: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn triage(input_path: Option<std::path::PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
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

    println!("[QA] {} bolero failure(s) detected:\n", failures.len());
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

    // Emit CNS QA spans for each failure
    for f in &failures {
        tracing::info!(
            target: "cns.qa.bolero_failure",
            crate_name = %f.crate_name,
            test_name = %f.test_name,
            "Bolero fuzz failure detected"
        );
    }

    println!(
        "[QA] {} failure(s) reported. Run with DEEPINFRA_API_KEY set for LLM triage.",
        failures.len()
    );

    Ok(())
}
