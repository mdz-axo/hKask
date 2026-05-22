//! hkask-mcp-memory — MCP server placeholder

/// Server version
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Placeholder server struct
pub struct PlaceholderServer;

impl PlaceholderServer {
    pub fn new() -> Self {
        Self
    }

    pub fn info(&self) -> &'static str {
        "hkask-mcp-memory server"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        let server = PlaceholderServer::new();
        assert!(server.info().contains("hkask-mcp"));
    }
}
