// Auto-extracted inline tests for hkask-types
// Extracted: Thu May 21 00:22:23 PDT 2026

// === From capability.rs ===
#[cfg(test)]
mod tests {
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
        assert_eq!(token.attenuation_level, 0);
    }

    #[test]
    fn test_capability_token_expiry() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();
        let current_time = 1000;

        // Create token with 100 second expiry
        let mut token = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        token.expires_at = Some(current_time + 100);

        assert!(!token.is_expired(current_time));
        assert!(token.is_expired(current_time + 101));
    }

    #[test]
    fn test_capability_token_verify_lazy() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();
        let current_time = 1000;

        let mut token = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        token.expires_at = Some(current_time + 100);

        // Valid token
        let result = token.verify_lazy(secret, current_time);
        assert_eq!(result, VerificationResult::Valid);
        assert!(result.is_valid());
        assert!(result.is_usable());

        // Expired token (zombie)
        let result = token.verify_lazy(secret, current_time + 101);
        assert_eq!(result, VerificationResult::Zombie);
        assert!(result.is_valid());
        assert!(!result.is_usable());

        // Invalid signature
        let bad_secret = b"wrong-secret";
        let result = token.verify_lazy(bad_secret, current_time);
        assert_eq!(result, VerificationResult::Invalid);
        assert!(!result.is_valid());
        assert!(!result.is_usable());
    }

    #[test]
    fn test_capability_token_fingerprint() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();

        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );

        let fingerprint = token.fingerprint();
        assert!(!fingerprint.is_empty());
        assert!(fingerprint.contains("tool"));
        assert!(fingerprint.contains("test"));
        assert!(fingerprint.contains("execute"));
    }

    #[test]
    fn test_capability_token_compatibility() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();

        let token1 = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );

        let token2 = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );

        // Same resource, action, delegated_to -> compatible
        assert!(token1.is_compatible_with(&token2));

        // Different resource -> incompatible
        let token3 = CapabilityToken::new(
            CapabilityResource::Registry,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        assert!(!token1.is_compatible_with(&token3));
    }

    #[test]
    fn test_capability_token_attenuation() {
        let secret = b"test-secret-key";
        let from = WebID::new();
        let to = WebID::new();
        let new_to = WebID::new();

        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );

        // Attenuate to new recipient
        let attenuated = token.attenuate(new_to.clone(), secret, 1000);
        assert!(attenuated.is_some());
        let attenuated = attenuated.unwrap();
        assert_eq!(attenuated.delegated_to, new_to);
        assert_eq!(attenuated.attenuation_level, token.attenuation_level + 1);
    }

    #[test]
    fn test_verification_result() {
        assert!(VerificationResult::Valid.is_valid());
        assert!(VerificationResult::Valid.is_usable());
        assert_eq!(VerificationResult::Valid.as_str(), "valid");

        assert!(VerificationResult::Zombie.is_valid());
        assert!(!VerificationResult::Zombie.is_usable());
        assert!(VerificationResult::Zombie.as_str().contains("zombie"));

        assert!(!VerificationResult::Invalid.is_valid());
        assert!(!VerificationResult::Invalid.is_usable());
        assert!(VerificationResult::Invalid.as_str().contains("invalid"));
    }
}

// === From cns.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variety_counter_increment() {
        let mut counter = VarietyCounter::new();
        assert_eq!(counter.0, 0);

        counter.increment();
        assert_eq!(counter.0, 1);
    }

    #[test]
    fn test_variety_counter_deficit() {
        let counter = VarietyCounter(50);
        assert_eq!(counter.deficit(100), 50);
        assert_eq!(counter.deficit(40), 0);
    }

    #[test]
    fn test_variety_counter_needs_alert() {
        let counter = VarietyCounter(0);
        assert!(counter.needs_alert());

        let counter = VarietyCounter(100);
        assert!(!counter.needs_alert());
    }

    #[test]
    fn test_algedonic_alert_new() {
        let alert = AlgedonicAlert::new(0, 100, CnsSpan::Variety);
        assert_eq!(alert.deficit, 100);
        assert!(!alert.escalated);
        assert_eq!(alert.span, CnsSpan::Variety);
    }

    #[test]
    fn test_algedonic_alert_escalate() {
        let mut alert = AlgedonicAlert::new(0, 100, CnsSpan::Variety);
        assert!(!alert.escalated);

        alert.escalate();
        assert!(alert.escalated);
    }

    #[test]
    fn test_cns_span_full_name() {
        assert_eq!(CnsSpan::Template.full_name(), "cns.template");
        assert_eq!(CnsSpan::Curation.full_name(), "cns.curation");
        assert_eq!(CnsSpan::KillZone.full_name(), "cns.killzone");
    }

    #[test]
    fn test_cns_event_new() {
        let event = CnsEvent::new(
            CnsSpan::Template,
            "invoke".to_string(),
            "success".to_string(),
        );
        assert_eq!(event.span, CnsSpan::Template);
        assert_eq!(event.action, "invoke");
        assert!(event.alert.is_none());
    }

    #[test]
    fn test_kill_zone_state_detection() {
        let mut state = KillZoneState::new("social_media".to_string());
        assert!(!state.is_kill_zone());

        state.record_acquisition();
        state.update_vc_investment(0.4);
        assert!(state.is_kill_zone());
    }

    #[test]
    fn test_kill_zone_state_safe() {
        let mut state = KillZoneState::new("open_source".to_string());
        state.update_vc_investment(0.8);
        assert!(!state.is_kill_zone());
    }
}

// === From curation.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curator_id_system() {
        let id1 = CuratorId::system();
        let id2 = CuratorId::system();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_curator_id_new() {
        let id1 = CuratorId::new();
        let id2 = CuratorId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_curation_decision_display() {
        assert_eq!(format!("{}", CurationDecision::Merge), "merge");
        assert_eq!(format!("{}", CurationDecision::Discard), "discard");
    }

    #[test]
    fn test_authority_level_display() {
        assert_eq!(format!("{}", AuthorityLevel::Explicit), "explicit");
        assert_eq!(format!("{}", AuthorityLevel::Denied), "denied");
    }

    #[test]
    fn test_ocap_boundary_explicit() {
        let boundary = OCAPBoundary::explicit("template.invoke".to_string());
        assert_eq!(boundary.authority, AuthorityLevel::Explicit);
        assert!(boundary.is_accessible());
    }

    #[test]
    fn test_ocap_boundary_denied() {
        let boundary = OCAPBoundary::denied("admin.delete_all".to_string());
        assert_eq!(boundary.authority, AuthorityLevel::Denied);
        assert!(!boundary.is_accessible());
    }

    #[test]
    fn test_curation_record_new() {
        let curator_id = CuratorId::system();
        let template_id = TemplateId::new();
        let bot_id = crate::id::BotID::new();
        let params = crate::template::LLMParameters::default();
        let input = serde_json::json!({"test": "value"});
        let invocation = TemplateInvocation::new(template_id, bot_id, params, input);

        let record = CurationRecord::new(
            curator_id,
            invocation.clone(),
            CurationDecision::Merge,
            Some("Logical and sound".to_string()),
        );

        assert_eq!(record.curator_id, curator_id);
        assert_eq!(record.decision, CurationDecision::Merge);
        assert!(record.ocap_boundaries.is_empty());
    }

    #[test]
    fn test_ideological_default() {
        let ide = Ideological::default();
        assert!(ide.0);
    }

    #[test]
    fn test_ideological_display() {
        assert_eq!(
            format!("{}", Ideological::yes()),
            "ideological (having logical ideas)"
        );
    }
}

// === From event.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    fn test_nu_event_with_outcome() {
        let event = NuEvent::new(
            WebID::new(),
            Span::prompt("select"),
            Phase::Observe,
            json!({"test": "data"}),
            0,
        )
        .with_outcome(json!({"result": "success"}));

        assert!(event.outcome.is_some());
    }

    #[test]
    fn test_nu_event_with_parent() {
        let parent_id = EventID::new();
        let event = NuEvent::new(
            WebID::new(),
            Span::prompt("select"),
            Phase::Observe,
            json!({"test": "data"}),
            0,
        )
        .with_parent(parent_id);

        assert_eq!(event.parent_event, Some(parent_id));
    }

    #[test]
    fn test_span_prompt() {
        let span = Span::prompt("select");
        assert_eq!(span.as_str(), "cns.prompt.select");
    }

    #[test]
    fn test_span_tool() {
        let span = Span::tool("invocation");
        assert_eq!(span.as_str(), "cns.tool.invocation");
    }

    #[test]
    fn test_span_agent_pod() {
        let span = Span::agent_pod("populated");
        assert_eq!(span.as_str(), "cns.agent_pod.populated");
    }

    #[test]
    fn test_phase_as_str() {
        assert_eq!(Phase::Observe.as_str(), "observe");
        assert_eq!(Phase::Regulate.as_str(), "regulate");
        assert_eq!(Phase::Outcome.as_str(), "outcome");
    }
}

// === From id.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webid_new() {
        let id1 = WebID::new();
        let id2 = WebID::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_webid_display() {
        let id = WebID::new();
        let display = format!("{}", id);
        assert_eq!(display.len(), 36); // UUID string length
    }

    #[test]
    fn test_template_id() {
        let id = TemplateID::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn test_bot_id() {
        let id = BotID::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn test_manifest_id() {
        let id = ManifestID::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn test_triple_id() {
        let id = TripleID::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn test_event_id() {
        let id = EventID::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::Random));
    }
}

// === From lexicon.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_as_str() {
        assert_eq!(TemplateType::Prompt.as_str(), "Prompt");
        assert_eq!(TemplateType::Process.as_str(), "Process");
        assert_eq!(TemplateType::Cognition.as_str(), "Cognition");
    }

    #[test]
    fn test_template_type_from_str() {
        assert_eq!(
            TemplateType::parse_str("Prompt"),
            Some(TemplateType::Prompt)
        );
        assert_eq!(
            TemplateType::parse_str("process"),
            Some(TemplateType::Process)
        );
        assert_eq!(TemplateType::parse_str("COGNITION"), None);
    }

    #[test]
    fn test_domain_as_str() {
        assert_eq!(Domain::WordAct.as_str(), "WordAct");
        assert_eq!(Domain::FlowDef.as_str(), "FlowDef");
        assert_eq!(Domain::KnowAct.as_str(), "KnowAct");
    }

    #[test]
    fn test_lexicon_term_new() {
        let term = LexiconTerm::new("test", Domain::WordAct, "A test term");
        assert_eq!(term.term, "test");
        assert_eq!(term.domain, Domain::WordAct);
        assert!(term.academic_citation.is_none());
    }

    #[test]
    fn test_lexicon_term_with_citation() {
        let term = LexiconTerm::new("test", Domain::WordAct, "A test term")
            .with_citation("Smith et al. 2024");
        assert_eq!(
            term.academic_citation,
            Some("Smith et al. 2024".to_string())
        );
    }

    #[test]
    fn test_hlexicon_add_and_get() {
        let mut lexicon = HLexicon::new();
        let term = LexiconTerm::new("test", Domain::WordAct, "A test term");
        lexicon.add(term.clone());

        let retrieved = lexicon.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().term, "test");
    }

    #[test]
    fn test_hlexicon_contains() {
        let mut lexicon = HLexicon::new();
        lexicon.add(LexiconTerm::new("test", Domain::WordAct, "A test term"));

        assert!(lexicon.contains("test"));
        assert!(!lexicon.contains("missing"));
    }

    #[test]
    fn test_hlexicon_validate() {
        let mut lexicon = HLexicon::new();
        lexicon.add(LexiconTerm::new("known", Domain::WordAct, "Known term"));

        let terms = vec!["known".to_string(), "unknown".to_string()];
        let invalid = lexicon.validate(&terms);

        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0], "unknown");
    }

    #[test]
    fn test_hlexicon_bootstrap() {
        let lexicon = HLexicon::bootstrap();
        assert!(lexicon.len() > 0);
        assert!(lexicon.contains("recognize"));
        assert!(lexicon.contains("select"));
        assert!(lexicon.contains("reflect"));
    }
}

// === From template.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_id_new() {
        let id1 = TemplateId::new();
        let id2 = TemplateId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_high_temp_template_type_display() {
        assert_eq!(
            format!("{}", HighTempTemplateType::CodeGeneration),
            "code_generation"
        );
        assert_eq!(format!("{}", HighTempTemplateType::Decision), "decision");
    }

    #[test]
    fn test_temperature_range_clamp() {
        let range = TemperatureRange::new(0.5, 0.8);
        assert_eq!(range.clamp(0.3), 0.5);
        assert_eq!(range.clamp(0.6), 0.6);
        assert_eq!(range.clamp(0.9), 0.8);
    }

    #[test]
    fn test_temperature_range_presets() {
        let anti = TemperatureRange::anti_inferno();
        assert_eq!(anti.min, 0.8);
        assert_eq!(anti.max, 1.0);

        let edge = TemperatureRange::edge_work();
        assert_eq!(edge.min, 0.4);
        assert_eq!(edge.max, 0.6);

        let clean = TemperatureRange::clean_place();
        assert_eq!(clean.min, 0.1);
        assert_eq!(clean.max, 0.3);
    }

    #[test]
    fn test_llm_parameters_anti_inferno() {
        let params = LLMParameters::anti_inferno();
        assert_eq!(params.temperature, 0.95);
        assert_eq!(params.top_p, 0.65);
        assert_eq!(params.top_k, 15);
        assert_eq!(params.frequency_penalty, 0.8);
        assert_eq!(params.presence_penalty, 0.8);
    }

    #[test]
    fn test_llm_parameters_clamping() {
        let params = LLMParameters::new(1.5, 1.5, 200, 3.0, 3.0, 100, None);
        assert_eq!(params.temperature, 1.0);
        assert_eq!(params.top_p, 1.0);
        assert_eq!(params.top_k, 100);
        assert_eq!(params.frequency_penalty, 2.0);
        assert_eq!(params.presence_penalty, 2.0);
    }

    #[test]
    fn test_template_outcome_display() {
        assert_eq!(format!("{}", TemplateOutcome::Success), "success");
        assert_eq!(format!("{}", TemplateOutcome::Failure), "failure");
    }

    #[test]
    fn test_template_invocation_new() {
        let template_id = TemplateId::new();
        let bot_id = BotID::new();
        let params = LLMParameters::default();
        let input = serde_json::json!({"test": "value"});

        let invocation = TemplateInvocation::new(template_id, bot_id, params, input);

        assert_eq!(invocation.template_id, template_id);
        assert_eq!(invocation.bot_id, bot_id);
        assert_eq!(invocation.outcome, TemplateOutcome::Failure);
        assert!(invocation.outputs.is_empty());
    }
}

// === From visibility.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::collections::HashMap;

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Private);
    }

    #[test]
    fn test_visibility_as_str() {
        assert_eq!(Visibility::Private.as_str(), "private");
        assert_eq!(Visibility::Public.as_str(), "public");
        assert_eq!(Visibility::Shared.as_str(), "shared");
    }

    #[test]
    fn test_signature_algorithm() {
        assert_eq!(SignatureAlgorithm::Ed25519.as_str(), "ed25519");
        assert_eq!(SignatureAlgorithm::HmacSha256.as_str(), "sha256-hmac");
        assert_eq!(
            SignatureAlgorithm::parse_str("ed25519"),
            Some(SignatureAlgorithm::Ed25519)
        );
        assert_eq!(SignatureAlgorithm::parse_str("invalid"), None);
    }

    #[test]
    fn test_capability_new() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        assert_eq!(cap.resource, "memory");
        assert_eq!(cap.action, "read");
        assert_eq!(cap.granted_by, "alice");
        assert_eq!(cap.granted_to, "bob");
    }

    #[test]
    fn test_capability_matches() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        assert!(cap.matches("memory", "read"));
        assert!(!cap.matches("memory", "write"));
        assert!(!cap.matches("storage", "read"));
    }

    #[test]
    fn test_capability_signing_data() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let data = cap.signing_data();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_capability_with_expiry() {
        let cap = Capability::new("memory", "read", "alice", "bob").with_expiry(1000);
        assert!(!cap.is_expired(500));
        assert!(cap.is_expired(1500));
    }

    #[test]
    fn test_delegation_expiry() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob").with_expiry(1000);

        assert!(!delegation.is_expired(500));
        assert!(delegation.is_expired(1500));
        assert!(delegation.is_valid(500));
        assert!(!delegation.is_valid(1500));
    }

    #[test]
    fn test_delegation_chain() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob")
            .with_expiry(1000)
            .with_parent("del-parent");

        assert_eq!(delegation.parent_delegation, Some("del-parent".to_string()));
    }

    #[test]
    fn test_delegation_store() {
        let mut store = DelegationStore::new();
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob");

        store.add(delegation.clone());
        assert!(store.get("del-1").is_some());
        assert!(store.get("del-2").is_none());

        store.remove("del-1");
        assert!(store.get("del-1").is_none());
    }

    #[test]
    fn test_revocation_list() {
        let mut list = RevocationList::new();
        assert!(!list.is_revoked("cap-1"));

        list.revoke("cap-1");
        assert!(list.is_revoked("cap-1"));

        list.unrevoke("cap-1");
        assert!(!list.is_revoked("cap-1"));
    }

    #[test]
    fn test_access_decision() {
        let allow = AccessDecision::allow();
        assert!(allow.allowed);

        let deny = AccessDecision::deny("test", vec!["missing".to_string()]);
        assert!(!deny.allowed);
        assert_eq!(deny.reason, Some("test".to_string()));
        assert_eq!(deny.missing_capabilities, vec!["missing".to_string()]);
    }

    #[test]
    fn test_access_evaluator_owner() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result = evaluator.evaluate(
            Visibility::Private,
            "alice",
            "alice",
            &caps,
            "memory",
            "read",
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_access_evaluator_public() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Public, "alice", "bob", &caps, "memory", "read");
        assert!(result.allowed);
    }

    #[test]
    fn test_access_evaluator_private() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Private, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
    }

    #[test]
    fn test_access_evaluator_shared_without_capability() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
    }

    #[test]
    fn test_access_evaluator_shared_with_ed25519_signature() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let mut cap = Capability::new("memory", "read", "alice", "bob");
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data).to_bytes();

        cap.signature = CapabilitySignature::new_ed25519(signature, "alice");

        let caps = vec![cap];
        let mut public_keys = HashMap::new();
        public_keys.insert("alice".to_string(), verifying_key.to_vec());

        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(result.allowed);
    }

    #[test]
    fn test_access_evaluator_shared_with_expired_capability() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let mut cap = Capability::new("memory", "read", "alice", "bob").with_expiry(1000);
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data).to_bytes();

        cap.signature = CapabilitySignature::new_ed25519(signature, "alice");

        let caps = vec![cap];
        let mut public_keys = HashMap::new();
        public_keys.insert("alice".to_string(), verifying_key.to_vec());

        let evaluator = AccessEvaluator::new(public_keys, 2000); // Time after expiry
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
        assert!(result
            .missing_capabilities
            .iter()
            .any(|s| s.contains("expired")));
    }

    #[test]
    fn test_access_evaluator_shared_with_revoked_capability() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let mut cap = Capability::new("memory", "read", "alice", "bob");
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data).to_bytes();

        cap.signature = CapabilitySignature::new_ed25519(signature, "alice");

        let caps = vec![cap];
        let mut public_keys = HashMap::new();
        public_keys.insert("alice".to_string(), verifying_key.to_vec());

        let mut revocation_list = RevocationList::new();
        revocation_list.revoke("alice");

        let evaluator = AccessEvaluator::new(public_keys, 0).with_revocation_list(revocation_list);
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
        assert!(result
            .missing_capabilities
            .iter()
            .any(|s| s.contains("revoked")));
    }

    #[test]
    fn test_ed25519_signature_verification() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let data = b"test data";
        let signature = signing_key.sign(data);

        let cap_sig = CapabilitySignature::new_ed25519(signature.to_bytes(), "test");
        assert!(cap_sig.verify(data, &verifying_key.to_vec()));

        let bad_data = b"bad data";
        assert!(!cap_sig.verify(bad_data, &verifying_key.to_vec()));
    }

    #[test]
    fn test_hmac_signature_verification() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let key = b"test_secret_key";
        let data = b"test data";

        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        let signature = mac.finalize().into_bytes().to_vec();

        let cap_sig =
            CapabilitySignature::new(signature.clone(), SignatureAlgorithm::HmacSha256, "test");
        assert!(cap_sig.verify(data, key));

        let bad_data = b"bad data";
        assert!(!cap_sig.verify(bad_data, key));
    }
}
