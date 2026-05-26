//! Improvisation mode — Generative improvisation capability for ensemble chat
//!
//! Improvisation is NOT an agent or persona. It is a coordination capability
//! that ensemble sessions apply: agents self-select to speak based on relevance
//! confidence, producing natural conversational flow.
//!
//! Principles:
//! - P1: Self-selection, not assignment — agents decide to speak
//! - P2: Confidence as the filter primitive — speak when high, silent when low
//! - P3: Mode governs orchestration style (freeform, curator_led, round_robin)
//! - P4: Filter level is adjustable with defaults
//! - P5: Natural conversational flow — later speakers see earlier contributions

use crate::chat::EnsembleError;
use crate::deliberation::AgentResponse;
use crate::ports::{GenerateOptions, GenerateRequest, InferenceClient};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisMode {
    Always,
    Optional,
    Never,
}

impl Default for SynthesisMode {
    fn default() -> Self {
        Self::Optional
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovSessionConfig {
    pub mode: ImprovMode,
    pub participation_threshold: f64,
    pub max_speakers_per_turn: usize,
    pub context_window: usize,
    pub relevance_model: String,
    pub relevance_max_tokens: i32,
    pub synthesis: SynthesisMode,
}

impl Default for ImprovSessionConfig {
    fn default() -> Self {
        Self {
            mode: ImprovMode::Freeform,
            participation_threshold: 0.75,
            max_speakers_per_turn: 3,
            context_window: 5,
            relevance_model: "qwen3:8b".to_string(),
            relevance_max_tokens: 100,
            synthesis: SynthesisMode::Optional,
        }
    }
}

impl ImprovSessionConfig {
    pub fn set_threshold(&mut self, threshold: f64) {
        self.participation_threshold = threshold.clamp(0.0, 1.0);
    }

    pub fn set_mode(&mut self, mode: ImprovMode) {
        self.mode = mode;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceJudgment {
    pub agent_webid: WebID,
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
struct Speaker {
    webid: WebID,
    name: String,
    confidence: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum ImprovError<E: std::error::Error + Send + Sync> {
    #[error("Inference error: {0}")]
    Inference(E),

    #[error("Relevance parse error: {0}")]
    RelevanceParse(String),

    #[error("Ensemble error: {0}")]
    Ensemble(#[from] EnsembleError),
}

pub async fn improv_turn<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    participants: &[(WebID, String, String)],
    chat_history: &[(WebID, String)],
) -> Result<ImprovTurn, ImprovError<C::Error>> {
    let judgments = match config.mode {
        ImprovMode::Freeform => {
            relevance_check(config, inference_client, user_message, participants, chat_history)
                .await?
        }
        ImprovMode::CuratorLed => curator_selected(participants),
        ImprovMode::RoundRobin => all_speakers(participants),
    };

    let speakers = filter_speakers(config, &judgments);

    if speakers.is_empty() {
        return Ok(ImprovTurn {
            user_message: user_message.to_string(),
            judgments,
            responses: vec![],
            curator_synthesis: None,
        });
    }

    let responses = sequential_speak(config, inference_client, user_message, &speakers, chat_history)
        .await?;

    let curator_synthesis = match config.synthesis {
        SynthesisMode::Always => Some(synthesize(config, inference_client, user_message, &responses).await),
        SynthesisMode::Optional if responses.len() > 3 => {
            Some(synthesize(config, inference_client, user_message, &responses).await)
        }
        _ => None,
    };

    Ok(ImprovTurn {
        user_message: user_message.to_string(),
        judgments,
        responses,
        curator_synthesis,
    })
}

async fn relevance_check<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    participants: &[(WebID, String, String)],
    chat_history: &[(WebID, String)],
) -> Result<Vec<RelevanceJudgment>, ImprovError<C::Error>> {
    let context_str = format_context(config, chat_history);
    let mut judgments = Vec::new();

    for (webid, name, description) in participants {
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
            model: config.relevance_model.clone(),
            prompt,
            options: Some(GenerateOptions {
                n_probs: None,
                temperature: Some(0.3),
                max_tokens: Some(config.relevance_max_tokens),
            }),
        };

        let response = inference_client
            .generate(&request)
            .await
            .map_err(ImprovError::Inference)?;

        let judgment = parse_relevance(config, webid, name, &response.response);
        judgments.push(judgment);
    }

    Ok(judgments)
}

fn parse_relevance(config: &ImprovSessionConfig, webid: &WebID, name: &str, raw: &str) -> RelevanceJudgment {
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
            let conf = extract_first_float(cleaned).unwrap_or(0.0);
            (conf, "parsed from non-JSON response".to_string())
        };

    let should_speak = confidence >= config.participation_threshold;

    RelevanceJudgment {
        agent_webid: webid.clone(),
        agent_name: name.to_string(),
        confidence,
        reason,
        should_speak,
    }
}

fn extract_first_float(s: &str) -> Option<f64> {
    for part in s.split_whitespace() {
        if let Ok(f) = part.parse::<f64>() {
            if (0.0..=1.0).contains(&f) {
                return Some(f);
            }
        }
    }
    None
}

fn curator_selected(participants: &[(WebID, String, String)]) -> Vec<RelevanceJudgment> {
    participants
        .iter()
        .map(|(webid, name, desc)| RelevanceJudgment {
            agent_webid: webid.clone(),
            agent_name: name.clone(),
            confidence: 1.0,
            reason: format!("Curator selected {} ({})", name, desc),
            should_speak: true,
        })
        .collect()
}

fn all_speakers(participants: &[(WebID, String, String)]) -> Vec<RelevanceJudgment> {
    participants
        .iter()
        .map(|(webid, name, _)| RelevanceJudgment {
            agent_webid: webid.clone(),
            agent_name: name.clone(),
            confidence: 1.0,
            reason: format!("round_robin: {} speaks", name),
            should_speak: true,
        })
        .collect()
}

fn filter_speakers(config: &ImprovSessionConfig, judgments: &[RelevanceJudgment]) -> Vec<Speaker> {
    let mut speakers: Vec<_> = judgments
        .iter()
        .filter(|j| j.should_speak)
        .map(|j| Speaker {
            webid: j.agent_webid.clone(),
            name: j.agent_name.clone(),
            confidence: j.confidence,
        })
        .collect();

    speakers.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    speakers.truncate(config.max_speakers_per_turn);
    speakers
}

async fn sequential_speak<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    speakers: &[Speaker],
    chat_history: &[(WebID, String)],
) -> Result<Vec<AgentResponse>, ImprovError<C::Error>> {
    let mut responses = Vec::new();
    let mut turn_context = chat_history.to_vec();

    for speaker in speakers {
        let context_str = format_context_with_earlier(config, &turn_context, &responses);

        let prompt = format!(
            "{}User: {}\n\nProvide your response as {}:",
            if context_str.is_empty() {
                String::new()
            } else {
                format!("Conversation so far:\n{}\n\n", context_str)
            },
            user_message,
            speaker.name,
        );

        let request = GenerateRequest {
            model: config.relevance_model.clone(),
            prompt,
            options: Some(GenerateOptions {
                n_probs: Some(5),
                temperature: Some(0.7),
                max_tokens: Some(512),
            }),
        };

        let response = inference_client
            .generate(&request)
            .await
            .map_err(ImprovError::Inference)?;

        let confidence = response
            .completion_probabilities
            .as_ref()
            .map(|probs| crate::confidence_router::compute_confidence(probs))
            .unwrap_or(speaker.confidence);

        let agent_response = AgentResponse::new(
            speaker.webid.clone(),
            response.response.trim().to_string(),
            confidence,
        );

        turn_context.push((speaker.webid.clone(), agent_response.content.clone()));
        responses.push(agent_response);
    }

    Ok(responses)
}

async fn synthesize<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    responses: &[AgentResponse],
) -> String {
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
        model: config.relevance_model.clone(),
        prompt,
        options: Some(GenerateOptions {
            n_probs: None,
            temperature: Some(0.5),
            max_tokens: Some(256),
        }),
    };

    match inference_client.generate(&request).await {
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

fn format_context(config: &ImprovSessionConfig, turns: &[(WebID, String)]) -> String {
    if turns.is_empty() {
        return String::new();
    }
    let window = turns.len().saturating_sub(config.context_window);
    turns[window..]
        .iter()
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_context_with_earlier(
    config: &ImprovSessionConfig,
    previous_turns: &[(WebID, String)],
    earlier_responses: &[AgentResponse],
) -> String {
    let mut parts = Vec::new();
    let window = previous_turns
        .len()
        .saturating_sub(config.context_window);
    for (_, content) in &previous_turns[window..] {
        parts.push(content.clone());
    }
    for resp in earlier_responses {
        parts.push(format!("[{}]: {}", resp.agent_webid, resp.content));
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn test_participants() -> Vec<(WebID, String, String)> {
        vec![
            (WebID::from_persona(b"ScholarBot"), "ScholarBot".to_string(), "Research and analysis".to_string()),
            (WebID::from_persona(b"MemoryBot"), "MemoryBot".to_string(), "Memory operations".to_string()),
        ]
    }

    #[test]
    fn test_improv_mode_parse() {
        assert_eq!(ImprovMode::parse_mode("freeform"), Some(ImprovMode::Freeform));
        assert_eq!(ImprovMode::parse_mode("curator_led"), Some(ImprovMode::CuratorLed));
        assert_eq!(ImprovMode::parse_mode("curator-led"), Some(ImprovMode::CuratorLed));
        assert_eq!(ImprovMode::parse_mode("round_robin"), Some(ImprovMode::RoundRobin));
        assert_eq!(ImprovMode::parse_mode("invalid"), None);
    }

    #[test]
    fn test_config_defaults() {
        let config = ImprovSessionConfig::default();
        assert_eq!(config.mode, ImprovMode::Freeform);
        assert!((config.participation_threshold - 0.75).abs() < f64::EPSILON);
        assert_eq!(config.max_speakers_per_turn, 3);
    }

    #[test]
    fn test_threshold_clamps() {
        let mut config = ImprovSessionConfig::default();
        config.set_threshold(1.5);
        assert!((config.participation_threshold - 1.0).abs() < f64::EPSILON);
        config.set_threshold(-0.5);
        assert!((config.participation_threshold - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_relevance_json() {
        let config = ImprovSessionConfig::default();
        let webid = WebID::from_persona(b"ScholarBot");
        let j = parse_relevance(&config, &webid, "ScholarBot",
            r#"{"confidence": 0.87, "reason": "I have research on this"}"#);
        assert!((j.confidence - 0.87).abs() < f64::EPSILON);
        assert!(j.should_speak);
    }

    #[test]
    fn test_parse_relevance_below_threshold() {
        let config = ImprovSessionConfig::default();
        let webid = WebID::from_persona(b"Bot");
        let j = parse_relevance(&config, &webid, "Bot",
            r#"{"confidence": 0.5, "reason": "tangential"}"#);
        assert!(!j.should_speak);
    }

    #[test]
    fn test_parse_relevance_markdown_wrapped() {
        let config = ImprovSessionConfig::default();
        let webid = WebID::from_persona(b"Bot");
        let j = parse_relevance(&config, &webid, "Bot",
            "```json\n{\"confidence\": 0.6, \"reason\": \"maybe\"}\n```");
        assert!((j.confidence - 0.6).abs() < f64::EPSILON);
        assert!(!j.should_speak);
    }

    #[tokio::test]
    async fn test_round_robin_all_speak() {
        let config = ImprovSessionConfig {
            mode: ImprovMode::RoundRobin,
            ..Default::default()
        };
        let mock = Arc::new(MockInference::new("My response", 0.9));
        let participants = test_participants();

        let result = improv_turn(&config, &mock, "What do you think?", &participants, &[])
            .await.unwrap();

        assert_eq!(result.responses.len(), 2);
        assert!(result.judgments.iter().all(|j| j.should_speak));
    }

    #[test]
    fn test_filter_speakers_respects_max() {
        let mut config = ImprovSessionConfig::default();
        config.max_speakers_per_turn = 2;
        let judgments = vec![
            RelevanceJudgment { agent_webid: WebID::from_persona(b"a"), agent_name: "a".to_string(), confidence: 0.95, reason: String::new(), should_speak: true },
            RelevanceJudgment { agent_webid: WebID::from_persona(b"b"), agent_name: "b".to_string(), confidence: 0.90, reason: String::new(), should_speak: true },
            RelevanceJudgment { agent_webid: WebID::from_persona(b"c"), agent_name: "c".to_string(), confidence: 0.85, reason: String::new(), should_speak: true },
        ];
        let speakers = filter_speakers(&config, &judgments);
        assert_eq!(speakers.len(), 2);
    }

    #[test]
    fn test_nan_confidence_no_panic() {
        let config = ImprovSessionConfig::default();
        let judgments = vec![
            RelevanceJudgment { agent_webid: WebID::from_persona(b"a"), agent_name: "a".to_string(), confidence: f64::NAN, reason: String::new(), should_speak: true },
            RelevanceJudgment { agent_webid: WebID::from_persona(b"b"), agent_name: "b".to_string(), confidence: 0.9, reason: String::new(), should_speak: true },
        ];
        let speakers = filter_speakers(&config, &judgments);
        assert_eq!(speakers.len(), 2);
    }
}
