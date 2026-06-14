//! Integration tests for Yes And mode.
//!
//! REQ: extends without substitution — verifies that Yes And accepts the
//! whole contribution and extends it additively, never replacing content.

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

// REQ: Yes And accepts the whole contribution
#[test]
fn accepts_whole_contribution() {
    let mode = ImprovMode::YesAnd;
    let contrib = make_contribution("We should add a caching layer to improve performance");
    let ctx = make_context();
    let response = mode.respond(&contrib, &ctx);
    match response {
        ImprovResponse::Extended {
            accepted_base,
            extension: _,
        } => {
            assert_eq!(
                accepted_base, "We should add a caching layer to improve performance",
                "Yes And must accept the whole contribution unchanged"
            );
        }
        other => panic!("Expected Extended, got {:?}", other),
    }
}

// REQ: Yes And extension is additive, not substitutive
#[test]
fn extension_is_additive_not_substitutive() {
    let mode = ImprovMode::YesAnd;
    let contrib = make_contribution("Let's use Rust for the backend");
    let ctx = make_context();
    let response = mode.respond(&contrib, &ctx);
    match response {
        ImprovResponse::Extended {
            accepted_base,
            extension,
        } => {
            // The extension should contain the accepted base or build on it.
            // It should NOT replace or contradict it.
            assert!(
                extension.contains("Rust") || extension.contains("backend"),
                "Extension should reference the original topic: {}",
                extension
            );
            // The accepted base must remain intact.
            assert_eq!(accepted_base, "Let's use Rust for the backend");
        }
        other => panic!("Expected Extended, got {:?}", other),
    }
}

// REQ: Yes And works with any contribution content
#[test]
fn works_with_varied_content() {
    let mode = ImprovMode::YesAnd;
    let ctx = make_context();

    let inputs = [
        "I think we need better error messages",
        "The architecture should be event-driven",
        "What if we used a graph database?",
        "", // Empty input edge case
    ];

    for input in &inputs {
        let contrib = make_contribution(input);
        let response = mode.respond(&contrib, &ctx);
        match response {
            ImprovResponse::Extended {
                accepted_base,
                extension,
            } => {
                assert_eq!(accepted_base, *input);
                assert!(!extension.is_empty(), "Extension must not be empty");
            }
            other => panic!("Expected Extended for input '{}', got {:?}", input, other),
        }
    }
}
