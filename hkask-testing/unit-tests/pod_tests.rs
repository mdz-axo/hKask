//! Pod Manager Unit Tests
//!
//! Tests for hkask-agents pod management

use hkask_agents::security::{
    AgentPersonaInput, ExpiryEnforcer, InputValidator, RateLimiter, ValidationError,
};
use hkask_agents::{
    AgentPersona, CNSSpanPort, GitCASPort, MCPRuntimePort, PodID, PodManager, TemplateCrate,
};
use hkask_types::capability::CapabilityToken;
use serde_json;

pub struct MockMCPRuntime;
impl MCPRuntimePort for MockMCPRuntime {
    fn grant_tool_access(
        &self,
        _token: hkask_types::capability::CapabilityToken,
    ) -> Result<(), String> {
        Ok(())
    }

    fn invoke_tool(
        &self,
        _tool_name: &str,
        _input: serde_json::Value,
        _token: &hkask_types::capability::CapabilityToken,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"result": "success"}))
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

#[tokio::test]
async fn test_pod_manager_security_context() {
    let manager = PodManager::new_mock();
    assert!(
        manager
            .security_context()
            .rate_limiter
            .get_available("test")
            .await
            > 0.0
    );
}

#[tokio::test]
async fn test_pod_creation_validation() {
    let persona_yaml = r#"
agent:
  name: test-bot
  type: Bot
  version: "0.1.0"
charter:
  description: Test bot
  editor: test
capabilities: []
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#;
    let persona = AgentPersona::from_yaml(persona_yaml).unwrap();

    // Validate persona input
    let input = AgentPersonaInput {
        name: persona.agent.name.clone(),
        agent_type: persona.agent.agent_type.to_string().to_lowercase(),
        version: persona.agent.version.clone(),
        description: persona.charter.description.clone(),
        editor: persona.charter.editor.clone(),
        capabilities: persona.capabilities.clone(),
    };

    assert!(input.validate(&input).is_ok());
}
