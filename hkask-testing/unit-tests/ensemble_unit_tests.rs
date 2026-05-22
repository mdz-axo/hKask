//! Ensemble Unit Tests
//!
//! Tests for hkask-ensemble crate - multi-agent chat and deliberation

use hkask_ensemble::{
    AgentResponse, ChatMessage, ChatParticipant, DeliberationCoordinator, DeliberationRequest,
    DeliberationSession, DeliberationStatus, EnsembleChat, EnsembleChatManager, EnsembleError,
    ParticipantRole,
};
use hkask_types::WebID;
use serde_json::json;

mod chat_tests {
    use super::*;

    #[test]
    fn test_ensemble_chat_new() {
        let curator = WebID::new();
        let chat = EnsembleChat::new(curator.clone());

        assert_eq!(chat.curator(), &curator);
        assert_eq!(chat.get_participants().len(), 1);
        assert_eq!(chat.get_history().len(), 0);
    }

    #[test]
    fn test_register_participant() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        let bot_participant = ChatParticipant {
            webid: WebID::new(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        };

        chat.register_participant(bot_participant);
        assert_eq!(chat.get_participants().len(), 2);
    }

    #[test]
    fn test_add_message() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        let message = ChatMessage::new(curator.clone(), "Hello, bots!".to_string());
        chat.add_message(message);

        assert_eq!(chat.get_history().len(), 1);
        assert_eq!(chat.get_history()[0].content, "Hello, bots!");
    }

    #[test]
    fn test_message_with_template() {
        let curator = WebID::new();
        let message = ChatMessage::new(curator.clone(), "Process this".to_string())
            .with_template("test_template".to_string());

        assert_eq!(message.template_id, Some("test_template".to_string()));
    }

    #[test]
    fn test_aggregate_responses() {
        let curator = WebID::new();
        let chat = EnsembleChat::new(curator.clone());

        let responses = HashMap::from([
            (WebID::new(), "Response from bot 1".to_string()),
            (WebID::new(), "Response from bot 2".to_string()),
        ]);

        let aggregated = chat.aggregate_responses(responses);
        assert!(aggregated.contains("Response from bot 1"));
        assert!(aggregated.contains("Response from bot 2"));
    }

    #[test]
    fn test_clear_chat() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        chat.add_message(ChatMessage::new(curator.clone(), "Message 1".to_string()));
        chat.add_message(ChatMessage::new(curator.clone(), "Message 2".to_string()));

        assert_eq!(chat.get_history().len(), 2);

        chat.clear();
        assert_eq!(chat.get_history().len(), 0);
    }

    #[tokio::test]
    async fn test_chat_manager_create() {
        let curator = WebID::new();
        let manager = EnsembleChatManager::new(curator.clone());

        let chat = manager.create_chat("session1").await;
        let chat_read = chat.read().await;
        assert_eq!(chat_read.curator(), &curator);
        drop(chat_read);

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains(&"session1".to_string()));
    }

    #[tokio::test]
    async fn test_chat_manager_delete() {
        let curator = WebID::new();
        let manager = EnsembleChatManager::new(curator.clone());

        manager.create_chat("session1").await;
        assert_eq!(manager.list_sessions().await.len(), 1);

        let deleted = manager.delete_chat("session1").await;
        assert!(deleted);

        assert_eq!(manager.list_sessions().await.len(), 0);
    }

    #[tokio::test]
    async fn test_dispatch_to_bot_success() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        chat.grant_consent();

        let bot_webid = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot_webid.clone(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        });

        let result = chat
            .dispatch_to_bot(&bot_webid, "test_template", json!({}))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_to_bot_not_found() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        chat.grant_consent();

        let unknown_bot = WebID::new();
        let result = chat
            .dispatch_to_bot(&unknown_bot, "test_template", json!({}))
            .await;

        assert!(matches!(result, Err(EnsembleError::ParticipantNotFound(_))));
    }
}

mod deliberation_tests {
    use super::*;

    #[test]
    fn test_deliberation_session_new() {
        let curator = WebID::new();
        let session = DeliberationSession::new("session1".to_string(), curator.clone());

        assert_eq!(session.session_id, "session1");
        assert_eq!(session.participant_count(), 0);
        assert_eq!(session.response_count(), 0);
        assert!(matches!(session.status(), DeliberationStatus::Pending));
    }

    #[test]
    fn test_add_participant() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("session1".to_string(), curator.clone());

        let participant = ChatParticipant {
            webid: WebID::new(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        };

        session.add_participant(participant);
        assert_eq!(session.participant_count(), 1);
    }

    #[test]
    fn test_record_response() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("session1".to_string(), curator.clone());

        let agent_webid = WebID::new();
        let response = AgentResponse::new(agent_webid, "Test response".to_string(), 0.85);

        session.record_response(response);
        assert_eq!(session.response_count(), 1);
    }

    #[test]
    fn test_synthesize_responses() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("session1".to_string(), curator.clone());

        let agent1 = WebID::new();
        let agent2 = WebID::new();

        session.record_response(AgentResponse::new(
            agent1,
            "First response".to_string(),
            0.9,
        ));
        session.record_response(AgentResponse::new(
            agent2,
            "Second response".to_string(),
            0.75,
        ));

        let result = session.synthesize();
        assert!(result.synthesized_response.contains("First response"));
        assert!(result.synthesized_response.contains("Second response"));
        assert_eq!(result.individual_responses.len(), 2);
        assert_eq!(result.synthesis_method, "concatenation");
    }

    #[test]
    fn test_deliberation_lifecycle() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("session1".to_string(), curator.clone());

        assert!(matches!(session.status(), DeliberationStatus::Pending));

        session.start();
        assert!(matches!(session.status(), DeliberationStatus::InProgress));

        session.complete();
        assert!(matches!(session.status(), DeliberationStatus::Completed));
    }

    #[test]
    fn test_deliberation_cancel() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("session1".to_string(), curator.clone());

        session.start();
        session.cancel();
        assert!(matches!(session.status(), DeliberationStatus::Cancelled));
    }

    #[test]
    fn test_agent_response_builder() {
        let agent = WebID::new();
        let response = AgentResponse::new(agent, "Response".to_string(), 0.8)
            .with_template("test_template".to_string())
            .with_processing_time(150);

        assert_eq!(response.template_used, Some("test_template".to_string()));
        assert_eq!(response.processing_time_ms, 150);
        assert!((response.confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_deliberation_request_builder() {
        let request = DeliberationRequest::new("What is the answer?".to_string())
            .with_context(json!({"key": "value"}))
            .with_template("query_template".to_string())
            .with_timeout(60000);

        assert_eq!(request.query, "What is the answer?");
        assert!(request.context.is_some());
        assert_eq!(request.template_id, Some("query_template".to_string()));
        assert_eq!(request.timeout_ms, 60000);
    }

    #[test]
    fn test_coordinator_create_session() {
        let curator = WebID::new();
        let mut coordinator = DeliberationCoordinator::new(curator.clone());

        let session = coordinator.create_session("session1");
        assert_eq!(session.session_id, "session1");
        assert_eq!(coordinator.session_count(), 1);
    }

    #[test]
    fn test_coordinator_list_sessions() {
        let curator = WebID::new();
        let mut coordinator = DeliberationCoordinator::new(curator.clone());

        coordinator.create_session("session1");
        coordinator.create_session("session2");
        coordinator.create_session("session3");

        let sessions = coordinator.list_sessions();
        assert_eq!(sessions.len(), 3);
        assert!(sessions.contains(&"session1"));
        assert!(sessions.contains(&"session2"));
        assert!(sessions.contains(&"session3"));
    }

    #[test]
    fn test_coordinator_remove_session() {
        let curator = WebID::new();
        let mut coordinator = DeliberationCoordinator::new(curator.clone());

        coordinator.create_session("session1");
        assert_eq!(coordinator.session_count(), 1);

        let removed = coordinator.remove_session("session1");
        assert!(removed.is_some());
        assert_eq!(coordinator.session_count(), 0);
    }

    #[test]
    fn test_synthesize_sorts_by_confidence() {
        let curator = WebID::new();
        let mut session = DeliberationSession::new("session1".to_string(), curator.clone());

        let agent1 = WebID::new();
        let agent2 = WebID::new();
        let agent3 = WebID::new();

        session.record_response(AgentResponse::new(
            agent1,
            "Low confidence".to_string(),
            0.5,
        ));
        session.record_response(AgentResponse::new(
            agent2,
            "High confidence".to_string(),
            0.95,
        ));
        session.record_response(AgentResponse::new(
            agent3,
            "Medium confidence".to_string(),
            0.75,
        ));

        let result = session.synthesize();

        assert!(result.individual_responses[0].confidence >= 0.9);
        assert!(result.individual_responses[1].confidence >= 0.7);
        assert!(result.individual_responses[2].confidence < 0.6);
    }
}

use std::collections::HashMap;
