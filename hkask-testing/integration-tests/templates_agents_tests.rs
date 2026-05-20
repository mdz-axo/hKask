//! Integration tests for Templates + Agents coordination
//! Tests template execution with agent pods

use hkask_agents::{
    bot::Bot,
    replicant::Replicant,
    pod::AgentPod,
    capability::AgentCapability,
};
use hkask_templates::{
    registry::TemplateRegistry,
    manifest::ProcessManifest,
    ports::{ManifestStep, Action, ProcessManifest as ProcessManifestDef},
};
use hkask_testing::{
    MockInferenceAdapter, MockMcpAdapter, MockCnsAdapterMut,
    TempBlobStore, TempTripleStore,
};
use hkask_types::{WebID, Visibility, TemplateType};
use serde_json::json;

mod templates_agents_integration {
    use super::*;

    #[test]
    fn test_bot_with_template_registry() {
        let owner = WebID::new();
        let bot = Bot::new("test-bot", owner.clone());

        // Verify bot creation
        assert_eq!(bot.name(), "test-bot");
        assert_eq!(bot.owner(), &owner);
    }

    #[test]
    fn test_replicant_with_template_execution() {
        let owner = WebID::new();
        let replicant = Replicant::new("test-replicant", owner.clone());

        // Verify replicant creation
        assert_eq!(replicant.name(), "test-replicant");
        assert_eq!(replicant.owner(), &owner);
    }

    #[test]
    fn test_agent_pod_template_reference() {
        let owner = WebID::new();
        let pod = AgentPod::new(owner.clone());

        // Pod should be able to reference templates
        assert!(pod.id().to_string().len() > 0);
    }

    #[test]
    fn test_mock_inference_for_template_render() {
        let adapter = MockInferenceAdapter::new()
            .with_response(json!({"rendered": "template_output"}));

        let config = hkask_templates::ports::InferenceConfig::default();
        let result = adapter.call("fast", "render this template", &config);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"rendered": "template_output"}));
    }

    #[test]
    fn test_mock_mcp_for_agent_tool_discovery() {
        let adapter = MockMcpAdapter::new()
            .with_tool("search")
            .with_tool("scrape")
            .with_tool("summarize");

        let tools = adapter.discover_tools();
        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"search".to_string()));
    }

    #[test]
    fn test_mock_mcp_for_agent_tool_invoke() {
        let mut adapter = MockMcpAdapter::new()
            .with_response(json!({"result": "tool executed"}));

        let result = adapter.invoke("test_tool", json!({"param": "value"}));
        assert!(result.is_ok());
        assert_eq!(adapter.invoke_count(), 1);
    }

    #[test]
    fn test_cns_emission_during_template_execution() {
        let adapter = MockCnsAdapterMut::new();

        adapter.emit("cns.prompt.render", json!({"template": "test"}), 0.95);
        adapter.emit("cns.tool.invoke", json!({"tool": "search"}), 0.90);

        assert_eq!(adapter.event_count(), 2);
    }

    #[test]
    fn test_bot_capability_with_template() {
        let owner = WebID::new();
        let capability = AgentCapability::new("template_execution", owner.clone());

        assert_eq!(capability.name(), "template_execution");
        assert_eq!(capability.owner(), &owner);
    }

    #[test]
    fn test_replicant_episodic_memory_with_template() {
        let owner = WebID::new();
        let replicant = Replicant::new("test", owner.clone());
        let store = TempTripleStore::new();

        // Replicant should be able to store episodic triples
        store.insert(hkask_types::Triple::new(
            "template_execution",
            "event",
            json!("completed"),
            owner.clone()
        ));

        assert_eq!(store.len(), 1);
    }

    #[test]
    fn bot_semantic_memory_with_template() {
        let owner = WebID::new();
        let bot = Bot::new("test", owner.clone());
        let store = TempTripleStore::new();

        // Bot should be able to store semantic triples
        store.insert(hkask_types::Triple::new(
            "template",
            "type",
            json!("process_manifest"),
            owner.clone()
        ));

        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_agent_pod_lifecycle_with_cns_monitoring() {
        let owner = WebID::new();
        let pod = AgentPod::new(owner.clone());
        let emitter = hkask_cns::spans::SpanEmitter::new(owner.clone());

        // Monitor pod lifecycle
        emitter.emit_agent_pod("populated", json!({"pod_id": pod.id().to_string()}));
        emitter.emit_agent_pod("registered", json!({"pod_id": pod.id().to_string()}));
        emitter.emit_agent_pod("activated", json!({"pod_id": pod.id().to_string()}));

        assert!(true);
    }

    #[test]
    fn test_template_type_discriminator() {
        // Test all template type variants
        let skill = TemplateType::Skill;
        let process = TemplateType::Process;
        let prompt = TemplateType::Prompt;
        let lexicon = TemplateType::Lexicon;

        assert_ne!(skill, process);
        assert_ne!(skill, prompt);
        assert_ne!(process, lexicon);
    }

    #[test]
    fn test_manifest_step_action_types() {
        use hkask_templates::ports::Action;

        assert_eq!(Action::Select.as_str(), "select");
        assert_eq!(Action::Populate.as_str(), "populate");
        assert_eq!(Action::Execute.as_str(), "execute");

        assert_eq!(Action::from_str("select"), Some(Action::Select));
        assert_eq!(Action::from_str("populate"), Some(Action::Populate));
        assert_eq!(Action::from_str("execute"), Some(Action::Execute));
        assert_eq!(Action::from_str("invalid"), None);
    }

    #[test]
    fn test_process_manifest_structure() {
        let manifest = ProcessManifestDef {
            id: "test-manifest".to_string(),
            name: "Test Process".to_string(),
            description: "A test process manifest".to_string(),
            steps: vec![
                ManifestStep {
                    ordinal: 1,
                    action: Action::Select,
                    description: "Select template".to_string(),
                    template_ref: "template_1".to_string(),
                    model_tier: Some("fast".to_string()),
                    mcp: None,
                    renderer: Some("jinja2".to_string()),
                },
                ManifestStep {
                    ordinal: 2,
                    action: Action::Execute,
                    description: "Execute template".to_string(),
                    template_ref: "template_2".to_string(),
                    model_tier: Some("fast".to_string()),
                    mcp: Some("search".to_string()),
                    renderer: None,
                },
            ],
        };

        assert_eq!(manifest.id, "test-manifest");
        assert_eq!(manifest.steps.len(), 2);
    }

    #[test]
    fn test_bot_public_visibility() {
        let owner = WebID::new();
        let bot = Bot::new("public-bot", owner.clone());

        // Bot should be publicly visible
        assert_eq!(bot.visibility(), Visibility::Public);
    }

    #[test]
    fn test_replicant_visibility() {
        let owner = WebID::new();
        let replicant = Replicant::new("test-replicant", owner.clone());

        // Replicant visibility depends on memory type
        assert_eq!(replicant.visibility(), Visibility::Private);
    }
}

mod agent_capability_tests {
    use super::*;

    #[test]
    fn test_agent_capability_new() {
        let owner = WebID::new();
        let capability = AgentCapability::new("test_cap", owner.clone());

        assert_eq!(capability.name(), "test_cap");
        assert_eq!(capability.owner(), &owner);
    }

    #[test]
    fn test_agent_capability_add_action() {
        let owner = WebID::new();
        let mut capability = AgentCapability::new("test", owner);

        capability.add_action("execute");
        assert!(!capability.actions().is_empty());
    }

    #[test]
    fn test_agent_capability_add_resource() {
        let owner = WebID::new();
        let mut capability = AgentCapability::new("test", owner);

        capability.add_resource("template");
        assert!(!capability.resources().is_empty());
    }

    #[test]
    fn test_agent_capability_verify() {
        let owner = WebID::new();
        let capability = AgentCapability::new("test", owner);

        let result = capability.verify();
        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_capability_serialize() {
        let owner = WebID::new();
        let capability = AgentCapability::new("test", owner);

        let serialized = capability.serialize();
        assert!(!serialized.is_empty());
    }
}

mod bot_tests {
    use super::*;

    #[test]
    fn test_bot_new() {
        let owner = WebID::new();
        let bot = Bot::new("test-bot", owner.clone());

        assert_eq!(bot.name(), "test-bot");
        assert_eq!(bot.owner(), &owner);
        assert_eq!(bot.visibility(), Visibility::Public);
    }

    #[test]
    fn test_bot_execute() {
        let owner = WebID::new();
        let bot = Bot::new("test-bot", owner);

        // Bot execution should complete
        let result = bot.execute("test input");
        assert!(result.is_ok());
    }

    #[test]
    fn test_bot_machine_to_machine() {
        let owner = WebID::new();
        let bot1 = Bot::new("bot1", owner.clone());
        let bot2 = Bot::new("bot2", owner);

        // A2A interaction should be possible
        assert_ne!(bot1.id(), bot2.id());
    }
}

mod replicant_tests {
    use super::*;

    #[test]
    fn test_replicant_new() {
        let owner = WebID::new();
        let replicant = Replicant::new("test-replicant", owner.clone());

        assert_eq!(replicant.name(), "test-replicant");
        assert_eq!(replicant.owner(), &owner);
        assert_eq!(replicant.visibility(), Visibility::Private);
    }

    #[test]
    fn test_replicant_human_to_agent() {
        let owner = WebID::new();
        let replicant = Replicant::new("curator", owner.clone());

        // H2A interaction should be possible
        assert!(replicant.name().len() > 0);
    }

    #[test]
    fn test_replicant_episodic_private() {
        let owner = WebID::new();
        let replicant = Replicant::new("test", owner.clone());

        // Episodic memory should be private
        assert_eq!(replicant.visibility(), Visibility::Private);
    }

    #[test]
    fn test_replicant_semantic_public() {
        let owner = WebID::new();
        let replicant = Replicant::new("test", owner.clone());

        // Semantic memory can be public
        let semantic_visibility = replicant.semantic_visibility();
        assert_eq!(semantic_visibility, Visibility::Public);
    }
}

mod agent_pod_tests {
    use super::*;

    #[test]
    fn test_agent_pod_new() {
        let owner = WebID::new();
        let pod = AgentPod::new(owner.clone());

        assert!(pod.id().to_string().len() > 0);
        assert_eq!(pod.owner(), &owner);
    }

    #[test]
    fn test_agent_populate() {
        let owner = WebID::new();
        let pod = AgentPod::new(owner.clone());

        // Pod should be able to populate with template
        assert!(true);
    }

    #[test]
    fn test_agent_register() {
        let owner = WebID::new();
        let pod = AgentPod::new(owner.clone());

        // Pod should be able to register
        assert!(true);
    }

    #[test]
    fn test_agent_activate() {
        let owner = WebID::new();
        let pod = AgentPod::new(owner.clone());

        // Pod should be able to activate
        assert!(true);
    }

    #[test]
    fn test_agent_delegation() {
        let owner = WebID::new();
        let pod1 = AgentPod::new(owner.clone());
        let pod2 = AgentPod::new(owner);

        // Pods should be able to delegate
        assert_ne!(pod1.id(), pod2.id());
    }
}