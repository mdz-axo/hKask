//! CompositeGasEstimator — Routes inference tools to InferenceGasEstimator,
//! all other tools to TableGasEstimator.

use crate::governed_tool::GasEstimator;
use crate::inference_estimator::InferenceGasEstimator;
use crate::table_gas_estimator::TableGasEstimator;
use serde_json::Value;

/// Composite gas estimator that routes inference tools to InferenceGasEstimator
/// and all other tools to TableGasEstimator.
///
/// This is the production estimator — it should be the default for all
/// GovernedTool instances. Inference calls (server == "hkask-mcp-inference")
/// use token-based estimation; everything else uses the per-server table.
pub struct CompositeGasEstimator {
    inference: InferenceGasEstimator,
    table: TableGasEstimator,
}

impl CompositeGasEstimator {
    /// Create a new CompositeGasEstimator with default table costs.
    pub fn new() -> Self {
        Self {
            inference: InferenceGasEstimator,
            table: TableGasEstimator::new(),
        }
    }

    /// The inference server identifier used for routing.
    pub const INFERENCE_SERVER: &'static str = "hkask-mcp-inference";
}

impl Default for CompositeGasEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl GasEstimator for CompositeGasEstimator {
    fn estimate_cost(&self, server: &str, tool: &str, args: &Value) -> u64 {
        if server == Self::INFERENCE_SERVER {
            self.inference.estimate_cost(server, tool, args)
        } else {
            self.table.estimate_cost(server, tool, args)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_inference_to_token_estimator() {
        let est = CompositeGasEstimator::new();
        let args = serde_json::json!({"prompt": "hello world", "max_tokens": 50});
        // Inference: prompt 11 chars / 4 = 2 tokens + 50 max = 52
        let cost = est.estimate_cost("hkask-mcp-inference", "chat", &args);
        assert_eq!(cost, 52);
    }

    #[test]
    fn routes_non_inference_to_table() {
        let est = CompositeGasEstimator::new();
        let args = serde_json::json!({});
        // Non-inference → table lookup
        assert_eq!(est.estimate_cost("hkask-mcp-ocap", "check", &args), 1);
        assert_eq!(est.estimate_cost("hkask-mcp-fal", "generate", &args), 100);
    }

    #[test]
    fn default_impl_matches_new() {
        let a = CompositeGasEstimator::new();
        let b = CompositeGasEstimator::default();
        let args = serde_json::json!({});
        assert_eq!(
            a.estimate_cost("hkask-mcp-ocap", "x", &args),
            b.estimate_cost("hkask-mcp-ocap", "x", &args)
        );
    }

    // ── InferenceGasEstimator via CompositeGasEstimator ─────────────────

    #[test]
    fn inference_cost_with_prompt_and_max_tokens() {
        let est = CompositeGasEstimator::new();
        // "hello" = 5 chars / 4 = 1 token + 100 max = 101
        let args = serde_json::json!({"prompt": "hello", "max_tokens": 100});
        let cost = est.estimate_cost("hkask-mcp-inference", "chat", &args);
        assert_eq!(cost, 101);
    }

    #[test]
    fn inference_cost_defaults_max_tokens_to_100() {
        let args = serde_json::json!({"prompt": "hi"});
        let est = CompositeGasEstimator::new();
        // 2 chars / 4 = 0 tokens + 100 default = 100
        let cost = est.estimate_cost("hkask-mcp-inference", "chat", &args);
        assert_eq!(cost, 100);
    }

    #[test]
    fn inference_cost_minimum_one() {
        let est = CompositeGasEstimator::new();
        // No prompt, max_tokens=0 → total 0 → returns 1
        let args = serde_json::json!({"max_tokens": 0});
        let cost = est.estimate_cost("hkask-mcp-inference", "chat", &args);
        assert_eq!(cost, 1);
    }

    #[test]
    fn inference_cost_with_empty_args() {
        let est = CompositeGasEstimator::new();
        let args = serde_json::json!({});
        let cost = est.estimate_cost("hkask-mcp-inference", "chat", &args);
        // No prompt (0 chars), default max_tokens=100 → 0/4 + 100 = 100
        assert_eq!(cost, 100);
    }
}
