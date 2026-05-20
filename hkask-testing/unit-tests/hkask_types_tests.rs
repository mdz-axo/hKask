use hkask_types::capability::BotCapabilities;
use hkask_types::capability::CapabilityChecker;
use hkask_types::visibility::AccessDecision;
use hkask_types::visibility::AccessEvaluator;
use hkask_types::{BotID, ManifestID, TemplateID, TripleID};
use hkask_types::{
    Capability, CapabilityAction, CapabilityResource, CapabilitySignature, CapabilityToken,
    Delegation, DelegationStore, Domain, EventID, HLexicon, LexiconTerm, NuEvent, Phase,
    RevocationList, SignatureAlgorithm, Span, TemplateType, Visibility, WebID,
};
use serde_json::json;
use std::collections::HashMap;

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

#[test]
fn test_capability_token_verification() {
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

    assert!(token.verify(secret));
}

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

#[test]
fn test_webid_new() {
    let id1 = WebID::new();
    let id2 = WebID::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_template_type_as_str() {
    assert_eq!(TemplateType::Prompt.as_str(), "Prompt");
    assert_eq!(TemplateType::Process.as_str(), "Process");
    assert_eq!(TemplateType::Cognition.as_str(), "Cognition");
}

#[test]
fn test_visibility_default() {
    assert_eq!(Visibility::default(), Visibility::Private);
}

#[test]
fn test_hlexicon_bootstrap() {
    let lexicon = HLexicon::bootstrap();
    assert!(lexicon.len() > 0);
    assert!(lexicon.contains("recognize"));
    assert!(lexicon.contains("select"));
    assert!(lexicon.contains("reflect"));
}

#[test]
fn test_bot_capabilities() {
    let bot_id = WebID::new();
    let caps = BotCapabilities::new(bot_id.clone())
        .with_capabilities(vec!["inference:call", "storage:read"]);

    assert!(caps.has_capability("inference:call"));
    assert!(caps.has_capability("storage:read"));
    assert!(!caps.has_capability("memory:write"));
}

#[test]
fn test_id_types() {
    let template_id = TemplateID::new();
    let bot_id = BotID::new();
    let manifest_id = ManifestID::new();
    let triple_id = TripleID::new();
    let event_id = EventID::new();

    assert_eq!(template_id.0.get_version(), Some(uuid::Version::Random));
    assert_eq!(bot_id.0.get_version(), Some(uuid::Version::Random));
    assert_eq!(manifest_id.0.get_version(), Some(uuid::Version::Random));
    assert_eq!(triple_id.0.get_version(), Some(uuid::Version::Random));
    assert_eq!(event_id.0.get_version(), Some(uuid::Version::Random));
}

#[test]
fn test_span_prompt() {
    let span = Span::prompt("select");
    assert_eq!(span.as_str(), "cns.prompt.select");
}

#[test]
fn test_phase_as_str() {
    assert_eq!(Phase::Observe.as_str(), "observe");
    assert_eq!(Phase::Regulate.as_str(), "regulate");
    assert_eq!(Phase::Outcome.as_str(), "outcome");
}
