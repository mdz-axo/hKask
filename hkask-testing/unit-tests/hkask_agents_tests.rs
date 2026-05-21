// Auto-extracted inline tests for hkask-agents
// Extracted: Thu May 21 00:22:14 PDT 2026

// === From acp.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acp_runtime_register_agent() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let webid = WebID::new();

        let token = runtime
            .register_agent(webid, "Bot".to_string(), vec!["inference:call".to_string()])
            .await
            .unwrap();

        assert!(runtime.is_registered(&webid).await);
        assert!(runtime.verify_capability(&token));

        let agent = runtime.get_agent(&webid).await.unwrap();
        assert_eq!(agent.webid, webid);
        assert_eq!(agent.agent_type, "Bot");
        assert_eq!(agent.capabilities.len(), 1);
    }

    #[tokio::test]
    async fn test_acp_runtime_unregister_agent() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let webid = WebID::new();

        runtime
            .register_agent(webid, "Bot".to_string(), vec![])
            .await
            .unwrap();

        assert!(runtime.is_registered(&webid).await);

        runtime.unregister_agent(&webid).await.unwrap();

        assert!(!runtime.is_registered(&webid).await);
    }

    #[tokio::test]
    async fn test_acp_runtime_duplicate_registration() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let webid = WebID::new();

        runtime
            .register_agent(webid, "Bot".to_string(), vec![])
            .await
            .unwrap();

        let result = runtime
            .register_agent(webid, "Bot".to_string(), vec![])
            .await;

        assert!(matches!(result, Err(_)));
    }

    #[tokio::test]
    async fn test_acp_runtime_wildcard_rejected() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let webid = WebID::new();

        let result = runtime
            .register_agent(webid, "Bot".to_string(), vec!["*".to_string()])
            .await;

        assert!(matches!(result, Err(_)));
    }

    #[tokio::test]
    async fn test_acp_runtime_send_message() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let from = WebID::new();
        let to = WebID::new();

        runtime
            .register_agent(from, "Bot".to_string(), vec![])
            .await
            .unwrap();
        runtime
            .register_agent(to, "Bot".to_string(), vec![])
            .await
            .unwrap();

        let handler = TemplateDispatchHandler::new(Arc::new(runtime));

        let correlation_id = handler
            .dispatch(
                from,
                Some(to),
                "test/template".to_string(),
                serde_json::json!({"test": "data"}),
            )
            .await
            .unwrap();

        assert!(!correlation_id.is_empty());
    }

    #[tokio::test]
    async fn test_acp_runtime_capability_check() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let webid = WebID::new();

        // Register agent with explicit capabilities
        runtime
            .register_agent(webid, "Bot".to_string(), vec!["inference:call".to_string()])
            .await
            .unwrap();

        // Explicit capability should work
        assert!(runtime.has_capability(&webid, "inference:call").await);

        // Other capabilities should not work (no wildcards)
        assert!(!runtime.has_capability(&webid, "memory:write").await);

        // Unregistered agent has no capabilities
        let other_webid = WebID::new();
        assert!(!runtime.has_capability(&other_webid, "inference:call").await);
    }

    #[tokio::test]
    async fn test_acp_runtime_list_agents() {
        let runtime = AcpRuntime::new(b"test-secret", None);

        runtime
            .register_agent(WebID::new(), "Bot".to_string(), vec![])
            .await
            .unwrap();
        runtime
            .register_agent(WebID::new(), "Replicant".to_string(), vec![])
            .await
            .unwrap();

        let agents = runtime.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_template_dispatch_handler() {
        let runtime = Arc::new(AcpRuntime::new(b"test-secret", None));
        let from = WebID::new();
        let to = WebID::new();

        runtime
            .register_agent(from, "Bot".to_string(), vec![])
            .await
            .unwrap();
        runtime
            .register_agent(to, "Bot".to_string(), vec![])
            .await
            .unwrap();

        let handler = TemplateDispatchHandler::new(runtime.clone());

        // Dispatch
        let correlation_id = handler
            .dispatch(
                from,
                Some(to),
                "test/template".to_string(),
                serde_json::json!({"input": "test"}),
            )
            .await
            .unwrap();

        // Get message
        let message = runtime.get_message(&correlation_id).await.unwrap();
        assert!(matches!(message, A2AMessage::TemplateDispatch { .. }));

        // Respond
        handler
            .respond(
                correlation_id.clone(),
                serde_json::json!({"result": "success"}),
                None,
            )
            .await
            .unwrap();

        // Get response
        let response = runtime.get_message(&correlation_id).await.unwrap();
        assert!(matches!(response, A2AMessage::TemplateResponse { .. }));
    }

    #[tokio::test]
    async fn test_root_authority_create_token() {
        let root_webid = WebID::new();
        let root_authority = RootAuthority::new(root_webid, b"root-secret");

        let target = WebID::new();
        let token = root_authority
            .create_root_token(
                CapabilityResource::Tool,
                "test:tool".to_string(),
                CapabilityAction::Execute,
                target,
            )
            .await
            .unwrap();

        assert_eq!(token.delegated_to, target);
        assert_eq!(token.attenuation_level, 0);
        assert_eq!(token.max_attenuation, 7);
    }

    #[tokio::test]
    async fn test_root_authority_verify_chain() {
        let root_webid = WebID::new();
        let root_authority = RootAuthority::new(root_webid, b"root-secret");

        let target = WebID::new();
        let token = root_authority
            .create_root_token(
                CapabilityResource::Tool,
                "test:tool".to_string(),
                CapabilityAction::Execute,
                target,
            )
            .await
            .unwrap();

        // Verify chain is valid
        let result = root_authority.verify_attenuation_chain(&token, &root_webid);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_capability_delegation() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let holder = WebID::new();
        let delegate = WebID::new();

        // Register holder and get token
        let token = runtime
            .register_agent(holder, "Bot".to_string(), vec!["tool:test".to_string()])
            .await
            .unwrap();

        // Delegate to new holder
        let current_time = 1000;
        let delegated = runtime
            .delegate_capability(&token, delegate, current_time)
            .await
            .unwrap();

        assert_eq!(delegated.delegated_to, delegate);
        assert_eq!(delegated.attenuation_level, 1);
        assert!(runtime.verify_capability(&delegated));
    }

    #[tokio::test]
    async fn test_capability_chain_verification() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        let holder = WebID::new();

        // Register and get token
        let token = runtime
            .register_agent(holder, "Bot".to_string(), vec!["tool:test".to_string()])
            .await
            .unwrap();

        // Verify chain
        let result = runtime.verify_capability_chain(&token);
        assert!(result.is_ok());
    }
}

// === From capability.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_token_creation() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            "inference:call".to_string(),
            from.clone(),
            to.clone(),
            secret,
        );

        assert!(!token.id.is_empty());
        assert_eq!(token.tool_name, "inference:call");
        assert_eq!(token.delegated_from, from);
        assert_eq!(token.delegated_to, to);
        assert!(!token.signature.is_empty());
    }

    #[test]
    fn test_capability_token_verification() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            "inference:call".to_string(),
            from.clone(),
            to.clone(),
            secret,
        );

        assert!(token.verify(secret));
    }

    #[test]
    fn test_capability_token_invalid_signature() {
        let secret = b"test-secret-key";
        let wrong_secret = b"wrong-secret-key";
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            "inference:call".to_string(),
            from.clone(),
            to.clone(),
            secret,
        );

        assert!(!token.verify(wrong_secret));
    }

    #[test]
    fn test_capability_checker() {
        let secret = b"test-secret-key";
        let checker = CapabilityChecker::new(secret);

        let from = WebID::new();
        let to = WebID::new();

        let token = checker.grant("inference:call".to_string(), from.clone(), to.clone());

        assert!(checker.check(&token, &to, "inference:call"));
        assert!(!checker.check(&token, &to, "storage:read"));
        assert!(!checker.check(&token, &from, "inference:call"));
    }

    #[test]
    fn test_bot_capabilities() {
        let bot_id = WebID::new();
        let caps = BotCapabilities::new(bot_id.clone())
            .with_capabilities(vec!["inference:call", "storage:read"]);

        assert!(caps.has_capability("inference:call"));
        assert!(caps.has_capability("storage:read"));
        assert!(!caps.has_capability("memory:write"));
    }
}

// === From pod.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub struct MockACPRuntime;
    impl ACPRuntimePort for MockACPRuntime {
        fn register_agent(
            &self,
            webid: WebID,
            _capabilities: Vec<String>,
        ) -> Result<CapabilityToken, String> {
            Ok(CapabilityToken::new(
                CapabilityResource::Tool,
                "*".to_string(),
                CapabilityAction::Execute,
                WebID::new(),
                webid,
                b"test-secret",
            ))
        }
    }

    pub struct MockMCPRuntime;
    impl MCPRuntimePort for MockMCPRuntime {
        fn grant_tool_access(&self, _token: CapabilityToken) -> Result<(), String> {
            Ok(())
        }

        fn invoke_tool(
            &self,
            _tool_name: &str,
            _input: serde_json::Value,
            _token: &CapabilityToken,
        ) -> Result<serde_json::Value, String> {
            Ok(json!({"result": "success"}))
        }
    }

    pub struct MockCNSSpan;
    impl CNSSpanPort for MockCNSSpan {
        fn emit_event(
            &self,
            _span: &str,
            _phase: &str,
            _observation: &serde_json::Value,
            _confidence: f64,
        ) {
            // No-op for tests
        }
    }

    pub struct MockGitCAS;
    impl GitCASPort for MockGitCAS {
        fn load_template_crate(&self, _crate_name: &str) -> Result<TemplateCrate, String> {
            Ok(TemplateCrate {
                name: "test-crate".to_string(),
                git_sha: "abc123".to_string(),
                persona_yaml: String::new(),
                dispatch_manifest_yaml: String::new(),
                templates: vec![],
                hlexicon_terms: vec![],
            })
        }

        fn resolve_sha(&self, _crate_name: &str) -> Result<String, String> {
            Ok("abc123".to_string())
        }
    }

    #[test]
    fn test_pod_lifecycle() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
  version: "0.1.0"
charter:
  description: "Test bot"
  editor: "curator"
capabilities:
  - "tool:inference:call"
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        assert_eq!(pod.state(), PodLifecycleState::Populated);

        let acp = MockACPRuntime;
        let cns = MockCNSSpan;
        pod.register(&acp, &cns).unwrap();
        assert_eq!(pod.state(), PodLifecycleState::Registered);

        let mcp = MockMCPRuntime;
        pod.activate(&mcp, &cns).unwrap();
        assert_eq!(pod.state(), PodLifecycleState::Activated);
        assert!(pod.is_active());

        pod.deactivate(&cns).unwrap();
        assert_eq!(pod.state(), PodLifecycleState::Deactivated);
        assert!(!pod.is_active());
    }

    #[test]
    fn test_invalid_state_transitions() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let cns = MockCNSSpan;
        let result = pod.deactivate(&cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn test_capability_attenuation() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let new_holder = WebID::new();
        let attenuated = pod.delegate(new_holder, 1000).unwrap();

        assert_eq!(attenuated.attenuation_level, 1);
    }

    #[test]
    fn test_attenuation_limit_enforcement() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let mut token = pod.capability_token.clone();
        token.attenuation_level = MAX_ATTENUATION_LEVEL;
        pod.capability_token = token;

        let new_holder = WebID::new();
        let result = pod.delegate(new_holder, 1000);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::AttenuationLimitExceeded
        ));
    }

    #[test]
    fn test_persona_parsing() {
        let yaml = r#"
agent:
  name: "memory-bot"
  type: "Bot"
  version: "0.2.0"
charter:
  description: "Expert bot for memory operations"
  editor: "curator"
capabilities:
  - "tool:memory:remember"
  - "tool:memory:recall"
rights:
  - read: "public_semantic_memory"
  - write: "own_episodic_memory"
responsibilities:
  - "respond_to: memory_tool_calls"
  - "emit: cns.agent_pod.*"
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(yaml).unwrap();
        assert_eq!(persona.agent.name, "memory-bot");
        assert_eq!(persona.agent.agent_type, AgentType::Bot);
        assert_eq!(persona.agent.version, "0.2.0");
        assert_eq!(persona.capabilities.len(), 2);
    }

    #[test]
    fn test_double_registration_fails() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let acp = MockACPRuntime;
        let cns = MockCNSSpan;
        pod.register(&acp, &cns).unwrap();

        let result = pod.register(&acp, &cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn test_double_activation_fails() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let acp = MockACPRuntime;
        let cns = MockCNSSpan;
        pod.register(&acp, &cns).unwrap();

        let mcp = MockMCPRuntime;
        pod.activate(&mcp, &cns).unwrap();

        let result = pod.activate(&mcp, &cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn test_deactivate_from_populated_fails() {
        let persona_yaml = r#"
agent:
  name: "test-bot"
  type: "Bot"
charter:
  description: "Test"
  editor: "curator"
capabilities: []
rights: []
responsibilities: []
visibility:
  default: "public"
  episodic_override: "private"
"#;

        let persona = AgentPersona::from_yaml(persona_yaml).unwrap();
        let git = MockGitCAS;
        let mut pod = AgentPod::new("test-crate", &persona, &git).unwrap();

        let cns = MockCNSSpan;
        let result = pod.deactivate(&cns);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentPodError::InvalidStateTransition(_, _)
        ));
    }
}

