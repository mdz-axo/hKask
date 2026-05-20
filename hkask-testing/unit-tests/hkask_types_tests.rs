use hkask_types::{
    CapabilityAction, CapabilityResource, CapabilityToken, NuEvent, Phase, Span, TemplateType,
    Visibility, WebID,
};
use serde_json::json;

mod capability_tests {
    use super::*;

    #[test]
    fn test_capability_token_creation() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "inference:call".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );

        assert!(!token.id.is_empty());
        assert_eq!(token.resource, CapabilityResource::Tool);
        assert_eq!(token.resource_id, "inference:call");
        assert_eq!(token.action, CapabilityAction::Execute);
        assert_eq!(token.delegated_from, from);
        assert_eq!(token.delegated_to, to);
        assert!(!token.signature.is_empty());
    }
}

mod event_tests {
    use super::*;

    #[test]
    fn test_nu_event_new() {
        let event = NuEvent::new(
            WebID::new(),
            Span::prompt("select"),
            Phase::Observe,
            json!({"test": "data"}),
            0,
        );

        assert_eq!(event.recursion_depth, 0);
        assert_eq!(event.visibility, "private");
        assert!(event.outcome.is_none());
    }
}

mod id_tests {
    use super::*;

    #[test]
    fn test_webid_new() {
        let id1 = WebID::new();
        let id2 = WebID::new();
        assert_ne!(id1, id2);
    }
}

mod lexicon_tests {
    use super::*;

    #[test]
    fn test_template_type_as_str() {
        assert_eq!(TemplateType::Prompt.as_str(), "Prompt");
        assert_eq!(TemplateType::Process.as_str(), "Process");
        assert_eq!(TemplateType::Cognition.as_str(), "Cognition");
    }
}

mod visibility_tests {
    use super::*;

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Private);
    }
}
