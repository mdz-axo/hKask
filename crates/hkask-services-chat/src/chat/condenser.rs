//! Auto-condensation of conversation history when approaching context limits.
//!
//! The condenser fetches raw episodes, splits at the saliency midpoint,
//! summarizes the oldest half via an inference call, and returns a rebuilt
//! input with condensed + recent conversation blocks.

use std::sync::Arc;

use hkask_agents::ports::EpisodicStoragePort;
use hkask_capability::DelegationToken;
use hkask_types::DataCategory;
use hkask_types::WebID;
use hkask_types::template::LLMParameters;

use super::types::TurnRequest;
use crate::memory::MemoryService;
use hkask_services_context::AgentService;
use hkask_services_core::ServiceError;

use super::service::ChatService;

/// System prompt for the auto-condense summarization request.
const CONDENSER_SYSTEM_PROMPT: &str = "You are a context condensation assistant. Produce structured summaries that \
     preserve technical details (file paths, error messages, decisions) while \
     eliminating verbosity. Use bullet points. Be concise.";

impl ChatService {
    /// Condense the oldest half of conversation history when approaching context limits.
    ///
    /// Fetches raw episodes, splits at midpoint, summarizes the oldest half via the
    /// inference port, and returns a rebuilt input with `[Condensed history]` +
    /// `[Recent conversation]` blocks. Returns `None` on any failure (graceful
    /// degradation — caller falls back to uncondensed context).
    pub(super) async fn condense_history(
        ctx: &AgentService,
        req: &TurnRequest,
        token: &DelegationToken,
        base_input: &str,
    ) -> Option<String> {
        // \[NORMATIVE\] Sovereignty gate (H3/P2): condensing reads episodic
        // (sovereign) history — only proceed when the owner has granted consent.
        if !MemoryService::has_memory_consent(ctx, &req.agent_webid, &DataCategory::EpisodicMemory)
        {
            return None;
        }
        let episodes = Self::recall_raw_episodes(
            &req.episodic_storage,
            &req.agent_webid,
            token,
            // Fetch enough episodes to cover the saliency window plus padding
            // for condensation. We need at least saliency_window * 2 episodes
            // to keep the anchor, plus the older ones to summarize.
            (req.condense_saliency_window * 4).max(8),
        );
        if episodes.len() < 4 {
            return None; // too few messages to meaningfully condense
        }

        // Saliency-based split: keep the most recent N exchanges verbatim
        // (where N = condense_saliency_window, each exchange = 2 episodes).
        // Older episodes are summarized. This preserves recent context as
        // anchors while condensing stale history.
        let keep_count = (req.condense_saliency_window * 2).min(episodes.len().saturating_sub(2));
        let old_half = &episodes[..episodes.len() - keep_count];
        let recent_half = &episodes[episodes.len() - keep_count..];

        let recent_text = hkask_condenser::inference::format_conversation_text(recent_half);
        let old_text = hkask_condenser::inference::format_conversation_text(old_half);
        let summary_prompt =
            hkask_condenser::inference::build_summarization_prompt(&old_text, &req.input);

        let full_prompt = format!("{CONDENSER_SYSTEM_PROMPT}\n\nUser: {summary_prompt}");

        let condenser_model = req.condenser_model.as_deref().unwrap_or(&req.model);
        let params = LLMParameters {
            temperature: 0.3,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 500,
            seed: None,
            disable_thinking: true,
            adapter: None,
            bypass_fusion: true,
        };

        let port = ctx.infra().inference.clone()?;
        let result = port
            .generate_with_model(&full_prompt, &params, Some(condenser_model), None)
            .await
            .ok()?;

        let summary = result.text;
        if summary.trim().is_empty() {
            return None;
        }

        tracing::debug!(
            target: "cns.chat.condense",
            agent = %req.agent_name,
            old_msgs = old_half.len(),
            recent_msgs = recent_half.len(),
            summary_len = summary.len(),
            "History condensed"
        );

        Some(format!(
            "{base_input}\n\n[Condensed history]\n{summary}\n\n[Recent conversation]\n{recent_text}"
        ))
    }
}
