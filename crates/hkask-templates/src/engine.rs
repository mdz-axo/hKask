//! High-temperature template engine for anti-normative generation
//!
//! This engine injects randomness into LLM invocations to prevent normative convergence.
//! Temperature is the primary control. Other parameters (top_p, top_k, frequency_penalty,
//! presence_penalty) support the anti-normative effect.

use hkask_types::{BotID, LLMParameters, TemplateId, TemplateInvocation, TemplateOutcome};
use minijinja::{context, Environment};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum TemplateEngineError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    #[error("Render error: {0}")]
    RenderError(#[from] minijinja::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("LLM invocation error: {0}")]
    LlmError(String),
}

/// High-temperature template engine
///
/// The engine maintains a registry of templates and invokes them with
/// temperature-controlled parameters to prevent normative behavior.
pub struct TemplateEngine {
    env: Arc<Mutex<Environment<'static>>>,
    registry: Arc<Mutex<TemplateRegistry>>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            env: Arc::new(Mutex::new(Environment::new())),
            registry: Arc::new(Mutex::new(TemplateRegistry::new())),
        }
    }

    /// Register a template
    pub async fn register(
        &self,
        id: TemplateId,
        name: &str,
        prompt_template: &str,
    ) -> Result<(), TemplateEngineError> {
        let mut registry = self.registry.lock().await;
        registry.add(id, name, prompt_template);
        Ok(())
    }

    /// Invoke a template with parameters
    pub async fn invoke(
        &self,
        template_id: TemplateId,
        bot_id: BotID,
        parameters: LLMParameters,
        input: Value,
    ) -> Result<TemplateInvocation, TemplateEngineError> {
        let registry = self.registry.lock().await;
        let template = registry
            .get(template_id)
            .ok_or_else(|| TemplateEngineError::TemplateNotFound(template_id.to_string()))?;

        // Render the prompt template
        let env = self.env.lock().await;
        let tmpl = env.template_from_named_str(&template.id, &template.prompt_template)?;

        let ctx = context!(input => input, parameters => parameters);
        let rendered = tmpl.render(ctx)?;

        // Create invocation record
        let mut invocation = TemplateInvocation::new(template_id, bot_id, parameters, input);
        invocation.outputs.push(Value::String(rendered));
        invocation.outcome = TemplateOutcome::Success;

        Ok(invocation)
    }

    /// Invoke with anti-inferno preset (maximum randomness)
    pub async fn invoke_anti_inferno(
        &self,
        template_id: TemplateId,
        bot_id: BotID,
        input: Value,
    ) -> Result<TemplateInvocation, TemplateEngineError> {
        let params = LLMParameters::anti_inferno();
        self.invoke(template_id, bot_id, params, input).await
    }

    /// Invoke with edge work preset (moderate randomness)
    pub async fn invoke_edge_work(
        &self,
        template_id: TemplateId,
        bot_id: BotID,
        input: Value,
    ) -> Result<TemplateInvocation, TemplateEngineError> {
        let params = LLMParameters::edge_work();
        self.invoke(template_id, bot_id, params, input).await
    }

    /// Invoke with clean place preset (minimal randomness)
    pub async fn invoke_clean_place(
        &self,
        template_id: TemplateId,
        bot_id: BotID,
        input: Value,
    ) -> Result<TemplateInvocation, TemplateEngineError> {
        let params = LLMParameters::clean_place();
        self.invoke(template_id, bot_id, params, input).await
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory template registry
#[derive(Debug)]
pub struct TemplateRegistry {
    templates: std::collections::HashMap<String, TemplateDef>,
}

#[derive(Debug, Clone)]
pub struct TemplateDef {
    pub id: String,
    pub name: String,
    pub prompt_template: String,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self {
            templates: std::collections::HashMap::new(),
        }
    }

    pub fn add(&mut self, id: TemplateId, name: &str, prompt_template: &str) {
        self.templates.insert(
            id.to_string(),
            TemplateDef {
                id: id.to_string(),
                name: name.to_string(),
                prompt_template: prompt_template.to_string(),
            },
        );
    }

    pub fn get(&self, id: TemplateId) -> Option<TemplateDef> {
        self.templates.get(&id.to_string()).cloned()
    }

    pub fn list(&self) -> Vec<TemplateDef> {
        self.templates.values().cloned().collect()
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}


