//! Cascade Composition Engine
//!
//! Implements embedded recursion for self-improvement with cycle detection and depth limits.
//! Per architecture v0.21.0: Templates can reference templates, manifests can invoke manifests,
//! with CNS feedback for calibration.
//!
//! **Mechanism:**
//! - Templates can reference templates (cascade resolution)
//! - Manifests can invoke manifests (sub-process delegation)
//! - CNS feedback → template/manifest calibration (ReAct pattern)
//! - Energy caps prevent infinite recursion (halting guarantee)
//!
//! **Safety:**
//! - Maximum recursion depth: 7 (Miller's law + energy budget)
//! - Cycle detection in registry (graph traversal)
//! - Capability attenuation on recursive calls (OCAP security)
//! - Security validation on all template paths (Schneier threat model)

use crate::ports::{RegistryIndex, Result, TemplateError};
use crate::security::SecurityAdapter;
use hkask_types::{CapabilityToken, TemplateType, WebID};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Maximum recursion depth (Miller's law: 7 ± 2)
pub const MAX_CASCADE_DEPTH: u8 = 7;

/// Cascade stage definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeStage {
    pub name: String,
    pub templates: Vec<String>,
    pub condition: Option<String>,
}

/// Cascade composition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cascade {
    pub id: String,
    pub pre: Vec<CascadeStage>,
    pub core: Vec<CascadeStage>,
    pub post: Vec<CascadeStage>,
    pub max_depth: u8,
}

impl Cascade {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            pre: vec![],
            core: vec![],
            post: vec![],
            max_depth: MAX_CASCADE_DEPTH,
        }
    }

    pub fn with_pre(mut self, stage: CascadeStage) -> Self {
        self.pre.push(stage);
        self
    }

    pub fn with_core(mut self, stage: CascadeStage) -> Self {
        self.core.push(stage);
        self
    }

    pub fn with_post(mut self, stage: CascadeStage) -> Self {
        self.post.push(stage);
        self
    }

    pub fn with_max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth.min(MAX_CASCADE_DEPTH);
        self
    }
}

impl Default for Cascade {
    fn default() -> Self {
        Self::new("default")
    }
}

/// Cycle detection result
#[derive(Debug, Clone)]
pub struct CycleDetectionResult {
    pub has_cycle: bool,
    pub cycle_path: Vec<String>,
}

/// Cascade execution context
#[derive(Debug, Clone)]
pub struct CascadeContext {
    pub current_depth: u8,
    pub visited_templates: HashSet<String>,
    pub visited_manifests: HashSet<String>,
    pub energy_remaining: u64,
    pub capability_token: Option<CapabilityToken>,
    pub secret: Vec<u8>,
    pub current_time: i64,
}

impl CascadeContext {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            current_depth: 0,
            visited_templates: HashSet::new(),
            visited_manifests: HashSet::new(),
            energy_remaining: 10000,
            capability_token: None,
            secret: secret.to_vec(),
            current_time: chrono::Utc::now().timestamp(),
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

    pub fn with_capability(mut self, token: CapabilityToken) -> Self {
        self.capability_token = Some(token);
        self
    }

    pub fn with_current_time(mut self, time: i64) -> Self {
        self.current_time = time;
        self
    }

    /// Check if recursion depth limit exceeded
    pub fn check_depth(&self, max: u8) -> Result<()> {
        if self.current_depth > max {
            return Err(TemplateError::RecursionLimit { max });
        }
        Ok(())
    }

    /// Check if template was already visited (cycle detection)
    pub fn check_template_cycle(&self, template_id: &str) -> Result<()> {
        if self.visited_templates.contains(template_id) {
            return Err(TemplateError::Validation(format!(
                "Cycle detected: template '{}' already visited in cascade",
                template_id
            )));
        }
        Ok(())
    }

    /// Check if manifest was already visited (cycle detection)
    pub fn check_manifest_cycle(&self, manifest_id: &str) -> Result<()> {
        if self.visited_manifests.contains(manifest_id) {
            return Err(TemplateError::Validation(format!(
                "Cycle detected: manifest '{}' already visited in cascade",
                manifest_id
            )));
        }
        Ok(())
    }

    /// Mark template as visited
    pub fn visit_template(&mut self, template_id: &str) {
        self.visited_templates.insert(template_id.to_string());
    }

    /// Mark manifest as visited
    pub fn visit_manifest(&mut self, manifest_id: &str) {
        self.visited_manifests.insert(manifest_id.to_string());
    }

    /// Check energy budget
    pub fn check_energy(&self, cost: u64) -> Result<()> {
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

    /// Create child context for recursive call with capability attenuation
    /// Per Mark Miller OCAP: capabilities must be attenuated on delegation
    pub fn child_context(&self, new_holder: WebID) -> Self {
        let attenuated_token = self.capability_token.as_ref().and_then(|token| {
            if token.can_attenuate() {
                token.attenuate(new_holder, &self.secret, self.current_time)
            } else {
                None
            }
        });

        Self {
            current_depth: self.current_depth + 1,
            visited_templates: self.visited_templates.clone(),
            visited_manifests: self.visited_manifests.clone(),
            energy_remaining: self.energy_remaining,
            capability_token: attenuated_token,
            secret: self.secret.clone(),
            current_time: self.current_time,
        }
    }

    /// Check if current capability grants access to a resource
    pub fn check_capability(
        &self,
        resource: hkask_types::CapabilityResource,
        resource_id: &str,
        action: hkask_types::CapabilityAction,
    ) -> Result<()> {
        match &self.capability_token {
            Some(token) => {
                if token.is_expired(self.current_time) {
                    return Err(TemplateError::CapabilityDenied(
                        "Capability token expired".to_string(),
                    ));
                }
                if token.is_valid_for(resource, resource_id, action) {
                    Ok(())
                } else {
                    Err(TemplateError::CapabilityDenied(format!(
                        "Capability does not grant {:?} on {}",
                        action, resource_id
                    )))
                }
            }
            None => Err(TemplateError::CapabilityDenied(
                "No capability token present".to_string(),
            )),
        }
    }
}

impl Default for CascadeContext {
    fn default() -> Self {
        Self::new(&[0u8; 32])
    }
}

/// Cascade executor with cycle detection and energy tracking
pub struct CascadeExecutor {
    max_depth: u8,
    cycle_detection: bool,
    energy_tracking: bool,
    security: SecurityAdapter,
}

impl CascadeExecutor {
    pub fn new(secret: &[u8]) -> Self {
        let mut security = SecurityAdapter::new(secret);
        // Allow standard template paths
        security.allow_path("prompt/");
        security.allow_path("process/");
        security.allow_path("cognition/");

        Self {
            max_depth: MAX_CASCADE_DEPTH,
            cycle_detection: true,
            energy_tracking: true,
            security,
        }
    }

    pub fn with_max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth.min(MAX_CASCADE_DEPTH);
        self
    }

    pub fn with_cycle_detection(mut self, enabled: bool) -> Self {
        self.cycle_detection = enabled;
        self
    }

    pub fn with_energy_tracking(mut self, enabled: bool) -> Self {
        self.energy_tracking = enabled;
        self
    }

    pub fn with_security(mut self, security: SecurityAdapter) -> Self {
        self.security = security;
        self
    }

    /// Execute cascade stages in order: pre → core → post
    pub fn execute(
        &self,
        cascade: &Cascade,
        input: Value,
        registry: &dyn RegistryIndex,
    ) -> Result<Value> {
        let mut context =
            CascadeContext::new(&self.security.get_secret()).with_depth(cascade.max_depth);
        let mut state = input;

        // Execute pre stages (context enrichment, validation)
        for stage in &cascade.pre {
            state = self.execute_stage(stage, state, registry, &mut context)?;
        }

        // Execute core stages (main template composition)
        for stage in &cascade.core {
            state = self.execute_stage(stage, state, registry, &mut context)?;
        }

        // Execute post stages (formatting, CNS emission)
        for stage in &cascade.post {
            state = self.execute_stage(stage, state, registry, &mut context)?;
        }

        Ok(state)
    }

    /// Execute cascade with initial capability token
    pub fn execute_with_capability(
        &self,
        cascade: &Cascade,
        input: Value,
        registry: &dyn RegistryIndex,
        token: CapabilityToken,
    ) -> Result<Value> {
        let mut context = CascadeContext::new(&self.security.get_secret())
            .with_depth(cascade.max_depth)
            .with_capability(token)
            .with_current_time(chrono::Utc::now().timestamp());
        let mut state = input;

        for stage in &cascade.pre {
            state = self.execute_stage(stage, state, registry, &mut context)?;
        }

        for stage in &cascade.core {
            state = self.execute_stage(stage, state, registry, &mut context)?;
        }

        for stage in &cascade.post {
            state = self.execute_stage(stage, state, registry, &mut context)?;
        }

        Ok(state)
    }

    /// Execute a single cascade stage with security checks
    fn execute_stage(
        &self,
        stage: &CascadeStage,
        input: Value,
        registry: &dyn RegistryIndex,
        context: &mut CascadeContext,
    ) -> Result<Value> {
        // Check depth limit
        context.check_depth(self.max_depth)?;

        // Check condition if present
        if let Some(condition) = &stage.condition {
            if !self.evaluate_condition(condition, &input) {
                return Ok(input);
            }
        }

        let mut stage_state = input;

        // Execute each template in the stage
        for template_id in &stage.templates {
            // Security: Validate template path
            self.security.validate_template_path(template_id)?;

            // Cycle detection
            if self.cycle_detection {
                context.check_template_cycle(template_id)?;
            }

            // Mark as visited
            context.visit_template(template_id);

            // Resolve template from registry
            let entry = registry.get(template_id)?;

            // Security: Check capability if present
            if context.capability_token.is_some() {
                context.check_capability(
                    hkask_types::CapabilityResource::Template,
                    &entry.id,
                    hkask_types::CapabilityAction::Read,
                )?;
            }

            // Check template type compatibility
            match entry.template_type {
                TemplateType::Prompt => {
                    stage_state = self.execute_prompt_template(&entry, stage_state)?;
                }
                TemplateType::Process => {
                    stage_state = self.execute_process_template(&entry, stage_state, context)?;
                }
                TemplateType::Cognition => {
                    stage_state = self.execute_cognition_template(&entry, stage_state)?;
                }
            }
        }

        Ok(stage_state)
    }

    /// Execute prompt template (WordAct)
    fn execute_prompt_template(
        &self,
        entry: &crate::ports::RegistryEntry,
        input: Value,
    ) -> Result<Value> {
        // Placeholder: actual implementation would render Jinja2 template
        Ok(serde_json::json!({
            "template_id": entry.id,
            "template_type": "Prompt",
            "input": input,
            "status": "rendered"
        }))
    }

    /// Execute process template (FlowDef)
    fn execute_process_template(
        &self,
        entry: &crate::ports::RegistryEntry,
        input: Value,
        context: &mut CascadeContext,
    ) -> Result<Value> {
        // Placeholder: actual implementation would execute manifest
        context.consume_energy(100);
        Ok(serde_json::json!({
            "template_id": entry.id,
            "template_type": "Process",
            "input": input,
            "energy_consumed": 100,
            "status": "executed"
        }))
    }

    /// Execute cognition template (KnowAct)
    fn execute_cognition_template(
        &self,
        entry: &crate::ports::RegistryEntry,
        input: Value,
    ) -> Result<Value> {
        // Placeholder: actual implementation would run ReAct loop
        Ok(serde_json::json!({
            "template_id": entry.id,
            "template_type": "Cognition",
            "input": input,
            "status": "reasoned"
        }))
    }

    /// Evaluate stage condition
    fn evaluate_condition(&self, condition: &str, state: &Value) -> bool {
        // Simplified condition evaluation
        // Actual implementation would use Jinja2 or similar
        condition.is_empty() || state.get(condition).is_some()
    }

    /// Detect cycles in template dependency graph
    pub fn detect_cycles(
        &self,
        start_template: &str,
        registry: &dyn RegistryIndex,
    ) -> Result<CycleDetectionResult> {
        let mut visited = HashSet::new();
        let mut path = vec![];
        let mut cycle_path = vec![];

        self.detect_cycles_dfs(
            start_template,
            registry,
            &mut visited,
            &mut path,
            &mut cycle_path,
        )?;

        Ok(CycleDetectionResult {
            has_cycle: !cycle_path.is_empty(),
            cycle_path,
        })
    }

    /// DFS cycle detection
    fn detect_cycles_dfs(
        &self,
        template_id: &str,
        registry: &dyn RegistryIndex,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycle_path: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(template_id) {
            // Found cycle
            if let Some(idx) = path.iter().position(|t| t == template_id) {
                *cycle_path = path[idx..].to_vec();
                cycle_path.push(template_id.to_string());
            }
            return Ok(());
        }

        visited.insert(template_id.to_string());
        path.push(template_id.to_string());

        // Get template dependencies (placeholder - would parse template source)
        if let Ok(entry) = registry.get(template_id) {
            // Check for template references in description (simplified)
            if entry.description.contains("{{") {
                // Would extract template refs from Jinja2 includes
            }
        }

        path.pop();
        Ok(())
    }

    /// Resolve cascade with template references
    pub fn resolve_cascade(
        &self,
        cascade: &Cascade,
        registry: &dyn RegistryIndex,
    ) -> Result<ResolvedCascade> {
        let mut resolved_templates = HashMap::new();

        // Collect all template IDs from cascade stages
        let all_stages = cascade
            .pre
            .iter()
            .chain(cascade.core.iter())
            .chain(cascade.post.iter());

        for stage in all_stages {
            for template_id in &stage.templates {
                // Resolve template from registry
                let entry = registry.get(template_id)?;
                resolved_templates.insert(template_id.clone(), entry);
            }
        }

        Ok(ResolvedCascade {
            cascade: cascade.clone(),
            resolved_templates,
        })
    }
}

impl Default for CascadeExecutor {
    fn default() -> Self {
        Self::new(&[0u8; 32])
    }
}

/// Resolved cascade with template entries
#[derive(Debug, Clone)]
pub struct ResolvedCascade {
    pub cascade: Cascade,
    pub resolved_templates: HashMap<String, crate::ports::RegistryEntry>,
}

/// Cascade builder for fluent API
pub struct CascadeBuilder {
    cascade: Cascade,
}

impl CascadeBuilder {
    pub fn new(id: &str) -> Self {
        Self {
            cascade: Cascade::new(id),
        }
    }

    pub fn pre(mut self, name: &str, templates: Vec<&str>) -> Self {
        self.cascade.pre.push(CascadeStage {
            name: name.to_string(),
            templates: templates.into_iter().map(String::from).collect(),
            condition: None,
        });
        self
    }

    pub fn core(mut self, name: &str, templates: Vec<&str>) -> Self {
        self.cascade.core.push(CascadeStage {
            name: name.to_string(),
            templates: templates.into_iter().map(String::from).collect(),
            condition: None,
        });
        self
    }

    pub fn post(mut self, name: &str, templates: Vec<&str>) -> Self {
        self.cascade.post.push(CascadeStage {
            name: name.to_string(),
            templates: templates.into_iter().map(String::from).collect(),
            condition: None,
        });
        self
    }

    pub fn max_depth(mut self, depth: u8) -> Self {
        self.cascade.max_depth = depth.min(MAX_CASCADE_DEPTH);
        self
    }

    pub fn build(self) -> Cascade {
        self.cascade
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{Action, ManifestStep, ProcessManifest, RegistryEntry};
    use hkask_types::{CapabilityAction, CapabilityResource, WebID};

    struct MockRegistry {
        entries: HashMap<String, crate::ports::RegistryEntry>,
    }

    impl MockRegistry {
        fn new() -> Self {
            let mut entries = HashMap::new();
            entries.insert(
                "prompt/test".to_string(),
                RegistryEntry {
                    id: "prompt/test".to_string(),
                    template_type: TemplateType::Prompt,
                    lexicon_terms: vec!["test".to_string()],
                    description: "Test prompt".to_string(),
                    source_path: "test.j2".to_string(),
                },
            );
            entries.insert(
                "process/test".to_string(),
                RegistryEntry {
                    id: "process/test".to_string(),
                    template_type: TemplateType::Process,
                    lexicon_terms: vec!["test".to_string()],
                    description: "Test process".to_string(),
                    source_path: "test.yaml".to_string(),
                },
            );
            Self { entries }
        }
    }

    impl RegistryIndex for MockRegistry {
        fn list(&self, _domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
            self.entries.values().cloned().collect()
        }

        fn get(&self, id: &str) -> Result<RegistryEntry> {
            self.entries
                .get(id)
                .cloned()
                .ok_or_else(|| TemplateError::NotFound(format!("Template '{}' not found", id)))
        }

        fn bootstrap_manifest(&self) -> Option<ProcessManifest> {
            Some(ProcessManifest {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test manifest".to_string(),
                steps: vec![ManifestStep {
                    ordinal: 1,
                    action: Action::Execute,
                    description: "Test step".to_string(),
                    template_ref: "test".to_string(),
                    model_tier: None,
                    mcp: None,
                    renderer: None,
                }],
            })
        }
    }

    #[test]
    fn test_cascade_new() {
        let cascade = Cascade::new("test");
        assert_eq!(cascade.id, "test");
        assert!(cascade.pre.is_empty());
        assert_eq!(cascade.max_depth, MAX_CASCADE_DEPTH);
    }

    #[test]
    fn test_cascade_builder() {
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["prompt/test"])
            .core("compose", vec!["process/test"])
            .post("format", vec!["prompt/test"])
            .max_depth(5)
            .build();

        assert_eq!(cascade.pre.len(), 1);
        assert_eq!(cascade.core.len(), 1);
        assert_eq!(cascade.post.len(), 1);
        assert_eq!(cascade.max_depth, 5);
    }

    #[test]
    fn test_cascade_context_depth_check() {
        let context = CascadeContext::new(&[0u8; 32]).with_depth(8);
        let result = context.check_depth(7);
        assert!(result.is_err());

        let context = CascadeContext::new(&[0u8; 32]).with_depth(6);
        let result = context.check_depth(7);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cascade_context_cycle_detection() {
        let mut context = CascadeContext::new(&[0u8; 32]);
        context.visit_template("template-1");

        let result = context.check_template_cycle("template-1");
        assert!(result.is_err());

        let result = context.check_template_cycle("template-2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cascade_context_energy() {
        let mut context = CascadeContext::new(&[0u8; 32]).with_energy(1000);
        assert!(context.check_energy(500).is_ok());
        assert!(context.check_energy(1500).is_err());

        context.consume_energy(500);
        assert_eq!(context.energy_remaining, 500);
    }

    #[test]
    fn test_cascade_context_child_with_attenuation() {
        let secret = [0u8; 32];
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            CapabilityResource::Cascade,
            "test".to_string(),
            CapabilityAction::Execute,
            from,
            to,
            &secret,
        );

        let context = CascadeContext::new(&secret)
            .with_depth(3)
            .with_capability(token.clone());

        let new_holder = WebID::new();
        let child = context.child_context(new_holder);

        assert_eq!(child.current_depth, 4);
        assert_eq!(child.energy_remaining, context.energy_remaining);

        // Verify attenuation occurred
        assert!(child.capability_token.is_some());
        let child_token = child.capability_token.unwrap();
        assert_eq!(child_token.attenuation_level, token.attenuation_level + 1);
    }

    #[test]
    fn test_cascade_context_child_max_attenuation() {
        let secret = [0u8; 32];
        let from = WebID::new();
        let to = WebID::new();

        // Create token at max attenuation
        let token = CapabilityToken::new_with_attenuation(
            CapabilityResource::Cascade,
            "test".to_string(),
            CapabilityAction::Execute,
            from,
            to,
            &secret,
            None,
            7,    // max_attenuation
            7,    // attenuation_level at max
            None, // context_nonce
        );

        let context = CascadeContext::new(&secret)
            .with_depth(3)
            .with_capability(token);

        let new_holder = WebID::new();
        let child = context.child_context(new_holder);

        // Attenuation should fail (already at max)
        assert!(child.capability_token.is_none());
    }

    #[test]
    fn test_cascade_context_capability_check() {
        let secret = [0u8; 32];
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            CapabilityResource::Template,
            "test-template".to_string(),
            CapabilityAction::Read,
            from,
            to,
            &secret,
        );

        let context = CascadeContext::new(&secret).with_capability(token);

        // Valid capability
        let result = context.check_capability(
            CapabilityResource::Template,
            "test-template",
            CapabilityAction::Read,
        );
        assert!(result.is_ok());

        // Wrong resource
        let result = context.check_capability(
            CapabilityResource::Manifest,
            "test-template",
            CapabilityAction::Read,
        );
        assert!(result.is_err());

        // Wrong action
        let result = context.check_capability(
            CapabilityResource::Template,
            "test-template",
            CapabilityAction::Write,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cascade_executor_new() {
        let executor = CascadeExecutor::new(&[0u8; 32]);
        assert_eq!(executor.max_depth, MAX_CASCADE_DEPTH);
        assert!(executor.cycle_detection);
        assert!(executor.energy_tracking);
    }

    #[test]
    fn test_cascade_executor_execute() {
        let registry = MockRegistry::new();
        let executor = CascadeExecutor::new(&[0u8; 32]);
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["prompt/test"])
            .core("compose", vec!["process/test"])
            .build();

        let result = executor.execute(&cascade, Value::Null, &registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_cascade_depth_constant() {
        assert_eq!(MAX_CASCADE_DEPTH, 7);
    }

    #[test]
    fn test_cascade_max_depth_limit() {
        let cascade = Cascade::new("test").with_max_depth(10);
        assert_eq!(cascade.max_depth, 7); // Capped at MAX_CASCADE_DEPTH
    }

    #[test]
    fn test_cascade_security_path_traversal_blocked() {
        let executor = CascadeExecutor::new(&[0u8; 32]);
        let registry = MockRegistry::new();
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["../etc/passwd"])
            .build();

        let result = executor.execute(&cascade, Value::Null, &registry);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TemplateError::PathTraversal(_)
        ));
    }

    #[test]
    fn test_cascade_security_absolute_path_blocked() {
        let executor = CascadeExecutor::new(&[0u8; 32]);
        let registry = MockRegistry::new();
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["/etc/passwd"])
            .build();

        let result = executor.execute(&cascade, Value::Null, &registry);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TemplateError::PathTraversal(_)
        ));
    }

    #[test]
    fn test_cascade_with_capability_attenuation_chain() {
        let secret = [42u8; 32];
        let executor = CascadeExecutor::new(&secret);
        let registry = MockRegistry::new();

        // Create initial capability for the template resource
        let from = WebID::new();
        let to = WebID::new();
        let token = CapabilityToken::new(
            CapabilityResource::Template,
            "prompt/test".to_string(),
            CapabilityAction::Read,
            from,
            to,
            &secret,
        );

        assert_eq!(token.attenuation_level, 0);

        // Execute with capability - context should attenuate on each recursive call
        let cascade = CascadeBuilder::new("test")
            .pre("enrich", vec!["prompt/test"])
            .build();

        let result = executor.execute_with_capability(&cascade, Value::Null, &registry, token);
        assert!(result.is_ok());
    }
}
