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
