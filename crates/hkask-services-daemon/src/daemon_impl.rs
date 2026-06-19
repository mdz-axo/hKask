//! DaemonHandler implementation — bridges the Unix socket daemon to hKask's
//! PodManager, UserStore, memory infrastructure, and internal narrative generation.
//!
//! This is the hKask-side implementation of the `DaemonHandler` trait defined
//! in `hkask-mcp`. It wires daemon queries to the live agent and memory stack.
//!
//! # Narrative Generation
//!
//! When an agent is in server mode, tool calls accumulate as episodic experiences.
//! Every N experiences (NARRATIVE_THRESHOLD), the handler triggers internal narrative
//! generation: it queries the agent's recent episodic memories, calls inference to
//! produce observations about patterns and user intent, and stores those observations
//! as additional episodic memories. This is how the agent "thinks about" what it's
//! observing in the MCP session — the same way a chat-mode agent thinks about
//! conversation turns.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use hkask_agents::pod::PodManager;
use hkask_mcp::daemon::DaemonHandler;
use hkask_storage::user_store::UserStore;
use hkask_types::ports::InferencePort;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;

/// Number of experiences before triggering internal narrative generation.
const NARRATIVE_THRESHOLD: usize = 10;

/// System prompt for narrative generation — the agent reflects on what it's observing.
const NARRATIVE_SYSTEM_PROMPT: &str = "You are an observant agent monitoring an MCP tool session. \
     Below is a log of recent tool calls made through the session. \
     Analyze the log and generate 2-3 concise observations about: \
     patterns in tool usage, what the user seems to be trying to accomplish, \
     notable events or anomalies. \
     Format each observation as a single sentence on its own line. \
     Be specific — reference actual tool names and patterns from the log. \
     Do not repeat the log content verbatim; synthesize insights.";

/// hKask-side implementation of the daemon handler trait.
///
/// Wraps PodManager for assignment/capability/memory queries,
/// UserStore for authentication, and InferencePort for narrative generation.
pub struct ServiceDaemonHandler {
    pod_manager: Arc<PodManager>,
    user_store: Arc<std::sync::Mutex<UserStore>>,
    /// Inference port for narrative generation (None if inference unavailable)
    inference_port: Option<Arc<dyn InferencePort>>,
    /// Per-replicant counter of stored experiences (triggers narrative generation)
    experience_counts: Mutex<HashMap<String, usize>>,
}

impl ServiceDaemonHandler {
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  pod_manager must be a valid Arc<PodManager>; user_store must be a valid Arc<Mutex<UserStore>>
    /// post: returns ServiceDaemonHandler with all fields initialized; inference_port may be None
    pub fn new(
        pod_manager: Arc<PodManager>,
        user_store: Arc<std::sync::Mutex<UserStore>>,
        inference_port: Option<Arc<dyn InferencePort>>,
    ) -> Self {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.daemon", operation = "new_handler", has_inference = inference_port.is_some(), "CNS");

        Self {
            pod_manager,
            user_store,
            inference_port,
            experience_counts: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl DaemonHandler for ServiceDaemonHandler {
    async fn check_auth(&self, replicant: &str) -> (bool, Option<String>) {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.daemon", operation = "check_auth", replicant = %replicant, "CNS");

        let has_sessions = {
            let store = match self.user_store.lock() {
                Ok(s) => s,
                Err(_) => {
                    tracing::error!(target: "hkask.daemon", "UserStore lock poisoned");
                    return (false, None);
                }
            };
            let exists = store.get_replicant(replicant).is_ok();
            if !exists {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, "Replicant not found in user store");
                return (false, None);
            }
            let sessions = store.list_sessions(replicant).unwrap_or_default();
            !sessions.is_empty()
        };

        if !has_sessions {
            tracing::debug!(target: "hkask.daemon", replicant = %replicant, "No active sessions — needs passphrase");
            return (false, None);
        }

        tracing::debug!(target: "hkask.daemon", replicant = %replicant, "Replicant has active sessions");
        if let Some(pod_id) = self.pod_manager.find_by_name(replicant).await {
            let webid = self.pod_manager.webid(&pod_id).await;
            (true, webid.map(|w| w.to_string()))
        } else {
            (false, None)
        }
    }

    async fn check_assignment(&self, replicant: &str, role: &str) -> bool {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.daemon", operation = "check_assignment", replicant = %replicant, role = %role, "CNS");

        match self.pod_manager.find_by_name(replicant).await {
            Some(pod_id) => {
                let assigned = self.pod_manager.has_role(&pod_id, role).await;
                tracing::debug!(target: "hkask.daemon", replicant = %replicant, role = %role, assigned = assigned, "Assignment check");
                assigned
            }
            None => {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, "Pod not found for assignment check");
                false
            }
        }
    }

    async fn check_capability(&self, replicant: &str, tool: &str) -> bool {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.daemon", operation = "check_capability", replicant = %replicant, tool = %tool, "CNS");

        match self.pod_manager.find_by_name(replicant).await {
            Some(pod_id) => {
                let granted = self.pod_manager.has_capability(&pod_id, tool).await;
                tracing::debug!(target: "hkask.daemon", replicant = %replicant, tool = %tool, granted = granted, "Capability check");
                granted
            }
            None => false,
        }
    }

    async fn store_experience(
        &self,
        replicant: &str,
        entity: &str,
        attribute: &str,
        value: &serde_json::Value,
        confidence: Option<f64>,
    ) -> (bool, Option<String>, Option<String>) {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.daemon", operation = "store_experience", replicant = %replicant, entity = %entity, attribute = %attribute, confidence = ?confidence, "CNS");

        let pod_id = match self.pod_manager.find_by_name(replicant).await {
            Some(id) => id,
            None => {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, "Pod not found for store_experience");
                return (false, None, None);
            }
        };

        let ctx = match self.pod_manager.context(&pod_id).await {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, error = %e, "Failed to create PodContext");
                return (false, None, None);
            }
        };

        let conf = confidence.unwrap_or(0.85);

        let episodic_result = ctx.store_episodic(
            entity,
            attribute,
            value.clone(),
            hkask_types::Confidence::new(conf),
        );

        let semantic_value = generalize_value(value);
        let semantic_result = ctx.store_semantic(
            entity,
            attribute,
            semantic_value,
            hkask_types::Confidence::new(conf),
        );

        let result = match (episodic_result, semantic_result) {
            (Ok(ep_id), Ok(sem_id)) => {
                tracing::debug!(target: "hkask.daemon", replicant = %replicant, episodic_id = %ep_id, semantic_id = %sem_id, "Dual-encoded experience");
                (true, Some(ep_id), Some(sem_id))
            }
            (Ok(ep_id), Err(e)) => {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, episodic_id = %ep_id, semantic_error = %e, "Episodic stored, semantic failed");
                (true, Some(ep_id), None)
            }
            (Err(e), _) => {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, error = %e, "Failed to store episodic experience");
                (false, None, None)
            }
        };

        // Check if we should trigger narrative generation
        if result.0
            && let Some(inference_port) = self.inference_port.as_ref()
        {
            let count = {
                let mut counts = self
                    .experience_counts
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let c = counts.entry(replicant.to_string()).or_insert(0);
                *c += 1;
                *c
            };

            if count % NARRATIVE_THRESHOLD == 0 {
                tracing::info!(target: "hkask.daemon.narrative", replicant = %replicant, count = count, "Triggering narrative generation");
                let pod_manager = Arc::clone(&self.pod_manager);
                let inference = Arc::clone(inference_port);
                let replicant_name = replicant.to_string();
                tokio::spawn(async move {
                    generate_narrative(&pod_manager, &*inference, &replicant_name).await;
                });
            }
        }

        result
    }

    async fn dispatch_tool(
        &self,
        replicant: &str,
        tool: &str,
        input: &serde_json::Value,
    ) -> (bool, Option<serde_json::Value>, Option<String>) {
        // contract: P9-CNS-SVC-001
        // expect: "The service layer provides CNS health and regulation queries" [P9]
        // P9: CNS span
        tracing::info!(target: "cns.daemon", operation = "dispatch_tool", replicant = %replicant, tool = %tool, "CNS");

        let pod_id = match self.pod_manager.find_by_name(replicant).await {
            Some(id) => id,
            None => {
                return (false, None, Some("Pod not found".into()));
            }
        };

        let ctx = match self.pod_manager.context(&pod_id).await {
            Ok(ctx) => ctx,
            Err(e) => {
                return (false, None, Some(format!("PodContext error: {}", e)));
            }
        };

        match ctx.invoke_tool(tool, input.clone()) {
            Ok(output) => (true, Some(output), None),
            Err(e) => (false, None, Some(e.to_string())),
        }
    }
}

/// Generate internal narrative observations from recent session experiences.
///
/// Queries the agent's episodic memory for recent "mcp_session" triples,
/// formats them as a log, calls inference to produce observations, and
/// stores those observations as new episodic memories.
async fn generate_narrative(
    pod_manager: &PodManager,
    inference: &dyn InferencePort,
    replicant: &str,
) {
    // contract: P9-CNS-SVC-001
    // expect: "The service layer provides CNS health and regulation queries" [P9]
    // P9: CNS span
    tracing::info!(target: "cns.daemon", operation = "generate_narrative", replicant = %replicant, "CNS");

    let pod_id = match pod_manager.find_by_name(replicant).await {
        Some(id) => id,
        None => {
            tracing::warn!(target: "hkask.daemon.narrative", replicant = %replicant, "Pod not found for narrative generation");
            return;
        }
    };

    let ctx = match pod_manager.context(&pod_id).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::warn!(target: "hkask.daemon.narrative", replicant = %replicant, error = %e, "Failed to create PodContext for narrative");
            return;
        }
    };

    // Query recent episodic memories for this session
    let episodes = match ctx.recall_episodic("mcp_session") {
        Ok(eps) => eps,
        Err(e) => {
            tracing::warn!(target: "hkask.daemon.narrative", replicant = %replicant, error = %e, "Failed to recall episodic memories");
            return;
        }
    };

    if episodes.is_empty() {
        return;
    }

    // Build a log summary from recent experiences (last 20 max)
    let recent: Vec<_> = episodes.iter().rev().take(20).collect();
    let mut log_lines = Vec::new();
    for ep in recent.iter().rev() {
        let val = &ep.value;
        if let Some(tool) = val.get("tool").and_then(|v| v.as_str()) {
            let input = val.get("input").and_then(|v| v.as_str()).unwrap_or("?");
            let outcome = val.get("outcome").and_then(|v| v.as_str()).unwrap_or("?");
            log_lines.push(format!(
                "- {}: input='{}', outcome={}",
                tool, input, outcome
            ));
        }
    }

    if log_lines.is_empty() {
        return;
    }

    let session_log = log_lines.join("\n");
    let prompt = format!(
        "{}\n\nRecent MCP session activity for replicant '{}':\n{}",
        NARRATIVE_SYSTEM_PROMPT, replicant, session_log
    );

    // Call inference
    let params = LLMParameters {
        temperature: 0.7,
        max_tokens: 256,
        ..LLMParameters::default()
    };

    let inference_result = match inference.generate(&prompt, &params).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "hkask.daemon.narrative", replicant = %replicant, error = %e, "Inference failed for narrative generation");
            return;
        }
    };

    // Parse observations (one per line)
    let observations: Vec<&str> = inference_result
        .text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('-') && !l.starts_with('*'))
        .collect();

    if observations.is_empty() {
        // If no clean lines, use the whole response as one observation
        let trimmed = inference_result.text.trim();
        if !trimmed.is_empty() {
            let _ = ctx.store_episodic(
                "narrative",
                "thought",
                serde_json::json!({"observation": trimmed, "timestamp": now_rfc3339()}),
                hkask_types::Confidence::new(0.7),
            );
        }
        return;
    }

    // Store each observation as an episodic memory
    let obs_count = observations.len();
    for obs in observations {
        let value = serde_json::json!({
            "observation": obs,
            "source": "internal_narrative",
            "triggered_by": format!("{} experiences", recent.len()),
            "timestamp": now_rfc3339(),
        });

        match ctx.store_episodic(
            "narrative",
            "thought",
            value,
            hkask_types::Confidence::new(0.7),
        ) {
            Ok(id) => {
                tracing::debug!(target: "hkask.daemon.narrative", replicant = %replicant, triple_id = %id, observation = %obs, "Narrative observation stored");
            }
            Err(e) => {
                tracing::warn!(target: "hkask.daemon.narrative", replicant = %replicant, error = %e, "Failed to store narrative observation");
            }
        }
    }

    tracing::info!(target: "hkask.daemon.narrative", replicant = %replicant, observation_count = obs_count, "Narrative generation complete");
}

/// Generalize a value for semantic memory by stripping caller-specific details.
fn generalize_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut generalized = serde_json::Map::new();
            if let Some(tool) = map.get("tool") {
                generalized.insert("tool".to_string(), tool.clone());
            }
            if let Some(outcome) = map.get("outcome") {
                generalized.insert("outcome".to_string(), outcome.clone());
            }
            generalized.insert("generalized".to_string(), serde_json::Value::Bool(true));
            serde_json::Value::Object(generalized)
        }
        other => other.clone(),
    }
}
