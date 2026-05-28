//! Goal memory — Records goal experiences to agent memory
//!
//! Goals are transient coordination substrates.
//! Long-term retention lives in agent memory:
//! - Semantic: factual data about the goal
//! - Episodic: first-person experience of working toward the goal

use hkask_types::goal::{Goal, GoalArtifact, GoalID};
use hkask_types::id::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Semantic memory of a goal — factual data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSemanticMemory {
    pub goal_id: GoalID,
    pub webid: WebID,
    pub goal_text: String,
    pub completion_state: String,
    pub artifact_count: usize,
    pub created_at: String,
    pub completed_at: Option<String>,
}

impl GoalSemanticMemory {
    pub fn from_goal(goal: &Goal, artifact_count: usize) -> Self {
        Self {
            goal_id: goal.id,
            webid: goal.webid,
            goal_text: goal.text.clone(),
            completion_state: goal.state.as_str().to_string(),
            artifact_count,
            created_at: goal.created_at.to_rfc3339(),
            completed_at: goal.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Episodic memory of a goal — first-person experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalEpisodicMemory {
    pub goal_id: GoalID,
    pub webid: WebID,
    pub experience: String,
    pub outcome_summary: String,
    pub lessons_learned: Vec<String>,
    pub timestamp: String,
}

impl GoalEpisodicMemory {
    pub fn new(
        goal_id: GoalID,
        webid: WebID,
        outcome_summary: &str,
        lessons_learned: Vec<String>,
    ) -> Self {
        Self {
            goal_id,
            webid,
            experience: format!("Working toward goal: {}", goal_id),
            outcome_summary: outcome_summary.to_string(),
            lessons_learned,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Goal memory manager — integrates with hkask-memory
pub struct GoalMemory {
    agent_webid: WebID,
    semantic_store: Arc<RwLock<HashMap<String, GoalSemanticMemory>>>,
    episodic_store: Arc<RwLock<HashMap<String, GoalEpisodicMemory>>>,
}

impl GoalMemory {
    pub fn new(agent_webid: WebID) -> Self {
        Self {
            agent_webid,
            semantic_store: Arc::new(RwLock::new(HashMap::new())),
            episodic_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record goal completion to semantic memory
    pub fn record_semantic(&self, goal: &Goal, artifacts: &[GoalArtifact]) -> GoalSemanticMemory {
        let memory = GoalSemanticMemory::from_goal(goal, artifacts.len());

        if let Ok(mut store) = self.semantic_store.write() {
            store.insert(goal.id.to_string(), memory.clone());
        }

        memory
    }

    /// Record goal experience to episodic memory
    pub fn record_episodic(
        &self,
        goal_id: GoalID,
        outcome_summary: &str,
        lessons_learned: Vec<String>,
    ) -> GoalEpisodicMemory {
        let memory =
            GoalEpisodicMemory::new(goal_id, self.agent_webid, outcome_summary, lessons_learned);

        if let Ok(mut store) = self.episodic_store.write() {
            store.insert(goal_id.to_string(), memory.clone());
        }

        memory
    }

    /// Record complete goal memory (both semantic and episodic)
    pub fn record_goal_completion(
        &self,
        goal: &Goal,
        artifacts: &[GoalArtifact],
        outcome_summary: &str,
        lessons_learned: Vec<String>,
    ) -> (GoalSemanticMemory, GoalEpisodicMemory) {
        let semantic = self.record_semantic(goal, artifacts);
        let episodic = self.record_episodic(goal.id, outcome_summary, lessons_learned);
        (semantic, episodic)
    }

    /// Recall goal experience from memory
    pub fn recall_goal_experience(&self, goal_id: GoalID) -> Option<GoalEpisodicMemory> {
        self.episodic_store
            .read()
            .ok()
            .and_then(|store| store.get(&goal_id.to_string()).cloned())
    }

    /// Recall semantic memory of goal
    pub fn recall_goal_semantic(&self, goal_id: GoalID) -> Option<GoalSemanticMemory> {
        self.semantic_store
            .read()
            .ok()
            .and_then(|store| store.get(&goal_id.to_string()).cloned())
    }

    /// List all goals for a webid
    pub fn list_goals(&self, webid: WebID) -> Vec<GoalSemanticMemory> {
        self.semantic_store
            .read()
            .ok()
            .map(|store| {
                store
                    .values()
                    .filter(|m| m.webid == webid)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Goal memory port — interface for memory operations
pub trait GoalMemoryPort {
    fn store_semantic(&self, memory: GoalSemanticMemory) -> Result<(), MemoryError>;
    fn store_episodic(&self, memory: GoalEpisodicMemory) -> Result<(), MemoryError>;
    fn recall_semantic(&self, goal_id: GoalID) -> Result<GoalSemanticMemory, MemoryError>;
    fn recall_episodic(&self, goal_id: GoalID) -> Result<GoalEpisodicMemory, MemoryError>;
    fn list_goals(&self, webid: WebID) -> Result<Vec<GoalSemanticMemory>, MemoryError>;
}

impl GoalMemoryPort for GoalMemory {
    fn store_semantic(&self, memory: GoalSemanticMemory) -> Result<(), MemoryError> {
        if let Ok(mut store) = self.semantic_store.write() {
            store.insert(memory.goal_id.to_string(), memory);
            Ok(())
        } else {
            Err(MemoryError::StorageFailed(
                "Cannot acquire write lock".to_string(),
            ))
        }
    }

    fn store_episodic(&self, memory: GoalEpisodicMemory) -> Result<(), MemoryError> {
        if let Ok(mut store) = self.episodic_store.write() {
            store.insert(memory.goal_id.to_string(), memory);
            Ok(())
        } else {
            Err(MemoryError::StorageFailed(
                "Cannot acquire write lock".to_string(),
            ))
        }
    }

    fn recall_semantic(&self, goal_id: GoalID) -> Result<GoalSemanticMemory, MemoryError> {
        self.recall_goal_semantic(goal_id)
            .ok_or_else(|| MemoryError::NotFound(goal_id.to_string()))
    }

    fn recall_episodic(&self, goal_id: GoalID) -> Result<GoalEpisodicMemory, MemoryError> {
        self.recall_goal_experience(goal_id)
            .ok_or_else(|| MemoryError::NotFound(goal_id.to_string()))
    }

    fn list_goals(&self, webid: WebID) -> Result<Vec<GoalSemanticMemory>, MemoryError> {
        Ok(self.list_goals(webid))
    }
}

/// Memory error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryError {
    #[error("Memory not found: {0}")]
    NotFound(String),

    #[error("Storage failed: {0}")]
    StorageFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
}
