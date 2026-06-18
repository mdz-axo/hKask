//! Proptest strategies for core hKask types.
//!
//! Free functions returning `BoxedStrategy<T>` for property-based testing.
//! The orphan rule prevents implementing `Arbitrary` for external types,
//! so we provide strategy constructors instead. Use with `proptest!`:
//!
//! ```ignore
//! proptest! {
//!     #[test]
//!     fn my_test(e in strategies::any_nu_event()) { ... }
//! }
//! ```
//!
//! # Principle grounding
//! - P8 (Semantic Grounding): strategies generate well-formed values
//! - P5 (Essentialism): one strategy function per type, no duplication

use proptest::sample::select;
use proptest::strategy::{BoxedStrategy, Strategy};

use chrono::Utc;
use hkask_storage::Triple;
use hkask_types::capability::{CapabilitySpec, DelegationAction, DelegationResource};
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::goal::{Goal, GoalState};
use hkask_types::id::{GoalID, WebID};
use hkask_types::transcript::TranscriptSegment;
use hkask_types::visibility::Visibility;
use serde_json::Value;

// ── CNS type strategies ─────────────────────────────────────────────────────────

fn non_empty_string() -> BoxedStrategy<String> {
    proptest::arbitrary::any::<String>()
        .prop_filter("must be non-empty", |s| !s.is_empty())
        .boxed()
}

fn webid_strategy() -> BoxedStrategy<WebID> {
    proptest::arbitrary::any::<[u8; 16]>()
        .prop_map(|bytes| WebID::from_persona(&bytes))
        .boxed()
}

fn span_strategy() -> BoxedStrategy<Span> {
    const NAMESPACES: &[&str] = &[
        "cns.tool",
        "cns.inference",
        "cns.agent_pod",
        "cns.gas",
        "cns.curation",
        "cns.variety",
        "cns.sovereignty",
        "cns.spec",
        "cns.chat",
    ];
    const PATHS: &[&str] = &["invoked", "completed", "error", "sensed", "compared"];

    (select(NAMESPACES), select(PATHS))
        .prop_map(|(ns, path)| Span::new(SpanNamespace::new(ns), path))
        .boxed()
}

fn json_value_strategy() -> BoxedStrategy<Value> {
    non_empty_string().prop_map(Value::String).boxed()
}

// ── Public strategy functions ─────────────────────────────────────────────────

/// Strategy generating valid `NuEvent` instances.
///
/// Produces events with random observer WebIDs, canonical CNS spans,
/// valid phases, string observations, and recursion depth 0–7.
///
/// REQ: HARN-007
/// post: returns `BoxedStrategy<NuEvent>` with valid observer, span, phase, observation, depth 0–7
/// expect: "I can generate valid ν-events with correct observer, canonical CNS spans, and valid phases for property-based testing" [P8]
/// [P5] Constraining: one strategy per type, no duplicate generators
pub fn any_nu_event() -> BoxedStrategy<NuEvent> {
    (
        webid_strategy(),
        span_strategy(),
        select(&[Phase::Sense, Phase::Compute, Phase::Compare, Phase::Act]),
        json_value_strategy(),
        (0u8..7u8),
    )
        .prop_map(|(observer, span, phase, observation, depth)| {
            NuEvent::new(observer, span, phase, observation, depth)
        })
        .boxed()
}

/// Strategy generating valid `Triple` instances.
///
/// Produces triples with random entity/attribute strings,
/// string JSON values, and random owner WebIDs.
///
/// REQ: HARN-008
/// post: returns `BoxedStrategy<Triple>` with non-empty entity, attribute, value, owner
/// expect: "I can generate valid RDF triples with non-empty entities, attributes, and authenticated owners for property-based testing" [P8]
/// [P12] Constraining: every triple carries an owner WebID — no anonymous agency
pub fn any_triple() -> BoxedStrategy<Triple> {
    (
        non_empty_string(),
        non_empty_string(),
        json_value_strategy(),
        webid_strategy(),
    )
        .prop_map(|(entity, attribute, value, owner)| {
            Triple::new(&entity, &attribute, value, owner)
        })
        .boxed()
}

/// Strategy generating valid `CapabilitySpec` instances.
///
/// Produces specs with random resource types, actions, and resource IDs.
///
/// REQ: HARN-009
/// post: returns BoxedStrategy<CapabilitySpec> with valid resource, action, resource_id
/// expect: "I can generate valid capability specifications with correct resource types and delegation actions for property-based testing" [P8]
/// [P4] Constraining: capabilities encode explicit boundaries — no ambient authority
pub fn any_capability_spec() -> BoxedStrategy<CapabilitySpec> {
    (
        select(&[
            DelegationResource::Tool,
            DelegationResource::Template,
            DelegationResource::Registry,
            DelegationResource::Key,
        ]),
        select(&[
            DelegationAction::Read,
            DelegationAction::Write,
            DelegationAction::Execute,
        ]),
        non_empty_string(),
    )
        .prop_map(|(resource, action, resource_id)| CapabilitySpec {
            resource,
            resource_id,
            action,
        })
        .boxed()
}

/// Strategy generating valid `Goal` instances.
///
/// Produces goals with random text, states, visibility, depth 0–7,
/// and optional display names.
///
/// REQ: HARN-010
/// post: returns BoxedStrategy<Goal> with valid webid, text, state, visibility, depth 0–7
/// expect: "I can generate valid goals with correct WebID ownership, state, and visibility classification for property-based testing" [P8]
/// [P1] Constraining: goals carry user-scoped WebIDs — sovereignty boundary respected
pub fn any_goal() -> BoxedStrategy<Goal> {
    (
        webid_strategy(),
        non_empty_string(),
        select(&[
            GoalState::Pending,
            GoalState::Active,
            GoalState::Completed,
            GoalState::Blocked,
            GoalState::Abandoned,
        ]),
        select(&[Visibility::Private, Visibility::Public]),
        (0u8..7u8),
        proptest::option::of(proptest::arbitrary::any::<String>()),
    )
        .prop_map(
            |(webid, text, state, visibility, depth, display_name)| Goal {
                id: GoalID::new(),
                webid,
                text,
                state,
                visibility,
                created_at: Utc::now(),
                completed_at: None,
                parent_goal_id: None,
                depth,
                display_name,
            },
        )
        .boxed()
}

/// Strategy generating valid `TranscriptSegment` instances.
///
/// Produces segments with random text, start times 0–1hr,
/// and durations 100ms–30s.
///
/// REQ: HARN-011
/// post: returns BoxedStrategy<TranscriptSegment> with non-empty text, start_ms 0–1hr, duration 100ms–30s
/// expect: "I can generate valid transcript segments with temporal ordering invariants (end > start) for property-based testing" [P8]
pub fn any_transcript_segment() -> BoxedStrategy<TranscriptSegment> {
    (non_empty_string(), (0u64..3600000u64), (100u64..30000u64))
        .prop_map(|(text, start_ms, duration_ms)| TranscriptSegment {
            text,
            start_ms,
            end_ms: start_ms + duration_ms,
        })
        .boxed()
}

// ── CNS proptest strategies ───────────────────────────────────────────────────

/// Strategy generating valid `EnergyCost` values (1..10000).
///
/// REQ: HARN-045
/// post: returns BoxedStrategy<EnergyCost> with values in 1..10000
/// expect: "I can generate valid energy costs within bounded ranges for gas-budget property-based testing" [P9]
pub fn any_energy_cost() -> BoxedStrategy<hkask_cns::EnergyCost> {
    (1u64..10000u64).prop_map(hkask_cns::EnergyCost).boxed()
}

/// Strategy generating valid `EnergyBudget` instances with hard limit.
///
/// REQ: HARN-046
/// post: returns BoxedStrategy<EnergyBudget> with cap 100..10000, replenish_rate 1..cap
/// expect: "I can generate valid energy budgets with caps that bound resource consumption for gas-guard property-based testing" [P9]
/// [P4] Constraining: budget caps enforce OCAP boundaries on resource consumption
pub fn any_energy_budget() -> BoxedStrategy<hkask_cns::EnergyBudget> {
    (100u64..10000u64)
        .prop_flat_map(|cap| {
            (1u64..cap).prop_map(move |rate| {
                hkask_cns::EnergyBudget::new(hkask_cns::EnergyCost(cap))
                    .with_replenish_rate(hkask_cns::EnergyCost(rate))
            })
        })
        .boxed()
}

// ── Strategy self-tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // REQ: HARN-007 — NuEvent strategy generates valid events (P8)
    proptest! {
        #[test]
        fn nu_event_strategy_is_valid(e in any_nu_event()) {
            prop_assert!(!e.id.as_uuid().is_nil());
            prop_assert!(!e.observer_webid.as_uuid().is_nil());
            prop_assert!(e.recursion_depth <= 7);
        }
    }

    // REQ: HARN-008 — Triple strategy generates valid triples (P8)
    proptest! {
        #[test]
        fn triple_strategy_is_valid(t in any_triple()) {
            prop_assert!(!t.id.as_uuid().is_nil());
            prop_assert!(!t.entity.is_empty());
            prop_assert!(!t.attribute.is_empty());
        }
    }

    // REQ: HARN-009 — CapabilitySpec strategy generates valid specs (P8)
    proptest! {
        #[test]
        fn capability_spec_strategy_is_valid(spec in any_capability_spec()) {
            prop_assert!(!spec.resource_id.is_empty());
        }
    }

    // REQ: HARN-010 — Goal strategy generates valid goals (P8)
    proptest! {
        #[test]
        fn goal_strategy_is_valid(g in any_goal()) {
            prop_assert!(!g.id.as_uuid().is_nil());
            prop_assert!(!g.webid.as_uuid().is_nil());
            prop_assert!(!g.text.is_empty());
            prop_assert!(g.depth <= 7);
        }
    }

    // REQ: HARN-011 — TranscriptSegment strategy generates valid segments (P8)
    proptest! {
        #[test]
        fn transcript_segment_strategy_is_valid(seg in any_transcript_segment()) {
            prop_assert!(!seg.text.is_empty());
            prop_assert!(seg.end_ms > seg.start_ms);
        }
    }
}
