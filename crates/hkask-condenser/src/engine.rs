//! CondenserEngine — Pure domain logic for context condensation
//!
//! No async, no MCP dependencies, no HTTP. This module owns compression
//! dispatch, profile management, and cumulative statistics.
//!
//! The `AlgorithmRegistry` is constructed once at startup and is immutable.
//! `CondenserEngine` holds only mutable state: profile and stats.

use crate::algorithms::{AlgorithmRegistry, classify_tool};
use crate::types::*;
use std::time::Instant;

// G4: profile is the single source of truth; stats.current_profile is always derived from it.
pub struct CondenserEngine {
    pub registry: AlgorithmRegistry,
    profile: Profile,
    pub stats: CondenserStats,
}

impl Default for CondenserEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CondenserEngine {
    pub fn new() -> Self {
        Self {
            registry: AlgorithmRegistry::new(),
            profile: Profile::Normal,
            stats: CondenserStats::default(),
        }
    }

    /// Resolve a tool name to its category and the algorithm that handles it.
    ///
    /// D3: single call site for the classify→select path. `condenser_classify`
    /// delegates here instead of calling `classify_tool` + `registry.select` directly.
    /// Returns an owned `String` for the algorithm name so callers can hold the
    /// result across a mutable borrow of `self`.
    pub fn classify(&self, tool_name: &str) -> (ContextCategory, String) {
        let cat = classify_tool(tool_name);
        let algo = self.registry.select(cat);
        (cat, algo.name().to_string())
    }

    pub fn compress(
        &mut self,
        tool_name: &str,
        output: &str,
        category: Option<ContextCategory>,
    ) -> CompressedOutput {
        let cat = category.unwrap_or_else(|| classify_tool(tool_name));
        let algo = self.registry.select(cat);
        let algorithm_name = algo.name().to_string();

        let start = Instant::now();

        // P9: CNS span
        tracing::info!(target: "cns.condenser", operation = "compress", algorithm = %algorithm_name, category = %cat.label(), tool_name = %tool_name, "CNS");

        let (compressed_content, health_signals) = algo.compress(output, self.profile, cat);

        let original_lines = output.lines().count();
        let compressed_lines = compressed_content.lines().count();
        let original_bytes = output.len();
        let compressed_bytes = compressed_content.len();
        let reduction_pct = if original_bytes == 0 {
            0.0
        } else {
            (1.0 - (compressed_bytes as f64 / original_bytes as f64)) * 100.0
        };

        *self
            .stats
            .algorithm_usage
            .entry(algorithm_name.clone())
            .or_insert(0) += 1;
        *self
            .stats
            .category_usage
            .entry(cat.label().to_string())
            .or_insert(0) += 1;
        self.stats.total_compressions += 1;
        self.stats.total_original_bytes += original_bytes as u64;
        self.stats.total_compressed_bytes += compressed_bytes as u64;

        // P9: CNS span
        tracing::info!(target: "cns.condenser", operation = "compression_ratio", algorithm = %algorithm_name, category = %cat.label(), reduction_pct = %format!("{:.1}", reduction_pct), original_bytes = original_bytes, compressed_bytes = compressed_bytes, latency_ms = start.elapsed().as_millis(), "CNS");

        CompressedOutput {
            content: compressed_content,
            algorithm: algorithm_name,
            category: cat.label().to_string(),
            profile: self.profile.to_string(),
            original_lines,
            compressed_lines,
            original_bytes,
            compressed_bytes,
            reduction_pct,
            health_signals,
        }
    }

    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        self.stats.current_profile = profile.to_string();
    }

    pub fn get_stats(&self) -> &CondenserStats {
        &self.stats
    }

    /// Check for global health violations across all compressions.
    ///
    /// Returns health signals for systemic issues: overall compression ratio
    /// below 2:1 or throughput anomalies. Callers should emit these as
    /// `cns.condenser.degraded` ν-events.
    pub fn check_global_health(&self) -> Vec<CondenserHealthSignal> {
        let mut signals = Vec::new();
        let stats = &self.stats;

        if stats.total_original_bytes > 0 {
            let ratio =
                stats.total_original_bytes as f64 / stats.total_compressed_bytes.max(1) as f64;
            if ratio < 2.0 && stats.total_compressions >= 10 {
                signals.push(CondenserHealthSignal {
                    algorithm: "global".into(),
                    signal_type: "low_compression_ratio".into(),
                    detail: format!(
                        "Overall compression ratio {:.2}:1 below 2:1 SLA ({} compressions)",
                        ratio, stats.total_compressions
                    ),
                    zero_score_count: None,
                    budget_requested: None,
                    budget_filled: None,
                });
            }
        }

        // P9: CNS span
        tracing::info!(target: "cns.condenser", operation = "health", total_compressions = stats.total_compressions, health_signal_count = signals.len(), "CNS");

        signals
    }
}
