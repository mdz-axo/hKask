//! Improvisor — Generative improvisation engine for multi-agent chat
//!
//! Agents self-select to speak based on relevance confidence rather than
//! being assigned turns. This produces natural, improvisational conversation
//! where agents contribute when they have something unique to say.
//!
//! Principles:
//! - P1: Self-selection, not assignment
//! - P2: Confidence as the filter primitive (speak when high, silent when low)
//! - P3: Mode governs orchestration style (freeform, curator_led, round_robin)
//! - P4: Filter level is adjustable with defaults
//! - P5: Natural conversational flow (later speakers see earlier contributions)

use crate::chat::{ChatParticipant, EnsembleError};
use crate::deliberation::AgentResponse;
use crate::ports::{GenerateOptions, GenerateRequest, InferenceClient};
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImprovMode {
    #[default]
    Freeform,
    CuratorLed,
    RoundRobin,
}

impl ImprovMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Freeform => "freeform",
            Self::CuratorLed => "curator_led",
            Self::RoundRobin => "round_robin",
        }
    }

    pub fn parse_mode(s: &str) -> Option<Self> {
        match s {
            "freeform" => Some(Self::Freeform),
            "curator_led" | "curator-led" => Some(Self::CuratorLed),
            "round_robin" | "round-robin" => Some(Self::RoundRobin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovisorConfig {
    pub mode: ImprovMode,
    pub participation_threshold: f64,
    pub max_speakers_per_turn: usize,
    pub conversational_context_window: usize,
    pub relevance_model: String,
    pub relevance_max_tokens: i32,
    pub curator_synthesis: SynthesisMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisMode {
    Always,
    Optional,
    Never,
}

impl Default for ImprovisorConfig {
    fn default() -> Self {
        Self {
            mode: ImprovMode::Freeform,
            participation_threshold: 0.75,
            max_speakers_per_turn: 3,
            conversational_context_window: 5,
            relevance_model: "qwen3:8b".to_string(),
            relevance_max_tokens: 100,
            curator_synthesis: SynthesisMode::Optional,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceJudgment {
    pub agent_name: String,
    pub confidence: f64,
    pub reason: String,
    pub should_speak: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovTurn {
    pub user_message: String,
    pub judgments: Vec<RelevanceJudgment>,
    pub responses: Vec<AgentResponse>,
    pub curator_synthesis: Option<String>,
}

#[derive(Debug, Clone)]
struct ParticipantSpeaker {
    name: String,
    confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub agent: String,
    pub content: String,
}

pub struct Improvisor<C: InferenceClient> {
    config: ImprovisorConfig,
    inference_client: Arc<C>,
    span_emitter: SpanEmitter,
}

#[derive(Debug, thiserror::Error)]
pub enum ImprovError<E: std::error::Error + Send + Sync> {
    #[error("Inference error: {0}")]
    Inference(E),

    #[error("Relevance parse error: {0}")]
    RelevanceParse(String),

    #[error("No speakers for turn")]
    NoSpeakers,

    #[error("Ensemble error: {0}")]
    Ensemble(#[from] EnsembleError),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl<C: InferenceClient> Improvisor<C> {
    pub fn new(config: ImprovisorConfig, inference_client: Arc<C>, curator_webid: WebID) -> Self {
        let span_emitter = SpanEmitter::new(curator_webid);
        Self {
            config,
            inference_client,
            span_emitter,
        }
    }

    pub fn config(&self) -> &ImprovisorConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: ImprovisorConfig) {
        self.config = config;
    }

    pub fn set_participation_threshold(&mut self, threshold: f64) {
        self.config.participation_threshold = threshold.clamp(0.0, 1.0);
    }

    pub fn set_mode(&mut self, mode: ImprovMode) {
        self.config.mode = mode;
    }

    pub async fn improv_turn(
        &self,
        user_message: &str,
        participants: &[(String, String, ChatParticipant)],
        conversation_context: &[ConversationTurn],
    ) -> Result<ImprovTurn, ImprovError<C::Error>> {
        self.span_emitter.emit_agent_pod(
            "improv_turn_started",
            json!({
                "mode": self.config.mode.as_str(),
                "participant_count": participants.len(),
                "threshold": self.config.participation_threshold,
            }),
        );

        let judgments = match self.config.mode {
            ImprovMode::Freeform => {
                self.relevance_check(user_message, participants, conversation_context)
                    .await?
            }
            ImprovMode::CuratorLed => self.curator_selected_speakers(participants),
            ImprovMode::RoundRobin => self.all_speakers(participants),
        };

        let speakers = self.filter_speakers(&judgments);

        self.span_emitter.emit_agent_pod(
            "improv_speakers_selected",
            json!({
                "candidates": judgments.len(),
                "speakers": speakers.len(),
                "names": speakers.iter().map(|s| &s.name).collect::<Vec<_>>(),
            }),
        );

        if speakers.is_empty() {
            self.span_emitter.emit_agent_pod(
                "improv_no_speakers",
                json!({"message_length": user_message.len()}),
            );
            return Ok(ImprovTurn {
                user_message: user_message.to_string(),
                judgments,
                responses: vec![],
                curator_synthesis: None,
            });
        }

        let responses = self
            .sequential_speak(user_message, &speakers, conversation_context)
            .await?;

        let curator_synthesis = match self.config.curator_synthesis {
            SynthesisMode::Always => Some(self.synthesize(user_message, &responses).await),
            SynthesisMode::Optional if responses.len() > 3 => {
                Some(self.synthesize(user_message, &responses).await)
            }
            _ => None,
        };

        self.span_emitter.emit_agent_pod(
            "improv_turn_completed",
            json!({
                "responses": responses.len(),
                "has_synthesis": curator_synthesis.is_some(),
            }),
        );

        Ok(ImprovTurn {
            user_message: user_message.to_string(),
            judgments,
            responses,
            curator_synthesis,
        })
    }

    async fn relevance_check(
        &self,
        user_message: &str,
        participants: &[(String, String, ChatParticipant)],
        conversation_context: &[ConversationTurn],
    ) -> Result<Vec<RelevanceJudgment>, ImprovError<C::Error>> {
        let context_str = self.format_context(conversation_context);

        let mut judgments = Vec::new();

        for (name, description, _participant) in participants {
            if *name == "Curator" {
                continue;
            }

            let prompt = format!(
                "You are {} ({}).\n\
                 The following message was sent in a group conversation:\n\n\
                 \"{}\"\n\n\
                 {}\n\n\
                 Do you have something unique and valuable to contribute to this discussion?\n\
                 Rate your relevance confidence from 0.0 to 1.0.\n\
                 Respond with ONLY a JSON object: {{\"confidence\": <float>, \"reason\": \"<brief explanation>\"}}",
                name,
                description,
                user_message,
                if context_str.is_empty() {
                    String::new()
                } else {
                    format!("Recent context:\n{}", context_str)
                }
            );

            let request = GenerateRequest {
                model: self.config.relevance_model.clone(),
                prompt,
                options: Some(GenerateOptions {
                    n_probs: None,
                    temperature: Some(0.3),
                    max_tokens: Some(self.config.relevance_max_tokens),
                }),
            };

            let response = self
                .inference_client
                .generate(&request)
                .await
                .map_err(ImprovError::Inference)?;

            let judgment = self.parse_relevance_response(name, &response.response);
            judgments.push(judgment);
        }

        Ok(judgments)
    }

    fn parse_relevance_response(&self, name: &str, raw: &str) -> RelevanceJudgment {
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let (confidence, reason) =
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cleaned) {
                let conf = parsed
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let reas = parsed
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no reason given")
                    .to_string();
                (conf, reas)
            } else {
                let conf = self.extract_first_float(cleaned).unwrap_or(0.0);
                (conf, "parsed from non-JSON response".to_string())
            };

        let should_speak = confidence >= self.config.participation_threshold;

        RelevanceJudgment {
            agent_name: name.to_string(),
            confidence,
            reason,
            should_speak,
        }
    }

    fn extract_first_float(&self, s: &str) -> Option<f64> {
        for part in s.split_whitespace() {
            if let Ok(f) = part.parse::<f64>()
                && (0.0..=1.0).contains(&f)
            {
                return Some(f);
            }
        }
        None
    }

    fn curator_selected_speakers(
        &self,
        participants: &[(String, String, ChatParticipant)],
    ) -> Vec<RelevanceJudgment> {
        participants
            .iter()
            .filter(|(name, _, _)| *name != "Curator")
            .map(|(name, description, _)| RelevanceJudgment {
                agent_name: name.clone(),
                confidence: 1.0,
                reason: format!("Curator selected {} ({})", name, description),
                should_speak: true,
            })
            .collect()
    }

    fn all_speakers(
        &self,
        participants: &[(String, String, ChatParticipant)],
    ) -> Vec<RelevanceJudgment> {
        participants
            .iter()
            .filter(|(name, _, _)| *name != "Curator")
            .map(|(name, _, _)| RelevanceJudgment {
                agent_name: name.clone(),
                confidence: 1.0,
                reason: "round_robin mode: all speak".to_string(),
                should_speak: true,
            })
            .collect()
    }

    fn filter_speakers(&self, judgments: &[RelevanceJudgment]) -> Vec<ParticipantSpeaker> {
        let mut speakers: Vec<_> = judgments
            .iter()
            .filter(|j| j.should_speak)
            .map(|j| ParticipantSpeaker {
                name: j.agent_name.clone(),
                confidence: j.confidence,
            })
            .collect();

        speakers.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        speakers.truncate(self.config.max_speakers_per_turn);

        speakers
    }

    async fn sequential_speak(
        &self,
        user_message: &str,
        speakers: &[ParticipantSpeaker],
        conversation_context: &[ConversationTurn],
    ) -> Result<Vec<AgentResponse>, ImprovError<C::Error>> {
        let mut responses = Vec::new();
        let mut turn_context = conversation_context.to_vec();

        for speaker in speakers {
            let context_with_earlier = self.format_context_with_earlier(&turn_context, &responses);

            let prompt = format!(
                "{}User: {}\n\nProvide your response as {}:",
                if context_with_earlier.is_empty() {
                    String::new()
                } else {
                    format!("Conversation so far:\n{}\n\n", context_with_earlier)
                },
                user_message,
                speaker.name,
            );

            let request = GenerateRequest {
                model: self.config.relevance_model.clone(),
                prompt,
                options: Some(GenerateOptions {
                    n_probs: Some(5),
                    temperature: Some(0.7),
                    max_tokens: Some(512),
                }),
            };

            let response = self
                .inference_client
                .generate(&request)
                .await
                .map_err(ImprovError::Inference)?;

            let confidence = response
                .completion_probabilities
                .as_ref()
                .map(|probs| crate::confidence_router::compute_confidence(probs))
                .unwrap_or(speaker.confidence);

            let agent_response = AgentResponse::new(
                WebID::from_persona(speaker.name.as_bytes()),
                response.response.trim().to_string(),
                confidence,
            );

            turn_context.push(ConversationTurn {
                agent: speaker.name.clone(),
                content: agent_response.content.clone(),
            });

            responses.push(agent_response);
        }

        Ok(responses)
    }

    async fn synthesize(&self, user_message: &str, responses: &[AgentResponse]) -> String {
        let mut summary = String::new();
        for r in responses {
            summary.push_str(&format!("[{}]: {}\n", r.agent_webid, r.content));
        }

        let prompt = format!(
            "The user asked: {}\n\n\
             Multiple agents responded:\n{}\n\n\
             Provide a brief synthesis that highlights key insights and any disagreements:",
            user_message, summary
        );

        let request = GenerateRequest {
            model: self.config.relevance_model.clone(),
            prompt,
            options: Some(GenerateOptions {
                n_probs: None,
                temperature: Some(0.5),
                max_tokens: Some(256),
            }),
        };

        match self.inference_client.generate(&request).await {
            Ok(response) => response.response.trim().to_string(),
            Err(_) => {
                let mut fallback = String::from("Synthesis: ");
                for r in responses {
                    fallback.push_str(&format!("[{}]: {}; ", r.agent_webid, r.content));
                }
                fallback
            }
        }
    }

    fn format_context(&self, turns: &[ConversationTurn]) -> String {
        if turns.is_empty() {
            return String::new();
        }

        let window = turns
            .len()
            .saturating_sub(self.config.conversational_context_window);
        turns[window..]
            .iter()
            .map(|t| format!("{}: {}", t.agent, t.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_context_with_earlier(
        &self,
        previous_turns: &[ConversationTurn],
        earlier_responses: &[AgentResponse],
    ) -> String {
        let mut parts = Vec::new();

        let window = previous_turns
            .len()
            .saturating_sub(self.config.conversational_context_window);
        for turn in &previous_turns[window..] {
            parts.push(format!("{}: {}", turn.agent, turn.content));
        }

        for resp in earlier_responses {
            parts.push(format!("{}: {}", resp.agent_webid, resp.content));
        }

        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ParticipantRole;
    use crate::ports::{GenerateResponse, TokenProbability};

    struct MockInference {
        response_text: String,
        confidence: f64,
    }

    impl MockInference {
        fn new(response_text: &str, confidence: f64) -> Self {
            Self {
                response_text: response_text.to_string(),
                confidence,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("mock error")]
    struct MockError;

    #[async_trait::async_trait]
    impl InferenceClient for MockInference {
        type Error = MockError;

        async fn generate(
            &self,
            _request: &GenerateRequest,
        ) -> Result<GenerateResponse, Self::Error> {
            Ok(GenerateResponse {
                response: self.response_text.clone(),
                model: "mock".to_string(),
                completion_probabilities: Some(vec![TokenProbability {
                    token: "test".to_string(),
                    prob: self.confidence,
                    top_k: vec![],
                }]),
            })
        }

        async fn chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _model: String,
        ) -> Result<serde_json::Value, Self::Error> {
            Ok(serde_json::json!({"response": self.response_text}))
        }
    }

    #[test]
    fn test_improv_mode_from_str() {
        assert_eq!(
            ImprovMode::parse_mode("freeform"),
            Some(ImprovMode::Freeform)
        );
        assert_eq!(
            ImprovMode::parse_mode("curator_led"),
            Some(ImprovMode::CuratorLed)
        );
        assert_eq!(
            ImprovMode::parse_mode("curator-led"),
            Some(ImprovMode::CuratorLed)
        );
        assert_eq!(
            ImprovMode::parse_mode("round_robin"),
            Some(ImprovMode::RoundRobin)
        );
        assert_eq!(ImprovMode::parse_mode("invalid"), None);
    }

    #[test]
    fn test_improv_config_defaults() {
        let config = ImprovisorConfig::default();
        assert_eq!(config.mode, ImprovMode::Freeform);
        assert!((config.participation_threshold - 0.75).abs() < f64::EPSILON);
        assert_eq!(config.max_speakers_per_turn, 3);
        assert_eq!(config.conversational_context_window, 5);
    }

    #[test]
    fn test_filter_speakers_respects_threshold() {
        let config = ImprovisorConfig::default();
        let mock = MockInference::new("test", 0.5);
        let improvisor = Improvisor::new(config, Arc::new(mock), WebID::new());

        let judgments = vec![
            RelevanceJudgment {
                agent_name: "high".to_string(),
                confidence: 0.9,
                reason: "very relevant".to_string(),
                should_speak: true,
            },
            RelevanceJudgment {
                agent_name: "low".to_string(),
                confidence: 0.3,
                reason: "not relevant".to_string(),
                should_speak: false,
            },
            RelevanceJudgment {
                agent_name: "mid".to_string(),
                confidence: 0.8,
                reason: "somewhat relevant".to_string(),
                should_speak: true,
            },
        ];

        let speakers = improvisor.filter_speakers(&judgments);
        assert_eq!(speakers.len(), 2);
        assert_eq!(speakers[0].name, "high");
        assert_eq!(speakers[1].name, "mid");
    }

    #[test]
    fn test_filter_speakers_respects_max() {
        let mut config = ImprovisorConfig::default();
        config.max_speakers_per_turn = 2;
        let mock = MockInference::new("test", 0.5);
        let improvisor = Improvisor::new(config, Arc::new(mock), WebID::new());

        let judgments = vec![
            RelevanceJudgment {
                agent_name: "a".to_string(),
                confidence: 0.95,
                reason: String::new(),
                should_speak: true,
            },
            RelevanceJudgment {
                agent_name: "b".to_string(),
                confidence: 0.90,
                reason: String::new(),
                should_speak: true,
            },
            RelevanceJudgment {
                agent_name: "c".to_string(),
                confidence: 0.85,
                reason: String::new(),
                should_speak: true,
            },
        ];

        let speakers = improvisor.filter_speakers(&judgments);
        assert_eq!(speakers.len(), 2);
        assert_eq!(speakers[0].name, "a");
        assert_eq!(speakers[1].name, "b");
    }

    #[test]
    fn test_parse_relevance_json() {
        let config = ImprovisorConfig::default();
        let mock = MockInference::new("test", 0.5);
        let improvisor = Improvisor::new(config, Arc::new(mock), WebID::new());

        let judgment = improvisor.parse_relevance_response(
            "ScholarBot",
            r#"{"confidence": 0.87, "reason": "I have research on this topic"}"#,
        );
        assert!((judgment.confidence - 0.87).abs() < f64::EPSILON);
        assert!(judgment.should_speak);
        assert_eq!(judgment.reason, "I have research on this topic");
    }

    #[test]
    fn test_parse_relevance_markdown_wrapped() {
        let config = ImprovisorConfig::default();
        let mock = MockInference::new("test", 0.5);
        let improvisor = Improvisor::new(config, Arc::new(mock), WebID::new());

        let judgment = improvisor.parse_relevance_response(
            "ScholarBot",
            "```json\n{\"confidence\": 0.6, \"reason\": \"tangential\"}\n```",
        );
        assert!((judgment.confidence - 0.6).abs() < f64::EPSILON);
        assert!(!judgment.should_speak);
    }

    #[tokio::test]
    async fn test_round_robin_all_speak() {
        let config = ImprovisorConfig {
            mode: ImprovMode::RoundRobin,
            ..Default::default()
        };
        let mock = MockInference::new("My response", 0.9);
        let improvisor = Improvisor::new(config, Arc::new(mock), WebID::new());

        let participants = vec![
            (
                "BotA".to_string(),
                "Expert in A".to_string(),
                ChatParticipant {
                    webid: WebID::new(),
                    role: ParticipantRole::Custom("a".to_string()),
                    pod_id: None,
                    capabilities: vec![],
                },
            ),
            (
                "BotB".to_string(),
                "Expert in B".to_string(),
                ChatParticipant {
                    webid: WebID::new(),
                    role: ParticipantRole::Custom("b".to_string()),
                    pod_id: None,
                    capabilities: vec![],
                },
            ),
        ];

        let result = improvisor
            .improv_turn("What do you think?", &participants, &[])
            .await
            .unwrap();

        assert_eq!(result.responses.len(), 2);
        assert!(result.judgments.iter().all(|j| j.should_speak));
    }

    #[test]
    fn test_set_threshold_clamps() {
        let config = ImprovisorConfig::default();
        let mock = MockInference::new("test", 0.5);
        let mut improvisor = Improvisor::new(config, Arc::new(mock), WebID::new());

        improvisor.set_participation_threshold(1.5);
        assert!((improvisor.config().participation_threshold - 1.0).abs() < f64::EPSILON);

        improvisor.set_participation_threshold(-0.5);
        assert!((improvisor.config().participation_threshold - 0.0).abs() < f64::EPSILON);
    }
}
