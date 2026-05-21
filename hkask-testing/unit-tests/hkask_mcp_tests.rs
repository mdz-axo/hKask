// Auto-extracted inline tests for hkask-mcp
// Extracted: Thu May 21 00:22:36 PDT 2026

// === From dispatch.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use hkask_cns::spans::SpanEmitter;

    struct MockCns;
    impl CnsPort for MockCns {
        fn emit(&self, _span: &str, _outcome: Value, _confidence: f64) {
            // Mock implementation
        }
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_new() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        assert_eq!(dispatcher.list_tools().await.len(), 0);
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_capability_check() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let bot_id = WebID::new();
        let caps =
            BotCapabilities::new(bot_id).with_capabilities(vec!["inference:call", "storage:read"]);

        dispatcher.register_bot_capabilities(caps).await;

        assert!(dispatcher.check_capability(&bot_id, "inference:call").await);
        assert!(dispatcher.check_capability(&bot_id, "storage:read").await);
        assert!(!dispatcher.check_capability(&bot_id, "memory:write").await);
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_invoke_capability_denied() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let bot_id = WebID::new();
        let caps = BotCapabilities::new(bot_id).with_capabilities(vec!["inference:call"]);

        dispatcher.register_bot_capabilities(caps).await;

        let result = dispatcher
            .invoke_async(&bot_id, "memory:write", Value::Null, &MockCns)
            .await;

        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("CapabilityDenied"));
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_invoke_tool_not_found() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let bot_id = WebID::new();
        let caps = BotCapabilities::new(bot_id).with_capabilities(vec!["nonexistent:tool"]);

        dispatcher.register_bot_capabilities(caps).await;

        let result = dispatcher
            .invoke_async(&bot_id, "nonexistent:tool", Value::Null, &MockCns)
            .await;

        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("not found"));
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_rate_limit() {
        use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};

        let runtime = McpRuntime::new();
        let mut dispatcher = McpDispatcher::new(runtime, b"test-secret");

        // Set very low rate limit for testing
        let bot_id = WebID::new();
        dispatcher.rate_limiter.configure_bot(
            &bot_id,
            RateLimitConfig {
                max_tokens: 2,
                refill_interval: std::time::Duration::from_secs(60),
            },
        );

        let caps = BotCapabilities::new(bot_id).with_capabilities(vec!["test:tool"]);
        dispatcher.register_bot_capabilities(caps).await;

        // Register a tool
        use crate::runtime::McpServer;
        dispatcher
            .runtime()
            .register_server(McpServer {
                id: "test".to_string(),
                name: "Test".to_string(),
                tools: vec![crate::runtime::McpTool {
                    name: "test:tool".to_string(),
                    description: "Test".to_string(),
                    input_schema: serde_json::json!({"type": "object"}),
                    server_id: "test".to_string(),
                }],
                connected: true,
            })
            .await;

        // First two calls should succeed
        assert!(dispatcher.check_rate_limit(&bot_id));
        assert!(dispatcher.check_rate_limit(&bot_id));

        // Third call should fail (rate limited)
        assert!(!dispatcher.check_rate_limit(&bot_id));
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_issue_capability() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let from = WebID::new();
        let to = WebID::new();

        let token = dispatcher.issue_capability("inference:call".to_string(), from, to);

        assert_eq!(token.tool_name, "inference:call");
        assert!(dispatcher.capability_checker.verify(&token));
    }
}

// === From runtime.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_runtime_new() {
        let runtime = McpRuntime::new();
        assert_eq!(runtime.tool_count().await, 0);
    }

    #[tokio::test]
    async fn test_mcp_runtime_register_server() {
        let runtime = McpRuntime::new();

        let server = McpServer {
            id: "test-server".to_string(),
            name: "Test Server".to_string(),
            tools: vec![McpTool {
                name: "test_tool".to_string(),
                description: "Test tool".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
                server_id: "test-server".to_string(),
            }],
            connected: true,
        };

        runtime.register_server(server).await;

        assert_eq!(runtime.tool_count().await, 1);
        assert!(runtime.tool_exists("test_tool").await);

        let tool = runtime.get_tool("test_tool").await;
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name, "test_tool");
    }

    #[tokio::test]
    async fn test_mcp_runtime_discover_tools() {
        let runtime = McpRuntime::new();

        runtime
            .register_server(McpServer {
                id: "server1".to_string(),
                name: "Server 1".to_string(),
                tools: vec![
                    McpTool {
                        name: "tool1".to_string(),
                        description: "Tool 1".to_string(),
                        input_schema: serde_json::json!({"type": "object"}),
                        server_id: "server1".to_string(),
                    },
                    McpTool {
                        name: "tool2".to_string(),
                        description: "Tool 2".to_string(),
                        input_schema: serde_json::json!({"type": "object"}),
                        server_id: "server1".to_string(),
                    },
                ],
                connected: true,
            })
            .await;

        let tools = runtime.discover_tools().await;
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"tool1".to_string()));
        assert!(tools.contains(&"tool2".to_string()));
    }

    #[tokio::test]
    async fn test_mcp_runtime_unregister_server() {
        let runtime = McpRuntime::new();

        runtime
            .register_server(McpServer {
                id: "server1".to_string(),
                name: "Server 1".to_string(),
                tools: vec![McpTool {
                    name: "tool1".to_string(),
                    description: "Tool 1".to_string(),
                    input_schema: serde_json::json!({"type": "object"}),
                    server_id: "server1".to_string(),
                }],
                connected: true,
            })
            .await;

        assert_eq!(runtime.tool_count().await, 1);

        runtime.unregister_server("server1").await;

        assert_eq!(runtime.tool_count().await, 0);
        assert!(!runtime.tool_exists("tool1").await);
    }

    #[tokio::test]
    async fn test_mcp_runtime_list_servers() {
        let runtime = McpRuntime::new();

        runtime
            .register_server(McpServer {
                id: "server1".to_string(),
                name: "Server 1".to_string(),
                tools: vec![],
                connected: true,
            })
            .await;

        runtime
            .register_server(McpServer {
                id: "server2".to_string(),
                name: "Server 2".to_string(),
                tools: vec![],
                connected: true,
            })
            .await;

        let servers = runtime.list_servers().await;
        assert_eq!(servers.len(), 2);
    }
}

// === From security.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();

        assert_eq!(policy.max_input_size, 1024 * 1024);
        assert!(policy.allowed_tools.is_empty());
        assert!(policy.denied_tools.contains("admin:"));
        assert!(policy.denied_tools.contains("system:"));
        assert!(policy.require_capabilities);
        assert!(policy.enable_rate_limiting);
    }

    #[test]
    fn test_security_gateway_validate_input_size() {
        let gateway = SecurityGateway::with_default_policy(b"test-secret");

        // Small input should pass
        let small_input = Value::String("small".to_string());
        assert!(gateway.validate_input_size(&small_input).is_ok());

        // Large input should fail
        let large_input = Value::String("x".repeat(2 * 1024 * 1024));
        assert!(gateway.validate_input_size(&large_input).is_err());
    }

    #[test]
    fn test_security_gateway_is_tool_allowed() {
        let gateway = SecurityGateway::with_default_policy(b"test-secret");

        // Normal tools should be allowed
        assert!(gateway.is_tool_allowed("inference:call"));
        assert!(gateway.is_tool_allowed("storage:read"));

        // Denied prefixes should be blocked
        assert!(!gateway.is_tool_allowed("admin:delete"));
        assert!(!gateway.is_tool_allowed("system:shutdown"));
        assert!(!gateway.is_tool_allowed("internal:debug"));
    }

    #[test]
    fn test_security_gateway_is_tool_allowed_custom() {
        let mut policy = SecurityPolicy::default();
        policy.allowed_tools.insert("inference:".to_string());
        policy.allowed_tools.insert("storage:".to_string());

        let gateway = SecurityGateway::new(b"test-secret", policy);

        // Allowed tools
        assert!(gateway.is_tool_allowed("inference:call"));
        assert!(gateway.is_tool_allowed("storage:read"));

        // Not in allowed list
        assert!(!gateway.is_tool_allowed("memory:write"));
    }

    #[test]
    fn test_security_gateway_rate_limit() {
        let gateway = SecurityGateway::with_default_policy(b"test-secret");

        let bot_id = WebID::new();

        // Default rate limit is 100 requests/minute
        // First check should succeed
        assert!(gateway.check_rate_limit(&bot_id));
    }

    #[test]
    fn test_security_gateway_issue_capability() {
        let gateway = SecurityGateway::with_default_policy(b"test-secret");

        let from = WebID::new();
        let to = WebID::new();

        let token = gateway.issue_capability("inference:call".to_string(), from, to);

        assert_eq!(token.tool_name, "inference:call");
        assert!(gateway.verify_capability(&token, &to, "inference:call"));
        assert!(!gateway.verify_capability(&token, &from, "inference:call")); // Wrong recipient
    }

    #[tokio::test]
    async fn test_security_gateway_audit() {
        let gateway = SecurityGateway::with_default_policy(b"test-secret");

        let bot_id = WebID::new();
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            bot_id,
            tool_name: "test:tool".to_string(),
            action: AuditAction::ToolInvocation,
            success: true,
            error_message: None,
        };

        gateway.audit(entry).await;

        let log = gateway.get_audit_log(10).await;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].tool_name, "test:tool");
    }

    #[tokio::test]
    async fn test_security_gateway_audit_trim() {
        let gateway = SecurityGateway::with_default_policy(b"test-secret");

        // Add more than 10,000 entries
        for i in 0..10100 {
            let entry = AuditEntry {
                timestamp: chrono::Utc::now(),
                bot_id: WebID::new(),
                tool_name: format!("tool:{}", i),
                action: AuditAction::ToolInvocation,
                success: true,
                error_message: None,
            };
            gateway.audit(entry).await;
        }

        let log = gateway.get_audit_log(20000).await;
        assert_eq!(log.len(), 10000); // Should be trimmed to 10,000
    }
}
