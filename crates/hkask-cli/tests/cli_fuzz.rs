//! CLI argument parser fuzz test — Wave 5 Task 5.4
//!
//! Verifies that the CLI argument parser (clap) never panics on
//! arbitrary command-line argument combinations.
//!
//! # Principle grounding
//! - P4 (Clear Boundaries): input surfaces must reject invalid input gracefully, never panic

use clap::Parser;
use hkask_cli::cli::Cli;
use proptest::prelude::*;

// Arbitrary command-line arguments never panic the CLI parser.

proptest! {
    #[test]
    fn cli_parser_never_panics(
        args in prop::collection::vec(proptest::arbitrary::any::<String>(), 0..20),
    ) {
        let args_clone = args.clone();
        let result = std::panic::catch_unwind(|| {
            let _ = Cli::try_parse_from(args);
        });
        prop_assert!(result.is_ok(),
            "CLI parser panicked on args: {:?}", args_clone);
    }
}

// ── QA subcommand integration tests ──────────────────────────────────────

#[test]
fn qa_subcommand_help_parses() {
    let cli = Cli::try_parse_from(["kask", "qa", "--help"]);
    // clap --help returns an error (Exit) by design — it prints help and exits.
    // We just verify it doesn't panic.
    let _ = cli;
}

#[test]
fn qa_triage_help_parses() {
    let cli = Cli::try_parse_from(["kask", "qa", "triage", "--help"]);
    let _ = cli;
}

#[test]
fn qa_suggest_fuzz_help_parses() {
    let cli = Cli::try_parse_from(["kask", "qa", "suggest-fuzz", "--help"]);
    let _ = cli;
}

#[test]
fn qa_triage_with_input_file_parses() {
    let cli = Cli::try_parse_from(["kask", "qa", "triage", "-i", "test.txt"]);
    assert!(cli.is_ok());
}

#[test]
fn qa_suggest_fuzz_with_input_file_parses() {
    let cli = Cli::try_parse_from(["kask", "qa", "suggest-fuzz", "-i", "mutants.txt"]);
    assert!(cli.is_ok());
}

#[test]
fn qa_triage_defaults_to_stdin() {
    let cli = Cli::try_parse_from(["kask", "qa", "triage"]);
    assert!(cli.is_ok());
}

#[test]
fn qa_unknown_subcommand_rejected() {
    let cli = Cli::try_parse_from(["kask", "qa", "nonexistent"]);
    assert!(cli.is_err());
}
