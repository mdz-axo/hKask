//! Memory service — semantic recall, episodic recall, episodic storage, and paired recall.
//!
//! Extracted from ChatService to bring it within the 7-function public surface limit (P5).
//! Memory operations are a separate concern from inference and prompt composition.

use std::sync::Arc;

use hkask_agents::ports::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
use hkask_capability::DelegationToken;
use hkask_types::{Confidence, DataCategory, WebID};

use hkask_services_context::AgentService;

pub struct MemoryService;

impl MemoryService {
    /// Check whether the owner has consent for a specific data category.
    ///
    /// \[NORMATIVE\] P1 User Sovereignty / P2 Affirmative Consent.
    /// Fails closed: no consent => no sovereign memory access.
    #[must_use]
    pub fn has_memory_consent(ctx: &AgentService, owner: &WebID, category: &DataCategory) -> bool {
        ctx.governance()
            .consent
            .has_consent(&owner.to_string(), category)
            .unwrap_or(false)
    }

    /// Recall semantic memory triples relevant to the input.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence.
    #[must_use]
    pub fn recall_semantic(
        semantic_port: &Arc<dyn SemanticStoragePort>,
        input: &str,
        token: &DelegationToken,
    ) -> Option<String> {
        let request = RecallRequest::semantic(input, token.clone());
        let triples = match semantic_port.recall_semantic(&request) {
            Ok(t) if !t.is_empty() => t,
            _ => return None,
        };
        let context: Vec<String> = triples
            .iter()
            .filter_map(|t: &RecalledSemantic| t.value.as_str().map(|s| s.to_string()))
            .collect();
        let context: Vec<String> = context.into_iter().take(10).collect();
        if context.is_empty() {
            None
        } else {
            Some(context.join("\n"))
        }
    }

    /// Recall episodic memories relevant to the input, sorted by salience.
    ///
    /// Mirrors `recall_semantic`: same return type, same take-top-N pattern.
    #[must_use]
    pub fn recall_episodic(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        input: &str,
        agent_webid: &WebID,
        token: &DelegationToken,
    ) -> Option<String> {
        let request = RecallRequest::episodic("chatted", *agent_webid, token.clone());
        let episodes: Vec<RecalledEpisode> = match episodic_port.recall_episodic(&request) {
            Ok(v) if !v.is_empty() => v,
            _ => return None,
        };
        let input_lower = input.to_lowercase();
        let keywords: Vec<&str> = input_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();
        let mut scored: Vec<(usize, String)> = episodes
            .iter()
            .filter_map(|e| {
                let v = e.value.as_object()?;
                let ui = v.get("user_input")?.as_str()?;
                let ar = v.get("agent_response")?.as_str()?;
                let combined = format!("{} {}", ui.to_lowercase(), ar.to_lowercase());
                let score = keywords.iter().filter(|kw| combined.contains(*kw)).count();
                Some((score, format!("User: {}\nAgent: {}", ui, ar)))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        let top: Vec<String> = scored.into_iter().take(10).map(|(_, text)| text).collect();
        if top.is_empty() {
            None
        } else {
            Some(top.join("\n\n"))
        }
    }

    /// Recall recent chat turns as formatted history context.
    #[must_use]
    pub fn recall_recent_turns(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        agent_webid: &WebID,
        token: &DelegationToken,
        limit: usize,
    ) -> Option<String> {
        let request = RecallRequest::episodic("chatted", *agent_webid, token.clone());
        let episodes: Vec<RecalledEpisode> = match episodic_port.recall_episodic(&request) {
            Ok(v) if !v.is_empty() => v,
            _ => return None,
        };
        let recent: Vec<String> = episodes
            .iter()
            .rev()
            .take(limit)
            .filter_map(|e| {
                let v = e.value.as_object()?;
                let input = v.get("user_input")?.as_str()?;
                let response = v.get("agent_response")?.as_str()?;
                Some(format!("User: {}\nAgent: {}", input, response))
            })
            .collect();
        if recent.is_empty() {
            None
        } else {
            let formatted = recent.into_iter().rev().collect::<Vec<_>>().join("\n\n");
            Some(format!(
                "[Previous conversation]\n{}\n[/Previous conversation]\n\n",
                formatted
            ))
        }
    }

    /// Store the chat exchange as an episodic triple.
    pub fn store_episodic(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        input: &str,
        response: &str,
        agent_webid: WebID,
        token: &DelegationToken,
        agent_name: &str,
    ) {
        let request = StorageRequest::episodic(
            "chatted",
            "chat_turn",
            serde_json::json!({
                "user_input": input,
                "agent_response": response,
            }),
            Confidence::new(0.7),
            agent_webid,
        );
        match episodic_port.store_episodic(request, token) {
            Ok(_) => {
                tracing::debug!(
                    target: "hkask.chat.memory",
                    agent = %agent_name,
                    "Episodic trace stored"
                );
            }
            Err(e) => {
                tracing::debug!(
                    target: "hkask.chat.memory",
                    agent = %agent_name,
                    error = %e,
                    "Episodic storage failed — response still returned"
                );
            }
        }
    }

    /// Recall raw episodes for condensation.
    #[allow(dead_code)]
    pub(crate) fn recall_raw_episodes(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        agent_webid: &WebID,
        token: &DelegationToken,
        limit: usize,
    ) -> Vec<serde_json::Value> {
        let request = RecallRequest::episodic("chatted", *agent_webid, token.clone());
        let episodes: Vec<RecalledEpisode> = match episodic_port.recall_episodic(&request) {
            Ok(v) if !v.is_empty() => v,
            _ => return vec![],
        };
        let mut messages: Vec<serde_json::Value> = Vec::new();
        for e in episodes.iter().rev().take(limit) {
            if let Some(v) = e.value.as_object() {
                if let Some(input) = v.get("user_input").and_then(|s| s.as_str()) {
                    messages.push(serde_json::json!({"role": "user", "content": input}));
                }
                if let Some(response) = v.get("agent_response").and_then(|s| s.as_str()) {
                    messages.push(serde_json::json!({"role": "assistant", "content": response}));
                }
            }
        }
        messages.reverse();
        messages
    }

    /// Paired memory recall — returns merged context from both stores.
    #[must_use]
    pub fn recall_memory(
        semantic_port: &Arc<dyn SemanticStoragePort>,
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        input: &str,
        agent_webid: &WebID,
        token: &DelegationToken,
    ) -> Option<String> {
        let semantic = Self::recall_semantic(semantic_port, input, token);
        let episodic = Self::recall_episodic(episodic_port, input, agent_webid, token);
        match (semantic, episodic) {
            (Some(s), Some(e)) => Some(format!("{}\n\n{}", s, e)),
            (Some(s), None) => Some(s),
            (None, Some(e)) => Some(e),
            (None, None) => None,
        }
    }
}
