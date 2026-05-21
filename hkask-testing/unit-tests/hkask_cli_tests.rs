// Auto-extracted inline tests for hkask-cli
// Extracted: Thu May 21 00:22:36 PDT 2026

// === From commands.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        // Test would require a mock registry
    }

    #[tokio::test]
    async fn test_list_mcp_servers() {
        let runtime = McpRuntime::new();
        let servers = list_mcp_servers(&runtime).await;
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_list_mcp_tools() {
        let runtime = McpRuntime::new();
        let tools = list_mcp_tools(&runtime).await;
        assert!(tools.is_empty());
    }
}
