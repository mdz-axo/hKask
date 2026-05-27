//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Provides compression algorithms (rtk_style, saliency_rank, flashrank) for reducing
//! tool output size while preserving essential information. Phase 1 implements local
//! CPU-only algorithms with no LLM dependency. Phase 2 (deferred) adds LLM-assisted
//! algorithms via hkask-templates.

use hkask_mcp::server::{
    McpToolError, McpToolOutput, emit_tool_span, run_stdio_server,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompressRequest {
    pub tool_name: String,
    pub output: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetProfileRequest {
    pub profile: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClassifyRequest {
    pub tool_name: String,
}

// =============================================================================
// Domain types
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Heavy,
    Normal,
    Soft,
    Light,
}

impl Profile {
    fn retention_pct(&self) -> f64 {
        match self {
            Profile::Heavy => 0.10,
            Profile::Normal => 0.20,
            Profile::Soft => 0.60,
            Profile::Light => 0.95,
        }
    }

    fn max_lines(&self) -> Option<usize> {
        match self {
            Profile::Heavy => Some(30),
            Profile::Normal => Some(80),
            Profile::Soft => Some(200),
            Profile::Light => None,
        }
    }
}

impl std::str::FromStr for Profile {
    type Err = McpToolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "heavy" => Ok(Profile::Heavy),
            "normal" => Ok(Profile::Normal),
            "soft" => Ok(Profile::Soft),
            "light" => Ok(Profile::Light),
            _ => Err(McpToolError::invalid_argument(format!(
                "Unknown profile '{s}'. Use: heavy, normal, soft, light"
            ))),
        }
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Profile::Heavy => write!(f, "heavy"),
            Profile::Normal => write!(f, "normal"),
            Profile::Soft => write!(f, "soft"),
            Profile::Light => write!(f, "light"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCategory {
    ShellCommand,
    TestOutput,
    BuildOutput,
    FileContents,
    ConversationHistory,
    StructuredData,
    LogOutput,
    Unknown,
}

impl ContextCategory {
    fn label(&self) -> &str {
        match self {
            ContextCategory::ShellCommand => "shell_command",
            ContextCategory::TestOutput => "test_output",
            ContextCategory::BuildOutput => "build_output",
            ContextCategory::FileContents => "file_contents",
            ContextCategory::ConversationHistory => "conversation_history",
            ContextCategory::StructuredData => "structured_data",
            ContextCategory::LogOutput => "log_output",
            ContextCategory::Unknown => "unknown",
        }
    }
}

impl std::str::FromStr for ContextCategory {
    type Err = McpToolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "shell_command" => Ok(ContextCategory::ShellCommand),
            "test_output" => Ok(ContextCategory::TestOutput),
            "build_output" => Ok(ContextCategory::BuildOutput),
            "file_contents" => Ok(ContextCategory::FileContents),
            "conversation_history" => Ok(ContextCategory::ConversationHistory),
            "structured_data" => Ok(ContextCategory::StructuredData),
            "log_output" => Ok(ContextCategory::LogOutput),
            _ => Ok(ContextCategory::Unknown),
        }
    }
}

fn classify_tool(tool_name: &str) -> ContextCategory {
    let lower = tool_name.to_lowercase();
    if lower.contains("git") || lower.contains("docker") || lower.contains("cargo")
        || lower.contains("npm") || lower.contains("shell") || lower.contains("exec")
        || lower.contains("run") || lower.contains("bash")
    {
        ContextCategory::ShellCommand
    } else if lower.contains("test") || lower.contains("pytest") || lower.contains("spec") {
        ContextCategory::TestOutput
    } else if lower.contains("build") || lower.contains("compile") || lower.contains("make") {
        ContextCategory::BuildOutput
    } else if lower.contains("file") || lower.contains("read") || lower.contains("cat") {
        ContextCategory::FileContents
    } else if lower.contains("chat") || lower.contains("conversation") || lower.contains("message") {
        ContextCategory::ConversationHistory
    } else if lower.contains("json") || lower.contains("api") || lower.contains("query") {
        ContextCategory::StructuredData
    } else if lower.contains("log") || lower.contains("journal") || lower.contains("trace") {
        ContextCategory::LogOutput
    } else {
        ContextCategory::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedOutput {
    pub content: String,
    pub algorithm: String,
    pub category: String,
    pub profile: String,
    pub original_lines: usize,
    pub compressed_lines: usize,
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub reduction_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondenserStats {
    pub total_compressions: u64,
    pub total_original_bytes: u64,
    pub total_compressed_bytes: u64,
    pub algorithm_usage: std::collections::HashMap<String, u64>,
    pub category_usage: std::collections::HashMap<String, u64>,
    pub current_profile: String,
}

impl Default for CondenserStats {
    fn default() -> Self {
        Self {
            total_compressions: 0,
            total_original_bytes: 0,
            total_compressed_bytes: 0,
            algorithm_usage: std::collections::HashMap::new(),
            category_usage: std::collections::HashMap::new(),
            current_profile: "normal".to_string(),
        }
    }
}

// =============================================================================
// Algorithm trait
// =============================================================================

trait CondenserAlgorithm: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_for(&self) -> &[ContextCategory];
    fn compress(&self, input: &str, profile: Profile, category: ContextCategory) -> String;
    fn handles(&self, category: ContextCategory) -> bool;
}

// =============================================================================
// RtkStyleAlgorithm — command-specific rules
// =============================================================================

struct RtkStyleAlgorithm;

impl CondenserAlgorithm for RtkStyleAlgorithm {
    fn name(&self) -> &str { "rtk_style" }
    fn description(&self) -> &str { "Command-specific rules: filter, group, truncate, dedup" }
    fn default_for(&self) -> &[ContextCategory] {
        &[ContextCategory::ShellCommand, ContextCategory::TestOutput, ContextCategory::BuildOutput]
    }
    fn handles(&self, category: ContextCategory) -> bool {
        matches!(category, ContextCategory::ShellCommand | ContextCategory::TestOutput | ContextCategory::BuildOutput)
    }
    fn compress(&self, input: &str, profile: Profile, category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let max_lines = profile.max_lines().unwrap_or(usize::MAX);
        let retention = profile.retention_pct();
        let target_lines = ((lines.len() as f64) * retention).max(1.0) as usize;
        let budget = target_lines.min(max_lines).min(lines.len());

        if budget >= lines.len() {
            return input.to_string();
        }

        let mut filtered: Vec<&str> = Vec::new();

        let (_important_patterns, _skip_patterns): (Vec<&str>, Vec<&str>) = match category {
            ContextCategory::ShellCommand => (
                vec!["error", "Error", "ERROR", "fatal", "warning", "Warning", "changed", "modified", "deleted", "created", "+++", "---"],
                vec!["^$", "  "],
            ),
            ContextCategory::TestOutput => (
                vec!["FAIL", "PASS", "fail", "pass", "error", "Error", "panic", "FAILED", "ok ", "test result"],
                vec!["running", "---"],
            ),
            ContextCategory::BuildOutput => (
                vec!["error", "Error", "ERROR", "warning", "Warning", "FAILED", "Compiling", "Building", "Finished", "error:"],
                vec!["^$", "  "],
            ),
            _ => (vec![], vec![]),
        };

        let head_count = (budget as f64 * 0.3) as usize;
        let tail_count = budget.saturating_sub(head_count);

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

// =============================================================================
// SaliencyRankAlgorithm — TF-IDF + entropy scoring
// =============================================================================

struct SaliencyRankAlgorithm;

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
        let tf_sum: f64 = words.iter()
            .map(|w| freq.get(&w.to_lowercase()).copied().unwrap_or(0.0))
            .sum();

        let structural_bonus = if line.contains("error") || line.contains("Error") || line.contains("ERROR") { 2.0 }
            else if line.contains("warning") || line.contains("Warning") { 1.0 }
            else if line.starts_with('#') || line.starts_with("##") { 0.5 }
            else if line.starts_with('-') || line.starts_with('*') { 0.2 }
            else { 0.0 };

        tf_sum / words.len() as f64 + structural_bonus
    }
}

impl CondenserAlgorithm for SaliencyRankAlgorithm {
    fn name(&self) -> &str { "saliency_rank" }
    fn description(&self) -> &str { "TF-IDF + entropy scoring with structural bonus" }
    fn default_for(&self) -> &[ContextCategory] {
        &[ContextCategory::ConversationHistory, ContextCategory::LogOutput, ContextCategory::Unknown]
    }
    fn handles(&self, category: ContextCategory) -> bool {
        matches!(category, ContextCategory::ConversationHistory | ContextCategory::LogOutput | ContextCategory::Unknown)
    }
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let max_lines = profile.max_lines().unwrap_or(usize::MAX);
        let target_lines = ((lines.len() as f64) * profile.retention_pct()).max(1.0) as usize;
        let budget = target_lines.min(max_lines).min(lines.len());

        if budget >= lines.len() {
            return input.to_string();
        }

        let freq = Self::compute_word_frequencies(&lines);

        let mut scored: Vec<(usize, f64, &str)> = lines.iter().enumerate()
            .map(|(i, line)| (i, Self::line_score(line, &freq), *line))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut selected_indices: Vec<usize> = scored.into_iter()
            .take(budget)
            .map(|(i, _, _)| i)
            .collect();
        selected_indices.sort_unstable();

        let mut result = String::new();
        let mut last_idx: Option<usize> = None;
        for idx in selected_indices {
            if let Some(li) = last_idx {
                if idx > li + 1 {
                    result.push_str("...\n");
                }
            }
            result.push_str(lines[idx]);
            result.push('\n');
            last_idx = Some(idx);
        }

        result.trim_end().to_string()
    }
}

// =============================================================================
// FlashrankAlgorithm — greedy marginal-utility selection
// =============================================================================

struct FlashrankAlgorithm;

impl FlashrankAlgorithm {
    fn relevance_score(line: &str, query_terms: &[String]) -> f64 {
        let lower = line.to_lowercase();
        query_terms.iter()
            .map(|term| if lower.contains(&term.to_lowercase()) { 1.0 } else { 0.0 })
            .sum()
    }

    fn novelty_score(line: &str, selected: &[&str]) -> f64 {
        let lower = line.to_lowercase();
        let words: std::collections::HashSet<&str> = lower.split_whitespace().collect();
        let mut overlap = 0usize;
        let mut total = 0usize;
        for prev in selected {
            let prev_lower = prev.to_lowercase();
            let prev_words: std::collections::HashSet<&str> = prev_lower.split_whitespace().collect();
            for w in &words {
                total += 1;
                if prev_words.contains(w) { overlap += 1; }
            }
        }
        if total == 0 { return 1.0; }
        1.0 - (overlap as f64 / total as f64)
    }

    fn brevity_score(line: &str) -> f64 {
        let len = line.len() as f64;
        if len == 0.0 { return 0.0; }
        1.0 / (1.0 + len / 100.0)
    }
}

impl CondenserAlgorithm for FlashrankAlgorithm {
    fn name(&self) -> &str { "flashrank" }
    fn description(&self) -> &str { "Greedy marginal-utility selection under token budget" }
    fn default_for(&self) -> &[ContextCategory] {
        &[ContextCategory::FileContents, ContextCategory::StructuredData]
    }
    fn handles(&self, category: ContextCategory) -> bool {
        matches!(category, ContextCategory::FileContents | ContextCategory::StructuredData)
    }
    fn compress(&self, input: &str, profile: Profile, _category: ContextCategory) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let max_lines = profile.max_lines().unwrap_or(usize::MAX);
        let target_lines = ((lines.len() as f64) * profile.retention_pct()).max(1.0) as usize;
        let budget = target_lines.min(max_lines).min(lines.len());

        if budget >= lines.len() {
            return input.to_string();
        }

        let alpha = 0.4f64;
        let beta = 0.3f64;
        let gamma = 0.3f64;

        let query_terms: Vec<String> = lines.iter()
            .take(5)
            .flat_map(|l| l.split_whitespace().filter(|w| w.len() > 3).map(|s| s.to_string()))
            .take(20)
            .collect();

        let mut selected_indices: Vec<usize> = Vec::new();
        let mut selected_lines: Vec<&str> = Vec::new();

        while selected_indices.len() < budget {
            let mut best_idx: Option<usize> = None;
            let mut best_score = f64::NEG_INFINITY;

            for (i, line) in lines.iter().enumerate() {
                if selected_indices.contains(&i) { continue; }

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
            if let Some(li) = last_idx {
                if idx > li + 1 {
                    result.push_str("...\n");
                }
            }
            result.push_str(lines[idx]);
            result.push('\n');
            last_idx = Some(idx);
        }

        result.trim_end().to_string()
    }
}

// =============================================================================
// AlgorithmRegistry
// =============================================================================

struct AlgorithmRegistry {
    algorithms: Vec<Box<dyn CondenserAlgorithm>>,
}

impl AlgorithmRegistry {
    fn new() -> Self {
        let algorithms: Vec<Box<dyn CondenserAlgorithm>> = vec![
            Box::new(RtkStyleAlgorithm),
            Box::new(SaliencyRankAlgorithm),
            Box::new(FlashrankAlgorithm),
        ];
        Self { algorithms }
    }

    fn select(&self, category: ContextCategory) -> &dyn CondenserAlgorithm {
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
        self.algorithms.last().expect("at least one algorithm").as_ref()
    }

    fn list_algorithms(&self) -> Vec<serde_json::Value> {
        self.algorithms.iter().map(|a| {
            serde_json::json!({
                "name": a.name(),
                "description": a.description(),
                "default_for": a.default_for().iter().map(|c| c.label()).collect::<Vec<_>>(),
            })
        }).collect()
    }
}

// =============================================================================
// CondenserEngine
// =============================================================================

struct CondenserEngine {
    registry: AlgorithmRegistry,
    profile: Profile,
    stats: CondenserStats,
}

impl CondenserEngine {
    fn new() -> Self {
        Self {
            registry: AlgorithmRegistry::new(),
            profile: Profile::Normal,
            stats: CondenserStats::default(),
        }
    }

    fn compress(&mut self, tool_name: &str, output: &str, category: Option<ContextCategory>) -> CompressedOutput {
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

        *self.stats.algorithm_usage.entry(algorithm_name.clone()).or_insert(0) += 1;
        *self.stats.category_usage.entry(cat.label().to_string()).or_insert(0) += 1;
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

    fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        self.stats.current_profile = profile.to_string();
    }

    fn get_stats(&self) -> &CondenserStats {
        &self.stats
    }
}

// =============================================================================
// CondenserServer
// =============================================================================

pub struct CondenserServer {
    engine: Mutex<CondenserEngine>,
}

impl CondenserServer {
    fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            engine: Mutex::new(CondenserEngine::new()),
        })
    }
}

#[tool_router(server_handler)]
impl CondenserServer {
    #[tool(description = "Liveness and profile info")]
    async fn condenser_ping(&self) -> String {
        let engine = self.engine.lock().unwrap();
        McpToolOutput::new(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "profile": engine.stats.current_profile,
            "algorithms": engine.registry.list_algorithms(),
        }))
        .to_json_string()
    }

    #[tool(description = "Compress tool output using context-aware algorithms")]
    async fn condenser_compress(
        &self,
        Parameters(CompressRequest { tool_name, output, category }): Parameters<CompressRequest>,
    ) -> String {
        let start = Instant::now();
        if output.is_empty() {
            return McpToolError::invalid_argument("output must not be empty").to_json_string();
        }
        let cat = category.as_deref().and_then(|c| c.parse::<ContextCategory>().ok());
        let mut engine = self.engine.lock().unwrap();
        let result = engine.compress(&tool_name, &output, cat);
        emit_tool_span("condenser_compress", "ok", start.elapsed().as_millis() as u64, None);
        McpToolOutput::with_timing(serde_json::to_value(&result).unwrap_or_default(), start).to_json_string()
    }

    #[tool(description = "Set compression profile (heavy/normal/soft/light)")]
    async fn condenser_set_profile(
        &self,
        Parameters(SetProfileRequest { profile }): Parameters<SetProfileRequest>,
    ) -> String {
        let start = Instant::now();
        let p = match profile.parse::<Profile>() {
            Ok(p) => p,
            Err(e) => return e.to_json_string(),
        };
        let mut engine = self.engine.lock().unwrap();
        engine.set_profile(p);
        emit_tool_span("condenser_set_profile", "ok", start.elapsed().as_millis() as u64, None);
        McpToolOutput::new(serde_json::json!({
            "profile": p.to_string(),
            "retention_pct": p.retention_pct(),
            "max_lines": p.max_lines(),
        }))
        .to_json_string()
    }

    #[tool(description = "Cumulative compression statistics")]
    async fn condenser_stats(&self) -> String {
        let engine = self.engine.lock().unwrap();
        McpToolOutput::new(serde_json::to_value(engine.get_stats()).unwrap_or_default()).to_json_string()
    }

    #[tool(description = "Classify tool name to context category")]
    async fn condenser_classify(
        &self,
        Parameters(ClassifyRequest { tool_name }): Parameters<ClassifyRequest>,
    ) -> String {
        let category = classify_tool(&tool_name);
        let engine = self.engine.lock().unwrap();
        let algo = engine.registry.select(category);
        McpToolOutput::new(serde_json::json!({
            "tool_name": tool_name,
            "category": category.label(),
            "algorithm": algo.name(),
        }))
        .to_json_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-condenser",
        SERVER_VERSION,
        CondenserServer::new,
        vec![],
    )
    .await
}