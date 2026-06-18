//! Mock ports for test infrastructure.
//!
//! Provides mock implementations of hKask port traits (InferencePort, etc.)
//! for integration testing without real external dependencies.
//!
//! # Principle grounding
//! - P5 (Essentialism): each mock does one thing well
//! - P8 (Semantic Grounding): mock responses are deterministic and verifiable

use hkask_types::ports::{InferenceError, InferencePort, InferenceResult, InferenceUsage};
use hkask_types::template::LLMParameters;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;

/// A mock `InferencePort` that returns canned responses.
///
/// Responses are keyed by prompt content (exact match or prefix match).
/// Unmatched prompts receive a configurable default response.
/// Supports error injection for testing failure paths.
///
/// # Example
/// ```ignore
/// let mock = MockInferencePort::new()
///     .with_response("hello", "Hello, world!")
///     .with_default("I don't understand.");
///
/// let result = mock.generate("hello", &params).await.unwrap();
/// assert_eq!(result.text, "Hello, world!");
/// ```
pub struct MockInferencePort {
    /// Map of prompt → response text. Matched by `starts_with` for flexibility.
    responses: Mutex<HashMap<String, String>>,
    /// Default response for unmatched prompts.
    default_response: String,
    /// Model name reported in InferenceResult.model.
    model_name: String,
    /// If set, all generate calls return this error instead of a response.
    error_override: Mutex<Option<InferenceError>>,
}

impl MockInferencePort {
    /// Create a new mock with a default response of "Mock response".
    ///
    /// post: returns MockInferencePort with empty responses, default="Mock response", model="mock-model"
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
            default_response: "Mock response".to_string(),
            model_name: "mock-model".to_string(),
            error_override: Mutex::new(None),
        }
    }

    /// Register a canned response for prompts starting with `prompt_prefix`.
    /// Later registrations take precedence (insert order).
    ///
    /// pre:  prompt_prefix and response are non-empty
    /// post: response registered for prefix matching
    /// post: returns Self for builder chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_response(self, prompt_prefix: &str, response: &str) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(prompt_prefix.to_string(), response.to_string());
        self
    }

    /// Set the default response for unmatched prompts.
    ///
    /// pre:  response is non-empty
    /// post: default_response updated
    /// post: returns Self for builder chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_default(mut self, response: &str) -> Self {
        self.default_response = response.to_string();
        self
    }

    /// Set the model name reported in results.
    ///
    /// pre:  model is non-empty
    /// post: model_name updated
    /// post: returns Self for builder chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_model(mut self, model: &str) -> Self {
        self.model_name = model.to_string();
        self
    }

    /// Inject an error — all subsequent `generate` calls will fail with this error.
    /// Call `clear_error()` to restore normal operation.
    ///
    /// post: error_override set — subsequent generate() calls return Err
    pub fn set_error(&self, error: InferenceError) {
        *self.error_override.lock().unwrap() = Some(error);
    }

    /// Clear any injected error, restoring normal responses.
    ///
    /// post: error_override cleared — subsequent generate() calls return Ok
    pub fn clear_error(&self) {
        *self.error_override.lock().unwrap() = None;
    }

    /// Resolve a prompt to its canned response.
    fn resolve_response(&self, prompt: &str) -> String {
        let responses = self.responses.lock().unwrap();
        // Find the longest matching prefix (most specific match)
        let mut best_match: Option<&String> = None;
        let mut best_len = 0;
        for (prefix, response) in responses.iter() {
            if prompt.starts_with(prefix.as_str()) && prefix.len() > best_len {
                best_match = Some(response);
                best_len = prefix.len();
            }
        }
        best_match
            .cloned()
            .unwrap_or_else(|| self.default_response.clone())
    }

    /// Build a standard InferenceResult from a text response.
    fn make_result(&self, text: String) -> InferenceResult {
        InferenceResult {
            text,
            model: self.model_name.clone(),
            usage: InferenceUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: "stop".to_string(),
            token_probabilities: None,
            tool_calls: Vec::new(),
        }
    }
}

impl Default for MockInferencePort {
    fn default() -> Self {
        Self::new()
    }
}

impl InferencePort for MockInferencePort {
    fn generate(
        &self,
        prompt: &str,
        _parameters: &LLMParameters,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        // Check for injected error
        if let Some(ref error) = *self.error_override.lock().unwrap() {
            let msg = error.to_string();
            return Box::pin(async move { Err(InferenceError::Generation(msg)) });
        }

        let text = self.resolve_response(prompt);
        let result = self.make_result(text);
        Box::pin(async move { Ok(result) })
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_override: Option<&str>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        // Same as generate — model override is informational in mock
        self.generate(prompt, parameters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::template::LLMParameters;

    fn test_params() -> LLMParameters {
        LLMParameters::default()
    }

    // contract: HARN-006
    #[tokio::test]
    async fn mock_returns_canned_response() {
        let mock = MockInferencePort::new()
            .with_response("hello", "Hello, world!")
            .with_default("default");

        let result = mock.generate("hello there", &test_params()).await.unwrap();
        assert_eq!(result.text, "Hello, world!");
        assert_eq!(result.model, "mock-model");
    }

    // contract: HARN-006
    #[tokio::test]
    async fn mock_returns_default_for_unmatched() {
        let mock = MockInferencePort::new()
            .with_response("hello", "Hello!")
            .with_default("I don't know");

        let result = mock.generate("goodbye", &test_params()).await.unwrap();
        assert_eq!(result.text, "I don't know");
    }

    // contract: HARN-006
    #[tokio::test]
    async fn mock_longest_prefix_wins() {
        let mock = MockInferencePort::new()
            .with_response("hello world", "specific")
            .with_response("hello", "generic");

        let result = mock
            .generate("hello world today", &test_params())
            .await
            .unwrap();
        assert_eq!(result.text, "specific");

        let result = mock.generate("hello there", &test_params()).await.unwrap();
        assert_eq!(result.text, "generic");
    }

    // contract: HARN-006
    #[tokio::test]
    async fn mock_error_injection() {
        let mock = MockInferencePort::new().with_response("test", "ok");
        mock.set_error(InferenceError::CircuitOpen("test circuit".into()));

        let result = mock.generate("test", &test_params()).await;
        assert!(result.is_err());

        mock.clear_error();
        let result = mock.generate("test", &test_params()).await.unwrap();
        assert_eq!(result.text, "ok");
    }

    // contract: HARN-006
    #[tokio::test]
    async fn mock_generate_with_model_delegates() {
        let mock = MockInferencePort::new().with_response("hi", "response");

        let result = mock
            .generate_with_model("hi there", &test_params(), Some("OM/qwen3:8b"))
            .await
            .unwrap();
        assert_eq!(result.text, "response");
    }
}

// ── MockDaemonClient ──────────────────────────────────────────────────────────

#[allow(clippy::items_after_test_module)]
/// A mock `DaemonClient` for ACP and MCP integration tests.
///
/// Returns canned responses for auth queries, assignments, capability checks,
/// and experience storage. Supports configurable auth state and error injection.
///
/// pre:  none
/// post: returns MockDaemonClient with default (authenticated, all capabilities granted)
pub struct MockDaemonClient {
    /// Whether auth queries report the replicant as authenticated.
    pub authenticated: bool,
    /// Whether assignment queries succeed.
    pub assigned: bool,
    /// Whether capability queries succeed.
    pub capabilities_granted: bool,
    /// Canned tool dispatch response.
    pub tool_response: Option<Value>,
    /// Stored experiences (entity → attribute → value).
    pub stored: Mutex<Vec<(String, String, Value)>>,
}

impl MockDaemonClient {
    /// post: returns new MockDaemonClient with default settings (authenticated, all granted)
    pub fn new() -> Self {
        Self {
            authenticated: true,
            assigned: true,
            capabilities_granted: true,
            tool_response: None,
            stored: Mutex::new(Vec::new()),
        }
    }

    /// Set authentication state to false (simulates daemon unavailable).
    pub fn unauthenticated(mut self) -> Self {
        self.authenticated = false;
        self
    }

    /// Set capabilities to denied.
    ///
    /// post: returns self with capabilities_granted=false
    pub fn capabilities_denied(mut self) -> Self {
        self.capabilities_granted = false;
        self
    }

    /// Set a canned tool dispatch response.
    ///
    /// pre:  response is a valid JSON Value
    /// post: returns self with tool_response set
    pub fn with_tool_response(mut self, response: Value) -> Self {
        self.tool_response = Some(response);
        self
    }

    /// Get stored experiences (for assertion in tests).
    ///
    /// post: returns clone of all stored experience triples
    pub fn stored_experiences(&self) -> Vec<(String, String, Value)> {
        self.stored.lock().unwrap().clone()
    }
}

impl Default for MockDaemonClient {
    fn default() -> Self {
        Self::new()
    }
}
