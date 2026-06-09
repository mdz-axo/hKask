//! hKask MCP Condenser — Compression algorithms (Phase 1: local, no LLM)

use super::types::*;

pub trait CondenserAlgorithm: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_for(&self) -> &[ContextCategory];
    fn compress(&self, input: &str, profile: Profile, category: ContextCategory) -> String;
    fn handles(&self, category: ContextCategory) -> bool;
}

pub struct RtkStyleAlgorithm;

impl CondenserAlgorithm for RtkStyleAlgorithm {
    fn name(&self) -> &str {
        "rtk_style"
    }
    fn description(&self) -> &str {
        "Command-specific rules: filter, group, truncate, dedup"
    }
    fn default_for(&self) -> &[ContextCategory] {
        &[
            ContextCategory::ShellCommand,
            ContextCategory::TestOutput,
            ContextCategory::BuildOutput,
        ]
    }
    fn handles(&self, category: ContextCategory) -> bool {
        matches!(
            category,
            ContextCategory::ShellCommand
                | ContextCategory::TestOutput
                | ContextCategory::BuildOutput
        )
    }
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let max_lines = profile.max_lines().unwrap_or(usize::MAX);
        let retention = profile.retention_pct();
        let target_lines = ((lines.len() as f64) * retention).max(1.0).round() as usize;
        let budget = target_lines.min(max_lines).min(lines.len());

        if budget >= lines.len() {
            return input.to_string();
        }

        let head_count = (budget as f64 * 0.3) as usize;
        let tail_count = budget.saturating_sub(head_count);

        let mut filtered: Vec<&str> = Vec::new();
        for line in lines.iter().take(head_count) {
            filtered.push(*line);
        }

        let tail_start = lines.len().saturating_sub(tail_count);
        if tail_start > head_count {
            filtered.push("...");
            for line in lines.iter().skip(tail_start) {
                filtered.push(*line);
            }
        }

        let mut result = filtered.join("\n");
        if result.lines().count() > budget {
            let result_lines: Vec<&str> = result.lines().collect();
            let head = &result_lines[..head_count.min(result_lines.len())];
            let tail_start = result_lines.len().saturating_sub(tail_count);
            let tail = &result_lines[tail_start.min(head_count)..];
            let mut final_lines = head.to_vec();
            if tail_start > head_count {
                final_lines.push("...");
                final_lines.extend_from_slice(tail);
            }
            result = final_lines.join("\n");
        }

        result
    }
}

pub struct SaliencyRankAlgorithm;

impl SaliencyRankAlgorithm {
    fn compute_word_frequencies(lines: &[&str]) -> std::collections::HashMap<String, f64> {
        let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut total = 0usize;
        for line in lines {
            for word in line.split_whitespace() {
                let w = word.to_lowercase();
                if w.len() > 2 {
                    *freq.entry(w).or_insert(0) += 1;
                    total += 1;
                }
            }
        }
        let t = total.max(1) as f64;
        freq.into_iter().map(|(k, v)| (k, v as f64 / t)).collect()
    }

    fn line_score(line: &str, freq: &std::collections::HashMap<String, f64>) -> f64 {
        let words: Vec<&str> = line.split_whitespace().filter(|w| w.len() > 2).collect();
        if words.is_empty() {
            return 0.0;
        }
        let tf_sum: f64 = words
            .iter()
            .map(|w| freq.get(&w.to_lowercase()).copied().unwrap_or(0.0))
            .sum();

        let structural_bonus =
            if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
                2.0
            } else if line.contains("warning") || line.contains("Warning") {
                1.0
            } else if line.starts_with('#') || line.starts_with("##") {
                0.5
            } else if line.starts_with('-') || line.starts_with('*') {
                0.2
            } else {
                0.0
            };

        tf_sum / words.len() as f64 + structural_bonus
    }
}

impl CondenserAlgorithm for SaliencyRankAlgorithm {
    fn name(&self) -> &str {
        "saliency_rank"
    }
    fn description(&self) -> &str {
        "TF-IDF + entropy scoring with structural bonus"
    }
    fn default_for(&self) -> &[ContextCategory] {
        &[
            ContextCategory::ConversationHistory,
            ContextCategory::LogOutput,
            ContextCategory::Unknown,
        ]
    }
    fn handles(&self, category: ContextCategory) -> bool {
        matches!(
            category,
            ContextCategory::ConversationHistory
                | ContextCategory::LogOutput
                | ContextCategory::Unknown
        )
    }
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let max_lines = profile.max_lines().unwrap_or(usize::MAX);
        let target_lines = ((lines.len() as f64) * profile.retention_pct())
            .max(1.0)
            .round() as usize;
        let budget = target_lines.min(max_lines).min(lines.len());

        if budget >= lines.len() {
            return input.to_string();
        }

        let freq = Self::compute_word_frequencies(&lines);

        let mut scored: Vec<(usize, f64, &str)> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| (i, Self::line_score(line, &freq), *line))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut selected_indices: Vec<usize> =
            scored.into_iter().take(budget).map(|(i, _, _)| i).collect();
        selected_indices.sort_unstable();

        let mut result = String::new();
        let mut last_idx: Option<usize> = None;
        for idx in selected_indices {
            if let Some(li) = last_idx
                && idx > li + 1
            {
                result.push_str("...\n");
            }
            result.push_str(lines[idx]);
            result.push('\n');
            last_idx = Some(idx);
        }

        result.trim_end().to_string()
    }
}

pub struct FlashrankAlgorithm;

impl FlashrankAlgorithm {
    fn relevance_score(line: &str, query_terms: &[String]) -> f64 {
        let lower = line.to_lowercase();
        query_terms
            .iter()
            .map(|term| {
                if lower.contains(&term.to_lowercase()) {
                    1.0
                } else {
                    0.0
                }
            })
            .sum()
    }

    fn novelty_score(line: &str, selected: &[&str]) -> f64 {
        let lower = line.to_lowercase();
        let words: std::collections::HashSet<&str> = lower.split_whitespace().collect();
        let mut overlap = 0usize;
        let mut total = 0usize;
        for prev in selected {
            let prev_lower = prev.to_lowercase();
            let prev_words: std::collections::HashSet<&str> =
                prev_lower.split_whitespace().collect();
            for w in &words {
                total += 1;
                if prev_words.contains(w) {
                    overlap += 1;
                }
            }
        }
        if total == 0 {
            return 1.0;
        }
        1.0 - (overlap as f64 / total as f64)
    }

    fn brevity_score(line: &str) -> f64 {
        let len = line.len() as f64;
        if len == 0.0 {
            return 0.0;
        }
        1.0 / (1.0 + len / 100.0)
    }
}

impl CondenserAlgorithm for FlashrankAlgorithm {
    fn name(&self) -> &str {
        "flashrank"
    }
    fn description(&self) -> &str {
        "Greedy marginal-utility selection under token budget"
    }
    fn default_for(&self) -> &[ContextCategory] {
        &[
            ContextCategory::FileContents,
            ContextCategory::StructuredData,
        ]
    }
    fn handles(&self, category: ContextCategory) -> bool {
        matches!(
            category,
            ContextCategory::FileContents | ContextCategory::StructuredData
        )
    }
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let max_lines = profile.max_lines().unwrap_or(usize::MAX);
        let target_lines = ((lines.len() as f64) * profile.retention_pct())
            .max(1.0)
            .round() as usize;
        let budget = target_lines.min(max_lines).min(lines.len());

        if budget >= lines.len() {
            return input.to_string();
        }

        let alpha = 0.4f64;
        let beta = 0.3f64;
        let gamma = 0.3f64;

        let query_terms: Vec<String> = lines
            .iter()
            .take(5)
            .flat_map(|l| {
                l.split_whitespace()
                    .filter(|w| w.len() > 3)
                    .map(|s| s.to_string())
            })
            .take(20)
            .collect();

        let mut selected_indices: Vec<usize> = Vec::new();
        let mut selected_lines: Vec<&str> = Vec::new();

        while selected_indices.len() < budget {
            let mut best_idx: Option<usize> = None;
            let mut best_score = f64::NEG_INFINITY;

            for (i, line) in lines.iter().enumerate() {
                if selected_indices.contains(&i) {
                    continue;
                }

                let rel = Self::relevance_score(line, &query_terms);
                let nov = Self::novelty_score(line, &selected_lines);
                let brev = Self::brevity_score(line);

                let score = alpha * rel + beta * nov - gamma * (1.0 - brev);
                if score > best_score {
                    best_score = score;
                    best_idx = Some(i);
                }
            }

            match best_idx {
                Some(idx) => {
                    selected_indices.push(idx);
                    selected_lines.push(lines[idx]);
                }
                None => break,
            }
        }

        selected_indices.sort_unstable();
        let mut result = String::new();
        let mut last_idx: Option<usize> = None;
        for idx in selected_indices {
            if let Some(li) = last_idx
                && idx > li + 1
            {
                result.push_str("...\n");
            }
            result.push_str(lines[idx]);
            result.push('\n');
            last_idx = Some(idx);
        }

        result.trim_end().to_string()
    }
}

pub struct AlgorithmRegistry {
    algorithms: Vec<Box<dyn CondenserAlgorithm>>,
}

impl AlgorithmRegistry {
    pub fn new() -> Self {
        let algorithms: Vec<Box<dyn CondenserAlgorithm>> = vec![
            Box::new(RtkStyleAlgorithm),
            Box::new(SaliencyRankAlgorithm),
            Box::new(FlashrankAlgorithm),
        ];
        Self { algorithms }
    }

    pub fn select(&self, category: ContextCategory) -> &dyn CondenserAlgorithm {
        for algo in &self.algorithms {
            if algo.default_for().contains(&category) {
                return algo.as_ref();
            }
        }
        for algo in &self.algorithms {
            if algo.handles(category) {
                return algo.as_ref();
            }
        }
        self.algorithms
            .last()
            .expect("at least one algorithm")
            .as_ref()
    }

    pub fn list_algorithms(&self) -> Vec<serde_json::Value> {
        self.algorithms
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name(),
                    "description": a.description(),
                    "default_for": a.default_for().iter().map(|c| c.label()).collect::<Vec<_>>(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AlgorithmRegistry ──

    // REQ: AlgorithmRegistry always selects an algorithm (never panics on valid categories)
    #[test]
    fn registry_selects_for_all_categories() {
        let registry = AlgorithmRegistry::new();
        for cat in [
            ContextCategory::ShellCommand,
            ContextCategory::TestOutput,
            ContextCategory::BuildOutput,
            ContextCategory::FileContents,
            ContextCategory::ConversationHistory,
            ContextCategory::StructuredData,
            ContextCategory::LogOutput,
            ContextCategory::Unknown,
        ] {
            let algo = registry.select(cat);
            assert!(!algo.name().is_empty());
        }
    }

    // REQ: AlgorithmRegistry selects rtk_style for ShellCommand
    #[test]
    fn registry_selects_rtk_for_shell() {
        let registry = AlgorithmRegistry::new();
        let algo = registry.select(ContextCategory::ShellCommand);
        assert_eq!(algo.name(), "rtk_style");
    }

    // REQ: AlgorithmRegistry selects rtk_style for TestOutput and BuildOutput
    #[test]
    fn registry_selects_rtk_for_test_and_build() {
        let registry = AlgorithmRegistry::new();
        assert_eq!(
            registry.select(ContextCategory::TestOutput).name(),
            "rtk_style"
        );
        assert_eq!(
            registry.select(ContextCategory::BuildOutput).name(),
            "rtk_style"
        );
    }

    // REQ: AlgorithmRegistry selects saliency_rank for ConversationHistory, LogOutput, Unknown
    #[test]
    fn registry_selects_saliency_for_conv_log_unknown() {
        let registry = AlgorithmRegistry::new();
        assert_eq!(
            registry.select(ContextCategory::ConversationHistory).name(),
            "saliency_rank"
        );
        assert_eq!(
            registry.select(ContextCategory::LogOutput).name(),
            "saliency_rank"
        );
        assert_eq!(
            registry.select(ContextCategory::Unknown).name(),
            "saliency_rank"
        );
    }

    // REQ: AlgorithmRegistry selects flashrank for FileContents and StructuredData
    #[test]
    fn registry_selects_flashrank_for_file_and_structured() {
        let registry = AlgorithmRegistry::new();
        assert_eq!(
            registry.select(ContextCategory::FileContents).name(),
            "flashrank"
        );
        assert_eq!(
            registry.select(ContextCategory::StructuredData).name(),
            "flashrank"
        );
    }

    // REQ: AlgorithmRegistry lists exactly 3 algorithms
    #[test]
    fn registry_lists_three_algorithms() {
        let registry = AlgorithmRegistry::new();
        let list = registry.list_algorithms();
        assert_eq!(list.len(), 3);
        let names: Vec<&str> = list.iter().map(|v| v["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"rtk_style"));
        assert!(names.contains(&"saliency_rank"));
        assert!(names.contains(&"flashrank"));
    }

    // ── RtkStyleAlgorithm ──

    // REQ: rtk_style compresses multi-line input to fewer lines under Heavy profile
    #[test]
    fn rtk_style_reduces_lines_under_heavy() {
        let algo = RtkStyleAlgorithm;
        let input = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Heavy, ContextCategory::ShellCommand);
        assert!(result.lines().count() < input.lines().count());
    }

    // REQ: rtk_style returns input unchanged when it fits within budget
    #[test]
    fn rtk_style_passthrough_when_under_budget() {
        let algo = RtkStyleAlgorithm;
        let input = "short\noutput";
        let result = algo.compress(input, Profile::Light, ContextCategory::ShellCommand);
        assert_eq!(result, input);
    }

    // REQ: rtk_style includes "..." ellipsis when truncating
    #[test]
    fn rtk_style_includes_ellipsis_when_truncating() {
        let algo = RtkStyleAlgorithm;
        let input = (0..200)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Heavy, ContextCategory::ShellCommand);
        assert!(
            result.contains("..."),
            "truncated output should contain ellipsis marker"
        );
    }

    // REQ: rtk_style produces at least 1 line for any non-empty input
    #[test]
    fn rtk_style_always_produces_output() {
        let algo = RtkStyleAlgorithm;
        let input = "single line";
        let result = algo.compress(input, Profile::Heavy, ContextCategory::ShellCommand);
        assert!(!result.is_empty());
    }

    // REQ: rtk_style preserves head and tail lines preferentially
    #[test]
    fn rtk_style_preserves_head_and_tail() {
        let algo = RtkStyleAlgorithm;
        let input = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Heavy, ContextCategory::ShellCommand);
        assert!(result.contains("line 0"), "head should be preserved");
        assert!(result.contains("line 49"), "tail should be preserved");
    }

    // ── SaliencyRankAlgorithm ──

    // REQ: saliency_rank produces fewer lines than input under Normal profile
    #[test]
    fn saliency_rank_reduces_lines() {
        let algo = SaliencyRankAlgorithm;
        let input = (0..200)
            .map(|i| format!("line {i} with some words"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Normal, ContextCategory::LogOutput);
        assert!(result.lines().count() < input.lines().count());
    }

    // REQ: saliency_rank boosts lines containing "error" in scoring
    #[test]
    fn saliency_rank_prioritizes_error_lines() {
        let algo = SaliencyRankAlgorithm;
        let mut lines: Vec<String> = (0..100).map(|i| format!("info line {i}")).collect();
        lines.push("CRITICAL error something went wrong".to_string());
        let input = lines.join("\n");
        let result = algo.compress(&input, Profile::Heavy, ContextCategory::LogOutput);
        assert!(
            result.contains("error"),
            "error lines should survive compression"
        );
    }

    // REQ: saliency_rank returns input unchanged when within budget
    #[test]
    fn saliency_rank_passthrough_when_under_budget() {
        let algo = SaliencyRankAlgorithm;
        let input = "short log";
        let result = algo.compress(input, Profile::Light, ContextCategory::LogOutput);
        assert_eq!(result, input);
    }

    // REQ: saliency_rank preserves original line order (stable after selection)
    #[test]
    fn saliency_rank_preserves_order() {
        let algo = SaliencyRankAlgorithm;
        let input = (0..20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Heavy, ContextCategory::Unknown);
        let nums: Vec<usize> = result
            .lines()
            .filter_map(|l| {
                l.strip_prefix("line ")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .collect();
        let mut sorted = nums.clone();
        sorted.sort();
        assert_eq!(
            nums, sorted,
            "selected lines should appear in original order"
        );
    }

    // ── FlashrankAlgorithm ──

    // REQ: flashrank produces fewer lines than input under Normal profile
    #[test]
    fn flashrank_reduces_lines() {
        let algo = FlashrankAlgorithm;
        let input = (0..200)
            .map(|i| format!("data line {i} with various content"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Normal, ContextCategory::FileContents);
        assert!(result.lines().count() < input.lines().count());
    }

    // REQ: flashrank returns input unchanged when within budget
    #[test]
    fn flashrank_passthrough_when_under_budget() {
        let algo = FlashrankAlgorithm;
        let input = "short file";
        let result = algo.compress(input, Profile::Light, ContextCategory::FileContents);
        assert_eq!(result, input);
    }

    // REQ: flashrank novelty_score returns 1.0 when no lines selected yet
    #[test]
    fn flashrank_novelty_is_one_for_empty_selected() {
        let score = FlashrankAlgorithm::novelty_score("any line", &[]);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    // REQ: flashrank brevity_score is higher for shorter lines
    #[test]
    fn flashrank_brevity_favors_shorter_lines() {
        let short = FlashrankAlgorithm::brevity_score("hi");
        let long = FlashrankAlgorithm::brevity_score(&"x".repeat(500));
        assert!(short > long, "shorter lines should score higher on brevity");
    }

    // REQ: flashrank relevance_score matches query terms
    #[test]
    fn flashrank_relevance_matches_terms() {
        let terms: Vec<String> = vec!["important".to_string(), "key".to_string()];
        let matching = FlashrankAlgorithm::relevance_score("this is an important key line", &terms);
        let non_matching = FlashrankAlgorithm::relevance_score("nothing relevant here", &terms);
        assert!(matching > non_matching);
    }

    // REQ: flashrank preserves original line order after greedy selection
    #[test]
    fn flashrank_preserves_order() {
        let algo = FlashrankAlgorithm;
        let input = (0..20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = algo.compress(&input, Profile::Heavy, ContextCategory::StructuredData);
        let nums: Vec<usize> = result
            .lines()
            .filter_map(|l| {
                l.strip_prefix("line ")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .collect();
        let mut sorted = nums.clone();
        sorted.sort();
        assert_eq!(
            nums, sorted,
            "selected lines should appear in original order"
        );
    }

    // ── Cross-algorithm invariants ──

    // REQ: All algorithms produce non-empty output for non-empty single-line input
    #[test]
    fn all_algorithms_non_empty_on_single_line() {
        let algos: Vec<Box<dyn CondenserAlgorithm>> = vec![
            Box::new(RtkStyleAlgorithm),
            Box::new(SaliencyRankAlgorithm),
            Box::new(FlashrankAlgorithm),
        ];
        let input = "hello world";
        for profile in [
            Profile::Heavy,
            Profile::Normal,
            Profile::Soft,
            Profile::Light,
        ] {
            for algo in &algos {
                let result = algo.compress(input, profile, ContextCategory::Unknown);
                assert!(
                    !result.is_empty(),
                    "{} should produce non-empty output for single line under {}",
                    algo.name(),
                    profile
                );
            }
        }
    }

    // REQ: All algorithms never expand output (compressed_bytes <= original_bytes for multi-line)
    #[test]
    fn all_algorithms_never_expand() {
        let algos: Vec<Box<dyn CondenserAlgorithm>> = vec![
            Box::new(RtkStyleAlgorithm),
            Box::new(SaliencyRankAlgorithm),
            Box::new(FlashrankAlgorithm),
        ];
        let input = (0..500)
            .map(|i| format!("line {i} with padding content"))
            .collect::<Vec<_>>()
            .join("\n");
        for profile in [Profile::Heavy, Profile::Normal, Profile::Soft] {
            for algo in &algos {
                let result = algo.compress(&input, profile, ContextCategory::Unknown);
                assert!(
                    result.len() <= input.len(),
                    "{} under {} expanded output: {} > {}",
                    algo.name(),
                    profile,
                    result.len(),
                    input.len()
                );
            }
        }
    }

    // REQ: handles() is consistent with default_for() for each algorithm
    #[test]
    fn handles_consistent_with_default_for() {
        let algos: Vec<Box<dyn CondenserAlgorithm>> = vec![
            Box::new(RtkStyleAlgorithm),
            Box::new(SaliencyRankAlgorithm),
            Box::new(FlashrankAlgorithm),
        ];
        for algo in &algos {
            for cat in algo.default_for() {
                assert!(
                    algo.handles(*cat),
                    "{} claims default_for {:?} but doesn't handle it",
                    algo.name(),
                    cat
                );
            }
        }
    }
}
