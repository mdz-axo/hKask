//! CondenserEngine — Pure domain logic for context condensation
//!
//! No async, no MCP dependencies, no HTTP. This module owns compression
//! dispatch, profile management, and cumulative statistics.
//!
//! The `AlgorithmRegistry` is constructed once at startup and is immutable.
//! `CondenserEngine` holds only mutable state: profile and stats.

use crate::algorithms::{AlgorithmRegistry, classify_tool};
use crate::types::*;

// G4: profile is the single source of truth; stats.current_profile is always derived from it.
pub struct CondenserEngine {
    pub(crate) registry: AlgorithmRegistry,
    profile: Profile,
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

    #[cfg(test)]
    pub fn profile(&self) -> Profile {
        self.profile
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── CondenserEngine ──

    // REQ: CondenserEngine starts with Normal profile
    #[test]
    fn engine_default_profile_is_normal() {
        let engine = CondenserEngine::new();
        assert_eq!(engine.profile(), Profile::Normal);
    }

    // REQ: CondenserEngine starts with zero compression stats
    #[test]
    fn engine_starts_with_zero_stats() {
        let engine = CondenserEngine::new();
        let stats = engine.get_stats();
        assert_eq!(stats.total_compressions, 0);
        assert_eq!(stats.total_original_bytes, 0);
        assert_eq!(stats.total_compressed_bytes, 0);
    }

    // REQ: CondenserEngine.compress updates stats counters
    #[test]
    fn engine_compress_updates_stats() {
        let mut engine = CondenserEngine::new();
        let input = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        engine.compress("git_status", &input, None);
        let stats = engine.get_stats();
        assert_eq!(stats.total_compressions, 1);
        assert!(stats.total_original_bytes > 0);
        assert!(stats.total_compressed_bytes > 0);
        assert!(stats.algorithm_usage.contains_key("rtk_style"));
        assert!(stats.category_usage.contains_key("shell_command"));
    }

    // REQ: CondenserEngine.compress auto-classifies tool name when category is None
    #[test]
    fn engine_compress_auto_classifies() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress("git_status", "some output", None);
        assert_eq!(result.category, "shell_command");
    }

    // REQ: CondenserEngine.compress uses provided category when given
    #[test]
    fn engine_compress_uses_explicit_category() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress(
            "git_status",
            "some output",
            Some(ContextCategory::LogOutput),
        );
        assert_eq!(result.category, "log_output");
    }

    // REQ: CondenserEngine.compress returns correct algorithm name
    #[test]
    fn engine_compress_returns_algorithm_name() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress("git_status", "some output", None);
        assert_eq!(result.algorithm, "rtk_style");
    }

    // REQ: CondenserEngine.compress tracks profile in output
    #[test]
    fn engine_compress_tracks_profile() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress("git_status", "some output", None);
        assert_eq!(result.profile, "normal");
    }

    // REQ: CondenserEngine.compress reports reduction_pct > 0 for long input under Heavy
    #[test]
    fn engine_compress_reports_reduction() {
        let mut engine = CondenserEngine::new();
        engine.set_profile(Profile::Heavy);
        let input = (0..500)
            .map(|i| format!("line {i} with content"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = engine.compress("git_status", &input, None);
        assert!(
            result.reduction_pct > 0.0,
            "should report positive reduction for long input under heavy profile"
        );
    }

    // REQ: CondenserEngine.compress reports 0% reduction for short input that passes through
    #[test]
    fn engine_compress_no_reduction_for_passthrough() {
        let mut engine = CondenserEngine::new();
        let input = "short";
        let result = engine.compress("git_status", input, None);
        assert_eq!(result.reduction_pct, 0.0);
    }

    // REQ: CondenserEngine.compress reports 0% reduction for empty-ish input
    #[test]
    fn engine_compress_zero_bytes_reduction() {
        let mut engine = CondenserEngine::new();
        // Empty string has 0 bytes; reduction_pct should be 0.0 per the guard
        let result = engine.compress("git_status", "", None);
        assert_eq!(result.reduction_pct, 0.0);
    }

    // REQ: CondenserEngine.set_profile changes the active profile
    #[test]
    fn engine_set_profile_changes_profile() {
        let mut engine = CondenserEngine::new();
        engine.set_profile(Profile::Heavy);
        assert_eq!(engine.profile(), Profile::Heavy);
        assert_eq!(engine.stats.current_profile, "heavy");
    }

    // REQ: CondenserEngine multiple compressions accumulate stats
    #[test]
    fn engine_multiple_compressions_accumulate() {
        let mut engine = CondenserEngine::new();
        let input = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        engine.compress("git_status", &input, None);
        engine.compress("pytest_run", &input, None);
        engine.compress("log_journal", &input, None);
        let stats = engine.get_stats();
        assert_eq!(stats.total_compressions, 3);
        assert!(stats.algorithm_usage.contains_key("rtk_style"));
        assert!(stats.algorithm_usage.contains_key("saliency_rank"));
    }

    // REQ: CompressedOutput line counts are accurate
    #[test]
    fn engine_compress_line_counts_accurate() {
        let mut engine = CondenserEngine::new();
        let input = "line1\nline2\nline3";
        let result = engine.compress("git_status", input, None);
        assert_eq!(result.original_lines, 3);
        assert_eq!(result.original_bytes, input.len());
    }

    // REQ: CompressedOutput byte counts match content.len()
    #[test]
    fn engine_compress_byte_counts_match() {
        let mut engine = CondenserEngine::new();
        let input = (0..200)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = engine.compress("git_status", &input, None);
        assert_eq!(result.compressed_bytes, result.content.len());
        assert_eq!(result.original_bytes, input.len());
    }

    // REQ: CondenserEngine.classify returns correct category and algorithm for a known tool
    #[test]
    fn engine_classify_shell_tool() {
        let engine = CondenserEngine::new();
        let (cat, algo) = engine.classify("git_status");
        assert_eq!(cat, ContextCategory::ShellCommand);
        assert_eq!(algo, "rtk_style");
    }

    // REQ: CondenserEngine.classify returns Unknown + saliency_rank for unrecognized tool
    #[test]
    fn engine_classify_unknown_tool() {
        let engine = CondenserEngine::new();
        let (cat, algo) = engine.classify("custom_mystery_tool");
        assert_eq!(cat, ContextCategory::Unknown);
        // Unknown is in SaliencyRankAlgorithm::default_for
        assert_eq!(algo, "saliency_rank");
    }

    // REQ: CondenserEngine.classify result matches what compress uses internally
    #[test]
    fn engine_classify_consistent_with_compress() {
        let mut engine = CondenserEngine::new();
        let (cat, algo_name) = engine.classify("pytest_run");
        let result = engine.compress("pytest_run", "some test output", None);
        assert_eq!(result.category, cat.label());
        assert_eq!(result.algorithm, algo_name);
    }
}
