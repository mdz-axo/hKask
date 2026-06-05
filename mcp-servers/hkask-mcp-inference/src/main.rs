//! hKask MCP Inference — Okapi-backed LLM inference MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `inference_generate` — Generate text via Okapi LLM
//! - `inference_metrics` — Get current inference metrics
//! - `inference_models` — List available model tiers

use hkask_mcp_inference::tools::InferenceServer;

hkask_mcp::mcp_server_main!("hkask-mcp-inference", InferenceServer);
