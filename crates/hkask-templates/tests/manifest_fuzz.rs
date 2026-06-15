//! Manifest parser fuzz test — Wave 5 Task 5.1
//!
//! Verifies that YAML manifest parsing never panics on arbitrary input.
//! Tests the core serde_yaml parsing step that all manifest loading goes through.
//!
//! # Principle grounding
//! - P4 (Clear Boundaries): input surfaces must reject invalid input gracefully, never panic

use proptest::prelude::*;

// REQ: FUZ-001 — Manifest parser panic-free (P4)
// Arbitrary input to YAML parser never panics.

proptest! {
    #[test]
    fn yaml_parser_never_panics_on_arbitrary_bytes(
        bytes in prop::collection::vec(proptest::arbitrary::any::<u8>(), 0..100_000),
    ) {
        let result = std::panic::catch_unwind(|| {
            let _: Result<serde_yaml::Value, _> = serde_yaml::from_slice(&bytes);
        });
        prop_assert!(result.is_ok(),
            "YAML parser panicked on {} bytes of arbitrary input", bytes.len());
    }

    #[test]
    fn yaml_parser_never_panics_on_arbitrary_strings(
        input in proptest::arbitrary::any::<String>(),
    ) {
        let result = std::panic::catch_unwind(|| {
            let _: Result<serde_yaml::Value, _> = serde_yaml::from_str(&input);
        });
        prop_assert!(result.is_ok(),
            "YAML parser panicked on string input len={}", input.len());
    }
}
