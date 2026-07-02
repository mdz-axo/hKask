//! Curator budget policy — efficiency limits for the Curator daemon.
//!
//! The Curator has unlimited gas (hard_limit = false) but is subject to
//! admin-configured efficiency limits to prevent runaway consumption.
//! The Human Administrator is the S5* observer-of-observer.

use serde::{Deserialize, Serialize};

/// Curator efficiency policy — limits token and tool usage per regulation cycle.
///
/// When limits are exceeded, CNS span `cns.curator.efficiency.exceeded` fires.
/// The Human Administrator reviews efficiency reports and adjusts limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorBudgetPolicy {
    /// Max LLM tokens per regulation cycle.
    /// 0 = unlimited (no token limit enforcement).
    pub max_tokens_per_cycle: u64,
    /// Max tool invocations per regulation cycle.
    /// 0 = unlimited (no tool limit enforcement).
    pub max_tool_calls_per_cycle: u32,
    /// When true and limits are exceeded, skip remaining tool calls this cycle.
    /// When false, log CNS span and continue.
    pub throttle_on_exceeded: bool,
}

impl Default for CuratorBudgetPolicy {
    fn default() -> Self {
        Self {
            max_tokens_per_cycle: 0,     // unlimited by default
            max_tool_calls_per_cycle: 0, // unlimited by default
            throttle_on_exceeded: false, // warn-only by default
        }
    }
}
