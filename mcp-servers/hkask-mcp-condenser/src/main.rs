//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Provides compression algorithms (rtk_style, saliency_rank, flashrank) for reducing
//! tool output size while preserving essential information. Phase 1 implements local
//! CPU-only algorithms with no LLM dependency. Phase 2 (deferred) adds LLM-assisted
//! algorithms via hkask-templates.

mod algorithms;
mod types;

use hkask_mcp::server::{McpToolError, McpToolOutput, emit_tool_span, run_stdio_server};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::Mutex;
use std::time::Instant;

use algorithms::AlgorithmRegistry;
use types::*;

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
        McpToolOutput::with_timing(serde_json::to_value(&result).unwrap_or_default(), start)
            .to_json_string()
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
