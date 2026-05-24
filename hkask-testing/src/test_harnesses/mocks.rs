//! Test Mocks - Mock implementations for hKask ports
//!
//! This module provides mock implementations of production port traits
//! for use in testing. Each mock implements the corresponding port trait.

use async_trait::async_trait;
use hkask_templates::ports::{SyncInferencePort, McpPort, CnsPort};
use hkask_types::{TemplateType, WebID};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Mock implementation of InferencePort
pub struct MockInferencePort {
    responses: Arc<RwLock<HashMap<String, String>>>,
}

impl MockInferencePort {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_response(mut self, prompt: &str, response: &str) -> Self {
        self.responses
            .write()
            .unwrap()
            .insert(prompt.to_string(), response.to_string());
        self
    }
}

impl Default for MockInferencePort {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncInferencePort for MockInferencePort {
    fn call(
        &self,
        _model_tier: &str,
        prompt: &str,
        _config: &hkask_templates::ports::InferenceConfig,
    ) -> hkask_templates::ports::Result<serde_json::Value> {
        let responses = self.responses.read().unwrap();
        responses
            .get(prompt)
            .map(|r| serde_json::from_str(r).unwrap_or(serde_json::Value::String(r.clone())))
            .ok_or_else(|| hkask_templates::ports::TemplateError::Inference(
                "No mock response for prompt".to_string(),
            ))
    }
}

/// Composite mock for complex test scenarios
pub struct TestMocks {
    pub inference: MockInferencePort,
    pub mcp: MockMcpPort,
    pub cns: MockCnsPort,
}

impl TestMocks {
    pub fn new() -> Self {
        Self {
            inference: MockInferencePort::new(),
            mcp: MockMcpPort::new(),
            cns: MockCnsPort::new(),
        }
    }
}

impl Default for TestMocks {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMcpPort {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_tool(mut self, tool_name: &str, enabled: bool) -> Self {
        self.tools
            .write()
            .unwrap()
            .insert(tool_name.to_string(), enabled);
        self
    }
}

impl Default for MockMcpPort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpPort for MockMcpPort {
    async fn call_tool(
        &self,
        tool_name: &str,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value, hkask_templates::ports::McpError> {
        let tools = self.tools.read().unwrap();
        if tools.get(tool_name).copied().unwrap_or(false) {
            Ok(serde_json::json!({"status": "success"}))
        } else {
            Err(hkask_templates::ports::McpError::ToolNotFound(
                tool_name.to_string(),
            ))
        }
    }
}

/// Mock implementation of CnsPort
pub struct MockCnsPort {
    events: Arc<RwLock<Vec<String>>>,
}

impl MockCnsPort {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.read().unwrap().len()
    }
}

impl Default for MockCnsPort {
    fn default() -> Self {
        Self::new()
    }
}

impl CnsPort for MockCnsPort {
    fn emit_event(
        &self,
        span: &str,
        _phase: &str,
        _observation: &serde_json::Value,
        _confidence: f64,
    ) {
        let mut events = self.events.write().unwrap();
        events.push(span.to_string());
    }
}

/// Composite mock for complex test scenarios
pub struct TestMocks {
    pub inference: MockInferencePort,
    pub mcp: MockMcpPort,
    pub cns: MockCnsPort,
}

impl TestMocks {
    pub fn new() -> Self {
        Self {
            inference: MockInferencePort::new(),
            mcp: MockMcpPort::new(),
            cns: MockCnsPort::new(),
        }
    }
}

impl Default for TestMocks {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSkillRegistryPort {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_template(mut self, template_id: &str, template_type: TemplateType) -> Self {
        self.templates
            .write()
            .unwrap()
            .insert(template_id.to_string(), template_type);
        self
    }
}

impl Default for MockSkillRegistryPort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillRegistryPort for MockSkillRegistryPort {
    async fn get_template(
        &self,
        template_id: &str,
    ) -> Result<hkask_templates::GeneratedTemplate, hkask_templates::ports::RegistryError> {
        let templates = self.templates.read().unwrap();
        let template_type = templates
            .get(template_id)
            .ok_or_else(|| hkask_templates::ports::RegistryError::NotFound)?;

        Ok(hkask_templates::GeneratedTemplate {
            id: template_id.to_string(),
            template_type: *template_type,
            source: "mock template".to_string(),
            lexicon_terms: vec![],
            contract: hkask_templates::TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
            energy_cap: 1000,
        })
    }

    async fn list_templates(
        &self,
        _template_type: TemplateType,
    ) -> Result<Vec<String>, hkask_templates::ports::RegistryError> {
        let templates = self.templates.read().unwrap();
        Ok(templates.keys().cloned().collect())
    }

    async fn search_by_lexicon(
        &self,
        _term: &str,
    ) -> Result<Vec<String>, hkask_templates::ports::RegistryError> {
        Ok(vec![])
    }
}

/// Composite mock for complex test scenarios
pub struct TestMocks {
    pub inference: MockInferencePort,
    pub mcp: MockMcpPort,
    pub cns: MockCnsPort,
    pub registry: MockSkillRegistryPort,
}

impl TestMocks {
    pub fn new() -> Self {
        Self {
            inference: MockInferencePort::new(),
            mcp: MockMcpPort::new(),
            cns: MockCnsPort::new(),
            registry: MockSkillRegistryPort::new(),
        }
    }
}

impl Default for TestMocks {
    fn default() -> Self {
        Self::new()
    }
}
