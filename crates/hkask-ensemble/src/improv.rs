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

// ── Config types ─────────────────────────────────────────────────────────

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

// ── Domain types ─────────────────────────────────────────────────────────

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

// ── Main turn orchestrator ───────────────────────────────────────────────

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
        ImprovMode::CuratorLed => make_judgments(participants, true),
        ImprovMode::RoundRobin => make_judgments(participants, false),
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

// ── Relevance check (Freeform) ───────────────────────────────────────────

async fn relevance_check<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    participants: &[(WebID, String, String)],
    chat_history: &[(WebID, String)],
) -> Result<Vec<RelevanceJudgment>, ImprovError<C::Error>> {
    let context_str = format_context_with_earlier(config, chat_history, &[]);
    let mut judgments = Vec::new();
    for (webid, name, description) in participants {
        let prompt = format!(
            "You are {} ({}).\nThe following message was sent in a group conversation:\n\n\"{}\"\n\n{}\n\n\
             Do you have something unique and valuable to contribute to this discussion?\n\
             Rate your relevance confidence from 0.0 to 1.0.\n\
             Respond with ONLY a JSON object: {{\"confidence\": <float>, \"reason\": \"<brief explanation>\"}}",
            name,
            description,
            user_message,
            if context_str.is_empty() {
                String::new()
            } else {
                format!("Recent context:\n{context_str}")
            }
        );
        let response = inference_client
            .generate(&GenerateRequest {
                model: config.relevance_model.clone(),
                prompt,
                options: Some(GenerateOptions {
                    n_probs: None,
                    temperature: Some(0.3),
                    max_tokens: Some(config.relevance_max_tokens),
                }),
            })
            .await
            .map_err(ImprovError::Inference)?;
        judgments.push(parse_relevance(config, webid, name, &response.response));
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
            (
                parsed
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                parsed
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no reason given")
                    .to_string(),
            )
        } else {
            (
                extract_first_float(cleaned).unwrap_or(0.0),
                "parsed from non-JSON response".to_string(),
            )
        };
    RelevanceJudgment {
        agent_webid: *webid,
        agent_name: name.to_string(),
        confidence,
        reason,
        should_speak: confidence >= config.participation_threshold,
    }
}

fn extract_first_float(s: &str) -> Option<f64> {
    s.split_whitespace()
        .find_map(|p| p.parse::<f64>().ok().filter(|f| (0.0..=1.0).contains(f)))
}

// ── Mode helpers ─────────────────────────────────────────────────────────

fn make_judgments(
    participants: &[(WebID, String, String)],
    is_curator: bool,
) -> Vec<RelevanceJudgment> {
    participants
        .iter()
        .map(|(webid, name, desc)| RelevanceJudgment {
            agent_webid: *webid,
            agent_name: name.clone(),
            confidence: 1.0,
            reason: if is_curator {
                format!("Curator selected {name} ({desc})")
            } else {
                format!("round_robin: {name} speaks")
            },
            should_speak: true,
        })
        .collect()
}

// ── Speaker filtering & sequential speak ─────────────────────────────────

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
        let c_str = format_context_with_earlier(config, &turn_context, &responses);
        let prefix = if c_str.is_empty() {
            String::new()
        } else {
            format!("Conversation so far:\n{c_str}\n\n")
        };
        let prompt = format!(
            "{prefix}User: {user_message}\n\nProvide your response as {}:",
            speaker.name
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
        let mut ar = AgentResponse::new(
            speaker.webid,
            response.response.trim().to_string(),
            confidence,
        );
        if let Some(ref cc) = config.confidence_config
            && confidence < cc.threshold
            && let Some(escalated) =
                crate::confidence_router::check_and_escalate(cc, &ar, inference_client, &prompt)
                    .await
        {
            tracing::info!(target: "cns.ensemble.improv", agent = %speaker.webid,
                original_confidence = confidence, escalated_confidence = escalated.confidence,
                "Confidence escalated response");
            ar = escalated;
        }
        turn_context.push((speaker.webid, ar.content.clone()));
        responses.push(ar);
    }
    Ok(responses)
}

// ── Synthesis ────────────────────────────────────────────────────────────

async fn synthesize<C: InferenceClient>(
    config: &ImprovSessionConfig,
    inference_client: &Arc<C>,
    user_message: &str,
    responses: &[AgentResponse],
) -> String {
    let summary = responses.iter().fold(String::new(), |mut acc, r| {
        use std::fmt::Write;
        let _ = writeln!(acc, "[{}]: {}", r.agent_webid, r.content);
        acc
    });
    let request = GenerateRequest {
        model: config.relevance_model.clone(),
        prompt: format!(
            "The user asked: {user_message}\n\nMultiple agents responded:\n{summary}\n\
             Provide a brief synthesis that highlights key insights and any disagreements:"
        ),
        options: Some(GenerateOptions {
            n_probs: None,
            temperature: Some(0.5),
            max_tokens: Some(256),
        }),
    };
    match inference_client.generate(&request).await {
        Ok(r) => r.response.trim().to_string(),
        Err(_) => responses
            .iter()
            .fold(String::from("Synthesis: "), |mut acc, r| {
                use std::fmt::Write;
                let _ = write!(acc, "[{}]: {}; ", r.agent_webid, r.content);
                acc
            }),
    }
}

// ── Context formatting ───────────────────────────────────────────────────

fn format_context_with_earlier(
    config: &ImprovSessionConfig,
    previous_turns: &[(WebID, String)],
    earlier_responses: &[AgentResponse],
) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(previous_turns.len() + earlier_responses.len());
    let window = previous_turns.len().saturating_sub(config.context_window);
    for (_, content) in &previous_turns[window..] {
        parts.push(content.clone());
    }
    for resp in earlier_responses {
        parts.push(format!("[{}]: {}", resp.agent_webid, resp.content));
    }
    parts.join("\n")
}
