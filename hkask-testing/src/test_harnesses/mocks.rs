//! Test Mocks - Mock implementations for hKask ports
//!
//! This module provides mock implementations of production port traits
//! for use in testing. Each mock implements the corresponding port trait.

use hkask_templates::ports::{CnsPort, McpPort, SyncInferencePort, TemplateError, ToolInfo};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Mock implementation of SyncInferencePort
pub struct MockInferencePort {
    responses: Arc<RwLock<HashMap<String, String>>>,
}

impl MockInferencePort {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_response(self, prompt: &str, response: &str) -> Self {
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
    ) -> hkask_templates::ports::Result<Value> {
        let responses = self.responses.read().unwrap();
        responses
            .get(prompt)
            .map(|r| serde_json::from_str(r).unwrap_or(Value::String(r.clone())))
            .ok_or_else(|| TemplateError::Inference("No mock response for prompt".to_string()))
    }
}

/// Mock implementation of McpPort
pub struct MockMcpPort {
    tools: Arc<RwLock<HashMap<String, bool>>>,
}

impl MockMcpPort {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_tool(self, tool_name: &str, enabled: bool) -> Self {
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

impl McpPort for MockMcpPort {
    fn discover_tools(&self) -> Vec<String> {
        let tools = self.tools.read().unwrap();
        tools
            .iter()
            .filter(|&(_, &enabled)| enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn invoke(&self, tool_name: &str, _input: Value) -> hkask_templates::ports::Result<Value> {
        let tools = self.tools.read().unwrap();
        if tools.get(tool_name).copied().unwrap_or(false) {
            Ok(serde_json::json!({"status": "success"}))
        } else {
            Err(TemplateError::Mcp(format!(
                "Tool not found: {}",
                tool_name
            )))
        }
    }

    fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        let tools = self.tools.read().unwrap();
        if tools.get(tool_name).copied().unwrap_or(false) {
            Some(ToolInfo {
                name: tool_name.to_string(),
                description: format!("Mock tool: {}", tool_name),
                input_schema: serde_json::json!({"type": "object"}),
                server_id: "mock".to_string(),
                required_capability: None,
                rate_limit_hint: None,
            })
        } else {
            None
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
    fn emit_event(&self, span: &str, _phase: &str, _observation: &Value, _confidence: f64) {
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
