//! Integration tests for Freestyling mode.
//!
//! REQ: time-bounded, multi-participant — verifies that freestyling sessions
//! enforce time bounds, support multiple participants, and cycle correctly.

use hkask_improv::freestyling::FreestyleSession;
use hkask_improv::protocol::ImprovResponse;
use hkask_types::id::WebID;
use std::time::Duration;

// REQ: IMPROV-FREESTYLING-TESTS-001 — Freestyling enforces time bounds
#[test]
fn enforces_time_bound() {
    let agent = WebID::new();
    let mut session = FreestyleSession::new(vec![agent], Duration::from_millis(50));

    // Should be active initially.
    assert!(!session.is_expired());
    let result = session.cycle("rapid idea", agent);
    assert!(result.is_some());

    // Wait for expiration.
    std::thread::sleep(Duration::from_millis(100));
    assert!(session.is_expired());

    // Should return None after expiration.
    let result = session.cycle("too late", agent);
    assert!(result.is_none(), "Expired session must return None");
}

// REQ: IMPROV-FREESTYLING-TESTS-002 — Freestyling supports multiple participants with round-robin
#[test]
fn multi_participant_round_robin() {
    let a1 = WebID::new();
    let a2 = WebID::new();
    let a3 = WebID::new();
    let mut session = FreestyleSession::new(vec![a1, a2, a3], Duration::from_secs(300));

    // First speaker is a1.
    assert_eq!(session.next_speaker(), a1);
    session.cycle("idea 1", a1);

    // After a1's turn, next should be a2.
    assert_eq!(session.next_speaker(), a2);
    session.cycle("idea 2", a2);

    // After a2's turn, next should be a3.
    assert_eq!(session.next_speaker(), a3);
    session.cycle("idea 3", a3);

    // After a3's turn, wraps back to a1.
    assert_eq!(session.next_speaker(), a1);

    // Should have 3 turns recorded.
    assert_eq!(session.turn_count(), 3);
}

// REQ: IMPROV-FREESTYLING-TESTS-003 — Freestyling produces FreestyleTurn responses with time_remaining
#[test]
fn produces_freestyle_turn_responses() {
    let agent = WebID::new();
    let mut session = FreestyleSession::new(vec![agent], Duration::from_secs(300));

    let result = session.cycle("an associative leap", agent);
    assert!(result.is_some());

    match result.unwrap() {
        ImprovResponse::FreestyleTurn {
            content,
            time_remaining,
        } => {
            assert!(content.contains("freestyle turn"));
            assert!(content.contains("associative leap"));
            assert!(time_remaining > Duration::from_secs(0));
            assert!(time_remaining <= Duration::from_secs(300));
        }
        other => panic!("Expected FreestyleTurn, got {:?}", other),
    }
}

// REQ: IMPROV-FREESTYLING-TESTS-004 — Freestyling session tracks turns correctly
#[test]
fn tracks_turns_correctly() {
    let a1 = WebID::new();
    let a2 = WebID::new();
    let mut session = FreestyleSession::new(vec![a1, a2], Duration::from_secs(300));

    assert_eq!(session.turn_count(), 0);
    session.cycle("turn 1", a1);
    assert_eq!(session.turn_count(), 1);
    session.cycle("turn 2", a2);
    assert_eq!(session.turn_count(), 2);
    session.cycle("turn 3", a1);
    assert_eq!(session.turn_count(), 3);

    // Verify turns are stored.
    assert_eq!(session.turns.len(), 3);
    assert_eq!(session.turns[0].content, "turn 1");
    assert_eq!(session.turns[1].content, "turn 2");
    assert_eq!(session.turns[2].content, "turn 3");
}

// REQ: IMPROV-FREESTYLING-TESTS-005 — Freestyling time_remaining decreases monotonically
#[test]
fn time_remaining_decreases_monotonically() {
    let agent = WebID::new();
    let session = FreestyleSession::new(vec![agent], Duration::from_secs(10));

    let t1 = session.time_remaining();
    std::thread::sleep(Duration::from_millis(100));
    let t2 = session.time_remaining();
    assert!(
        t2 < t1,
        "Time remaining should decrease: {} -> {}",
        t1.as_millis(),
        t2.as_millis()
    );
}
