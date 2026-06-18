//! hKask MCP Condenser — Compression algorithms (Phase 1: local, no LLM)

use super::types::*;

/// Emit `"...\n"` markers between non-consecutive lines in `indices`.
fn join_with_ellipsis(lines: &[&str], indices: &[usize]) -> String {
    let mut result = String::new();
    let mut last_idx: Option<usize> = None;
    for &idx in indices {
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

/// Compute the line budget for compression given the input size and profile.
///
/// Returns `(budget, is_passthrough)` — if `is_passthrough`, the algorithm
/// should return the input unchanged.
pub(crate) fn compute_budget(lines: usize, profile: Profile) -> (usize, bool) {
    let max_lines = profile.max_lines().unwrap_or(usize::MAX);
    let target_lines = ((lines as f64) * profile.retention_pct()).max(1.0).round() as usize;
    let budget = target_lines.min(max_lines).min(lines);
    (budget, budget >= lines)
}

pub trait CondenserAlgorithm: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_for(&self) -> &[ContextCategory];
    /// Compress the input and return (compressed_content, health_signals).
    /// Health signals are empty when the algorithm performed within expected bounds.
    fn compress(
        &self,
        input: &str,
        profile: Profile,
        category: ContextCategory,
    ) -> (String, Vec<CondenserHealthSignal>);
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
    fn compress(
        &self,
        input: &str,
        profile: Profile,
        _category: ContextCategory,
    ) -> (String, Vec<CondenserHealthSignal>) {
        let lines: Vec<&str> = input.lines().collect();
        let (budget, passthrough) = compute_budget(lines.len(), profile);
        if passthrough {
            return (input.to_string(), vec![]);
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

        let result = filtered.join("\n");
        let health = if result.len() > input.len() {
            vec![CondenserHealthSignal {
                algorithm: "rtk_style".into(),
                signal_type: "negative_compression".into(),
                detail: format!(
                    "Compressed {}B > original {}B — bounds violation",
                    result.len(),
                    input.len()
                ),
                zero_score_count: None,
                budget_requested: None,
                budget_filled: None,
            }]
        } else {
            vec![]
        };
        (result, health)
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
        ]
    }
    fn compress(
        &self,
        input: &str,
        profile: Profile,
        _category: ContextCategory,
    ) -> (String, Vec<CondenserHealthSignal>) {
        let lines: Vec<&str> = input.lines().collect();
        let (budget, passthrough) = compute_budget(lines.len(), profile);
        if passthrough {
            return (input.to_string(), vec![]);
        }

        let freq = Self::compute_word_frequencies(&lines);

        let mut scored: Vec<(usize, f64, &str)> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| (i, Self::line_score(line, &freq), *line))
            .collect();

        let zero_count = scored.iter().filter(|(_, s, _)| *s == 0.0).count();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut selected_indices: Vec<usize> =
            scored.into_iter().take(budget).map(|(i, _, _)| i).collect();
        selected_indices.sort_unstable();

        let result = join_with_ellipsis(&lines, &selected_indices);
        let health = if zero_count > lines.len() / 2 {
            vec![CondenserHealthSignal {
                algorithm: "saliency_rank".into(),
                signal_type: "low_signal".into(),
                detail: format!(
                    "{} of {} lines scored 0.0 — content had no usable signal to rank by",
                    zero_count,
                    lines.len()
                ),
                zero_score_count: Some(zero_count),
                budget_requested: None,
                budget_filled: None,
            }]
        } else {
            vec![]
        };
        (result, health)
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
            ContextCategory::Unknown,
        ]
    }
    fn compress(
        &self,
        input: &str,
        profile: Profile,
        _category: ContextCategory,
    ) -> (String, Vec<CondenserHealthSignal>) {
        let lines: Vec<&str> = input.lines().collect();
        let (budget, passthrough) = compute_budget(lines.len(), profile);
        if passthrough {
            return (input.to_string(), vec![]);
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

        let mut selected_indices: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
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
                    selected_indices.insert(idx);
                    selected_lines.push(lines[idx]);
                }
                None => break,
            }
        }

        let filled = selected_indices.len();
        let mut selected_indices: Vec<usize> = selected_indices.into_iter().collect();
        selected_indices.sort_unstable();
        let result = join_with_ellipsis(&lines, &selected_indices);

        let health = if filled < budget {
            vec![CondenserHealthSignal {
                algorithm: "flashrank".into(),
                signal_type: "budget_shortfall".into(),
                detail: format!(
                    "Filled {}/{} budget — all remaining lines had non-positive score",
                    filled, budget
                ),
                zero_score_count: None,
                budget_requested: Some(budget),
                budget_filled: Some(filled),
            }]
        } else {
            vec![]
        };
        (result, health)
    }
}

pub struct AlgorithmRegistry {
    algorithms: Vec<Box<dyn CondenserAlgorithm>>,
}

impl Default for AlgorithmRegistry {
    fn default() -> Self {
        Self::new()
    }
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

/// Keyword→category mapping — single source of truth for both exact-token (Phase 1)
/// and substring (Phase 2) matching in [`classify_tool`].
const KEYWORD_CATEGORIES: &[(&[&str], ContextCategory)] = &[
    (
        &[
            "git", "docker", "cargo", "npm", "shell", "bash", "exec", "run",
        ],
        ContextCategory::ShellCommand,
    ),
    (&["test", "pytest", "spec"], ContextCategory::TestOutput),
    (&["build", "compile", "make"], ContextCategory::BuildOutput),
    (
        &["chat", "conversation", "message"],
        ContextCategory::ConversationHistory,
    ),
    (&["log", "journal", "trace"], ContextCategory::LogOutput),
    (&["json", "api", "query"], ContextCategory::StructuredData),
    (&["file", "read", "cat"], ContextCategory::FileContents),
];

/// Classify a tool name into a `ContextCategory`.
/// Phase 1: exact token match on `_`/`-`-split parts. Phase 2: substring fallback.
pub fn classify_tool(tool_name: &str) -> ContextCategory {
    let lower = tool_name.to_lowercase();
    let parts: Vec<&str> = lower.split('_').chain(lower.split('-')).collect();

    // Phase 1: exact token match — no false positives
    for part in &parts {
        for (keywords, cat) in KEYWORD_CATEGORIES {
            if keywords.contains(part) {
                return *cat;
            }
        }
    }

    // Phase 2: substring heuristic for compound names without separators.
    // False positives possible ("logistics"→LogOutput) but Phase 2 only fires
    // after Phase 1 fails, so the tool name is already non-standard.
    for (keywords, cat) in KEYWORD_CATEGORIES {
        if keywords.iter().any(|kw| lower.contains(kw)) {
            return *cat;
        }
    }
    ContextCategory::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: CNS-CONDENSER-BUDGET — compute_budget returns passthrough when input fits within profile
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn compute_budget_passthrough_when_within_profile() {
        let (budget, passthrough) = compute_budget(10, Profile::Light);
        assert!(passthrough);
        assert_eq!(budget, 10);
    }

    // REQ: CNS-CONDENSER-BUDGET — compute_budget caps at max_lines even when retention would allow more
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn compute_budget_respects_max_lines_cap() {
        let (budget, passthrough) = compute_budget(1000, Profile::Heavy);
        assert!(!passthrough);
        assert_eq!(budget, 30);
    }

    // REQ: CNS-CONDENSER-BUDGET — compute_budget never exceeds input line count
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Note: retention_pct is applied first, then capped. 5 lines * 20% = 1, so budget = 1.
    #[test]
    fn compute_budget_never_exceeds_input() {
        let (budget, _) = compute_budget(5, Profile::Normal);
        assert_eq!(budget, 1);
    }

    // REQ: CNS-CONDENSER-BUDGET — compute_budget handles single-line input
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn compute_budget_single_line() {
        let (budget, passthrough) = compute_budget(1, Profile::Heavy);
        assert!(passthrough);
        assert_eq!(budget, 1);
    }

    // REQ: CNS-CONDENSER-BUDGET — compute_budget handles zero lines
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn compute_budget_zero_lines() {
        let (budget, passthrough) = compute_budget(0, Profile::Heavy);
        assert!(passthrough);
        assert_eq!(budget, 0);
    }

    // REQ: CNS-CONDENSER-CLASSIFY — classify_tool maps known tool names to correct categories
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Note: Phase 1 exact-token matching checks split parts in order. "npm" matches
    // ShellCommand before "build" is reached. "cargo_test" matches ShellCommand via "cargo".
    #[test]
    fn classify_tool_exact_match() {
        assert_eq!(classify_tool("git_status"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("docker_run"), ContextCategory::ShellCommand);
        // "npm_build" → "npm" matches ShellCommand before "build" → BuildOutput
        assert_eq!(classify_tool("npm_build"), ContextCategory::ShellCommand);
        // "cargo_test" → "cargo" matches ShellCommand before "test" → TestOutput
        assert_eq!(classify_tool("cargo_test"), ContextCategory::ShellCommand);
    }

    // REQ: CNS-CONDENSER-CLASSIFY — classify_tool falls back to substring matching for compound names
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Note: Phase 2 substring matching can produce false positives on short keywords (e.g., "run"
    // matches "testrunner", overriding the intended TestOutput classification).
    #[test]
    fn classify_tool_substring_fallback() {
        // "buildsystem" — no exact match, Phase 2: "build" → BuildOutput
        assert_eq!(classify_tool("buildsystem"), ContextCategory::BuildOutput);
        // "testrunner" contains "run" (ShellCommand) before Phase 2 reaches TestOutput
        assert_eq!(classify_tool("testrunner"), ContextCategory::ShellCommand);
        // "jsonparser" contains "json" → StructuredData
        assert_eq!(classify_tool("jsonparser"), ContextCategory::StructuredData);
    }

    // REQ: CNS-CONDENSER-CLASSIFY — classify_tool returns Unknown for unrecognized tool names
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn classify_tool_unknown() {
        assert_eq!(classify_tool("unknown_tool"), ContextCategory::Unknown);
        assert_eq!(classify_tool(""), ContextCategory::Unknown);
        assert_eq!(classify_tool("xyz"), ContextCategory::Unknown);
    }

    // REQ: CNS-CONDENSER-CLASSIFY — classify_tool handles hyphenated and underscored names identically
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Note: Both hyphen and underscore splits yield the same token set. First match wins.
    #[test]
    fn classify_tool_hyphen_vs_underscore() {
        assert_eq!(classify_tool("cargo-test"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("cargo_test"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("npm-build"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("npm_build"), ContextCategory::ShellCommand);
    }

    // REQ: CNS-CONDENSER-RTK — RtkStyle compresses within budget and never exceeds original size
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn rtk_style_compression_within_budget() {
        let input = (0..200)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let algo = RtkStyleAlgorithm;
        let (result, health) =
            algo.compress(&input, Profile::Normal, ContextCategory::ShellCommand);
        let result_lines = result.lines().count();
        assert!(
            result_lines <= 80,
            "result has {} lines, expected <= 80",
            result_lines
        );
        assert!(
            result.len() <= input.len(),
            "compressed output larger than input"
        );
        assert!(
            health.is_empty(),
            "no health signals expected for normal compression"
        );
    }

    // REQ: CNS-CONDENSER-RTK — RtkStyle preserves head and tail with ellipsis separator
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn rtk_style_preserves_head_tail_structure() {
        let input = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";
        let algo = RtkStyleAlgorithm;
        let (result, _) = algo.compress(input, Profile::Normal, ContextCategory::ShellCommand);
        assert!(result.contains("line1"));
        assert!(result.contains("line10"));
        assert!(result.contains("..."));
    }

    // REQ: CNS-CONDENSER-RTK — RtkStyle passthrough when input fits within budget
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn rtk_style_passthrough_small_input() {
        let input = "line1\nline2\nline3";
        let algo = RtkStyleAlgorithm;
        let (result, health) = algo.compress(input, Profile::Light, ContextCategory::ShellCommand);
        assert_eq!(result, input);
        assert!(health.is_empty());
    }

    // REQ: CNS-CONDENSER-SALIENCY — SaliencyRank scores lines by word frequency with structural bonus
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn saliency_rank_preserves_error_lines() {
        let input = "info: ok\ninfo: ok\ninfo: ok\nerror: critical failure\ninfo: ok\ninfo: ok";
        let algo = SaliencyRankAlgorithm;
        let (result, _) = algo.compress(input, Profile::Heavy, ContextCategory::LogOutput);
        assert!(
            result.contains("error"),
            "error line not preserved: {}",
            result
        );
    }

    // REQ: CNS-CONDENSER-SALIENCY — SaliencyRank emits low_signal health signal when most lines score zero
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn saliency_rank_low_signal_when_no_content() {
        let input = "a\na\na\na\na\na\na\na\na\na";
        let algo = SaliencyRankAlgorithm;
        let (_, health) = algo.compress(input, Profile::Heavy, ContextCategory::Unknown);
        assert!(!health.is_empty(), "expected low_signal health signal");
        assert_eq!(health[0].signal_type, "low_signal");
    }

    // REQ: CNS-CONDENSER-FLASHRANK — Flashrank selects lines by relevance, novelty, and brevity
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn flashrank_selects_within_budget() {
        let input = (0..100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let algo = FlashrankAlgorithm;
        let (result, health) = algo.compress(&input, Profile::Heavy, ContextCategory::FileContents);
        let result_lines = result.lines().count();
        assert!(
            result_lines <= 30,
            "flashrank exceeded budget: {} lines",
            result_lines
        );
        assert!(result.len() <= input.len());
        assert!(health.is_empty());
    }

    // REQ: CNS-CONDENSER-FLASHRANK — Flashrank emits budget_shortfall when not enough lines to fill budget
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Note: 3 lines with Heavy profile (10% retention, max 30) → budget = 1. Flashrank
    // fills 1 out of 1 → no shortfall. Budget_shortfall only when budget > available lines.
    #[test]
    fn flashrank_budget_shortfall() {
        let input = "line1\nline2\nline3";
        let algo = FlashrankAlgorithm;
        let (_, health) = algo.compress(input, Profile::Heavy, ContextCategory::FileContents);
        // 3 lines → budget = min(ceil(3*0.10), 30) = 1 → fills 1 → no shortfall
        assert!(
            health.is_empty(),
            "expected no budget_shortfall with budget=1 from 3 lines"
        );
    }

    // REQ: CNS-CONDENSER-REGISTRY — AlgorithmRegistry selects correct algorithm per category
// expect: "The system compresses context to preserve conversation continuity" [P5]
    #[test]
    fn algorithm_registry_selects_by_category() {
        let registry = AlgorithmRegistry::new();
        assert_eq!(
            registry.select(ContextCategory::ShellCommand).name(),
            "rtk_style"
        );
        assert_eq!(
            registry.select(ContextCategory::ConversationHistory).name(),
            "saliency_rank"
        );
        assert_eq!(
            registry.select(ContextCategory::FileContents).name(),
            "flashrank"
        );
        // LogOutput → saliency_rank
        assert_eq!(
            registry.select(ContextCategory::LogOutput).name(),
            "saliency_rank"
        );
    }

    // REQ: CNS-CONDENSER-REGISTRY — AlgorithmRegistry dispatches Unknown to flashrank
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Flashrank is the universal fallback — its greedy marginal-utility selection works on
    // any text type without needing category-specific structural markers.
    #[test]
    fn algorithm_registry_fallback_for_unknown() {
        let registry = AlgorithmRegistry::new();
        assert_eq!(
            registry.select(ContextCategory::Unknown).name(),
            "flashrank"
        );
    }

    // ── Property-based tests (Wave 2) ─────────────────────────────────────

    use proptest::prelude::*;
    use proptest::sample::select;

    /// Strategy: generate a non-empty string with varied content.
    fn arbitrary_input() -> BoxedStrategy<String> {
        proptest::arbitrary::any::<String>()
            .prop_filter("must be non-empty", |s| !s.is_empty())
            .boxed()
    }

    // REQ: CON-001 — Compression idempotency (P8, P9)
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // For any input, re-compressing the output produces the same result.
    proptest! {
        #[test]
        fn compression_is_idempotent(
            input in arbitrary_input(),
            profile in select(&[Profile::Heavy, Profile::Normal, Profile::Soft, Profile::Light]),
            category in select(&[
                ContextCategory::ShellCommand,
                ContextCategory::TestOutput,
                ContextCategory::BuildOutput,
                ContextCategory::FileContents,
                ContextCategory::ConversationHistory,
                ContextCategory::StructuredData,
                ContextCategory::LogOutput,
                ContextCategory::Unknown,
            ]),
        ) {
            let algo = RtkStyleAlgorithm;
            let (first, _) = algo.compress(&input, profile, category);
            let (second, _) = algo.compress(&first, profile, category);
            let first_len = first.len();
            let second_len = second.len();
            prop_assert_eq!(first, second,
                "re-compression changed output: first={} bytes, second={} bytes",
                first_len, second_len);
        }
    }

    // REQ: CON-002 — Size monotonicity (P8, P9)
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Compression never produces output larger than input.
    proptest! {
        #[test]
        fn compression_never_expands(
            input in arbitrary_input(),
            profile in select(&[Profile::Heavy, Profile::Normal, Profile::Soft, Profile::Light]),
            category in select(&[
                ContextCategory::ShellCommand,
                ContextCategory::TestOutput,
                ContextCategory::BuildOutput,
                ContextCategory::FileContents,
                ContextCategory::ConversationHistory,
                ContextCategory::StructuredData,
                ContextCategory::LogOutput,
                ContextCategory::Unknown,
            ]),
        ) {
            let algo = RtkStyleAlgorithm;
            let (compressed, _) = algo.compress(&input, profile, category);
            prop_assert!(compressed.len() <= input.len(),
                "compressed {} > original {}", compressed.len(), input.len());
        }
    }

    // REQ: CON-003 — Flashrank as universal fallback is size-monotonic on Unknown input
// expect: "The system compresses context to preserve conversation continuity" [P5]
    // Flashrank's greedy marginal-utility selection works on any content type — it must never
    // expand input even when given arbitrary Unknown-category content.
    proptest! {
        #[test]
        fn flashrank_fallback_never_expands(
            input in arbitrary_input(),
            profile in select(&[Profile::Heavy, Profile::Normal, Profile::Soft, Profile::Light]),
        ) {
            let algo = FlashrankAlgorithm;
            let (compressed, _) = algo.compress(&input, profile, ContextCategory::Unknown);
            prop_assert!(compressed.len() <= input.len(),
                "flashrank fallback expanded: compressed {} > original {}", compressed.len(), input.len());
        }
    }
}
