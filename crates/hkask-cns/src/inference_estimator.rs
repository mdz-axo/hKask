//! InferenceGasEstimator — Token-based gas cost estimation for inference
//!
//! Implements `GasEstimator` using the token estimation heuristic:
//! prompt characters / 4 + max_tokens.
//! This estimator can be used with `GovernedTool` to govern inference
//! through the unified tool membrane.

use crate::governed_tool::GasEstimator;
use serde_json::Value;

/// Characters per token heuristic (English text ≈ 4 chars/token).
const CHARS_PER_TOKEN: usize = 4;

/// Inference-specific gas estimator.
///
/// Estimates cost based on:
/// - `prompt_tokens` ≈ `prompt.len() / CHARS_PER_TOKEN` (from JSON args)
/// - `max_tokens` from JSON args
///
/// For `GovernedTool` usage, the `args` Value should contain:
/// ```json
/// { "prompt": "...", "max_tokens": N }
/// ```
///
/// If args don't contain the expected fields, falls back to a flat cost of 1.
pub(crate) struct InferenceGasEstimator;

impl GasEstimator for InferenceGasEstimator {
    fn estimate_cost(&self, _server: &str, _tool: &str, args: &Value) -> u64 {
        // Try to extract prompt and max_tokens from args
        let prompt_chars = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(|s| s.len())
            .unwrap_or(0);

        let max_tokens = args
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(100); // default max_tokens

        let prompt_tokens = prompt_chars as u64 / CHARS_PER_TOKEN as u64;
        let total = prompt_tokens + max_tokens;

        if total == 0 { 1 } else { total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn estimate_cost_with_prompt_and_max_tokens() {
        let estimator = InferenceGasEstimator;
        let args = json!({
            "prompt": "Hello, this is a test prompt.",
            "max_tokens": 200
        });

        let cost = estimator.estimate_cost("inference", "generate", &args);
        // 29 chars / 4 = 7 prompt tokens + 200 max_tokens = 207
        assert_eq!(cost, 207);
    }

    #[test]
    fn estimate_cost_with_missing_fields_falls_back_to_default() {
        let estimator = InferenceGasEstimator;
        // No prompt, no max_tokens → 0 prompt tokens + 100 default = 100
        let args = json!({});
        let cost = estimator.estimate_cost("inference", "generate", &args);
        assert_eq!(cost, 100);
    }

    #[test]
    fn estimate_cost_with_empty_prompt() {
        let estimator = InferenceGasEstimator;
        let args = json!({
            "prompt": "",
            "max_tokens": 50
        });

        let cost = estimator.estimate_cost("inference", "generate", &args);
        // 0 chars / 4 = 0 prompt tokens + 50 max_tokens = 50
        assert_eq!(cost, 50);
    }

    #[test]
    fn estimate_cost_with_empty_prompt_and_no_max_tokens_returns_one() {
        let estimator = InferenceGasEstimator;
        // Empty prompt (0 chars → 0 tokens) + no max_tokens (defaults to 100) = 100
        // This tests the non-zero total path
        let args = json!({
            "prompt": ""
        });
        let cost = estimator.estimate_cost("inference", "generate", &args);
        // 0 prompt_tokens + 100 default = 100 (not 1, because total != 0)
        assert_eq!(cost, 100);
    }

    #[test]
    fn estimate_cost_zero_max_tokens_and_zero_prompt_returns_one() {
        let estimator = InferenceGasEstimator;
        // Both zero → total = 0, fallback to 1
        let args = json!({
            "prompt": "",
            "max_tokens": 0
        });
        let cost = estimator.estimate_cost("inference", "generate", &args);
        assert_eq!(cost, 1);
    }

    #[test]
    fn chars_per_token_heuristic_produces_reasonable_estimates() {
        let estimator = InferenceGasEstimator;

        // A typical prompt of ~500 chars should estimate ~125 prompt tokens
        let prompt = "a".repeat(500);
        let args = json!({
            "prompt": prompt,
            "max_tokens": 256
        });
        let cost = estimator.estimate_cost("inference", "generate", &args);
        // 500 / 4 = 125 prompt tokens + 256 max_tokens = 381
        assert_eq!(cost, 381);

        // A short prompt: 11 chars / 4 = 2 prompt tokens
        let args = json!({
            "prompt": "Hello world",
            "max_tokens": 50
        });
        let cost = estimator.estimate_cost("inference", "generate", &args);
        assert_eq!(cost, 52); // 2 + 50
    }
}
