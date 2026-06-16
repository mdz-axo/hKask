//! Integration tests for Yes But mode.
//!
//! REQ: constrains without contradiction — verifies that Yes But accepts the
//! whole contribution and appends a constraint that narrows without contradicting.

use hkask_improv::ConversationContext;
use hkask_improv::modes::ImprovMode;
use hkask_improv::protocol::{Contribution, ImprovResponse};
use hkask_types::id::WebID;

fn make_contribution(content: &str) -> Contribution {
    Contribution {
        source: WebID::new(),
        content: content.to_string(),
        turn_index: 0,
    }
}

fn make_context() -> ConversationContext {
    ConversationContext::new(WebID::new())
}

// REQ: IMPROV-YES-BUT-TESTS-001 — Yes But accepts the whole contribution
#[test]
fn accepts_whole_contribution() {
    let mode = ImprovMode::YesBut;
    let contrib = make_contribution("We should add caching everywhere for maximum performance");
    let ctx = make_context();
    let response = mode.respond(&contrib, &ctx);
    match response {
        ImprovResponse::Constrained {
            accepted_base,
            constraint: _,
        } => {
            assert_eq!(
                accepted_base, "We should add caching everywhere for maximum performance",
                "Yes But must accept the whole contribution unchanged"
            );
        }
        other => panic!("Expected Constrained, got {:?}", other),
    }
}

// REQ: IMPROV-YES-BUT-TESTS-002 — Yes But constraint narrows, does not contradict
#[test]
fn constraint_narrows_does_not_contradict() {
    let mode = ImprovMode::YesBut;
    let contrib = make_contribution("Let's migrate everything to microservices");
    let ctx = make_context();
    let response = mode.respond(&contrib, &ctx);
    match response {
        ImprovResponse::Constrained {
            accepted_base,
            constraint,
        } => {
            // The constraint should reference the original topic.
            assert!(
                constraint.contains("microservices") || constraint.contains("migrate"),
                "Constraint should reference the original topic: {}",
                constraint
            );
            // The accepted base must remain intact.
            assert_eq!(accepted_base, "Let's migrate everything to microservices");
            // The constraint should NOT directly contradict (no "no", "wrong", "can't").
            let contradiction_markers = ["no,", "wrong", "can't", "impossible", "don't"];
            for marker in &contradiction_markers {
                assert!(
                    !constraint.to_lowercase().contains(marker),
                    "Constraint should not contradict with '{}': {}",
                    marker,
                    constraint
                );
            }
        }
        other => panic!("Expected Constrained, got {:?}", other),
    }
}

// REQ: IMPROV-YES-BUT-TESTS-003 — Yes But works with varied content
#[test]
fn works_with_varied_content() {
    let mode = ImprovMode::YesBut;
    let ctx = make_context();

    let inputs = [
        "We should rewrite the entire codebase",
        "Let's add a new dependency for this feature",
        "The system should be fully synchronous",
        "", // Empty input edge case
    ];

    for input in &inputs {
        let contrib = make_contribution(input);
        let response = mode.respond(&contrib, &ctx);
        match response {
            ImprovResponse::Constrained {
                accepted_base,
                constraint,
            } => {
                assert_eq!(accepted_base, *input);
                assert!(!constraint.is_empty(), "Constraint must not be empty");
            }
            other => panic!(
                "Expected Constrained for input '{}', got {:?}",
                input, other
            ),
        }
    }
}
