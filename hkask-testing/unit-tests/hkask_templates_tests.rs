// Auto-extracted inline tests for hkask-templates
// Extracted: Thu May 21 00:22:28 PDT 2026

// === From curator_pipeline.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{BotID, LLMParameters, TemplateId};

    #[tokio::test]
    async fn test_curator_pipeline_new() {
        let pipeline = CuratorPipeline::new(CuratorId::system());
        assert_eq!(pipeline.curator_id, CuratorId::system());
    }

    #[tokio::test]
    async fn test_curator_pipeline_submit_evaluate() {
        let pipeline = CuratorPipeline::new(CuratorId::system());
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let mut invocation = TemplateInvocation::new(template_id, bot_id, params, input);
        invocation.outputs.push(serde_json::json!("Generated code"));
        invocation.outcome = TemplateOutcome::Success;

        pipeline.submit(invocation).await;
        let results = pipeline.evaluate_pending().await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].decision, CurationDecision::Merge);
    }

    #[tokio::test]
    async fn test_curator_pipeline_empty_outputs() {
        let pipeline = CuratorPipeline::new(CuratorId::system());
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let invocation = TemplateInvocation::new(template_id, bot_id, params, input);

        let result = pipeline.evaluate_invocation(&invocation).await;
        assert_eq!(result.decision, CurationDecision::Discard);
        assert!(result.rationale.unwrap().contains("No outputs"));
    }

    #[tokio::test]
    async fn test_curator_pipeline_error_output() {
        let pipeline = CuratorPipeline::new(CuratorId::system());
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let mut invocation = TemplateInvocation::new(template_id, bot_id, params, input);
        invocation
            .outputs
            .push(serde_json::json!("ERROR: compilation failed"));
        invocation.outcome = TemplateOutcome::Success;

        let result = pipeline.evaluate_invocation(&invocation).await;
        assert_eq!(result.decision, CurationDecision::Discard);
        assert!(result.rationale.unwrap().contains("error"));
    }

    #[tokio::test]
    async fn test_curator_pipeline_variety_tracking() {
        let pipeline = CuratorPipeline::new(CuratorId::system());
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let mut invocation = TemplateInvocation::new(template_id, bot_id, params, input);
        invocation.outputs.push(serde_json::json!("Good code"));
        invocation.outcome = TemplateOutcome::Success;

        let before = pipeline.get_variety().await;
        pipeline.evaluate_invocation(&invocation).await;
        let after = pipeline.get_variety().await;

        // Merge should increase variety
        assert!(after.0 > before.0);
    }

    #[tokio::test]
    async fn test_curator_pipeline_multiple_outputs() {
        let pipeline = CuratorPipeline::new(CuratorId::system());
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let mut invocation = TemplateInvocation::new(template_id, bot_id, params, input);
        invocation.outputs.push(serde_json::json!("Option 1"));
        invocation.outputs.push(serde_json::json!("Option 2"));
        invocation.outputs.push(serde_json::json!("Option 3"));
        invocation.outcome = TemplateOutcome::Success;

        let result = pipeline.evaluate_invocation(&invocation).await;
        assert_eq!(result.decision, CurationDecision::Merge);
        assert!(result.rationale.unwrap().contains("3 outputs"));
    }

    #[tokio::test]
    async fn test_merge_outputs() {
        let outputs = vec![
            serde_json::json!("First output"),
            serde_json::json!("Second output"),
            serde_json::json!("Third output"),
        ];

        let merged = merge_outputs(&outputs);
        assert!(merged.is_some());
        let merged = merged.unwrap();
        assert!(merged.contains("First output"));
        assert!(merged.contains("Second output"));
        assert!(merged.contains("Third output"));
        assert!(merged.contains("---"));
    }

    #[tokio::test]
    async fn test_merge_outputs_empty() {
        let outputs: Vec<serde_json::Value> = vec![];
        let merged = merge_outputs(&outputs);
        assert!(merged.is_none());
    }
}

// === From engine.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_template_engine_register() {
        let engine = TemplateEngine::new();
        let template_id = TemplateId::new();

        engine
            .register(
                template_id,
                "test_template",
                "Generate code for: {{ input.task }}",
            )
            .await
            .unwrap();

        let registry = engine.registry.lock().await;
        let template = registry.get(template_id).unwrap();
        assert_eq!(template.name, "test_template");
    }

    #[tokio::test]
    async fn test_template_engine_invoke() {
        let engine = TemplateEngine::new();
        let template_id = TemplateId::new();
        let bot_id = BotID::new();

        engine
            .register(
                template_id,
                "test_template",
                "Generate code for: {{ input.task }}",
            )
            .await
            .unwrap();

        let input = serde_json::json!({"task": "hello world"});
        let params = LLMParameters::default();

        let invocation = engine
            .invoke(template_id, bot_id, params, input)
            .await
            .unwrap();

        assert_eq!(invocation.outcome, TemplateOutcome::Success);
        assert!(!invocation.outputs.is_empty());
    }

    #[tokio::test]
    async fn test_template_engine_presets() {
        let engine = TemplateEngine::new();
        let template_id = TemplateId::new();
        let bot_id = BotID::new();

        engine
            .register(template_id, "test_template", "Reframe: {{ input.problem }}")
            .await
            .unwrap();

        let input = serde_json::json!({"problem": "how to scale"});

        // Test anti-inferno preset
        let invocation = engine
            .invoke_anti_inferno(template_id, bot_id, input.clone())
            .await
            .unwrap();
        assert_eq!(invocation.outcome, TemplateOutcome::Success);

        // Test edge work preset
        let invocation = engine
            .invoke_edge_work(template_id, bot_id, input.clone())
            .await
            .unwrap();
        assert_eq!(invocation.outcome, TemplateOutcome::Success);

        // Test clean place preset
        let invocation = engine
            .invoke_clean_place(template_id, bot_id, input)
            .await
            .unwrap();
        assert_eq!(invocation.outcome, TemplateOutcome::Success);
    }

    #[tokio::test]
    async fn test_template_not_found() {
        let engine = TemplateEngine::new();
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let input = serde_json::json!({"task": "test"});
        let params = LLMParameters::default();

        let result = engine.invoke(template_id, bot_id, params, input).await;
        assert!(matches!(
            result,
            Err(TemplateEngineError::TemplateNotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_template_registry_list() {
        let mut registry = TemplateRegistry::new();
        let id1 = TemplateId::new();
        let id2 = TemplateId::new();

        registry.add(id1, "template1", "template 1");
        registry.add(id2, "template2", "template 2");

        let templates = registry.list();
        assert_eq!(templates.len(), 2);
    }
}

// === From inference_port.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okapi_inference_new() {
        let inference = OkapiInference::new("test-model", "http://test:8080");
        assert_eq!(inference.model, "test-model");
        assert_eq!(inference.base_url, "http://test:8080");
    }

    #[test]
    fn test_okapi_inference_local() {
        let inference = OkapiInference::local("test-model");
        assert_eq!(inference.model, "test-model");
        assert_eq!(inference.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_okapi_inference_fast_local() {
        let inference = OkapiInference::fast_local();
        assert_eq!(inference.model, "fast-local-model");
    }

    #[tokio::test]
    async fn test_invoke_template_with_okapi() {
        let inference = OkapiInference::fast_local();
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let invocation = invoke_template_with_okapi(
            &inference,
            template_id,
            bot_id,
            params,
            "Test prompt",
            input,
        )
        .await
        .unwrap();

        assert_eq!(invocation.outcome, TemplateOutcome::Success);
        assert!(!invocation.outputs.is_empty());
    }

    #[tokio::test]
    async fn test_invoke_template_with_selection() {
        let inference = OkapiInference::fast_local();
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"task": "test"});

        let invocation = invoke_template_with_selection(
            &inference,
            template_id,
            bot_id,
            params,
            "Test prompt",
            input,
            3,
        )
        .await
        .unwrap();

        assert_eq!(invocation.outcome, TemplateOutcome::Merged);
        assert_eq!(invocation.outputs.len(), 3);
        assert_eq!(invocation.selected_index, Some(0));
    }
}

// === From russell_mapper.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_russell_skill_manifest() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
id: test-skill
version: 1.0.0
symptoms:
  - test_symptom
probes: []
interventions: []
safety:
  max_auto_risk: none
"#
        )
        .unwrap();

        let mapper = RussellMapper::new();
        let result = mapper.analyze_skill_manifest(file.path());

        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.id, "test-skill");
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_transform_to_hkask_manifest() {
        let russell = RussellSkillManifest {
            id: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            authored: "2026-05-20".to_string(),
            min_harness_version: "0.1.0".to_string(),
            symptoms: vec!["test_symptom".to_string()],
            applies_when: vec![],
            probes: vec![],
            interventions: vec![],
            safety: RussellSafety {
                max_auto_risk: "none".to_string(),
                require_human_for: vec![],
                allowed_env_keys: vec![],
                needs_network: false,
            },
            references: vec![],
        };

        let mapper = RussellMapper::new();
        let result = mapper.transform_to_hkask_manifest(&russell);

        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.id, "skill/test-skill");
        assert_eq!(manifest.steps.len(), 1);
    }

    #[test]
    fn test_extract_jinja2_variables() {
        let content = r#"
{{ variable1 }}
{{ variable2 }}
{{ variable1 }}
{{ nested.object.field }}
"#;

        let vars = RussellMapper::extract_jinja2_variables(content);

        assert!(vars.contains(&"variable1".to_string()));
        assert!(vars.contains(&"variable2".to_string()));
        assert!(vars.contains(&"nested".to_string()));
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn test_infer_lexicon_terms() {
        let body = r#"
## Subjective
Patient reports symptoms.

## Assessment
Based on observation.

## Plan
ACTION: skill/probe
"#;

        let terms = RussellMapper::infer_lexicon_terms(body);

        assert!(terms.contains(&"observe".to_string()));
        assert!(terms.contains(&"assess".to_string()));
        assert!(terms.contains(&"plan".to_string()));
        assert!(terms.contains(&"act".to_string()));
    }
}
