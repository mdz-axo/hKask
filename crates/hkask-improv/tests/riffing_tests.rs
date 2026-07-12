//! Integration tests for Riffing mode.
//!
//! REQ: diverges and resolves — verifies that riffing diverges from a seed
//! into a solo tangent and resolves according to the return policy.

use hkask_improv::protocol::Contribution;
use hkask_improv::riffing::{RiffOutcome, RiffReturn, diverge, resolve};
use hkask_types::id::WebID;

fn make_contribution(content: &str) -> Contribution {
    Contribution {
        source: WebID::new(),
        content: content.to_string(),
        turn_index: 0,
    }
}

#[test]
fn diverges_into_independent_tangent() {
    let seed = make_contribution("We should consider using a message queue for async tasks");
    let tangent = diverge(&seed);
    assert!(tangent.contains("[riff]"), "Tangent must be marked as riff");
    assert!(
        tangent.contains("message queue") || tangent.contains("async"),
        "Tangent must reference the seed topic"
    );
    assert!(
        tangent.contains("different perspective") || tangent.contains("independently"),
        "Tangent must signal independent exploration"
    );
}

#[test]
fn resolves_return_to_group_with_synthesis() {
    let tangent = make_contribution("Exploring event sourcing as an alternative to message queues");
    let outcome = resolve(&tangent, &RiffReturn::ReturnToGroup, 5);
    match outcome {
        RiffOutcome::Returned { synthesis } => {
            assert!(
                synthesis.contains("riff resolved"),
                "Synthesis must signal resolution"
            );
            assert!(
                synthesis.contains("event sourcing"),
                "Synthesis must reference the tangent topic"
            );
            assert!(
                synthesis.contains("integrating") || synthesis.contains("alternative"),
                "Synthesis must bridge back to group context"
            );
        }
        other => panic!("Expected Returned, got {:?}", other),
    }
}

#[test]
fn spawns_new_thread_with_unique_id() {
    let tangent1 = make_contribution("What if we used a graph database instead?");
    let tangent2 = make_contribution("Another thought: consider event sourcing patterns");
    let outcome1 = resolve(&tangent1, &RiffReturn::SpawnThread, 1);
    let outcome2 = resolve(&tangent2, &RiffReturn::SpawnThread, 1);

    match (outcome1, outcome2) {
        (RiffOutcome::Spawned { thread_id: id1 }, RiffOutcome::Spawned { thread_id: id2 }) => {
            // Each spawn should produce a unique thread ID.
            assert_ne!(id1, id2, "Spawned threads must have unique IDs");
        }
        other => panic!("Expected Spawned for both, got {:?}", other),
    }
}

#[test]
fn respects_step_boundary() {
    let tangent = make_contribution("Deep dive into CQRS and event sourcing patterns");
    let policy = RiffReturn::ReturnAfterSteps { max_steps: 3 };

    // Step 1: still exploring.
    let outcome = resolve(&tangent, &policy, 1);
    match outcome {
        RiffOutcome::Returned { synthesis } => {
            assert!(synthesis.contains("1/3"), "Should show step 1 of 3");
            assert!(
                !synthesis.contains("complete"),
                "Should not signal completion at step 1"
            );
        }
        other => panic!("Expected Returned with progress, got {:?}", other),
    }

    // Step 2: still exploring.
    let outcome = resolve(&tangent, &policy, 2);
    match outcome {
        RiffOutcome::Returned { synthesis } => {
            assert!(synthesis.contains("2/3"), "Should show step 2 of 3");
        }
        other => panic!("Expected Returned with progress, got {:?}", other),
    }

    // Step 3: complete.
    let outcome = resolve(&tangent, &policy, 3);
    match outcome {
        RiffOutcome::Returned { synthesis } => {
            assert!(
                synthesis.contains("complete") || synthesis.contains("resolved after 3"),
                "Should signal completion at max steps: {}",
                synthesis
            );
        }
        other => panic!("Expected Returned with completion, got {:?}", other),
    }

    // Step 4 (beyond max): should still complete gracefully.
    let outcome = resolve(&tangent, &policy, 4);
    match outcome {
        RiffOutcome::Returned { synthesis } => {
            assert!(
                synthesis.contains("complete") || synthesis.contains("resolved after 4"),
                "Should handle beyond-max steps gracefully"
            );
        }
        other => panic!("Expected Returned, got {:?}", other),
    }
}

#[test]
fn handles_empty_seed() {
    let seed = make_contribution("");
    let tangent = diverge(&seed);
    assert!(
        tangent.contains("[riff]"),
        "Must produce riff even for empty seed"
    );
    assert!(!tangent.is_empty(), "Tangent must not be empty");
}
