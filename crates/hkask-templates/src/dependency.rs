//! Template dependency graph
//!
//! Tracks which templates call which (matroshka relationships).
//! Builds dependency graph at parse time with cycle detection.

use std::collections::{HashMap, HashSet};

/// Dependency edge in the template graph
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Caller template ID
    pub caller: String,
    /// Callee template ID
    pub callee: String,
    /// Depth level in matroshka nesting
    pub depth: u8,
}

/// Template dependency graph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Adjacency list: template_id -> list of templates it calls
    edges: HashMap<String, Vec<DependencyEdge>>,
    /// Reverse adjacency: template_id -> list of templates that call it
    reverse_edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Add a dependency edge
    pub fn add_edge(&mut self, caller: String, callee: String, depth: u8) {
        let edge = DependencyEdge {
            caller: caller.clone(),
            callee: callee.clone(),
            depth,
        };

        self.edges
            .entry(caller.clone())
            .or_insert_with(Vec::new)
            .push(edge);

        self.reverse_edges
            .entry(callee)
            .or_insert_with(Vec::new)
            .push(caller);
    }

    /// Get all templates called by a template
    pub fn get_dependencies(&self, template_id: &str) -> Vec<&DependencyEdge> {
        self.edges
            .get(template_id)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get all templates that call this template
    pub fn get_dependents(&self, template_id: &str) -> Vec<&String> {
        self.reverse_edges
            .get(template_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Check if adding an edge would create a cycle
    pub fn would_create_cycle(&self, caller: &str, callee: &str) -> bool {
        // If callee can reach caller, adding caller->callee creates a cycle
        self.can_reach(callee, caller)
    }

    /// Check if there's a path from source to target
    pub fn can_reach(&self, source: &str, target: &str) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![source.to_string()];

        while let Some(current) = stack.pop() {
            if current == target {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(edges) = self.edges.get(&current) {
                for edge in edges {
                    stack.push(edge.callee.clone());
                }
            }
        }

        false
    }

    /// Detect all cycles in the graph
    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.edges.keys() {
            if !visited.contains(node) {
                self.find_cycles_dfs(node, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    fn find_cycles_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(edges) = self.edges.get(node) {
            for edge in edges {
                let callee = &edge.callee;
                if !visited.contains(callee) {
                    self.find_cycles_dfs(callee, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(callee) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|x| x == callee).unwrap();
                    let cycle = path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Get maximum matroshka depth for a template
    pub fn get_max_depth(&self, template_id: &str) -> u8 {
        let mut max_depth = 0;
        let mut visited = HashSet::new();
        let mut stack = vec![(template_id.to_string(), 0u8)];

        while let Some((current, depth)) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            max_depth = max_depth.max(depth);

            if let Some(edges) = self.edges.get(&current) {
                for edge in edges {
                    stack.push((edge.callee.clone(), depth + 1));
                }
            }
        }

        max_depth
    }

    /// Get all template IDs in the graph
    pub fn get_all_template_ids(&self) -> Vec<&String> {
        self.edges.keys().collect()
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.edges.clear();
        self.reverse_edges.clear();
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse template source for dependency directives
pub fn parse_dependencies(_template_id: &str, source: &str) -> Vec<String> {
    let mut dependencies = Vec::new();

    // Look for {% include "template_id" %} directives
    for line in source.lines() {
        // Match {% include "..." %} or {% include '...' %}
        if let Some(include_start) = line.find("{% include") {
            let rest = &line[include_start..];
            if let Some(quote_start) = rest.find('"').or_else(|| rest.find('\'')) {
                let quote_char = rest.chars().nth(quote_start).unwrap();
                let after_quote = &rest[quote_start + 1..];
                if let Some(quote_end) = after_quote.find(quote_char) {
                    let included = &after_quote[..quote_end];
                    if !included.is_empty() {
                        dependencies.push(included.to_string());
                    }
                }
            }
        }

        // Look for {% call "template_id" %} directives
        if let Some(call_start) = line.find("{% call") {
            let rest = &line[call_start..];
            if let Some(quote_start) = rest.find('"').or_else(|| rest.find('\'')) {
                let quote_char = rest.chars().nth(quote_start).unwrap();
                let after_quote = &rest[quote_start + 1..];
                if let Some(quote_end) = after_quote.find(quote_char) {
                    let called = &after_quote[..quote_end];
                    if !called.is_empty() {
                        dependencies.push(called.to_string());
                    }
                }
            }
        }
    }

    dependencies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_new() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_dependency_graph_add_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("caller".to_string(), "callee".to_string(), 1);

        assert_eq!(graph.edge_count(), 1);
        let deps = graph.get_dependencies("caller");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].callee, "callee");
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        graph.add_edge("b".to_string(), "c".to_string(), 1);

        // Adding d->a would NOT create a cycle (d is not in graph)
        assert!(!graph.would_create_cycle("d", "a"));
        
        // Adding c->d would NOT create a cycle (d is not reachable from a or b)
        assert!(!graph.would_create_cycle("c", "d"));
    }

    #[test]
    fn test_dependency_graph_detect_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        graph.add_edge("b".to_string(), "c".to_string(), 1);
        graph.add_edge("c".to_string(), "a".to_string(), 1);

        assert!(graph.would_create_cycle("c", "a"));
        
        let cycles = graph.find_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_dependency_graph_max_depth() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("root".to_string(), "level1".to_string(), 1);
        graph.add_edge("level1".to_string(), "level2".to_string(), 2);
        graph.add_edge("level2".to_string(), "level3".to_string(), 3);

        assert_eq!(graph.get_max_depth("root"), 3);
    }

    #[test]
    fn test_parse_dependencies_include() {
        let source = r#"
        Some text
        {% include "prompt/selector" %}
        More text
        {% include 'process/dispatch' %}
        "#;

        let deps = parse_dependencies("test", source);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"prompt/selector".to_string()));
        assert!(deps.contains(&"process/dispatch".to_string()));
    }

    #[test]
    fn test_parse_dependencies_call() {
        let source = r#"
        {% call "cognition/detect" %}
        "#;

        let deps = parse_dependencies("test", source);
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&"cognition/detect".to_string()));
    }

    #[test]
    fn test_parse_dependencies_empty() {
        let source = r#"
        No dependencies here
        Just regular content
        "#;

        let deps = parse_dependencies("test", source);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_dependency_graph_clear() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        
        graph.clear();
        
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_dependency_graph_dependents() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a".to_string(), "b".to_string(), 1);
        graph.add_edge("c".to_string(), "b".to_string(), 1);

        let dependents = graph.get_dependents("b");
        assert_eq!(dependents.len(), 2);
    }
}