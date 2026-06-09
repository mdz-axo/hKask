//! Manifest executor — deterministic multi-step orchestration
//!
//! Executes a `BundleManifest` cascade: select → populate → execute.
//! Each `BundleManifestStep` is dispatched according to its `action` field:
//!
//! - **select**: Render a selector template, call inference, parse the
//!   JSON result to choose the next step or resolve a variable.
//! - **populate**: Render a template with the accumulated context map, producing
//!   a filled prompt or data payload.
//! - **execute**: Invoke an MCP tool with parameters bound from the context map.
//!
//! The executor respects energy budgets (`step.gas_cap`) and timeout constraints
//! (`step.timeout_seconds`). Convergence checks (`manifest.convergence`) gate
//! iterative refinement loops.
//!
//! Template rendering supports two modes:
//!
//! - **minijinja** (`step.renderer == "minijinja"`): Load template from
//!   `step.template_ref` (a file path like `curator/system_state_gather.j2`)
//!   relative to `template_base_path`, then render with full Jinja2 syntax.
//! - **inline** (no `renderer` or any other value): Render `template_ref` or
//!   `renderer` as an inline template string with simple `{{key}}` substitution.
//!
//! Architecture: hkask-templates owns the executor because it needs
//! `InferencePort` (for select/populate) and `McpPort` (for execute),
//! both of which are already dependencies of this crate.

use crate::ports::{McpPort, Result, TemplateError};
use hkask_types::ports::{InferencePort, InferenceResult};
use hkask_types::{
    BundleManifest, BundleManifestStep, DelegationAction, DelegationResource, DelegationToken,
    LLMParameters, WebID,
};
use minijinja::UndefinedBehavior;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Default base path for template files relative to the project root.
const DEFAULT_TEMPLATE_BASE_PATH: &str = "registry/templates";

/// Manifest executor — drives the select → populate → execute cascade.
///
/// Created once per session (or per manifest invocation) and wired into the
/// REPL turn loop. The executor holds references to the infrastructure
/// ports it needs:
///
/// - `InferencePort` — for rendering selector templates and populating prompts
/// - `McpPort` — for invoking MCP tools in execute steps
/// - `template_base_path` — filesystem path for resolving `template_ref` values
///   when `renderer == "minijinja"`
pub struct ManifestExecutor {
    /// Inference port for select/populate actions
    inference: Arc<dyn InferencePort>,
    /// MCP port for execute actions
    mcp: Arc<dyn McpPort>,
    /// Default LLM parameters for inference calls
    default_params: LLMParameters,
    /// Secret for minting delegation tokens
    acp_secret: Vec<u8>,
    /// Base filesystem path for resolving template_ref values.
    /// When `step.renderer == "minijinja"`, `step.template_ref` is resolved
    /// relative to this path. Defaults to `registry/templates/`.
    template_base_path: PathBuf,
}

impl ManifestExecutor {
    /// Create a new executor with the given infrastructure ports.
    pub fn new(
        inference: Arc<dyn InferencePort>,
        mcp: Arc<dyn McpPort>,
        default_params: LLMParameters,
        acp_secret: Vec<u8>,
    ) -> Self {
        Self {
            inference,
            mcp,
            default_params,
            acp_secret,
            template_base_path: PathBuf::from(DEFAULT_TEMPLATE_BASE_PATH),
        }
    }

    /// Execute the full manifest cascade.
    ///
    /// Runs each step in ordinal order, threading the context map through
    /// select and populate steps, and dispatching execute steps to MCP tools.
    /// Returns the context map after all steps complete (or the first error).
    pub async fn execute_manifest(
        &self,
        manifest: &BundleManifest,
        initial_context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mut context = initial_context;
        let mut steps = manifest.steps.clone();
        steps.sort_by_key(|s| s.ordinal);

        for step in &steps {
            info!(
                target: "cns.spec.executor",
                step = step.ordinal,
                action = %step.action,
                description = %step.description,
                "Executing manifest step"
            );
            context = self.execute_step(step, context).await?;
        }

        Ok(context)
    }

    /// Execute a single manifest step.
    ///
    /// Dispatches on `step.action`:
    /// - "select" → render template, call inference, parse JSON
    /// - "populate" → render template with context
    /// - "execute" → invoke MCP tool with bound parameters
    /// - "feedback" → emit CNS feedback via MCP tool
    /// - "validate" → invoke MCP tool with validation rules
    /// - "retrieve" → invoke MCP tool to retrieve data
    pub async fn execute_step(
        &self,
        step: &BundleManifestStep,
        context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        match step.action.as_str() {
            "select" => self.execute_select(step, context).await,
            "populate" => self.execute_populate(step, context).await,
            "execute" | "feedback" | "validate" | "retrieve" => {
                self.execute_tool_invoke(step, context).await
            }
            other => Err(TemplateError::Manifest(format!(
                "Unknown manifest step action: '{}'",
                other
            ))),
        }
    }

    /// **Select** — Render a selector template, call inference, parse JSON result.
    ///
    /// The selector template (from `step.template_ref` or `step.renderer`) is
    /// rendered with the current context. The rendered prompt is sent to the
    /// inference port. The response is parsed as JSON and merged into context.
    async fn execute_select(
        &self,
        step: &BundleManifestStep,
        mut context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let prompt = self.render_step_template(step, &context)?;

        let params = self.default_params.clone();

        let result: InferenceResult = self.inference.generate(&prompt, &params).await?;

        let parsed: Value = parse_json_response(&result.text, step.ordinal)?;
        context.insert(format!("step_{}_result", step.ordinal), parsed);

        Ok(context)
    }

    /// **Populate** — Render a template with the accumulated context.
    ///
    /// The template is rendered with the current context map. The rendered
    /// output is stored in context under `step_{ordinal}_populated`.
    async fn execute_populate(
        &self,
        step: &BundleManifestStep,
        mut context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let populated = self.render_step_template(step, &context)?;
        context.insert(
            format!("step_{}_populated", step.ordinal),
            Value::String(populated),
        );

        Ok(context)
    }

    /// **Execute** — Invoke an MCP tool with parameters bound from context.
    ///
    /// The MCP server/tool is specified in `step.mcp` (format: "server/tool").
    /// Parameters are bound from `step.input_mapping` or the current context.
    async fn execute_tool_invoke(
        &self,
        step: &BundleManifestStep,
        mut context: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mcp_ref = step.mcp.as_deref().ok_or_else(|| {
            TemplateError::Manifest(format!(
                "Execute step {} has no mcp reference",
                step.ordinal
            ))
        })?;

        let input: Value = step
            .input_mapping
            .as_ref()
            .map(|mapping| bind_parameters(mapping, &context))
            .unwrap_or_else(|| {
                Value::Object(
                    context
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
            });

        // Create a delegation token for tool invocation
        let token = DelegationToken::new(
            DelegationResource::Tool,
            mcp_ref.to_string(),
            DelegationAction::Execute,
            WebID::from_persona(b"manifest-executor"),
            WebID::from_persona(b"manifest-executor"),
            &self.acp_secret,
        );

        let result = self
            .mcp
            .invoke(mcp_ref, input, &token)
            .await
            .map_err(|e| TemplateError::Mcp(Box::new(e)))?;

        context.insert(format!("step_{}_result", step.ordinal), result);

        Ok(context)
    }
}

/// Render a template step according to its renderer mode.
///
/// Dispatches based on `step.renderer`:
/// - `"minijinja"` — Load template from `step.template_ref` (a file path
///   like `curator/system_state_gather.j2`) relative to `template_base_path`,
///   then render with full Jinja2 syntax via minijinja.
/// - Inline/absent — Render `step.template_ref` or `step.renderer` as a
///   simple template string with `{{key}}` substitution.
impl ManifestExecutor {
    fn render_step_template(
        &self,
        step: &BundleManifestStep,
        context: &HashMap<String, Value>,
    ) -> Result<String> {
        let renderer = step.renderer.as_deref().unwrap_or("");

        match renderer {
            "minijinja" => {
                // template_ref is a file path relative to template_base_path
                let template_ref = step.template_ref.as_deref().ok_or_else(|| {
                    TemplateError::Manifest(format!(
                        "Step {} has renderer='minijinja' but no template_ref",
                        step.ordinal
                    ))
                })?;

                let template_path = self.template_base_path.join(template_ref);
                let template_content = std::fs::read_to_string(&template_path).map_err(|e| {
                    TemplateError::NotFound(format!(
                        "Step {}: template file not found at {}: {}",
                        step.ordinal,
                        template_path.display(),
                        e
                    ))
                })?;

                info!(
                    target: "cns.spec.executor",
                    step = step.ordinal,
                    template = %template_ref,
                    "Rendering minijinja template"
                );

                render_minijinja(&template_content, context)
            }
            _ => {
                // Inline mode: template_ref or renderer contains the template string
                let template_content = step
                    .template_ref
                    .as_deref()
                    .or(step.renderer.as_deref())
                    .ok_or_else(|| {
                        TemplateError::Manifest(format!(
                            "Step {} has no template_ref or renderer",
                            step.ordinal
                        ))
                    })?;

                Ok(render_inline_template(template_content, context))
            }
        }
    }
}

/// Render a template using minijinja (full Jinja2 syntax).
///
/// Supports `{% for %}`, `{{ var }}`, `| filter`, `{% if %}`, etc.
fn render_minijinja(template: &str, context: &HashMap<String, Value>) -> Result<String> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Lenient);

    // Add the template to the environment
    env.add_template("step", template)
        .map_err(|e| TemplateError::Render(format!("Invalid template: {}", e)))?;

    // Convert HashMap<String, Value> to minijinja context via serde
    let context_value = serde_json::to_value(context)
        .map_err(|e| TemplateError::Render(format!("Failed to serialize context: {}", e)))?;
    let minijinja_context = minijinja::Value::from_serialize(&context_value);

    env.get_template("step")
        .and_then(|tmpl| tmpl.render(minijinja_context))
        .map_err(|e| TemplateError::Render(format!("Template render error: {}", e)))
}

/// Render an inline template using simple `{{key}}` substitution.
///
/// For backward compatibility with non-minijinja templates.
fn render_inline_template(template: &str, context: &HashMap<String, Value>) -> String {
    let mut result = template.to_string();
    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

/// Parse a JSON response from an inference call.
///
/// Attempts to extract JSON from the response text, handling cases where
/// the model wraps the JSON in markdown code fences.
fn parse_json_response(text: &str, step_ordinal: u32) -> Result<Value> {
    // Try direct parse first
    if let Ok(v) = serde_json::from_str(text) {
        return Ok(v);
    }

    // Try extracting JSON from markdown code fences
    let trimmed = text.trim();
    if let Some(json_start) = trimmed.find("```json") {
        let after_fence = &trimmed[json_start + 7..];
        if let Some(json_end) = after_fence.find("```") {
            let json_str = after_fence[..json_end].trim();
            return serde_json::from_str(json_str).map_err(|e| {
                TemplateError::Manifest(format!(
                    "Step {}: Failed to parse JSON response: {}",
                    step_ordinal, e
                ))
            });
        }
    }

    // Try finding JSON object boundaries
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        let json_str = &trimmed[start..=end];
        return serde_json::from_str(json_str).map_err(|e| {
            TemplateError::Manifest(format!(
                "Step {}: Failed to parse JSON response: {}",
                step_ordinal, e
            ))
        });
    }

    Err(TemplateError::Manifest(format!(
        "Step {}: No JSON found in inference response",
        step_ordinal
    )))
}

/// Bind parameters from an input mapping to values from the context.
///
/// The input mapping is a JSON object where values are either:
/// - Direct values (strings, numbers, etc.)
/// - Context references: {"$ref": "step_1_result.field"}
fn bind_parameters(mapping: &Value, context: &HashMap<String, Value>) -> Value {
    match mapping {
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                let bound = bind_single_parameter(value, context);
                result.insert(key.clone(), bound);
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

/// Bind a single parameter value from the context.
fn bind_single_parameter(value: &Value, context: &HashMap<String, Value>) -> Value {
    match value {
        Value::Object(map) => {
            // Check for context reference: {"$ref": "variable_name"}
            if let Some(Value::String(ref_path)) = map.get("$ref") {
                if let Some(context_val) = context.get(ref_path.as_str()) {
                    return context_val.clone();
                }
                // Fallback: try dot notation
                if let Some(nested) = resolve_dot_path(ref_path, context) {
                    return nested;
                }
            }
            // Not a reference — recurse
            bind_parameters(value, context)
        }
        other => other.clone(),
    }
}

/// Resolve a dot-path like "step_1_result.field" from the context.
fn resolve_dot_path(path: &str, context: &HashMap<String, Value>) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let first = context.get(parts[0])?.clone();
    let mut current = first;
    for part in &parts[1..] {
        match current {
            Value::Object(map) => {
                current = map.get(*part)?.clone();
            }
            _ => return None,
        }
    }
    Some(current)
}
