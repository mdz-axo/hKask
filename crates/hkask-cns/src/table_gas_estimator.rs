//! TableGasEstimator — Per-server gas cost table
//!
//! Per-server configurable gas cost table.
//! Each (server, tool) pair maps to a gas cost. Inference tools use
//! token-based estimation via `InferenceGasEstimator`; all other
//! tools use the flat costs from this table.
//!
//! Gas costs are dimensionless units on a shared scale, analogous to
//! Ethereum gas. The principle is: cheap operations cost little, expensive
//! operations cost much. This prevents runaway agents while keeping the
//! implementation minimal.
//!
//! ## Cost Tiers
//!
//! | Tier | Servers | Cost Range | Rationale |
//! |------|---------|------------|----------|
//! | Internal | ocap, keystore, cns, registry | 1-2 | In-process, negligible compute |
//! | Local I/O | spec, git, goal | 5 | Local I/O, no network |
//! | Moderate | condenser | 10 | Some computation + local I/O |
//! | External API | web, github, fmp, telnyx, rss-reader | 20-50 | Network I/O, rate-limited |
//! | Heavy external | fal | 100 | GPU compute, expensive |
//! | Inference | hkask-mcp-inference | 0 (table) | Handled by `InferenceGasEstimator` |
//!
//! Inference uses a token-based cost model (`InferenceGasEstimator`):
//! `prompt_chars / 4 + max_tokens`. This reflects that LLM compute scales
//! with token count, not with a flat per-call cost. The table returns 0 for
//! the inference server as a signal to use the token-based estimator.
//!
//! Unknown servers default to 10 (moderate — conservative middle ground).
//!
//! For production, use `CompositeGasEstimator` which routes inference to
//! `InferenceGasEstimator` and all other tools to this table.

use crate::governed_tool::GasEstimator;
use serde_json::Value;
use std::collections::HashMap;

/// Default gas costs per MCP server.
///
/// These are intentionally conservative — they prevent infinite loops
/// while being simple to understand and calibrate.
pub(crate) fn default_gas_table() -> HashMap<&'static str, u64> {
    let mut table = HashMap::new();
    // Internal tools — cheap
    table.insert("hkask-mcp-ocap", 1);
    table.insert("hkask-mcp-keystore", 2);
    table.insert("hkask-mcp-cns", 1);
    table.insert("hkask-mcp-registry", 2);
    table.insert("hkask-mcp-spec", 5);
    table.insert("hkask-mcp-git", 5);

    // Moderate tools
    table.insert("hkask-mcp-condenser", 10);
    table.insert("hkask-mcp-goal", 5);

    // External API tools — expensive
    table.insert("hkask-mcp-web", 50);
    table.insert("hkask-mcp-github", 30);
    table.insert("hkask-mcp-fmp", 40);
    table.insert("hkask-mcp-telnyx", 50);
    table.insert("hkask-mcp-fal", 100);
    table.insert("hkask-mcp-rss-reader", 20);

    // Inference is handled separately by InferenceGasEstimator
    table.insert("hkask-mcp-inference", 0); // Overridden by InferenceGasEstimator

    table
}

/// Table-based gas estimator with configurable per-server costs.
///
/// # Gas Cost Philosophy
///
/// Gas units are dimensionless — they represent computational cost on a shared
/// scale, analogous to Ethereum gas. The principle is: cheap operations cost
/// little, expensive operations cost much. This prevents runaway agents while
/// keeping the implementation minimal.
///
/// ## Cost Tiers
///
/// | Tier | Servers | Cost Range | Rationale |
/// |------|---------|------------|----------|
/// | Internal | ocap, keystore, cns, registry | 1-2 | In-process, negligible compute |
/// | Local I/O | spec, git, goal | 5 | Local I/O, no network |
/// | Moderate | condenser | 10 | Some computation + local I/O |
/// | External API | web, github, fmp, telnyx, rss-reader | 20-50 | Network I/O, rate-limited |
/// | Heavy external | fal | 100 | GPU compute, expensive |
/// | Inference | hkask-mcp-inference | 0 (table) | Handled by `InferenceGasEstimator` |
///
/// Inference uses a token-based cost model (`InferenceGasEstimator`):
/// `prompt_chars / 4 + max_tokens`. This reflects that LLM compute scales
/// with token count, not with a flat per-call cost.
///
/// Unknown servers default to 10 (moderate — conservative middle ground).
///
/// For production, use `CompositeGasEstimator` which routes inference to
/// `InferenceGasEstimator` and all other tools to this table.
///
/// # Lookup Priority
///
/// Looks up gas cost by server name. If the server has a specific cost,
/// uses that. If not found, falls back to the `default_cost`.
///
/// For tools within a server, you can optionally provide per-tool costs
/// via `with_tool_cost()`. If no per-tool cost is found, the server cost
/// is used.
pub struct TableGasEstimator {
    /// Per-server gas costs.
    server_costs: HashMap<String, u64>,
    /// Per-(server, tool) gas costs (overrides server cost).
    tool_costs: HashMap<(String, String), u64>,
    /// Default cost when neither server nor tool cost is found.
    default_cost: u64,
}

impl TableGasEstimator {
    /// Create a TableGasEstimator with the default gas table.
    pub fn new() -> Self {
        let server_costs: HashMap<String, u64> = default_gas_table()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        Self {
            server_costs,
            tool_costs: HashMap::new(),
            default_cost: 10,
        }
    }

    /// Create a TableGasEstimator with custom server costs.
    pub fn with_server_costs(server_costs: HashMap<String, u64>) -> Self {
        Self {
            server_costs,
            tool_costs: HashMap::new(),
            default_cost: 10,
        }
    }

    /// Set a per-tool gas cost (overrides server cost for specific tools).
    pub fn with_tool_cost(mut self, server: &str, tool: &str, cost: u64) -> Self {
        self.tool_costs
            .insert((server.to_string(), tool.to_string()), cost);
        self
    }

    /// Set the default cost for unknown servers.
    pub fn with_default_cost(mut self, cost: u64) -> Self {
        self.default_cost = cost;
        self
    }

    /// Look up the gas cost for a (server, tool) pair.
    pub fn lookup(&self, server: &str, tool: &str) -> u64 {
        // Per-tool cost takes priority
        if let Some(cost) = self.tool_costs.get(&(server.to_string(), tool.to_string())) {
            return *cost;
        }
        // Then per-server cost
        if let Some(cost) = self.server_costs.get(server) {
            return *cost;
        }
        // Default
        self.default_cost
    }
}

impl Default for TableGasEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl GasEstimator for TableGasEstimator {
    fn estimate_cost(&self, server: &str, tool: &str, _args: &Value) -> u64 {
        self.lookup(server, tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_table_has_all_servers() {
        let table = default_gas_table();
        // Internal tools are cheap
        assert_eq!(table.get("hkask-mcp-ocap"), Some(&1));
        assert_eq!(table.get("hkask-mcp-cns"), Some(&1));
        // External tools are expensive
        assert_eq!(table.get("hkask-mcp-web"), Some(&50));
        assert_eq!(table.get("hkask-mcp-fal"), Some(&100));
        // Inference is 0 (handled by InferenceGasEstimator)
        assert_eq!(table.get("hkask-mcp-inference"), Some(&0));
    }

    #[test]
    fn table_estimator_uses_server_cost() {
        let estimator = TableGasEstimator::new();
        assert_eq!(
            estimator.estimate_cost("hkask-mcp-web", "search", &serde_json::json!({})),
            50
        );
    }

    #[test]
    fn table_estimator_uses_tool_cost_override() {
        let estimator = TableGasEstimator::new().with_tool_cost("hkask-mcp-web", "scrape", 200);
        assert_eq!(
            estimator.estimate_cost("hkask-mcp-web", "scrape", &serde_json::json!({})),
            200
        );
        // Other tools still use server cost
        assert_eq!(
            estimator.estimate_cost("hkask-mcp-web", "search", &serde_json::json!({})),
            50
        );
    }

    #[test]
    fn table_estimator_uses_default_for_unknown() {
        let estimator = TableGasEstimator::new();
        assert_eq!(
            estimator.estimate_cost("unknown-server", "unknown-tool", &serde_json::json!({})),
            10
        );
    }

    #[test]
    fn table_estimator_custom_default() {
        let estimator = TableGasEstimator::new().with_default_cost(20);
        assert_eq!(
            estimator.estimate_cost("unknown-server", "unknown-tool", &serde_json::json!({})),
            20
        );
    }

    #[test]
    fn inference_server_cost_is_zero() {
        // Inference gas is handled by InferenceGasEstimator, not the table.
        // The table returns 0 for inference as a signal to use the token-based estimator.
        let estimator = TableGasEstimator::new();
        assert_eq!(
            estimator.estimate_cost("hkask-mcp-inference", "generate", &serde_json::json!({})),
            0
        );
    }
}
