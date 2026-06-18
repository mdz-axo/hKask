//! Gas Report — query/aggregation layer over the NuEventStore for CNS gas consumption data.
//!
//! Provides tools for querying gas events (reserved, settled, depleted) across agents
//! and time windows, aggregating into per-agent summaries and grand-total reports.
//!
//! ## Design
//!
//! - **Query layer**: reads raw events from the NuEventStore via [`NuEventStore::query_algedonic`].
//! - **Aggregation**: groups events by agent and tool, sums reserved/consumed/depleted metrics.
//! - **Limitation**: GasDepleted events use `Phase::Sense` and are not captured by `query_algedonic`
//!   (which filters `phase = 'act'`). A future iteration may add a dedicated query method.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let report = GasReport::new(store);
//! let summary = report.query_by_agent(&agent_webid, since, until)?;
//! let totals = report.query_total(since, until)?;
//! ```

use chrono::{DateTime, Utc};
use hkask_storage::NuEventStore;
use hkask_types::InfrastructureError;
use hkask_types::event::NuEvent;
use hkask_types::id::WebID;
use std::collections::HashMap;
use std::sync::Arc;

use crate::dynamic_gas_table::DynamicGasTable;

// ── Public report types ──────────────────────────────────────────────────────

/// Per-tool gas consumption breakdown.
///
/// Aggregates gas events (reserved, settled, depleted) for a single tool
/// across all invocations within a time window.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolGasBreakdown {
    /// The tool name (e.g., `"web_search"`, `"condenser"`, `"inference"`)
    pub tool: String,
    /// Total gas reserved across all invocations of this tool.
    pub reserved: u64,
    /// Total gas consumed (settled) across all invocations of this tool.
    pub consumed: u64,
    /// Total gas depleted (budget exhausted before settlement) across all invocations.
    pub depleted: u64,
    /// Number of invocations of this tool within the window.
    pub invocations: u64,
}

/// Per-agent gas consumption summary.
///
/// Aggregates gas events for a single agent across all tools within a time window.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentGasSummary {
    /// The agent this summary pertains to.
    pub agent: WebID,
    /// Total gas reserved across all tools for this agent.
    pub total_reserved: u64,
    /// Total gas consumed (settled) across all tools for this agent.
    pub total_consumed: u64,
    /// Total gas depleted across all tools for this agent.
    pub total_depleted: u64,
    /// Per-tool breakdowns — one entry per distinct tool.
    pub tools: Vec<ToolGasBreakdown>,
    /// Start of the query window (inclusive).
    pub window_start: DateTime<Utc>,
    /// End of the query window (exclusive).
    pub window_end: DateTime<Utc>,
}

/// Complete gas report aggregating all agents.
///
/// Includes per-agent summaries and grand-total aggregates across all agents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentGasReport {
    /// Per-agent gas summaries, sorted descending by total_consumed.
    pub agents: Vec<AgentGasSummary>,
    /// Grand-total aggregates across all agents.
    pub totals: GasTotals,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
    /// Start of the query window (inclusive).
    pub window_start: DateTime<Utc>,
    /// End of the query window (exclusive).
    pub window_end: DateTime<Utc>,
}

/// Grand-total gas aggregates across all agents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GasTotals {
    /// Total gas reserved across all agents and all tools.
    pub total_reserved: u64,
    /// Total gas consumed (settled) across all agents and all tools.
    pub total_consumed: u64,
    /// Total gas depleted across all agents and all tools.
    pub total_depleted: u64,
    /// Number of distinct agents with at least one gas event.
    pub distinct_agents: u64,
    /// Total number of invocations across all agents and all tools.
    pub total_invocations: u64,
}

// ── Private gas event kind for classification ───────────────────────────────

/// Classifies a gas event by the type of gas operation it represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GasEventKind {
    /// Gas was reserved for a future operation.
    Reserved,
    /// Gas was settled after an operation completed.
    Settled,
    /// Gas budget was depleted (exhausted before settlement).
    Depleted,
}

// ── GasReport struct ────────────────────────────────────────────────────────────

/// Query and aggregation layer for CNS gas consumption data.
///
/// Wraps the [`NuEventStore`] and provides methods for querying gas events
/// by agent, by time window, and aggregating into reports.
#[derive(Clone)]
pub struct GasReport {
    store: Arc<NuEventStore>,
}

impl GasReport {
    /// Create a new GasReport backed by the given event store.
    ///
    /// # Arguments
    /// * `store` — An `Arc<NuEventStore>` providing access to raw CNS events.
    pub fn new(store: Arc<NuEventStore>) -> Self {
        Self { store }
    }

    /// Query gas events for a single agent within a time window.
    ///
    /// Filters raw events by agent WebID and aggregates into an [`AgentGasSummary`].
    ///
    /// # Arguments
    /// * `agent` — The agent to query gas data for.
    /// * `since` — Start of the query window (inclusive).
    /// * `until` — End of the query window (exclusive).
    ///
    /// # Returns
    /// * `Ok(AgentGasSummary)` — Aggregated gas data for the agent.
    /// * `Err(InfrastructureError)` — If the underlying store query fails.
    pub fn query_by_agent(
        &self,
        agent: &WebID,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<AgentGasSummary, InfrastructureError> {
        let events = self.query_gas_events(since, until)?;
        let filtered: Vec<NuEvent> = events
            .into_iter()
            .filter(|ev| ev.observer_webid == *agent)
            .collect();
        Self::aggregate_agent_events(agent, &filtered, since, until)
    }

    /// Query gas events for all agents within a time window.
    ///
    /// Groups events by observer WebID, aggregates per-agent, and sorts
    /// descending by total consumed.
    ///
    /// # Arguments
    /// * `since` — Start of the query window (inclusive).
    /// * `until` — End of the query window (exclusive).
    ///
    /// # Returns
    /// * `Ok(Vec<AgentGasSummary>)` — Sorted per-agent summaries.
    /// * `Err(InfrastructureError)` — If the underlying store query fails.
    pub fn query_all_agents(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<AgentGasSummary>, InfrastructureError> {
        let events = self.query_gas_events(since, until)?;

        // Group events by observer WebID
        let mut grouped: HashMap<WebID, Vec<NuEvent>> = HashMap::new();
        for ev in events {
            grouped.entry(ev.observer_webid).or_default().push(ev);
        }

        // Aggregate each group into an AgentGasSummary
        let mut summaries: Vec<AgentGasSummary> = grouped
            .into_iter()
            .map(|(agent, agent_events)| {
                Self::aggregate_agent_events(&agent, &agent_events, since, until)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Sort descending by total_consumed
        summaries.sort_by(|a, b| b.total_consumed.cmp(&a.total_consumed));

        Ok(summaries)
    }

    /// Query grand-total gas aggregates across all agents.
    ///
    /// Sums reserved, consumed, and depleted across all agents within the window.
    ///
    /// # Arguments
    /// * `since` — Start of the query window (inclusive).
    /// * `until` — End of the query window (exclusive).
    ///
    /// # Returns
    /// * `Ok(GasTotals)` — Grand-total aggregates.
    /// * `Err(InfrastructureError)` — If the underlying store query fails.
    pub fn query_total(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<GasTotals, InfrastructureError> {
        let events = self.query_gas_events(since, until)?;

        // Count distinct agents
        let mut seen_agents: HashMap<WebID, bool> = HashMap::new();
        let mut total_reserved: u64 = 0;
        let mut total_consumed: u64 = 0;
        let mut total_depleted: u64 = 0;
        let mut total_invocations: u64 = 0;

        for ev in &events {
            seen_agents.entry(ev.observer_webid).or_insert(true);
            let kind = classify_event_kind(ev);
            match kind {
                GasEventKind::Reserved => {
                    total_reserved += extract_cost(ev);
                }
                GasEventKind::Settled => {
                    total_reserved += extract_reserved(ev);
                    total_consumed += extract_actual(ev);
                }
                GasEventKind::Depleted => {
                    total_depleted += extract_cost(ev);
                }
            }
            total_invocations += 1;
        }

        Ok(GasTotals {
            total_reserved,
            total_consumed,
            total_depleted,
            distinct_agents: seen_agents.len() as u64,
            total_invocations,
        })
    }

    /// Feed settled gas observations into a DynamicGasTable and calibrate it.
    ///
    /// REQ: GAS-CALIB-003 — GasReport settled events feed DynamicGasTable
    /// pre:  `table` is a valid DynamicGasTable
    /// post: every `cns.gas.settled` event in [since, until) with a server field
    ///       is recorded in `table`; returns the number of servers adjusted
    ///
    /// Iterates over `cns.gas.settled` events in the window and calls
    /// `DynamicGasTable::record_observation(server, reserved, actual)` for each.
    /// After all observations are recorded, `DynamicGasTable::calibrate()` is invoked
    /// and the number of adjusted servers is returned.
    ///
    /// # Arguments
    /// * `table` — The `DynamicGasTable` to feed and calibrate.
    /// * `since` — Start of the query window (inclusive).
    /// * `until` — End of the query window (exclusive).
    ///
    /// # Returns
    /// * `Ok(usize)` — Number of servers whose costs were adjusted.
    /// * `Err(InfrastructureError)` — If the underlying store query fails.
    pub fn calibrate_table(
        &self,
        table: &mut DynamicGasTable,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<usize, InfrastructureError> {
        let events = self.query_gas_events(since, until)?;
        for ev in &events {
            if classify_event_kind(ev) == GasEventKind::Settled {
                let server = extract_server_name(ev);
                let reserved = extract_reserved(ev);
                let actual = extract_actual(ev);
                table.record_observation(&server, reserved, actual);
            }
        }
        Ok(table.calibrate())
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Query gas events from the underlying store within a time window.
    ///
    /// Uses [`NuEventStore::query_algedonic`] to fetch events with `span_category = 'gas'`
    /// and `phase = 'act'`, then filters to only gas-related span kinds.
    ///
    /// **Limitation**: GasDepleted events use `Phase::Sense` and will not appear in
    /// algedonic results. Only GasReserved and GasSettled events are returned.
    fn query_gas_events(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<NuEvent>, InfrastructureError> {
        // Use a large limit to get all events in the window.
        // The algedonic query filters span_category IN ('gas', 'variety', 'agent_pod', ...)
        // with phase = 'act'.
        const LARGE_LIMIT: u64 = 10000;
        let raw_events = self.store.query_algedonic(since, LARGE_LIMIT)?;

        // Filter to only events within our time window and with gas span kinds
        let gas_events: Vec<NuEvent> = raw_events
            .into_iter()
            .filter(|ev| ev.timestamp >= since && ev.timestamp < until && is_gas_event(ev))
            .collect();

        Ok(gas_events)
    }

    /// Aggregate gas events for a single agent into an AgentGasSummary.
    ///
    /// Groups events by tool name, sums reserved/consumed/depleted,
    /// counts invocations, and computes per-tool breakdowns.
    fn aggregate_agent_events(
        agent: &WebID,
        events: &[NuEvent],
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<AgentGasSummary, InfrastructureError> {
        // Group events by tool name
        let mut tool_map: HashMap<String, ToolGasBreakdown> = HashMap::new();

        for ev in events {
            let tool_name = extract_tool_name(ev);
            let entry = tool_map
                .entry(tool_name)
                .or_insert_with(|| ToolGasBreakdown {
                    tool: extract_tool_name(ev),
                    reserved: 0,
                    consumed: 0,
                    depleted: 0,
                    invocations: 0,
                });

            let kind = classify_event_kind(ev);
            match kind {
                GasEventKind::Reserved => {
                    entry.reserved += extract_cost(ev);
                }
                GasEventKind::Settled => {
                    entry.reserved += extract_reserved(ev);
                    entry.consumed += extract_actual(ev);
                }
                GasEventKind::Depleted => {
                    entry.depleted += extract_cost(ev);
                }
            }
            entry.invocations += 1;
        }

        // Compute totals from tool breakdowns
        let mut total_reserved = 0u64;
        let mut total_consumed = 0u64;
        let mut total_depleted = 0u64;
        let tools: Vec<ToolGasBreakdown> = tool_map.into_values().collect();

        for t in &tools {
            total_reserved += t.reserved;
            total_consumed += t.consumed;
            total_depleted += t.depleted;
        }

        Ok(AgentGasSummary {
            agent: *agent,
            total_reserved,
            total_consumed,
            total_depleted,
            tools,
            window_start: since,
            window_end: until,
        })
    }
}
fn classify_event_kind(event: &NuEvent) -> GasEventKind {
    match event.span.as_str() {
        "cns.gas.reserved" => GasEventKind::Reserved,
        "cns.gas.settled" => GasEventKind::Settled,
        "cns.gas.depleted" => GasEventKind::Depleted,
        _ => GasEventKind::Reserved,
    }
}

fn is_gas_event(event: &NuEvent) -> bool {
    let s = event.span.as_str();
    s == "cns.gas.reserved" || s == "cns.gas.settled" || s == "cns.gas.depleted"
}

fn extract_server_name(event: &NuEvent) -> String {
    event
        .observation
        .get("server")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_tool_name(event: &NuEvent) -> String {
    event
        .observation
        .get("tool")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_cost(event: &NuEvent) -> u64 {
    event
        .observation
        .get("estimated_cost")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

fn extract_reserved(event: &NuEvent) -> u64 {
    event
        .observation
        .get("reserved")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

fn extract_actual(event: &NuEvent) -> u64 {
    event
        .observation
        .get("actual")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::{Phase, Span, SpanKind};
    use hkask_types::id::WebID;
    use proptest::prelude::*;

    fn test_agent() -> WebID {
        WebID::new()
    }

    fn make_gas_event(
        agent: &WebID,
        kind: SpanKind,
        server: &str,
        tool: &str,
        cost: u64,
    ) -> NuEvent {
        let (obs, phase) = match kind {
            SpanKind::GasReserved => (
                serde_json::json!({"server": server, "tool": tool, "estimated_cost": cost}),
                Phase::Act,
            ),
            SpanKind::GasSettled => {
                let actual = cost / 2;
                (
                    serde_json::json!({
                        "server": server,
                        "tool": tool,
                        "reserved": cost,
                        "actual": actual,
                        "refunded": cost - actual,
                    }),
                    Phase::Act,
                )
            }
            SpanKind::GasDepleted => (
                serde_json::json!({"server": server, "tool": tool, "estimated_cost": cost}),
                Phase::Sense,
            ),
            _ => unreachable!("unexpected span kind"),
        };
        NuEvent::new(*agent, Span::from_kind(kind), phase, obs, 0)
    }

    proptest! {
        fn gas_report_001_insert_known_events_query_by_agent(
            tool_name in "[a-z_]{4,12}",
            cost in 1u64..10000u64,
            count in 1usize..20usize,
        ) {
            let agent = test_agent();
            let tool = tool_name.clone();
            let mut events = Vec::new();
            for _ in 0..count {
                events.push(make_gas_event(
                    &agent,
                    SpanKind::GasReserved,
                    "hkask-mcp-test",
                    &tool,
                    cost,
                ));
            }
            let computed_reserved: u64 = events.iter().map(extract_cost).sum();
            prop_assert_eq!(computed_reserved, cost * count as u64);
        }

        fn gas_report_003_multiple_agents_sorted_descending(
            cost_a in 1u64..500u64,
            cost_b in 1u64..500u64,
        ) {
            let a1 = test_agent();
            let b1 = test_agent();
            let ev_a = make_gas_event(&a1, SpanKind::GasReserved, "hkask-mcp-test", "search", cost_a);
            let ev_b = make_gas_event(&b1, SpanKind::GasReserved, "hkask-mcp-test", "search", cost_b);
            prop_assert_eq!(extract_cost(&ev_a), cost_a);
            prop_assert_eq!(extract_cost(&ev_b), cost_b);
        }
    }

    // REQ: GAS-REPORT-001 — GasTotals defaults to zero aggregates
    #[test]
    fn gas_report_002_empty_store_returns_zero() {
        let totals = GasTotals {
            total_reserved: 0,
            total_consumed: 0,
            total_depleted: 0,
            distinct_agents: 0,
            total_invocations: 0,
        };
        assert_eq!(totals.total_reserved, 0);
        assert_eq!(totals.total_consumed, 0);
        assert_eq!(totals.total_depleted, 0);
    }

    // REQ: GAS-REPORT-002 — classify_event_kind recognizes reserved spans
    #[test]
    fn test_classify_event_kind_reserved() {
        let agent = test_agent();
        let event = make_gas_event(&agent, SpanKind::GasReserved, "hkask-mcp-test", "grep", 42);
        let kind = classify_event_kind(&event);
        assert_eq!(kind, GasEventKind::Reserved);
    }

    // REQ: GAS-REPORT-003 — classify_event_kind recognizes settled spans
    #[test]
    fn test_classify_event_kind_settled() {
        let agent = test_agent();
        let event = make_gas_event(&agent, SpanKind::GasSettled, "hkask-mcp-test", "grep", 100);
        let kind = classify_event_kind(&event);
        assert_eq!(kind, GasEventKind::Settled);
    }

    // REQ: GAS-REPORT-004 — classify_event_kind recognizes depleted spans
    #[test]
    fn test_classify_event_kind_depleted() {
        let agent = test_agent();
        let event = make_gas_event(&agent, SpanKind::GasDepleted, "hkask-mcp-test", "grep", 77);
        let kind = classify_event_kind(&event);
        assert_eq!(kind, GasEventKind::Depleted);
    }
}
