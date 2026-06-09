//! hKask MCP Condenser — Compression algorithms (Phase 1: local, no LLM)

use super::types::*;

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
    fn compress(&self, input: &str, profile: Profile, category: ContextCategory) -> String;
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
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let (budget, passthrough) = compute_budget(lines.len(), profile);
        if passthrough {
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

        filtered.join("\n")
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
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let (budget, passthrough) = compute_budget(lines.len(), profile);
        if passthrough {
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
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let (budget, passthrough) = compute_budget(lines.len(), profile);
        if passthrough {
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

        let mut selected_indices: Vec<usize> = selected_indices.into_iter().collect();
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

/// Classify a tool name into a `ContextCategory` based on keyword matching.
///
/// Phase 1 splits on `_` and `-` and matches exact tokens against known prefixes.
/// Phase 2 falls back to substring matching for compound names without separators.
pub fn classify_tool(tool_name: &str) -> ContextCategory {
    let lower = tool_name.to_lowercase();
    let parts: Vec<&str> = lower.split('_').chain(lower.split('-')).collect();

    // Phase 1: known tool name prefixes — exact, no false positives
    for part in &parts {
        match *part {
            "git" | "docker" | "cargo" | "npm" | "shell" | "bash" | "exec" | "run" => {
                return ContextCategory::ShellCommand;
            }
            "test" | "pytest" | "spec" => return ContextCategory::TestOutput,
            "build" | "compile" | "make" => return ContextCategory::BuildOutput,
            "chat" | "conversation" | "message" => return ContextCategory::ConversationHistory,
            "log" | "journal" | "trace" => return ContextCategory::LogOutput,
            "json" | "api" | "query" => return ContextCategory::StructuredData,
            "file" | "read" | "cat" => return ContextCategory::FileContents,
            _ => {}
        }
    }

    // Phase 2: substring heuristic for compound/unknown tool names
    //
    // Tradeoff: substring matching produces false positives. E.g. "logistics"
    // contains "log" → LogOutput, "catalog" contains "cat" → FileContents.
    // This is acceptable because Phase 2 only fires when Phase 1 (exact token
    // matching) fails, so the tool name is already non-standard. The alternative
    // (word-boundary matching) would miss compound names like "gitlog" that
    // lack separators. If false positives become a problem, add word-boundary
    // checks (\b) for the shortest keywords (log, cat, run, api).
    if lower.contains("git")
        || lower.contains("docker")
        || lower.contains("cargo")
        || lower.contains("npm")
        || lower.contains("shell")
        || lower.contains("exec")
        || lower.contains("run")
        || lower.contains("bash")
    {
        ContextCategory::ShellCommand
    } else if lower.contains("test") || lower.contains("pytest") || lower.contains("spec") {
        ContextCategory::TestOutput
    } else if lower.contains("build") || lower.contains("compile") || lower.contains("make") {
        ContextCategory::BuildOutput
    } else if lower.contains("chat") || lower.contains("conversation") || lower.contains("message")
    {
        ContextCategory::ConversationHistory
    } else if lower.contains("log") || lower.contains("journal") || lower.contains("trace") {
        ContextCategory::LogOutput
    } else if lower.contains("json") || lower.contains("api") || lower.contains("query") {
        ContextCategory::StructuredData
    } else if lower.contains("file") || lower.contains("read") || lower.contains("cat") {
        ContextCategory::FileContents
    } else {
        ContextCategory::Unknown
    }
}

