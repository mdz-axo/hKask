//! Type serialization contract tests — Wave 4 Task 4.1
//!
//! Verifies that all shared hKask types survive JSON serialization
//! round-trips. Uses proptest strategies from hkask-test-harness.
//!
//! # Principle grounding
//! - P4 (Clear Boundaries): serialization format drift breaks all downstream consumers
//! - P8 (Semantic Grounding): each contract asserts a stated behavioral property

use hkask_test_harness::strategies::{any_capability_spec, any_goal, any_nu_event};
use proptest::prelude::*;

// REQ: CTR-001 — Type serialization round-trip (P4, P8)
// All shared types survive JSON serialization round-trips.

proptest! {
    #[test]
    fn nu_event_json_roundtrip(e in any_nu_event()) {
        let json = serde_json::to_string(&e).unwrap();
        let back: hkask_types::event::NuEvent = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(e.id, back.id);
        prop_assert_eq!(e.observer_webid, back.observer_webid);
        prop_assert_eq!(e.recursion_depth, back.recursion_depth);
    }

    // REQ: CTR-001 — goal json roundtrip
    #[test]
    fn goal_json_roundtrip(g in any_goal()) {
        let json = serde_json::to_string(&g).unwrap();
        let back: hkask_types::goal::Goal = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(g.id, back.id);
        prop_assert_eq!(g.text, back.text);
        prop_assert_eq!(g.state, back.state);
    }

    // REQ: CTR-001 — capability spec json roundtrip
    #[test]
    fn capability_spec_json_roundtrip(spec in any_capability_spec()) {
        let json = serde_json::to_string(&spec).unwrap();
        let back: hkask_types::capability::CapabilitySpec = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(spec.resource, back.resource);
        prop_assert_eq!(spec.resource_id, back.resource_id);
        prop_assert_eq!(spec.action, back.action);
    }
}
