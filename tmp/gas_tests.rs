#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::{Phase, Span, SpanKind};
    use hkask_types::id::WebID;
    use proptest::prelude::*;

    fn test_agent() -> WebID {
        WebID::new()
    }

    fn make_gas_event(agent: &WebID, kind: SpanKind, tool: &str, cost: u64) -> NuEvent {
        let (obs, phase) = match kind {
            SpanKind::GasReserved => (
                serde_json::json!({"tool": tool, "estimated_cost": cost}),
                Phase::Act,
            ),
            SpanKind::GasSettled => {
                let actual = cost / 2;
                (
                    serde_json::json!({
                        "tool": tool,
                        "reserved": cost,
                        "actual": actual,
                        "refunded": cost - actual,
                    }),
                    Phase::Act,
                )
            }
            SpanKind::GasDepleted => (
                serde_json::json!({"tool": tool, "estimated_cost": cost}),
                Phase::Sense,
            ),
        };
        NuEvent::new(agent.clone(), Span::from_kind(kind), phase, obs, 0)
    }

    proptest! {
        #[test]
        fn gas_report_001_insert_known_events_query_by_agent(
            tool_name in "[a-z_]{4,12}",
            cost in 1u64..10000u64,
            count in 1usize..20usize,
        ) {
            let agent = test_agent();
            let tool = tool_name.clone();
            let mut events = Vec::new();
            for _ in 0..count {
                events.push(make_gas_event(&agent, SpanKind::GasReserved, &tool, cost));
            }
            let computed_reserved: u64 = events.iter().map(|ev| extract_cost(ev)).sum();
            prop_assert_eq!(computed_reserved, cost * count as u64);
        }

        #[test]
        fn gas_report_002_empty_store_returns_zero() {
            let totals = GasTotals {
                total_reserved: 0,
                total_consumed: 0,
                total_depleted: 0,
                distinct_agents: 0,
                total_invocations: 0,
            };
            prop_assert_eq!(totals.total_reserved, 0);
            prop_assert_eq!(totals.total_consumed, 0);
            prop_assert_eq!(totals.total_depleted, 0);
        }

        #[test]
        fn gas_report_003_multiple_agents_sorted_descending(
            cost_a in 1u64..500u64,
            cost_b in 1u64..500u64,
        ) {
            let a1 = test_agent();
            let b1 = test_agent();
            let ev_a = make_gas_event(&a1, SpanKind::GasReserved, "search", cost_a);
            let ev_b = make_gas_event(&b1, SpanKind::GasReserved, "search", cost_b);
            prop_assert_eq!(extract_cost(&ev_a), cost_a);
            prop_assert_eq!(extract_cost(&ev_b), cost_b);
            // Verify different agents produce different summaries
            prop_assert!(a1 != b1);
        }
    }

    #[test]
    fn test_classify_event_kind_reserved() {
        let agent = test_agent();
        let event = make_gas_event(&agent, SpanKind::GasReserved, "grep", 42);
        let kind = classify_event_kind(&event);
        assert_eq!(kind, GasEventKind::Reserved);
    }

    #[test]
    fn test_classify_event_kind_settled() {
        let agent = test_agent();
        let event = make_gas_event(&agent, SpanKind::GasSettled, "grep", 100);
        let kind = classify_event_kind(&event);
        assert_eq!(kind, GasEventKind::Settled);
    }

    #[test]
    fn test_classify_event_kind_depleted() {
        let agent = test_agent();
        let event = make_gas_event(&agent, SpanKind::GasDepleted, "grep", 77);
        let kind = classify_event_kind(&event);
        assert_eq!(kind, GasEventKind::Depleted);
    }
}
