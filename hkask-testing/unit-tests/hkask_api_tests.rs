// Auto-extracted inline tests for hkask-api
// Extracted: Thu May 21 00:22:36 PDT 2026

// === From lib.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_state_new() {
        let registry = SqliteRegistry::new(None).unwrap();
        let mcp_runtime = hkask_mcp::runtime::McpRuntime::new();
        let pod_manager = PodManager::new_mock();
        let system_webid = WebID::new();
        let state = ApiState::new(
            registry,
            mcp_runtime,
            pod_manager,
            b"test-secret",
            system_webid,
        );
        assert_eq!(state.mcp_runtime.tool_count().await, 0);
    }
}
