//! Mock Adapter Port - Inbound port for testing
//!
//! Implements production port traits with mock/stub behavior for testing.
//! Provides deterministic, testable implementations of external dependencies.

use async_trait::async_trait;
use hkask_templates::ports::{
    CnsPort, SyncInferencePort, McpPort, Result as TemplateResult, TemplateError,
};
use serde_json::Value;
use std::cell::Cell;

/// Mock inference adapter for testing
pub struct MockInferenceAdapter {
    responses: Vec<Value>,
    call_count: Cell<usize>,
    should_fail: bool,
    failure_message: Option<String>,
}

impl MockInferenceAdapter {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
            call_count: Cell::new(0),
            should_fail: false,
            failure_message: None,
        }
    }

    pub fn with_response(mut self, response: Value) -> Self {
        self.responses.push(response);
        self
    }

    pub fn with_responses(mut self, responses: Vec<Value>) -> Self {
        self.responses = responses;
        self
    }

    pub fn should_fail(mut self, message: &str) -> Self {
        self.should_fail = true;
        self.failure_message = Some(message.to_string());
        self
    }

    pub fn call_count(&self) -> usize {
        self.call_count.get()
    }

    pub fn reset(&self) {
        self.call_count.set(0);
    }
}

impl Default for MockInferenceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SyncInferencePort for MockInferenceAdapter {
    fn call(
        &self,
        _model_tier: &str,
        _prompt: &str,
        _config: &hkask_templates::ports::InferenceConfig,
    ) -> TemplateResult<Value> {
        if self.should_fail {
            return Err(TemplateError::Inference(
                self.failure_message.clone().unwrap_or_default(),
            ));
        }

        let count = self.call_count.get();
        self.call_count.set(count + 1);

        if count >= self.responses.len() {
            return Ok(Value::Null);
        }

        Ok(self.responses[count].clone())
    }
}

/// Mock MCP adapter for testing
pub struct MockMcpAdapter {
    tools: Vec<String>,
    responses: Vec<Value>,
    invoke_count: Cell<usize>,
    should_fail: bool,
}

impl MockMcpAdapter {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            responses: Vec::new(),
            invoke_count: Cell::new(0),
            should_fail: false,
        }
    }

    pub fn with_tool(mut self, tool_name: &str) -> Self {
        self.tools.push(tool_name.to_string());
        self
    }

    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.tools = tools.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_response(mut self, response: Value) -> Self {
        self.responses.push(response);
        self
    }

    pub fn should_fail(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn invoke_count(&self) -> usize {
        self.invoke_count.get()
    }
}

impl Default for MockMcpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl McpPort for MockMcpAdapter {
    fn discover_tools(&self) -> Vec<String> {
        self.tools.clone()
    }

    fn invoke(&self, _tool_name: &str, _input: Value) -> TemplateResult<Value> {
        if self.should_fail {
            return Err(TemplateError::Mcp("Mock MCP failure".to_string()));
        }

        let count = self.invoke_count.get();
        self.invoke_count.set(count + 1);

        if count >= self.responses.len() {
            return Ok(Value::Null);
        }

        Ok(self.responses[count].clone())
    }

    fn get_tool_info(&self, _tool_name: &str) -> Option<hkask_templates::ports::ToolInfo> {
        // Mock implementation - returns None
        None
    }
}

/// Mock CNS adapter for testing
pub struct MockCnsAdapter {
    emit_count: Cell<usize>,
}

impl MockCnsAdapter {
    pub fn new() -> Self {
        Self {
            emit_count: Cell::new(0),
        }
    }

    pub fn emit_count(&self) -> usize {
        self.emit_count.get()
    }

    pub fn clear(&self) {
        self.emit_count.set(0);
    }
}

impl Default for MockCnsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CnsPort for MockCnsAdapter {
    fn emit_event(&self, _span: &str, _phase: &str, _observation: &Value, _confidence: f64) {
        self.emit_count.set(self.emit_count.get() + 1);
    }
}

/// Mock CNS adapter for tests that need to verify emissions
pub struct MockCnsAdapterMut {
    emit_count: Cell<usize>,
}

impl MockCnsAdapterMut {
    pub fn new() -> Self {
        Self {
            emit_count: Cell::new(0),
        }
    }

    pub fn event_count(&self) -> usize {
        self.emit_count.get()
    }
}

impl Default for MockCnsAdapterMut {
    fn default() -> Self {
        Self::new()
    }
}

impl CnsPort for MockCnsAdapterMut {
    fn emit_event(&self, _span: &str, _phase: &str, _observation: &Value, _confidence: f64) {
        self.emit_count.set(self.emit_count.get() + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_templates::ports::InferenceConfig;
    use serde_json::json;

    #[test]
    fn test_mock_inference_adapter_new() {
        let adapter = MockInferenceAdapter::new();
        assert_eq!(adapter.call_count(), 0);
    }

    #[test]
    fn test_mock_inference_adapter_with_response() {
        let adapter = MockInferenceAdapter::new().with_response(json!({"result": "test"}));
        let config = InferenceConfig::default();
        let result = adapter.call("fast", "test prompt", &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"result": "test"}));
    }

    #[test]
    fn test_mock_inference_adapter_failure() {
        let adapter = MockInferenceAdapter::new().should_fail("test error");
        let config = InferenceConfig::default();
        let result = adapter.call("fast", "test prompt", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_mcp_adapter_discover_tools() {
        let adapter = MockMcpAdapter::new()
            .with_tool("search")
            .with_tool("scrape");
        let tools = adapter.discover_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"search".to_string()));
        assert!(tools.contains(&"scrape".to_string()));
    }

    #[test]
    fn test_mock_mcp_adapter_invoke() {
        let adapter = MockMcpAdapter::new().with_response(json!({"status": "ok"}));
        let result = adapter.invoke("test_tool", json!({}));
        assert!(result.is_ok());
        assert_eq!(adapter.invoke_count(), 1);
    }

    #[test]
    fn test_mock_cns_adapter_emit() {
        let adapter = MockCnsAdapterMut::new();
        adapter.emit("cns.tool", json!({"action": "test"}), 0.95);
        assert_eq!(adapter.event_count(), 1);
    }
}
