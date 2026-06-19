//! Freestyling — Rapid collaborative short-response cycling.
//!
//! Freestyling is time-bounded group exploration with no single owner.
//! The session owns its state — no `Arc<Mutex<>>` unless concurrency
//! is proven necessary.

use crate::ConversationContext;
use crate::protocol::{Contribution, ImprovResponse};
use hkask_types::id::WebID;
use std::time::{Duration, Instant};

/// A freestyle session — owned by the session, not shared.
///
/// Tracks participants, turns, and time bounds. The session is the
/// single source of truth for freestyling state.
#[derive(Debug, Clone)]
pub struct FreestyleSession {
    /// Agents participating in the freestyle.
    pub participants: Vec<WebID>,
    /// All turns taken so far in this session.
    pub turns: Vec<Contribution>,
    /// When the session started.
    pub started_at: Instant,
    /// Maximum duration for the session.
    pub time_bound: Duration,
    /// Index of the next participant to take a turn (round-robin).
    next_speaker: usize,
}

impl FreestyleSession {
    /// Create a new freestyle session.
    ///
    /// # Panics
    /// Panics if `participants` is empty — freestyling requires at least one participant.
    pub fn new(participants: Vec<WebID>, time_bound: Duration) -> Self {
        assert!(
            !participants.is_empty(),
            "FreestyleSession requires at least one participant"
        );
        Self {
            participants,
            turns: Vec::new(),
            started_at: Instant::now(),
            time_bound,
            next_speaker: 0,
        }
    }

    /// Check if the session has exceeded its time bound.
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() >= self.time_bound
    }

    /// Time remaining in the session.
    pub fn time_remaining(&self) -> Duration {
        self.time_bound.saturating_sub(self.started_at.elapsed())
    }

    /// Get the next speaker in round-robin order.
    pub fn next_speaker(&self) -> WebID {
        self.participants[self.next_speaker % self.participants.len()]
    }

    /// Advance to the next speaker.
    pub fn advance_speaker(&mut self) {
        self.next_speaker = (self.next_speaker + 1) % self.participants.len();
    }

    /// Record a turn in the session.
    pub fn record_turn(&mut self, contribution: Contribution) {
        self.turns.push(contribution);
        self.advance_speaker();
    }

    /// Number of turns taken so far.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Cycle the session — produce the next rapid turn.
    ///
    /// Returns a freestyle turn response if the session is still active,
    /// or `None` if the session has expired.
    pub fn cycle(&mut self, content: &str, source: WebID) -> Option<ImprovResponse> {
        if self.is_expired() {
            return None;
        }

        let contribution = Contribution {
            source,
            content: content.to_string(),
            turn_index: self.turn_count(),
        };

        self.record_turn(contribution);

        let rapid = format!(
            "[freestyle turn {} by {}] {}",
            self.turn_count(),
            source.redacted_display(),
            truncate_for_freestyle(content, 80)
        );

        Some(ImprovResponse::FreestyleTurn {
            content: rapid,
            time_remaining: self.time_remaining(),
        })
    }

    /// Build a conversation context from this session for a specific agent.
    pub fn to_context(&self, agent_id: WebID) -> ConversationContext {
        ConversationContext {
            agent_id,
            participants: self.participants.clone(),
            turn_count: self.turn_count(),
            recursion_depth: 0,
        }
    }
}

/// Truncate content for rapid freestyle display.
fn truncate_for_freestyle(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // contract: IMPROV-FREESTYLING-001
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    fn session_expires_after_time_bound() {
        let agent = WebID::new();
        let session = FreestyleSession::new(vec![agent], Duration::from_millis(10));
        assert!(!session.is_expired());
        thread::sleep(Duration::from_millis(20));
        assert!(session.is_expired());
    }

    // contract: IMPROV-FREESTYLING-002
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    fn supports_multiple_participants() {
        let a1 = WebID::new();
        let a2 = WebID::new();
        let a3 = WebID::new();
        let session = FreestyleSession::new(vec![a1, a2, a3], Duration::from_secs(300));
        assert_eq!(session.participants.len(), 3);
    }

    // contract: IMPROV-FREESTYLING-003
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    fn cycles_participants_round_robin() {
        let a1 = WebID::new();
        let a2 = WebID::new();
        let a3 = WebID::new();
        let mut session = FreestyleSession::new(vec![a1, a2, a3], Duration::from_secs(300));

        assert_eq!(session.next_speaker(), a1);
        session.advance_speaker();
        assert_eq!(session.next_speaker(), a2);
        session.advance_speaker();
        assert_eq!(session.next_speaker(), a3);
        session.advance_speaker();
        assert_eq!(session.next_speaker(), a1); // Wraps around
    }

    // contract: IMPROV-FREESTYLING-004
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    fn records_turns_and_increments_count() {
        let a1 = WebID::new();
        let a2 = WebID::new();
        let mut session = FreestyleSession::new(vec![a1, a2], Duration::from_secs(300));

        assert_eq!(session.turn_count(), 0);
        let result = session.cycle("first idea", a1);
        assert!(result.is_some());
        assert_eq!(session.turn_count(), 1);

        let result = session.cycle("second idea", a2);
        assert!(result.is_some());
        assert_eq!(session.turn_count(), 2);
    }

    // contract: IMPROV-FREESTYLING-005
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    fn cycle_returns_none_when_expired() {
        let a1 = WebID::new();
        let mut session = FreestyleSession::new(vec![a1], Duration::from_millis(1));
        thread::sleep(Duration::from_millis(5));
        let result = session.cycle("too late", a1);
        assert!(result.is_none(), "Expired session should return None");
    }

    // contract: IMPROV-FREESTYLING-006
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    #[should_panic(expected = "requires at least one participant")]
    fn panics_on_empty_participants() {
        FreestyleSession::new(vec![], Duration::from_secs(60));
    }

    // contract: IMPROV-FREESTYLING-007
    // expect: "The system supports structured improvisational agent interaction" [P3]
    #[test]
    fn time_remaining_decreases() {
        let a1 = WebID::new();
        let session = FreestyleSession::new(vec![a1], Duration::from_secs(3600));
        let initial = session.time_remaining();
        thread::sleep(Duration::from_millis(50));
        let later = session.time_remaining();
        assert!(later < initial, "Time remaining should decrease");
    }
}
