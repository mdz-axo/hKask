//! hkask-mcp-inference — Okapi-backed LLM inference MCP server
//!
//! Exposes 3 MCP tools:
//! - `inference:generate` — Generate text via Okapi LLM
//! - `inference:metrics` — Get current inference metrics
//! - `inference:models` — List available model tiers

pub mod tools;

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_version() {
        let _version = super::SERVER_VERSION;
    }
}
