//! Engagement gate — the CAT "speak or remain silent" decision point.
//!
//! Evaluates a Matrix communication event against an agent's convergence bias.
//! The `convergence_bias` IS the decision: high → speak readily, low → stay silent.
//!
//! This is a pure function — no state, no I/O, no crate coupling. Callers
//! pass in the bias and agent name extracted from their persona.
//!
//! Architecture:
//!   CommunicationWatcher → CurationInput::Communication
//!   → per-agent evaluation via evaluate()
//!   → Speak: route to LLM response | Silent: drop

use hkask_cns::types::loops::channels::CommunicationEvent;

/// Outcome of the engagement evaluation.
#[derive(Debug, PartialEq)]
pub enum Decision {
    /// Agent should respond. `convergence_level` mirrors the bias —
    /// it governs how much the agent accommodates the interlocutor.
    Speak { convergence_level: f64 },
    /// Agent should remain silent.
    Silent,
}

/// Evaluate whether an agent should engage with a communication event.
///
/// Convergence bias thresholds:
/// - > 0.0: speaks to direct @mentions (divergent-to-balanced posture)
/// - ≥ 0.7: speaks to any message in a monitored room (convergent posture)
///
/// `invariant_traits` don't affect the speak/silent decision — they govern
/// HOW the agent responds, not WHETHER. Pass them to the response composer.
pub fn evaluate(convergence_bias: f64, agent_name: &str, event: &CommunicationEvent) -> Decision {
    let addressed = body_mentions(event, agent_name);

    if convergence_bias >= 0.7 || (convergence_bias > 0.0 && addressed) {
        Decision::Speak {
            convergence_level: convergence_bias,
        }
    } else {
        Decision::Silent
    }
}

/// Check whether the event's message body mentions the agent by name.
fn body_mentions(event: &CommunicationEvent, agent_name: &str) -> bool {
    event
        .observation
        .get("body")
        .and_then(|v| v.as_str())
        .map(|body| body.contains(agent_name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_with_body(body: &str) -> CommunicationEvent {
        CommunicationEvent {
            span_category: "communication.message".into(),
            span_path: "observed".into(),
            observation: serde_json::json!({
                "room_id": "!test:localhost",
                "sender": "@alice:localhost",
                "body": body,
                "timestamp": 1700000000_i64,
            }),
            observed_at: "2024-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn low_bias_speaks_only_when_mentioned() {
        let event = event_with_body("Hey @curator, what's the status?");
        assert_eq!(
            evaluate(0.1, "curator", &event),
            Decision::Speak {
                convergence_level: 0.1
            }
        );
        assert_eq!(evaluate(0.1, "other-agent", &event), Decision::Silent);
    }

    #[test]
    fn moderate_bias_speaks_when_mentioned() {
        let event = event_with_body("Hey @helper, can you check this?");
        assert_eq!(
            evaluate(0.5, "helper", &event),
            Decision::Speak {
                convergence_level: 0.5
            }
        );
    }

    #[test]
    fn moderate_bias_silent_when_not_mentioned() {
        let event = event_with_body("General discussion about the project");
        assert_eq!(evaluate(0.5, "helper", &event), Decision::Silent);
    }

    #[test]
    fn high_bias_speaks_even_without_mention() {
        let event = event_with_body("General discussion about the project");
        assert_eq!(
            evaluate(0.9, "helper", &event),
            Decision::Speak {
                convergence_level: 0.9
            }
        );
    }

    #[test]
    fn zero_bias_always_silent() {
        let event = event_with_body("@agent urgent help needed!");
        assert_eq!(evaluate(0.0, "agent", &event), Decision::Silent);
    }
}
