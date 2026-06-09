//! CondenserEngine — Pure domain logic for context condensation
//!
//! No async, no MCP dependencies, no HTTP. This module owns compression
//! dispatch, profile management, and cumulative statistics.

use crate::algorithms::AlgorithmRegistry;
use crate::types::*;

pub struct CondenserEngine {
    pub registry: AlgorithmRegistry,
    pub profile: Profile,
    pub stats: CondenserStats,
}

impl CondenserEngine {
    pub fn new() -> Self {
        Self {
            registry: AlgorithmRegistry::new(),
            profile: Profile::Normal,
            stats: CondenserStats::default(),
        }
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

        let compressed_content = algo.compress(output, self.profile, cat);

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
        }
    }

    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        self.stats.current_profile = profile.to_string();
    }

    pub fn get_stats(&self) -> &CondenserStats {
        &self.stats
    }
}
