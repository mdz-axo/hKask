//! Goal memory — Records goal experiences to agent memory
//!
//! Goals are transient coordination substrates.
//! Long-term retention lives in agent memory:
//! - Semantic: factual data about the goal (goal_semantic_memory table)
//! - Episodic: first-person experience of working toward the goal (goal_episodic_memory table)

use hkask_types::goal::{Goal, GoalArtifact, GoalID};
use hkask_types::id::WebID;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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

/// Goal memory manager — SQLite-backed persistence
///
/// Stores goal memories in `goal_semantic_memory` and `goal_episodic_memory` tables.
pub struct GoalMemory {
    agent_webid: WebID,
    conn: Arc<Mutex<Connection>>,
}

impl GoalMemory {
    pub fn new(agent_webid: WebID, conn: Arc<Mutex<Connection>>) -> Self {
        Self { agent_webid, conn }
    }

    /// Record goal completion to semantic memory
    pub fn record_semantic(
        &self,
        goal: &Goal,
        artifacts: &[GoalArtifact],
    ) -> Result<GoalSemanticMemory, MemoryError> {
        let memory = GoalSemanticMemory::from_goal(goal, artifacts.len());
        let json = serde_json::to_string(&memory)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let id = format!("gsm_{}", uuid::Uuid::new_v4().simple());

        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        conn.execute(
            "INSERT INTO goal_semantic_memory (id, webid, goal_id, memory_json) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, memory.webid.to_string(), memory.goal_id.to_string(), json],
        )?;
        Ok(memory)
    }

    /// Record goal experience to episodic memory
    pub fn record_episodic(
        &self,
        goal_id: GoalID,
        outcome_summary: &str,
        lessons_learned: Vec<String>,
    ) -> Result<GoalEpisodicMemory, MemoryError> {
        let memory =
            GoalEpisodicMemory::new(goal_id, self.agent_webid, outcome_summary, lessons_learned);
        let json = serde_json::to_string(&memory)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let id = format!("gem_{}", uuid::Uuid::new_v4().simple());

        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        conn.execute(
            "INSERT INTO goal_episodic_memory (id, webid, goal_id, memory_json) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, memory.webid.to_string(), memory.goal_id.to_string(), json],
        )?;
        Ok(memory)
    }

    /// Record complete goal memory (both semantic and episodic)
    pub fn record_goal_completion(
        &self,
        goal: &Goal,
        artifacts: &[GoalArtifact],
        outcome_summary: &str,
        lessons_learned: Vec<String>,
    ) -> Result<(GoalSemanticMemory, GoalEpisodicMemory), MemoryError> {
        let semantic = self.record_semantic(goal, artifacts)?;
        let episodic = self.record_episodic(goal.id, outcome_summary, lessons_learned)?;
        Ok((semantic, episodic))
    }

    /// Recall goal semantic memory
    pub fn recall_goal_semantic(
        &self,
        goal_id: GoalID,
    ) -> Result<Option<GoalSemanticMemory>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT memory_json FROM goal_semantic_memory WHERE goal_id = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(rusqlite::params![goal_id.to_string()])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let memory: GoalSemanticMemory = serde_json::from_str(&json)
                .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
            Ok(Some(memory))
        } else {
            Ok(None)
        }
    }

    /// Recall goal episodic memory
    pub fn recall_goal_experience(
        &self,
        goal_id: GoalID,
    ) -> Result<Option<GoalEpisodicMemory>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT memory_json FROM goal_episodic_memory WHERE goal_id = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(rusqlite::params![goal_id.to_string()])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let memory: GoalEpisodicMemory = serde_json::from_str(&json)
                .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
            Ok(Some(memory))
        } else {
            Ok(None)
        }
    }

    /// List all goal semantic memories for a webid
    pub fn list_goals(&self, webid: WebID) -> Result<Vec<GoalSemanticMemory>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT memory_json FROM goal_semantic_memory WHERE webid = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![webid.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut memories = Vec::new();
        for row in rows {
            let json = row?;
            let memory: GoalSemanticMemory = serde_json::from_str(&json)
                .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
            memories.push(memory);
        }
        Ok(memories)
    }

    /// Store a semantic goal memory
    pub fn store_semantic(&self, memory: GoalSemanticMemory) -> Result<(), MemoryError> {
        let json = serde_json::to_string(&memory)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let id = format!("gsm_{}", uuid::Uuid::new_v4().simple());

        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        conn.execute(
            "INSERT INTO goal_semantic_memory (id, webid, goal_id, memory_json) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, memory.webid.to_string(), memory.goal_id.to_string(), json],
        )?;
        Ok(())
    }

    /// Store an episodic goal memory
    pub fn store_episodic(&self, memory: GoalEpisodicMemory) -> Result<(), MemoryError> {
        let json = serde_json::to_string(&memory)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let id = format!("gem_{}", uuid::Uuid::new_v4().simple());

        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::StorageFailed("Cannot acquire database lock".to_string()))?;
        conn.execute(
            "INSERT INTO goal_episodic_memory (id, webid, goal_id, memory_json) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, memory.webid.to_string(), memory.goal_id.to_string(), json],
        )?;
        Ok(())
    }

    /// Recall semantic goal memory with error handling
    pub fn recall_semantic(&self, goal_id: GoalID) -> Result<GoalSemanticMemory, MemoryError> {
        self.recall_goal_semantic(goal_id)?
            .ok_or_else(|| MemoryError::NotFound(goal_id.to_string()))
    }

    /// Recall episodic goal memory with error handling
    pub fn recall_episodic(&self, goal_id: GoalID) -> Result<GoalEpisodicMemory, MemoryError> {
        self.recall_goal_experience(goal_id)?
            .ok_or_else(|| MemoryError::NotFound(goal_id.to_string()))
    }

    /// List goals with error handling
    pub fn list_goals_result(&self, webid: WebID) -> Result<Vec<GoalSemanticMemory>, MemoryError> {
        self.list_goals(webid)
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

    #[error("Database error: {0}")]
    Database(String),
}

impl From<rusqlite::Error> for MemoryError {
    fn from(e: rusqlite::Error) -> Self {
        MemoryError::Database(e.to_string())
    }
}
