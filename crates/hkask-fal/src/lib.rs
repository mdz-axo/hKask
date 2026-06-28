//! hkask-fal — Fal.ai API client for hKask.
//!
//! Provides workflow execution and model dispatch against Fal's REST API.
//! Strategy D: bi-directional template ↔ workflow generation.
//!
//! # Architecture
//!
//! ```text
//! FlowDef execute step → FalClient::execute_workflow()
//!   ├── Parse workflow JSON (DAG of nodes)
//!   ├── Topological sort by dependency order
//!   ├── Execute nodes sequentially, resolving $references
//!   └── Return output URLs + metadata
//! ```
//!
//! # CNS spans
//!
//! - `cns.execute.fal.workflow.submit` — workflow submission started
//! - `cns.execute.fal.workflow.node_complete` — individual node finished
//! - `cns.execute.fal.workflow.complete` — all nodes done
//! - `cns.execute.fal.workflow.error` — workflow failed

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// ── Error types ─────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum FalError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    #[error("Missing $reference target: {0}")]
    UnresolvedReference(String),

    #[error("Circular dependency detected at node: {0}")]
    CircularDependency(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Workflow must contain at least one run node and one output node")]
    MissingRequiredNodes,
}

// ── Workflow types ──────────────────────────────────────────────────────

/// A node in a workflow plan.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkflowNode {
    #[serde(rename = "input")]
    Input {
        id: String,
        #[serde(default)]
        depends: Vec<String>,
        #[serde(default)]
        input: Value,
    },
    #[serde(rename = "run")]
    Run {
        id: String,
        #[serde(default)]
        depends: Vec<String>,
        app: String,
        input: Value,
    },
    #[serde(rename = "display")]
    Display {
        id: String,
        #[serde(default)]
        depends: Vec<String>,
        fields: Value,
    },
}

impl WorkflowNode {
    pub fn id(&self) -> &str {
        match self {
            WorkflowNode::Input { id, .. } => id,
            WorkflowNode::Run { id, .. } => id,
            WorkflowNode::Display { id, .. } => id,
        }
    }

    pub fn depends(&self) -> &[String] {
        match self {
            WorkflowNode::Input { depends, .. } => depends,
            WorkflowNode::Run { depends, .. } => depends,
            WorkflowNode::Display { depends, .. } => depends,
        }
    }
}

/// Result of executing a full workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use = "WorkflowResult contains output URLs that should not be discarded"]
pub struct WorkflowResult {
    /// URLs of generated outputs from the display node.
    pub output_urls: Vec<String>,
    /// Raw output from the display node's fields.
    pub output_fields: Value,
    /// Per-node execution metadata.
    pub node_results: HashMap<String, Value>,
    /// Total wall-clock time in seconds.
    pub elapsed_seconds: f64,
}

// ── Client ──────────────────────────────────────────────────────────────

/// Fal.ai API client.
///
/// ```no_run
/// use hkask_fal::FalClient;
/// let client = FalClient::new("key-xxx".into());
/// ```
pub struct FalClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    request_timeout_secs: u64,
}

impl FalClient {
    /// Create a new Fal client.
    ///
    /// `api_key` is the Fal API key (from `HKASK_FAL_API_KEY` env var).
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://fal.run".into(),
            client: reqwest::Client::new(),
            request_timeout_secs: 120,
        }
    }

    /// Override the base URL (for testing or proxies).
    #[doc(hidden)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Set per-request timeout in seconds.
    #[allow(dead_code)]
    pub(crate) fn with_timeout(mut self, secs: u64) -> Self {
        self.request_timeout_secs = secs;
        self
    }

    // ── Workflow execution ─────────────────────────────────────────────

    /// Execute a workflow plan JSON.
    ///
    /// Parses the DAG, topologically sorts nodes, executes them in order,
    /// resolves `$node.field.path` references, and collects output URLs.
    pub async fn execute_workflow(&self, workflow: &Value) -> Result<WorkflowResult, FalError> {
        let start = std::time::Instant::now();

        // Parse nodes from the flat JSON object
        let nodes = parse_workflow_nodes(workflow)?;

        // Validate required node types
        validate_workflow_structure(&nodes)?;

        // Build index for O(1) lookup during execution
        let node_map: HashMap<&str, &WorkflowNode> = nodes.iter().map(|n| (n.id(), n)).collect();

        // Topological sort
        let order = topological_sort(&nodes)?;

        tracing::debug!(
            target: "hkask.fal",
            node_count = nodes.len(),
            execution_order = ?order.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            "Workflow execution started"
        );

        // Execute nodes, storing results
        let mut results: HashMap<String, Value> = HashMap::new();
        let mut output_fields = Value::Null;
        let mut output_urls: Vec<String> = Vec::new();

        for node_id in &order {
            let node = node_map.get(node_id.as_str()).ok_or_else(|| {
                FalError::InvalidWorkflow(format!("Node '{node_id}' not found in workflow"))
            })?;

            match node {
                WorkflowNode::Input { input, .. } => {
                    // Input node's value is available directly
                    results.insert(node_id.clone(), input.clone());
                }
                WorkflowNode::Run {
                    app,
                    input,
                    depends,
                    ..
                } => {
                    // Resolve $references in input
                    let resolved_input = resolve_references(input, &results, depends)?;
                    let node_result = self.execute_node(app, &resolved_input).await?;
                    results.insert(node_id.clone(), node_result);
                }
                WorkflowNode::Display {
                    fields, depends, ..
                } => {
                    // Resolve $references in output fields
                    let resolved = resolve_references(fields, &results, depends)?;
                    output_fields = resolved.clone();

                    // Extract image URLs from the resolved fields
                    output_urls = extract_urls(&resolved);
                }
            }
        }

        let elapsed = start.elapsed().as_secs_f64();

        tracing::debug!(
            target: "hkask.fal",
            output_count = output_urls.len(),
            elapsed_seconds = elapsed,
            "Workflow execution complete"
        );

        Ok(WorkflowResult {
            output_urls,
            output_fields,
            node_results: results,
            elapsed_seconds: elapsed,
        })
    }

    // ── Single model execution ─────────────────────────────────────────

    /// Execute a single Fal model.
    ///
    /// `app` is the model ID (e.g., `fal-ai/flux/dev`).
    /// `input` is the model-specific input parameters.
    pub async fn execute_node(&self, app: &str, input: &Value) -> Result<Value, FalError> {
        let url = format!("{}/{}", self.base_url, app);

        tracing::debug!(
            target: "hkask.fal",
            app = app,
            "Executing Fal model"
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(self.request_timeout_secs))
            .json(input)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| {
                tracing::warn!(
                    target: "hkask.fal",
                    error = %e,
                    "Failed to read error response body"
                );
                format!("(could not read error body: {e})")
            });
            return Err(FalError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let result: Value = response.json().await?;
        Ok(result)
    }
}

// ── Workflow parsing & sorting ──────────────────────────────────────────

/// Parse a flat workflow JSON object into a list of typed nodes.
fn parse_workflow_nodes(workflow: &Value) -> Result<Vec<WorkflowNode>, FalError> {
    let obj = workflow
        .as_object()
        .ok_or_else(|| FalError::InvalidWorkflow("Workflow must be a JSON object".into()))?;

    let nodes: Result<Vec<_>, _> = obj
        .iter()
        .map(|(_key, node_value)| {
            serde_json::from_value::<WorkflowNode>(node_value.clone()).map_err(FalError::from)
        })
        .collect();

    nodes
}

/// Validate that a workflow has the required node types.
///
/// Every workflow must have: at least one input node that declares what the
/// caller provides, at least one run node that performs work, and at least
/// one output/display node that collects results. Returns
/// `MissingRequiredNodes` if any of these are absent.
fn validate_workflow_structure(nodes: &[WorkflowNode]) -> Result<(), FalError> {
    let has_input = nodes
        .iter()
        .any(|n| matches!(n, WorkflowNode::Input { .. }));
    let has_run = nodes.iter().any(|n| matches!(n, WorkflowNode::Run { .. }));
    let has_output = nodes
        .iter()
        .any(|n| matches!(n, WorkflowNode::Display { .. }));

    if !has_input || !has_run || !has_output {
        return Err(FalError::MissingRequiredNodes);
    }
    Ok(())
}

/// Topological sort of workflow nodes by dependency order.
///
/// Detects circular dependencies (P5: fail fast, clear error).
fn topological_sort(nodes: &[WorkflowNode]) -> Result<Vec<String>, FalError> {
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id()).collect();

    // Build adjacency: node → nodes that depend on it (forward edges)
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in nodes {
        let id = node.id();
        in_degree.entry(id).or_insert(0);
        for dep in node.depends() {
            if !node_ids.contains(dep.as_str()) {
                return Err(FalError::InvalidWorkflow(format!(
                    "Node '{id}' depends on unknown node '{dep}'"
                )));
            }
            dependents.entry(dep.as_str()).or_default().push(id);
            *in_degree.entry(id).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm: process nodes with in-degree 0
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut sorted: Vec<String> = Vec::new();

    while let Some(node) = queue.pop() {
        sorted.push(node.to_string());
        if let Some(deps) = dependents.get(node) {
            for &dependent in deps {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(dependent);
                    }
                }
            }
        }
    }

    if sorted.len() != nodes.len() {
        // Find the cycle participants for a clear error
        let unsorted: Vec<_> = nodes
            .iter()
            .filter(|n| !sorted.contains(&n.id().to_string()))
            .map(|n| n.id().to_string())
            .collect();
        return Err(FalError::CircularDependency(unsorted.join(", ")));
    }

    Ok(sorted)
}

// ── Reference resolution ────────────────────────────────────────────────

/// Resolve `$node_id.field.path` references in a JSON value.
///
/// Walks the value tree and replaces string references with the actual
/// values from previous node results.
fn resolve_references(
    value: &Value,
    results: &HashMap<String, Value>,
    depends: &[String],
) -> Result<Value, FalError> {
    match value {
        Value::String(s) if s.starts_with('$') => resolve_single_reference(s, results, depends),
        Value::Object(obj) => {
            let mut resolved = serde_json::Map::new();
            for (k, v) in obj {
                resolved.insert(k.clone(), resolve_references(v, results, depends)?);
            }
            Ok(Value::Object(resolved))
        }
        Value::Array(arr) => {
            let resolved: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| resolve_references(v, results, depends))
                .collect();
            Ok(Value::Array(resolved?))
        }
        other => Ok(other.clone()),
    }
}

/// Resolve a single `$node_id.field.subfield` reference.
fn resolve_single_reference(
    reference: &str,
    results: &HashMap<String, Value>,
    depends: &[String],
) -> Result<Value, FalError> {
    // Strip leading $
    let path = reference.strip_prefix('$').unwrap_or(reference);

    // Split on first dot to get node_id and field_path
    let (node_id, field_path) = path
        .split_once('.')
        .ok_or_else(|| FalError::UnresolvedReference(reference.to_string()))?;

    // Verify the referenced node is in our dependencies
    if !depends.contains(&node_id.to_string()) {
        return Err(FalError::UnresolvedReference(format!(
            "'{reference}' references node '{node_id}' which is not in depends: {depends:?}"
        )));
    }

    let node_result = results.get(node_id).ok_or_else(|| {
        FalError::UnresolvedReference(format!(
            "'{reference}' — node '{node_id}' has no result yet"
        ))
    })?;

    // Navigate the field path
    let mut current = node_result;
    for segment in field_path.split('.') {
        // Try numeric index for array access
        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx).ok_or_else(|| {
                FalError::UnresolvedReference(format!("'{reference}' — index {idx} out of range"))
            })?;
        } else {
            current = current.get(segment).ok_or_else(|| {
                FalError::UnresolvedReference(format!(
                    "'{reference}' — field '{segment}' not found"
                ))
            })?;
        }
    }

    Ok(current.clone())
}

/// Extract media URLs from a resolved output value.
///
/// Walks the value tree and collects any string values that look like
/// media URLs (contain typical media extensions or are from fal's CDN).
fn extract_urls(value: &Value) -> Vec<String> {
    let mut urls = Vec::new();
    extract_urls_recursive(value, &mut urls);
    urls
}

fn extract_urls_recursive(value: &Value, urls: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            let is_media_url = s.starts_with("https://")
                && (s.contains("fal.media")
                    || s.contains(".png")
                    || s.contains(".jpg")
                    || s.contains(".jpeg")
                    || s.contains(".webp")
                    || s.contains(".gif")
                    || s.contains(".mp4")
                    || s.contains(".mp3")
                    || s.contains(".svg")
                    || s.contains(".wav"));
            if is_media_url {
                urls.push(s.clone());
            }
        }
        Value::Object(obj) => {
            // Prefer known image fields first
            for key in &["url", "image", "image_url", "images"] {
                if let Some(v) = obj.get(*key) {
                    extract_urls_recursive(v, urls);
                }
            }
            // Then scan remaining fields
            for (k, v) in obj {
                if !["url", "image", "image_url", "images"].contains(&k.as_str()) {
                    extract_urls_recursive(v, urls);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                extract_urls_recursive(v, urls);
            }
        }
        _ => {}
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_linear() {
        let workflow = serde_json::json!({
            "input": { "id": "input", "type": "input", "depends": [], "input": {} },
            "generate": { "id": "generate", "type": "run", "depends": ["input"], "app": "test", "input": {} },
            "output": { "id": "output", "type": "display", "depends": ["generate"], "fields": {} }
        });
        let nodes = parse_workflow_nodes(&workflow).unwrap();
        let order = topological_sort(&nodes).unwrap();
        assert_eq!(order, vec!["input", "generate", "output"]);
    }

    #[test]
    fn test_topological_sort_parallel_branches() {
        let workflow = serde_json::json!({
            "input": { "id": "input", "type": "input", "depends": [], "input": {} },
            "branch_a": { "id": "branch_a", "type": "run", "depends": ["input"], "app": "a", "input": {} },
            "branch_b": { "id": "branch_b", "type": "run", "depends": ["input"], "app": "b", "input": {} },
            "output": { "id": "output", "type": "display", "depends": ["branch_a", "branch_b"], "fields": {} }
        });
        let nodes = parse_workflow_nodes(&workflow).unwrap();
        let order = topological_sort(&nodes).unwrap();
        // Input first, then branches in some order, then output
        assert_eq!(order[0], "input");
        assert_eq!(order[3], "output");
    }

    #[test]
    fn test_circular_dependency_detected() {
        let workflow = serde_json::json!({
            "input": { "id": "input", "type": "input", "depends": ["output"], "input": {} },
            "output": { "id": "output", "type": "display", "depends": ["input"], "fields": {} }
        });
        let nodes = parse_workflow_nodes(&workflow).unwrap();
        let result = topological_sort(&nodes);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular"));
    }

    #[test]
    fn test_reference_resolution_simple_field() {
        let mut results = HashMap::new();
        results.insert(
            "generate".to_string(),
            serde_json::json!({"seed": 42, "model": "flux"}),
        );
        let input = serde_json::json!("$generate.seed");
        let resolved = resolve_references(&input, &results, &["generate".to_string()]).unwrap();
        assert_eq!(resolved, serde_json::json!(42));
    }

    #[test]
    fn test_reference_resolution_missing_field() {
        let mut results = HashMap::new();
        results.insert("generate".to_string(), serde_json::json!({"seed": 42}));
        let input = serde_json::json!("$generate.nonexistent");
        let result = resolve_references(&input, &results, &["generate".to_string()]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'nonexistent' not found")
        );
    }

    #[test]
    fn test_reference_resolution_nested_object() {
        let mut results = HashMap::new();
        results.insert(
            "gen".to_string(),
            serde_json::json!({"outer": {"inner": "value"}}),
        );
        let input = serde_json::json!("$gen.outer.inner");
        let resolved = resolve_references(&input, &results, &["gen".to_string()]).unwrap();
        assert_eq!(resolved, serde_json::json!("value"));
    }

    #[test]
    fn test_reference_resolution_in_object_value() {
        let mut results = HashMap::new();
        results.insert("input".to_string(), serde_json::json!({"text": "a cat"}));
        let input = serde_json::json!({"prompt": "$input.text", "size": "square_hd"});
        let resolved = resolve_references(&input, &results, &["input".to_string()]).unwrap();
        assert_eq!(
            resolved,
            serde_json::json!({"prompt": "a cat", "size": "square_hd"})
        );
    }

    #[test]
    fn test_extract_urls() {
        let value = serde_json::json!({
            "image": "https://v3.fal.media/files/abc.png",
            "metadata": { "seed": 42 },
            "variants": [
                "https://v3.fal.media/files/def.jpg",
                "not-a-url"
            ]
        });
        let urls = extract_urls(&value);
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_missing_required_nodes_rejected() {
        // Workflow with no run node — should fail validation
        let workflow = serde_json::json!({
            "input": { "id": "input", "type": "input", "depends": [], "input": {} },
            "output": { "id": "output", "type": "display", "depends": ["input"], "fields": {} }
        });
        let nodes = parse_workflow_nodes(&workflow).unwrap();
        let result = validate_workflow_structure(&nodes);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FalError::MissingRequiredNodes
        ));
    }

    #[test]
    fn test_valid_workflow_passes_validation() {
        let workflow = serde_json::json!({
            "input": { "id": "input", "type": "input", "depends": [], "input": {} },
            "generate": { "id": "generate", "type": "run", "depends": ["input"], "app": "test", "input": {} },
            "output": { "id": "output", "type": "display", "depends": ["generate"], "fields": {} }
        });
        let nodes = parse_workflow_nodes(&workflow).unwrap();
        assert!(validate_workflow_structure(&nodes).is_ok());
    }

    #[test]
    fn test_client_builder_methods() {
        // Verify builder methods don't panic and chain correctly
        let _client = FalClient::new("k".into())
            .with_base_url("https://test.example.com".into())
            .with_timeout(30);
    }
}
