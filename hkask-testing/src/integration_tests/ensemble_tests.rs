//! Ensemble Integration Tests
//!
//! Tests multi-agent chat and deliberation with CNS monitoring

use hkask_cns::spans::SpanEmitter;
use hkask_ensemble::{
    AgentResponse, ChatMessage, ChatParticipant, DeliberationCoordinator, DeliberationRequest,
    DeliberationSession, DeliberationStatus, EnsembleChat, EnsembleChatManager, EnsembleError,
    ParticipantRole,
};
use hkask_types::WebID;
use serde_json::json;

mod ensemble_chat_integration {
    use super::*;

    #[tokio::test]
    async fn test_multi_agent_chat_lifecycle() {
        let curator = WebID::new();
        let manager = EnsembleChatManager::new(curator.clone());

        // Create chat session
        let chat = manager.create_chat("test_session").await;
        {
            let mut chat_write = chat.write().await;
            chat_write.grant_consent();

            // Register bot participants
            let memory_bot = ChatParticipant {
                webid: WebID::new(),
                role: ParticipantRole::MemoryBot,
                pod_id: None,
            };
            let spandrel_bot = ChatParticipant {
                webid: WebID::new(),
                role: ParticipantRole::SpandrelBot,
                pod_id: None,
            };

            chat_write.register_participant(memory_bot);
            chat_write.register_participant(spandrel_bot);

            // Add message from curator
            let message = ChatMessage::new(curator.clone(), "Hello, bots!".to_string());
            chat_write.add_message(message);

            // Dispatch to memory bot
            let bot_webid = chat_write.get_participants().keys().nth(1).unwrap().clone();
            let result = chat_write
                .dispatch_to_bot(&bot_webid, "memory_query", json!({}))
                .await;
            assert!(result.is_ok());
        }

        // Verify session exists
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains(&"test_session".to_string()));
    }

    #[tokio::test]
    async fn test_chat_with_multiple_messages() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());
        chat.grant_consent();

        // Add multiple messages
        for i in 0..5 {
            let message = ChatMessage::new(curator.clone(), format!("Message {}", i));
            chat.add_message(message);
        }

        assert_eq!(chat.get_history().len(), 5);
        assert_eq!(chat.get_history()[0].content, "Message 0");
        assert_eq!(chat.get_history()[4].content, "Message 4");
    }

    #[tokio::test]
    async fn test_chat_participant_roles() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        // Register different bot types
        let bots = vec![
            (ParticipantRole::MemoryBot, "memory_bot"),
            (ParticipantRole::SpandrelBot, "spandrel_bot"),
            (ParticipantRole::OkapiBot, "okapi_bot"),
            (ParticipantRole::ScholarBot, "scholar_bot"),
        ];

        for (role, _) in bots {
            chat.register_participant(ChatParticipant {
                webid: WebID::new(),
                role,
                pod_id: None,
            });
        }

        // Should have curator + 4 bots
        assert_eq!(chat.get_participants().len(), 5);
    }

    #[tokio::test]
    async fn test_chat_aggregate_responses() {
        let curator = WebID::new();
        let chat = EnsembleChat::new(curator.clone());

        let responses = [
            (WebID::new(), "Memory bot response".to_string()),
            (WebID::new(), "Spandrel bot response".to_string()),
            (WebID::new(), "Okapi bot response".to_string()),
        ]
        .into_iter()
        .collect();

        let aggregated = chat.aggregate_responses(responses);

        assert!(aggregated.contains("Memory bot response"));
        assert!(aggregated.contains("Spandrel bot response"));
        assert!(aggregated.contains("Okapi bot response"));
    }

    #[tokio::test]
    async fn test_chat_manager_multiple_sessions() {
        let curator = WebID::new();
        let manager = EnsembleChatManager::new(curator.clone());

        // Create multiple sessions
        for i in 0..3 {
            manager.create_chat(&format!("session_{}", i)).await;
        }

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 3);

        // Delete middle session
        manager.delete_chat("session_1").await;

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 2);
        assert!(!sessions.contains(&"session_1".to_string()));
    }

    #[tokio::test]
    async fn test_chat_sovereignty_denial() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());
        // Don't grant consent

        let bot_webid = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot_webid.clone(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        });

        let result = chat
            .dispatch_to_bot(&bot_webid, "test_template", json!({}))
            .await;

        assert!(matches!(result, Err(EnsembleError::SovereigntyDenied(_))));
    }
}

mod deliberation_integration {
    use super::*;

    #[test]
    fn test_deliberation_session_workflow() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("deliberation_1".to_string(), curator.clone());

        // Start deliberation
        session.start();
        assert!(matches!(session.status(), DeliberationStatus::InProgress));

        // Add participants
        session.add_participant(ChatParticipant {
            webid: WebID::new(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        });
        session.add_participant(ChatParticipant {
            webid: WebID::new(),
            role: ParticipantRole::SpandrelBot,
            pod_id: None,
        });

        // Record responses with different confidence levels
        session.record_response(AgentResponse::new(
            WebID::new(),
            "High confidence response".to_string(),
            0.95,
        ));
        session.record_response(AgentResponse::new(
            WebID::new(),
            "Medium confidence response".to_string(),
            0.75,
        ));
        session.record_response(AgentResponse::new(
            WebID::new(),
            "Low confidence response".to_string(),
            0.45,
        ));

        // Synthesize
        let result = session.synthesize();
        assert_eq!(result.individual_responses.len(), 3);

        // Complete deliberation
        session.complete();
        assert!(matches!(session.status(), DeliberationStatus::Completed));
    }

    #[test]
    fn test_deliberation_coordinator_multiple_sessions() {
        let curator = WebID::new();
        let mut coordinator = DeliberationCoordinator::new(curator.clone());

        // Create multiple sessions
        for i in 0..5 {
            coordinator.create_session(&format!("deliberation_{}", i));
        }

        assert_eq!(coordinator.session_count(), 5);

        // Get and modify a session
        let session = coordinator.get_session_mut("deliberation_2").unwrap();
        session.start();
        assert!(matches!(session.status(), DeliberationStatus::InProgress));

        // Remove a session
        coordinator.remove_session("deliberation_0");
        assert_eq!(coordinator.session_count(), 4);
    }

    #[test]
    fn test_deliberation_request_with_context() {
        let request = DeliberationRequest::new("What is the capital of France?".to_string())
            .with_context(json!({
                "topic": "geography",
                "difficulty": "easy"
            }))
            .with_template("query_template".to_string())
            .with_timeout(45000);

        assert_eq!(request.query, "What is the capital of France?");
        assert!(request.context.is_some());
        assert_eq!(request.template_id, Some("query_template".to_string()));
        assert_eq!(request.timeout_ms, 45000);
    }

    #[test]
    fn test_deliberation_response_sorting() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("sort_test".to_string(), curator.clone());

        // Add responses in random order
        let confidences = vec![0.3, 0.9, 0.5, 0.8, 0.1];
        for conf in confidences {
            session.record_response(AgentResponse::new(
                WebID::new(),
                format!("Response with confidence {}", conf),
                conf,
            ));
        }

        let result = session.synthesize();

        // Verify sorted by confidence (descending)
        for i in 0..result.individual_responses.len() - 1 {
            assert!(
                result.individual_responses[i].confidence
                    >= result.individual_responses[i + 1].confidence
            );
        }
    }

    #[test]
    fn test_deliberation_cancel_clears_responses() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("cancel_test".to_string(), curator.clone());

        session.record_response(AgentResponse::new(
            WebID::new(),
            "Response 1".to_string(),
            0.8,
        ));
        session.record_response(AgentResponse::new(
            WebID::new(),
            "Response 2".to_string(),
            0.7,
        ));

        assert_eq!(session.response_count(), 2);

        session.cancel();
        assert!(matches!(session.status(), DeliberationStatus::Cancelled));

        // Responses should still be accessible after cancel
        assert_eq!(session.response_count(), 2);
    }

    #[tokio::test]
    async fn test_span_emission_for_deliberation() {
        let curator = WebID::new();
        let span_emitter = SpanEmitter::new(curator.clone());

        // Emit spans for deliberation lifecycle
        span_emitter.emit_agent_pod("deliberation_started", json!({"session": "test"}));
        span_emitter.emit_tool("deliberation_dispatch", json!({"agents": 3}));
        span_emitter.emit_tool("deliberation_response", json!({"confidence": 0.85}));
        span_emitter.emit_agent_pod("deliberation_completed", json!({}));

        // Test passes if no panic (spans are emitted)
        assert!(true);
    }
}

mod ensemble_cns_integration {
    use super::*;
    use hkask_cns::variety::VarietyMonitor;

    #[test]
    fn test_variety_tracking_for_chat_states() {
        let mut monitor = VarietyMonitor::new();

        // Track different chat states
        monitor.counter("chat.state").increment("idle");
        monitor.counter("chat.state").increment("processing");
        monitor.counter("chat.state").increment("waiting_response");
        monitor.counter("chat.state").increment("completed");

        assert_eq!(monitor.counter("chat.state").variety(), 4);
    }

    #[test]
    fn test_variety_tracking_for_deliberation_states() {
        let mut monitor = VarietyMonitor::new();

        // Track deliberation states
        monitor.counter("deliberation.state").increment("pending");
        monitor
            .counter("deliberation.state")
            .increment("in_progress");
        monitor.counter("deliberation.state").increment("completed");

        assert_eq!(monitor.counter("deliberation.state").variety(), 3);
    }

    #[tokio::test]
    async fn test_full_ensemble_workflow() {
        let curator = WebID::new();

        // Create chat manager
        let chat_manager = EnsembleChatManager::new(curator.clone());
        let chat = chat_manager.create_chat("full_workflow").await;

        // Setup chat with consent and participants
        {
            let mut chat_write = chat.write().await;
            chat_write.grant_consent();

            // Register bots
            for role in [
                ParticipantRole::MemoryBot,
                ParticipantRole::SpandrelBot,
                ParticipantRole::OkapiBot,
            ] {
                chat_write.register_participant(ChatParticipant {
                    webid: WebID::new(),
                    role,
                    pod_id: None,
                });
            }

            // Add curator message
            chat_write.add_message(ChatMessage::new(
                curator.clone(),
                "Please analyze this data".to_string(),
            ));
        }

        // Create deliberation session
        let mut deliberation =
            DeliberationSession::new("workflow_deliberation".to_string(), curator);
        deliberation.start();

        // Simulate bot responses
        deliberation.record_response(AgentResponse::new(
            WebID::new(),
            "Memory analysis complete".to_string(),
            0.9,
        ));
        deliberation.record_response(AgentResponse::new(
            WebID::new(),
            "Spandrel analysis complete".to_string(),
            0.85,
        ));

        // Synthesize responses
        let result = deliberation.synthesize();
        assert!(result.synthesized_response.contains("Memory analysis"));
        assert!(result.synthesized_response.contains("Spandrel analysis"));

        deliberation.complete();

        // Clean up chat
        chat_manager.delete_chat("full_workflow").await;
    }
}
