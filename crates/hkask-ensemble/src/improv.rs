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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SynthesisMode {
    Always,
    #[default]
    Optional,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovSessionConfig {
    pub mode: ImprovMode,
    pub participation_threshold: f64,
    pub max_speakers_per_turn: usize,
    pub context_window: usize,
    pub relevance_model: String,
    pub relevance_max_tokens: i32,
    pub(crate) synthesis: SynthesisMode,
    /// Confidence configuration for automatic model escalation.
    /// When set, responses below the confidence threshold are re-generated
    /// using a larger model via `check_and_escalate`.
    #[serde(default)]
    pub confidence_config: Option<crate::confidence_router::ConfidenceConfig>,
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
            confidence_config: None,
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

    #[error("Ensemble error: {0}")]
    Ensemble(#[from] EnsembleError),
}

pub(crate) async fn improv_turn<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    participants: &[(WebID, String, String)],
    chat_history: &[(WebID, String)],
) -> Result<ImprovTurn, ImprovError<C::Error>> {
    let judgments = match config.mode {
        ImprovMode::Freeform => {
            relevance_check(
                config,
                inference_client,
                user_message,
                participants,
                chat_history,
            )
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

    let responses = sequential_speak(
        config,
        inference_client,
        user_message,
        &speakers,
        chat_history,
    )
    .await?;

    let curator_synthesis = match config.synthesis {
        SynthesisMode::Always => {
            Some(synthesize(config, inference_client, user_message, &responses).await)
        }
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

fn parse_relevance(
    config: &ImprovSessionConfig,
    webid: &WebID,
    name: &str,
    raw: &str,
) -> RelevanceJudgment {
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
        agent_webid: *webid,
        agent_name: name.to_string(),
        confidence,
        reason,
        should_speak,
    }
}

fn extract_first_float(s: &str) -> Option<f64> {
    for part in s.split_whitespace() {
        if let Ok(f) = part.parse::<f64>()
            && (0.0..=1.0).contains(&f)
        {
            return Some(f);
        }
    }
    None
}

fn curator_selected(participants: &[(WebID, String, String)]) -> Vec<RelevanceJudgment> {
    participants
        .iter()
        .map(|(webid, name, desc)| RelevanceJudgment {
            agent_webid: *webid,
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
            agent_webid: *webid,
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
            webid: j.agent_webid,
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
            prompt: prompt.clone(),
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

        let mut agent_response = AgentResponse::new(
            speaker.webid,
            response.response.trim().to_string(),
            confidence,
        );

        // Confidence-based escalation: if confidence is below threshold,
        // re-generate the response using a larger model
        if let Some(ref conf_config) = config.confidence_config
            && confidence < conf_config.threshold
            && let Some(escalated) = crate::confidence_router::check_and_escalate(
                conf_config,
                &agent_response,
                inference_client,
                &prompt,
            )
            .await
        {
            tracing::info!(
                target: "cns.ensemble.improv",
                agent = %speaker.webid,
                original_confidence = confidence,
                escalated_confidence = escalated.confidence,
                "Confidence escalated response"
            );
            agent_response = escalated;
        }

        turn_context.push((speaker.webid, agent_response.content.clone()));
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
    let window = previous_turns.len().saturating_sub(config.context_window);
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
    use hkask_types::WebID;

    #[test]
    fn improv_mode_default_is_freeform() {
        assert_eq!(ImprovMode::default(), ImprovMode::Freeform);
    }

    #[test]
    fn improv_mode_as_str() {
        assert_eq!(ImprovMode::Freeform.as_str(), "freeform");
        assert_eq!(ImprovMode::CuratorLed.as_str(), "curator_led");
        assert_eq!(ImprovMode::RoundRobin.as_str(), "round_robin");
    }

    #[test]
    fn improv_mode_parse_mode_valid() {
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
        assert_eq!(
            ImprovMode::parse_mode("round-robin"),
            Some(ImprovMode::RoundRobin)
        );
    }

    #[test]
    fn improv_mode_parse_mode_invalid() {
        assert_eq!(ImprovMode::parse_mode("unknown"), None);
        assert_eq!(ImprovMode::parse_mode(""), None);
    }

    #[test]
    fn improv_session_config_default() {
        let config = ImprovSessionConfig::default();
        assert_eq!(config.mode, ImprovMode::Freeform);
        assert!((config.participation_threshold - 0.75).abs() < f64::EPSILON);
        assert_eq!(config.max_speakers_per_turn, 3);
        assert_eq!(config.context_window, 5);
        assert_eq!(config.relevance_model, "qwen3:8b");
        assert_eq!(config.relevance_max_tokens, 100);
        assert!(config.confidence_config.is_none());
    }

    #[test]
    fn improv_session_config_set_threshold_clamps() {
        let mut config = ImprovSessionConfig::default();
        config.set_threshold(1.5);
        assert!((config.participation_threshold - 1.0).abs() < f64::EPSILON);
        config.set_threshold(-0.5);
        assert!((config.participation_threshold - 0.0).abs() < f64::EPSILON);
        config.set_threshold(0.5);
        assert!((config.participation_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn improv_session_config_set_mode() {
        let mut config = ImprovSessionConfig::default();
        config.set_mode(ImprovMode::CuratorLed);
        assert_eq!(config.mode, ImprovMode::CuratorLed);
    }

    #[test]
    fn extract_first_float_finds_valid() {
        assert_eq!(extract_first_float("0.85 confidence"), Some(0.85));
    }

    #[test]
    fn extract_first_float_no_valid() {
        assert_eq!(extract_first_float("no number here"), None);
    }

    #[test]
    fn extract_first_float_out_of_range() {
        assert_eq!(extract_first_float("42.0 is too big"), None);
    }

    #[test]
    fn format_context_empty() {
        let config = ImprovSessionConfig::default();
        let turns: &[(WebID, String)] = &[];
        assert_eq!(format_context(&config, turns), "");
    }

    #[test]
    fn format_context_within_window() {
        let config = ImprovSessionConfig::default(); // context_window = 5
        let turns: Vec<(WebID, String)> = (0..3)
            .map(|i| (WebID::new(), format!("msg{}", i)))
            .collect();
        let result = format_context(&config, &turns);
        assert_eq!(result, "msg0\nmsg1\nmsg2");
    }

    #[test]
    fn format_context_truncates_window() {
        let mut config = ImprovSessionConfig::default();
        config.context_window = 3;
        let turns: Vec<(WebID, String)> = (0..10)
            .map(|i| (WebID::new(), format!("msg{}", i)))
            .collect();
        let result = format_context(&config, &turns);
        assert_eq!(result, "msg7\nmsg8\nmsg9");
    }
}
