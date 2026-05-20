//! Unified Composition Abstraction
//!
//! Provides common composition primitives shared between cascade and pipeline execution.
//! Deduplicates logic for stage execution, dependency injection, and energy tracking.
//!
//! **Design Principles:**
//! - Single source of truth for composition semantics
//! - Injectable dependencies for testability
//! - Unified stage execution model
//! - Shared energy and cycle tracking

use crate::error::{CompositionError, RetryConfig};
use crate::ports::{RegistryIndex, Result as PortsResult, TemplateError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Maximum composition depth (Miller's law: 7 ± 2)
pub const MAX_COMPOSITION_DEPTH: u8 = 7;

/// Composition stage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionStage {
    pub name: String,
    pub description: String,
    pub energy_cap: u64,
    pub timeout_ms: u64,
    pub retry_config: RetryConfig,
    pub dependencies: Vec<String>,
}

impl CompositionStage {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            energy_cap: 1000,
            timeout_ms: 30000,
            retry_config: RetryConfig::default(),
            dependencies: vec![],
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_energy_cap(mut self, cap: u64) -> Self {
        self.energy_cap = cap;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<&str>) -> Self {
        self.dependencies = deps.into_iter().map(String::from).collect();
        self
    }
}

/// Composition execution context
#[derive(Debug, Clone)]
pub struct CompositionContext {
    pub current_depth: u8,
    pub visited: HashSet<String>,
    pub energy_remaining: u64,
    pub state: Value,
}

impl CompositionContext {
    pub fn new(initial_state: Value) -> Self {
        Self {
            current_depth: 0,
            visited: HashSet::new(),
            energy_remaining: 10000,
            state: initial_state,
        }
    }

    pub fn with_depth(mut self, depth: u8) -> Self {
        self.current_depth = depth;
        self
    }

    pub fn with_energy(mut self, energy: u64) -> Self {
        self.energy_remaining = energy;
        self
    }

    /// Check if depth limit exceeded
    pub fn check_depth(&self, max: u8) -> PortsResult<()> {
        if self.current_depth > max {
            return Err(TemplateError::RecursionLimit { max });
        }
        Ok(())
    }

    /// Check if node was already visited (cycle detection)
    pub fn check_cycle(&self, node_id: &str) -> PortsResult<()> {
        if self.visited.contains(node_id) {
            return Err(TemplateError::Validation(format!(
                "Cycle detected: '{}' already visited",
                node_id
            )));
        }
        Ok(())
    }

    /// Mark node as visited
    pub fn visit(&mut self, node_id: &str) {
        self.visited.insert(node_id.to_string());
    }

    /// Check energy budget
    pub fn check_energy(&self, cost: u64) -> PortsResult<()> {
        if cost > self.energy_remaining {
            return Err(TemplateError::Manifest(format!(
                "Energy budget exceeded: requested {}, remaining {}",
                cost, self.energy_remaining
            )));
        }
        Ok(())
    }

    /// Consume energy
    pub fn consume_energy(&mut self, cost: u64) {
        self.energy_remaining = self.energy_remaining.saturating_sub(cost);
    }

    /// Create child context for nested execution
    pub fn child(&self) -> Self {
        Self {
            current_depth: self.current_depth + 1,
            visited: self.visited.clone(),
            energy_remaining: self.energy_remaining,
            state: self.state.clone(),
        }
    }
}

/// Dependency injection provider trait
pub trait DependencyProvider: Send + Sync {
    /// Get a dependency by name
    fn get_dependency(&self, name: &str) -> Option<Value>;

    /// Register a dependency
    fn register(&mut self, name: &str, value: Value);

    /// List all registered dependencies
    fn list_dependencies(&self) -> Vec<&str>;
}

/// In-memory dependency provider
pub struct InMemoryDependencyProvider {
    dependencies: HashMap<String, Value>,
}

impl InMemoryDependencyProvider {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn with_dependency(mut self, name: &str, value: Value) -> Self {
        self.dependencies.insert(name.to_string(), value);
        self
    }
}

impl Default for InMemoryDependencyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyProvider for InMemoryDependencyProvider {
    fn get_dependency(&self, name: &str) -> Option<Value> {
        self.dependencies.get(name).cloned()
    }

    fn register(&mut self, name: &str, value: Value) {
        self.dependencies.insert(name.to_string(), value);
    }

    fn list_dependencies(&self) -> Vec<&str> {
        self.dependencies.keys().map(|s| s.as_str()).collect()
    }
}

/// Stage executor trait for unified execution model
pub trait CompositionStageExecutor: Send + Sync {
    /// Execute the stage with given input
    fn execute(
        &self,
        input: Value,
        context: &mut CompositionContext,
        deps: &dyn DependencyProvider,
    ) -> Result<Value, CompositionError>;

    /// Get stage name
    fn name(&self) -> &str;
}

/// Composition graph for tracking dependencies
#[derive(Debug, Clone)]
pub struct CompositionGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<String>>,
}

impl CompositionGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node_id: &str) {
        self.nodes.insert(node_id.to_string());
        self.edges.entry(node_id.to_string()).or_default();
    }

    /// Add an edge (dependency) to the graph
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.nodes.insert(from.to_string());
        self.nodes.insert(to.to_string());
        self.edges.entry(from.to_string()).or_default().push(to.to_string());
    }

    /// Detect cycles using DFS
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                if self.has_cycle_dfs(node, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    fn has_cycle_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_dfs(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Get all dependencies of a node
    pub fn dependencies_of(&self, node: &str) -> Vec<&str> {
        self.edges
            .get(node)
            .map(|deps| deps.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get topological order (if acyclic)
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        if self.has_cycle() {
            return None;
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                self.topo_dfs(node, &mut visited, &mut result);
            }
        }

        result.reverse();
        Some(result)
    }

    fn topo_dfs(&self, node: &str, visited: &mut HashSet<String>, result: &mut Vec<String>) {
        visited.insert(node.to_string());

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.topo_dfs(neighbor, visited, result);
                }
            }
        }

        result.push(node.to_string());
    }
}

impl Default for CompositionGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified composition configuration with dependency injection
pub struct PipelineConfig {
    /// Pipeline stages in execution order
    pub stages: Vec<CompositionStage>,
    /// Maximum execution depth
    pub max_depth: u8,
    /// Initial energy budget
    pub energy_budget: u64,
    /// Dependency provider
    dependencies: Arc<dyn DependencyProvider>,
    /// Registry for template resolution
    registry: Arc<dyn RegistryIndex>,
}

impl Clone for PipelineConfig {
    fn clone(&self) -> Self {
        Self {
            stages: self.stages.clone(),
            max_depth: self.max_depth,
            energy_budget: self.energy_budget,
            dependencies: self.dependencies.clone(),
            registry: self.registry.clone(),
        }
    }
}

impl PipelineConfig {
    pub fn new(
        stages: Vec<CompositionStage>,
        registry: Arc<dyn RegistryIndex>,
    ) -> Self {
        Self {
            stages,
            max_depth: MAX_COMPOSITION_DEPTH,
            energy_budget: 10000,
            dependencies: Arc::new(InMemoryDependencyProvider::new()),
            registry,
        }
    }

    pub fn with_max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth.min(MAX_COMPOSITION_DEPTH);
        self
    }

    pub fn with_energy_budget(mut self, budget: u64) -> Self {
        self.energy_budget = budget;
        self
    }

    pub fn with_dependencies(mut self, provider: Arc<dyn DependencyProvider>) -> Self {
        self.dependencies = provider;
        self
    }

    /// Get the dependency provider
    pub fn dependencies(&self) -> &dyn DependencyProvider {
        &*self.dependencies
    }

    /// Get the registry
    pub fn registry(&self) -> &dyn RegistryIndex {
        &*self.registry
    }

    /// Validate configuration (check for cycles, etc.)
    pub fn validate(&self) -> PortsResult<()> {
        let mut graph = CompositionGraph::new();

        // Build dependency graph from stages
        for stage in &self.stages {
            graph.add_node(&stage.name);
            for dep in &stage.dependencies {
                graph.add_edge(&stage.name, dep);
            }
        }

        // Check for cycles
        if graph.has_cycle() {
            return Err(TemplateError::Validation(
                "Cycle detected in pipeline dependencies".to_string(),
            ));
        }

        Ok(())
    }
}

/// Unified composition executor
pub struct CompositionExecutor {
    config: PipelineConfig,
}

impl CompositionExecutor {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Execute composition with given input
    pub fn execute(&self, initial_input: Value) -> Result<Value, CompositionError> {
        // Validate configuration first
        self.config
            .validate()
            .map_err(|e| CompositionError::permanent(&e.to_string(), None))?;

        let mut context = CompositionContext::new(initial_input)
            .with_depth(self.config.max_depth)
            .with_energy(self.config.energy_budget);

        let mut state = context.state.clone();

        // Execute stages in order
        for stage in &self.config.stages {
            // Check depth
            context
                .check_depth(self.config.max_depth)
                .map_err(|e| CompositionError::permanent(&e.to_string(), None))?;

            // Check energy
            context
                .check_energy(stage.energy_cap)
                .map_err(|e| CompositionError::permanent(&e.to_string(), None))?;

            // Mark as visited
            context.visit(&stage.name);

            // Execute stage with retry
            let mut attempt = 0;
            let stage_result = loop {
                match self.execute_stage(stage, state.clone(), &mut context) {
                    Ok(result) => break Ok(result),
                    Err(e) => {
                        if e.is_retryable() && stage.retry_config.should_retry(attempt) {
                            attempt += 1;
                            let delay = stage.retry_config.backoff_delay(attempt);
                            std::thread::sleep(std::time::Duration::from_millis(delay));
                            continue;
                        }
                        break Err(e);
                    }
                }
            }?;

            state = stage_result;
            context.consume_energy(stage.energy_cap);
            context.state = state.clone();
        }

        Ok(state)
    }

    /// Execute a single stage
    fn execute_stage(
        &self,
        stage: &CompositionStage,
        input: Value,
        _context: &mut CompositionContext,
    ) -> Result<Value, CompositionError> {
        // Placeholder - actual implementation would dispatch to registered executor
        // For now, return input with stage metadata
        Ok(serde_json::json!({
            "stage": stage.name,
            "input": input,
            "status": "executed"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRegistry;

    impl RegistryIndex for TestRegistry {
        fn list(&self, _domain_hint: Option<TemplateType>) -> Vec<crate::ports::RegistryEntry> {
            vec![]
        }

        fn get(&self, _id: &str) -> PortsResult<crate::ports::RegistryEntry> {
            Err(TemplateError::NotFound("test".to_string()))
        }

        fn bootstrap_manifest(&self) -> Option<crate::ports::ProcessManifest> {
            None
        }
    }

    #[test]
    fn test_composition_stage_builder() {
        let stage = CompositionStage::new("test")
            .with_description("Test stage")
            .with_energy_cap(500)
            .with_timeout(5000)
            .with_dependencies(vec!["dep1", "dep2"]);

        assert_eq!(stage.name, "test");
        assert_eq!(stage.description, "Test stage");
        assert_eq!(stage.energy_cap, 500);
        assert_eq!(stage.timeout_ms, 5000);
        assert_eq!(stage.dependencies.len(), 2);
    }

    #[test]
    fn test_composition_context_depth() {
        let context = CompositionContext::new(Value::Null).with_depth(8);
        assert!(context.check_depth(7).is_err());

        let context = CompositionContext::new(Value::Null).with_depth(6);
        assert!(context.check_depth(7).is_ok());
    }

    #[test]
    fn test_composition_context_cycle() {
        let mut context = CompositionContext::new(Value::Null);
        context.visit("node-1");

        assert!(context.check_cycle("node-1").is_err());
        assert!(context.check_cycle("node-2").is_ok());
    }

    #[test]
    fn test_composition_context_energy() {
        let mut context = CompositionContext::new(Value::Null).with_energy(1000);
        assert!(context.check_energy(500).is_ok());
        assert!(context.check_energy(1500).is_err());

        context.consume_energy(500);
        assert_eq!(context.energy_remaining, 500);
    }

    #[test]
    fn test_composition_context_child() {
        let context = CompositionContext::new(Value::Null).with_depth(3);
        let child = context.child();

        assert_eq!(child.current_depth, 4);
        assert_eq!(child.energy_remaining, context.energy_remaining);
    }

    #[test]
    fn test_dependency_provider() {
        let mut provider = InMemoryDependencyProvider::new();
        provider.register("key1", serde_json::json!("value1"));
        provider.register("key2", serde_json::json!("value2"));

        assert_eq!(provider.get_dependency("key1"), Some(serde_json::json!("value1")));
        assert_eq!(provider.get_dependency("key2"), Some(serde_json::json!("value2")));
        assert_eq!(provider.get_dependency("key3"), None);

        let deps = provider.list_dependencies();
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_dependency_provider_builder() {
        let provider = InMemoryDependencyProvider::new()
            .with_dependency("key1", serde_json::json!("value1"))
            .with_dependency("key2", serde_json::json!("value2"));

        assert_eq!(provider.get_dependency("key1"), Some(serde_json::json!("value1")));
        assert_eq!(provider.get_dependency("key2"), Some(serde_json::json!("value2")));
    }

    #[test]
    fn test_composition_graph_no_cycle() {
        let mut graph = CompositionGraph::new();
        graph.add_node("a");
        graph.add_node("b");
        graph.add_node("c");
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");

        assert!(!graph.has_cycle());

        let sorted = graph.topological_sort();
        assert!(sorted.is_some());
        let sorted = sorted.unwrap();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_composition_graph_with_cycle() {
        let mut graph = CompositionGraph::new();
        graph.add_node("a");
        graph.add_node("b");
        graph.add_node("c");
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("c", "a"); // Cycle!

        assert!(graph.has_cycle());
        assert!(graph.topological_sort().is_none());
    }

    #[test]
    fn test_pipeline_config_validation() {
        let registry = Arc::new(TestRegistry);
        let stages = vec![
            CompositionStage::new("stage1")
                .with_dependencies(vec!["stage2"]),
            CompositionStage::new("stage2"),
        ];

        let config = PipelineConfig::new(stages, registry);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pipeline_config_validation_with_cycle() {
        let registry = Arc::new(TestRegistry);
        let stages = vec![
            CompositionStage::new("stage1")
                .with_dependencies(vec!["stage2"]),
            CompositionStage::new("stage2")
                .with_dependencies(vec!["stage1"]), // Cycle!
        ];

        let config = PipelineConfig::new(stages, registry);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_composition_executor_new() {
        let registry = Arc::new(TestRegistry);
        let stages = vec![CompositionStage::new("test")];
        let config = PipelineConfig::new(stages, registry);
        let executor = CompositionExecutor::new(config);

        // Executor created successfully
        let _ = executor;
    }

    #[test]
    fn test_composition_executor_execute() {
        let registry = Arc::new(TestRegistry);
        let stages = vec![
            CompositionStage::new("stage1"),
            CompositionStage::new("stage2"),
        ];
        let config = PipelineConfig::new(stages, registry);
        let executor = CompositionExecutor::new(config);

        let result = executor.execute(serde_json::json!({"input": "test"}));
        assert!(result.is_ok());
    }
}
