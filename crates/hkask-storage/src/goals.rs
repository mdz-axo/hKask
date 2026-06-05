//! Goal storage — SQLite repository for goal persistence
//!
//! Goals are transient coordination substrates.
//! Long-term retention lives in agent memory (episodic/semantic).

use chrono::Utc;
use hkask_types::InfrastructureError;
use hkask_types::event::NuEventSink;
use hkask_types::goal::{Goal, GoalArtifact, GoalCriterion, GoalID, GoalState};

use hkask_types::id::WebID;
use hkask_types::visibility::Visibility;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoalRepositoryError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("Visibility denied: {0}")]
    VisibilityDenied(String),

    #[error("Goal not found: {0}")]
    NotFound(String),

    #[error("Invalid goal state transition: {0}")]
    InvalidTransition(String),

    #[error("Subgoal depth exceeded: {0}")]
    MaxDepthExceeded(String),

    #[error("Corrupt goal data: {0}")]
    Corrupt(String),

    #[error("Quarantine failed: {0}")]
    QuarantineFailed(String),
}

impl_from_rusqlite!(GoalRepositoryError, Infra);

pub type Result<T> = std::result::Result<T, GoalRepositoryError>;

/// A goal moved to quarantine due to data corruption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuarantinedGoal {
    pub id: GoalID,
    pub original_data: String,
    pub quarantine_reason: String,
    pub quarantined_at: chrono::DateTime<chrono::Utc>,
    pub repair_attempts: u32,
    pub repaired: bool,
}

pub struct SqliteGoalRepository {
    pub(crate) conn: Arc<Mutex<Connection>>,
    /// Optional CNS telemetry sink for observability.
    telemetry: Option<Arc<dyn NuEventSink>>,
}

impl SqliteGoalRepository {
    /// Create a new goal repository over the given SQLite connection.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            telemetry: None,
        }
    }

    /// Attach a CNS telemetry sink for observability.
    #[must_use = "builder returns the configured repository"]
    pub fn with_telemetry(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Acquire the mutex lock on the shared connection.
    ///
    /// Returns `InfrastructureError::LockPoisoned` if another thread
    /// panicked while holding the lock.
    fn lock_conn(
        &self,
    ) -> std::result::Result<std::sync::MutexGuard<'_, Connection>, InfrastructureError> {
        self.conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)
    }

    /// Load a goal by ID for internal use (no authorization gate).
    fn load_goal(&self, goal_id: GoalID) -> Result<Goal> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE id = ?1")?;
        let mut rows = stmt.query([goal_id.to_string()])?;
        match rows.next()? {
            Some(row) => Ok(Self::goal_from_row(row)?),
            None => Err(GoalRepositoryError::NotFound(format!(
                "Goal {} not found",
                goal_id
            ))),
        }
    }

    /// Try to parse a goal row, converting corruption errors to
    /// `GoalRepositoryError::Corrupt` instead of `rusqlite::Error`.
    ///
    /// Use this in code paths that should surface corruption for quarantine
    /// handling. Prefer `goal_from_row` when the caller already maps
    /// rusqlite errors to `GoalRepositoryError::Infra`.
    pub fn try_goal_from_row(
        row: &rusqlite::Row,
    ) -> std::result::Result<Goal, GoalRepositoryError> {
        let id_str: String = row.get(0).map_err(|e| corrupt_to_repo_error(0, &e))?;
        let webid_str: String = row.get(1).map_err(|e| corrupt_to_repo_error(1, &e))?;
        let text: String = row.get(2).map_err(|e| corrupt_to_repo_error(2, &e))?;
        let state_str: String = row.get(3).map_err(|e| corrupt_to_repo_error(3, &e))?;
        let visibility_str: String = row.get(4).map_err(|e| corrupt_to_repo_error(4, &e))?;
        let created_at: String = row.get(5).map_err(|e| corrupt_to_repo_error(5, &e))?;
        let completed_at: Option<String> = row.get(6).map_err(|e| corrupt_to_repo_error(6, &e))?;
        let parent_goal_id: Option<String> =
            row.get(7).map_err(|e| corrupt_to_repo_error(7, &e))?;
        let depth: i32 = row.get(8).map_err(|e| corrupt_to_repo_error(8, &e))?;
        let display_name: Option<String> = row.get(9).ok().unwrap_or(None);

        let id = GoalID::from_string(&id_str);
        let webid = WebID::from_string(&webid_str);
        let state = GoalState::parse_str(&state_str).ok_or_else(|| {
            GoalRepositoryError::Corrupt(format!("unparseable goal state: {state_str:?}"))
        })?;
        let visibility = Visibility::parse_str(&visibility_str).ok_or_else(|| {
            GoalRepositoryError::Corrupt(format!("unparseable visibility: {visibility_str:?}"))
        })?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                GoalRepositoryError::Corrupt(format!("unparseable created_at: {created_at:?}"))
            })?;
        let completed_at = match completed_at {
            Some(s) => Some(
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| {
                        GoalRepositoryError::Corrupt(format!("unparseable completed_at: {s:?}"))
                    })?,
            ),
            None => None,
        };
        let parent_goal_id = parent_goal_id.map(|s| GoalID::from_string(&s));
        let depth = u8::try_from(depth)
            .map_err(|_| GoalRepositoryError::Corrupt(format!("depth out of range: {depth}")))?;

        Ok(Goal {
            id,
            webid,
            text,
            state,
            visibility,
            created_at,
            completed_at,
            parent_goal_id,
            depth,
            display_name,
        })
    }

    pub fn goal_from_row(row: &rusqlite::Row) -> rusqlite::Result<Goal> {
        let id_str: String = row.get(0)?;
        let webid_str: String = row.get(1)?;
        let text: String = row.get(2)?;
        let state_str: String = row.get(3)?;
        let visibility_str: String = row.get(4)?;
        let created_at: String = row.get(5)?;
        let completed_at: Option<String> = row.get(6)?;
        let parent_goal_id: Option<String> = row.get(7)?;
        let depth: i32 = row.get(8)?;
        let display_name: Option<String> = row.get(9).unwrap_or(None);

        let id = GoalID::from_string(&id_str);
        let webid = WebID::from_string(&webid_str);
        // Persisted enum/timestamp columns are authority- and lifecycle-bearing.
        // Corruption must surface as an error, never be silently coerced to a
        // default (which could, e.g., reopen a terminal goal or downgrade
        // visibility). Map parse failures to a real SQLite conversion error so
        // they propagate through `query_map` and become `GoalRepositoryError`.
        let state =
            GoalState::parse_str(&state_str).ok_or_else(|| corrupt_column(3, &state_str))?;
        let visibility = Visibility::parse_str(&visibility_str)
            .ok_or_else(|| corrupt_column(4, &visibility_str))?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| corrupt_column(5, &created_at))?;
        let completed_at = match completed_at {
            Some(s) => Some(
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| corrupt_column(6, &s))?,
            ),
            None => None,
        };
        let parent_goal_id = parent_goal_id.map(|s| GoalID::from_string(&s));
        let depth = u8::try_from(depth).map_err(|_| corrupt_column(8, &depth.to_string()))?;

        Ok(Goal {
            id,
            webid,
            text,
            state,
            visibility,
            created_at,
            completed_at,
            parent_goal_id,
            depth,
            display_name,
        })
    }
}

/// Build a SQLite conversion error describing a corrupt persisted column.
fn corrupt_column(index: usize, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unparseable goal column {index}: {value:?}"),
        )),
    )
}

/// Map a rusqlite row-extraction error to GoalRepositoryError.
///
/// `FromSqlConversionFailure` indicates a persisted value was unparseable
/// (i.e., data corruption), so it maps to `GoalRepositoryError::Corrupt`.
/// All other rusqlite errors (schema mismatch, I/O, etc.) map to
/// `GoalRepositoryError::Infra`.
fn corrupt_to_repo_error(col: usize, err: &rusqlite::Error) -> GoalRepositoryError {
    match err {
        rusqlite::Error::FromSqlConversionFailure(_, _, _) => {
            GoalRepositoryError::Corrupt(format!("column {col}: {err}"))
        }
        other => GoalRepositoryError::Infra(InfrastructureError::Database(other.to_string())),
    }
}

impl SqliteGoalRepository {
    /// Create a new goal.
    pub fn create_goal(&self, webid: &WebID, text: &str, visibility: Visibility) -> Result<Goal> {
        let goal = Goal::new(*webid, text, visibility);

        // Persist created_at explicitly in RFC3339 so it round-trips through
        // the strict reader. The SQLite `datetime('now')` default produces a
        // non-RFC3339 string that the reader (correctly) rejects as corrupt.
        self.lock_conn()?.execute(
            "INSERT INTO goals (id, webid, text, state, visibility, depth, created_at, display_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (goal.id.to_string(), goal.webid.to_string(), goal.text.clone(), goal.state.as_str(), goal.visibility.as_str(), goal.depth as i32, goal.created_at.to_rfc3339(), goal.display_name.clone()),
        )?;

        Ok(goal)
    }

    /// Get a goal by ID.
    pub fn get_goal(&self, goal_id: GoalID) -> Result<Option<Goal>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE id = ?1")?;
        let mut rows = stmt.query([goal_id.to_string()])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::goal_from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Transition a goal to a new state.
    pub fn update_goal_state(&self, goal_id: GoalID, state: GoalState) -> Result<()> {
        let goal = self.load_goal(goal_id)?;

        // Reject illegal lifecycle transitions (e.g. reopening a terminal goal).
        if !goal.state.can_transition_to(state) {
            return Err(GoalRepositoryError::InvalidTransition(format!(
                "{} -> {} is not a legal transition for goal {}",
                goal.state.as_str(),
                state.as_str(),
                goal_id
            )));
        }

        let completed_at = if state.is_terminal() {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        let conn = self.lock_conn()?;
        if let Some(completed) = completed_at {
            conn.execute(
                "UPDATE goals SET state = ?1, completed_at = ?2 WHERE id = ?3",
                (state.as_str(), completed, goal_id.to_string()),
            )?;
        } else {
            conn.execute(
                "UPDATE goals SET state = ?1 WHERE id = ?2",
                (state.as_str(), goal_id.to_string()),
            )?;
        }

        Ok(())
    }

    /// List goals for a WebID, optionally filtered by state.
    pub fn list_goals(&self, webid: &WebID, state_filter: Option<GoalState>) -> Result<Vec<Goal>> {
        let mut goals = Vec::new();

        let conn = self.lock_conn()?;
        match state_filter {
            Some(state) => {
                let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE webid = ?1 AND state = ?2 ORDER BY created_at DESC")?;
                let rows = stmt.query_map((webid.to_string(), state.as_str()), |row| {
                    Self::goal_from_row(row)
                })?;
                for goal in rows.flatten() {
                    goals.push(goal);
                }
            }
            None => {
                let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE webid = ?1 ORDER BY created_at DESC")?;
                let rows = stmt.query_map([webid.to_string()], Self::goal_from_row)?;
                for goal in rows.flatten() {
                    goals.push(goal);
                }
            }
        }

        Ok(goals)
    }

    /// Add a criterion to a goal.
    pub fn add_criterion(&self, goal_id: GoalID, criterion: GoalCriterion) -> Result<()> {
        // The criterion must target the goal named by the caller.
        if criterion.goal_id != goal_id {
            return Err(GoalRepositoryError::InvalidTransition(format!(
                "Criterion targets goal {} but operation named goal {}",
                criterion.goal_id, goal_id
            )));
        }
        let _goal = self.load_goal(goal_id)?;

        self.lock_conn()?.execute(
            "INSERT INTO goal_criteria (id, goal_id, type, description, satisfied) VALUES (?1, ?2, ?3, ?4, ?5)",
            (criterion.id, criterion.goal_id.to_string(), criterion.criterion_type, criterion.description, criterion.satisfied as i32),
        )?;
        Ok(())
    }

    /// Add an artifact to a goal.
    pub fn add_artifact(&self, goal_id: GoalID, artifact: GoalArtifact) -> Result<()> {
        if artifact.goal_id != goal_id {
            return Err(GoalRepositoryError::InvalidTransition(format!(
                "Artifact targets goal {} but operation named goal {}",
                artifact.goal_id, goal_id
            )));
        }
        let _goal = self.load_goal(goal_id)?;

        self.lock_conn()?.execute(
            "INSERT INTO goal_artifacts (id, goal_id, artifact_ref, artifact_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            (artifact.id, artifact.goal_id.to_string(), artifact.artifact_ref, artifact.artifact_type, artifact.created_at.to_rfc3339()),
        )?;
        Ok(())
    }

    /// Get criteria for a goal.
    pub fn get_criteria(&self, goal_id: GoalID) -> Result<Vec<GoalCriterion>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, goal_id, type, description, satisfied FROM goal_criteria WHERE goal_id = ?1")?;
        let rows = stmt.query_map([goal_id.to_string()], |row| {
            Ok(GoalCriterion {
                id: row.get(0)?,
                goal_id: GoalID::from_string(&row.get::<_, String>(1)?),
                criterion_type: row.get(2)?,
                description: row.get(3)?,
                satisfied: row.get::<_, i32>(4)? != 0,
            })
        })?;

        let mut criteria = Vec::new();
        for criterion in rows.flatten() {
            criteria.push(criterion);
        }

        Ok(criteria)
    }

    /// Get artifacts for a goal.
    pub fn get_artifacts(&self, goal_id: GoalID) -> Result<Vec<GoalArtifact>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, goal_id, artifact_ref, artifact_type, created_at FROM goal_artifacts WHERE goal_id = ?1")?;
        let rows = stmt.query_map([goal_id.to_string()], |row| {
            Ok(GoalArtifact {
                id: row.get(0)?,
                goal_id: GoalID::from_string(&row.get::<_, String>(1)?),
                artifact_ref: row.get(2)?,
                artifact_type: row.get(3)?,
                created_at: {
                    let raw: String = row.get(4)?;
                    chrono::DateTime::parse_from_rfc3339(&raw)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|_| corrupt_column(4, &raw))?
                },
            })
        })?;

        let mut artifacts = Vec::new();
        for artifact in rows.flatten() {
            artifacts.push(artifact);
        }

        Ok(artifacts)
    }

    /// Create a subgoal under a parent goal.
    pub fn create_subgoal(
        &self,
        parent_id: GoalID,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal> {
        let parent = self.get_goal(parent_id)?.ok_or_else(|| {
            GoalRepositoryError::NotFound(format!("Parent goal {} not found", parent_id))
        })?;

        if !parent.can_have_subgoals() {
            return Err(GoalRepositoryError::MaxDepthExceeded(format!(
                "Parent goal at depth {} cannot have subgoals",
                parent.depth
            )));
        }

        let subgoal = Goal::new(*webid, text, visibility).with_parent(parent_id, parent.depth);

        self.lock_conn()?.execute(
            "INSERT INTO goals (id, webid, text, state, visibility, parent_goal_id, depth, created_at, display_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (subgoal.id.to_string(), subgoal.webid.to_string(), subgoal.text.clone(), subgoal.state.as_str(), subgoal.visibility.as_str(), parent_id.to_string(), subgoal.depth as i32, subgoal.created_at.to_rfc3339(), subgoal.display_name.clone()),
        )?;

        Ok(subgoal)
    }

    /// Get subgoals of a parent goal.
    pub fn get_subgoals(&self, parent_id: GoalID) -> Result<Vec<Goal>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE parent_goal_id = ?1 ORDER BY created_at ASC")?;
        let rows = stmt.query_map([parent_id.to_string()], Self::goal_from_row)?;

        let mut subgoals = Vec::new();
        for goal in rows.flatten() {
            subgoals.push(goal);
        }

        Ok(subgoals)
    }

    /// Delete a goal.
    pub fn delete_goal(&self, goal_id: GoalID) -> Result<()> {
        let _goal = self.load_goal(goal_id)?;

        self.lock_conn()?
            .execute("DELETE FROM goals WHERE id = ?1", [goal_id.to_string()])?;
        Ok(())
    }

    /// Move a corrupted goal to the quarantine table.
    ///
    /// This removes the goal from the main `goals` table and inserts a forensic
    /// record into `quarantined_goals` for later repair or human review.
    /// The goal's current state is serialized into `original_data` so it can be
    /// restored during repair.
    pub fn quarantine_goal(&self, goal_id: GoalID, reason: &str) -> Result<()> {
        // Load the goal before removing it so we can snapshot its state.
        let goal = self.load_goal(goal_id)?;
        let original_data = serde_json::to_string(&goal).unwrap_or_default();

        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO quarantined_goals (id, original_data, quarantine_reason, quarantined_at, repair_attempts, repaired)
             VALUES (?1, ?2, ?3, ?4, 0, 0)",
            rusqlite::params![goal_id.to_string(), original_data, reason, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;

        // Remove from main goals table
        conn.execute("DELETE FROM goals WHERE id = ?1", [goal_id.to_string()])
            .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;

        Ok(())
    }

    /// Attempt to repair a quarantined goal.
    ///
    /// If `original_data` is present and parseable, re-inserts the goal into
    /// the `goals` table and marks the quarantine record as repaired.
    /// If `original_data` is empty or corrupt (legacy data), increments
    /// `repair_attempts` and returns `false` for Curation/human review.
    ///
    /// TODO: Implement weighted event replay (F.1) to replay NuEvents
    /// since `quarantined_at` against the goal's original state for full
    /// reconstruction, using per-domain decay weights for recency bias.
    pub fn repair_quarantined_goal(
        &self,
        goal_id: GoalID,
        _event_sink: &dyn NuEventSink,
    ) -> Result<bool> {
        // Read the quarantined record to inspect original_data.
        let quarantined = {
            let conn = self.lock_conn()?;
            let mut stmt = conn.prepare(
                "SELECT id, original_data, quarantine_reason, quarantined_at, repair_attempts, repaired FROM quarantined_goals WHERE id = ?1",
            )?;
            let mut rows = stmt.query([goal_id.to_string()])?;
            match rows.next()? {
                Some(row) => QuarantinedGoal {
                    id: GoalID::from_string(&row.get::<_, String>(0)?),
                    original_data: row.get::<_, String>(1)?,
                    quarantine_reason: row.get::<_, String>(2)?,
                    quarantined_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_default(),
                    repair_attempts: row.get::<_, u32>(4)?,
                    repaired: row.get::<_, i32>(5)? != 0,
                },
                None => return Err(GoalRepositoryError::NotFound(goal_id.to_string())),
            }
        };

        // Try to deserialize original_data back into a Goal.
        if quarantined.original_data.is_empty() {
            // Legacy data — no baseline to reconstruct from.
            let conn = self.lock_conn()?;
            conn.execute(
                "UPDATE quarantined_goals SET repair_attempts = repair_attempts + 1 WHERE id = ?1",
                [goal_id.to_string()],
            )
            .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;
            return Ok(false);
        }

        let goal: Goal = match serde_json::from_str(&quarantined.original_data) {
            Ok(g) => g,
            Err(_) => {
                // Corrupt serialized data — cannot reconstruct.
                let conn = self.lock_conn()?;
                conn.execute(
                    "UPDATE quarantined_goals SET repair_attempts = repair_attempts + 1 WHERE id = ?1",
                    [goal_id.to_string()],
                )
                .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;
                return Ok(false);
            }
        };

        // Re-insert the restored goal into the main goals table.
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO goals (id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                goal.id.to_string(),
                goal.webid.to_string(),
                goal.text,
                goal.state.as_str(),
                goal.visibility.as_str(),
                goal.created_at.to_rfc3339(),
                goal.completed_at.map(|dt| dt.to_rfc3339()),
                goal.parent_goal_id.map(|id| id.to_string()),
                goal.depth as i32,
                goal.display_name,
            ],
        )
        .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;

        // Mark the quarantine record as repaired.
        conn.execute(
            "UPDATE quarantined_goals SET repaired = 1, repair_attempts = repair_attempts + 1 WHERE id = ?1",
            [goal_id.to_string()],
        )
        .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;

        Ok(true)
    }

    /// List all quarantined goals, most recent first.
    pub fn list_quarantined_goals(&self) -> Result<Vec<QuarantinedGoal>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, original_data, quarantine_reason, quarantined_at, repair_attempts, repaired FROM quarantined_goals ORDER BY quarantined_at DESC",
        )?;
        let goals = stmt
            .query_map([], |row| {
                Ok(QuarantinedGoal {
                    id: GoalID::from_string(&row.get::<_, String>(0)?),
                    original_data: row.get(1)?,
                    quarantine_reason: row.get(2)?,
                    quarantined_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_default(),
                    repair_attempts: row.get::<_, u32>(4)?,
                    repaired: row.get::<_, i32>(5)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(goals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use hkask_types::NuEvent;
    use hkask_types::id::WebID;
    use hkask_types::visibility::Visibility;

    /// Helper: create an in-memory repository for testing.
    fn test_repo() -> SqliteGoalRepository {
        let db = Database::in_memory().expect("in-memory db");
        SqliteGoalRepository::new(db.conn_arc())
    }

    #[test]
    fn quarantine_goal_moves_goal_out_of_main_table() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "Test goal", Visibility::Private)
            .expect("create goal");

        // Verify goal exists in main table
        assert!(repo.get_goal(goal.id).expect("get goal").is_some());

        // Quarantine the goal
        repo.quarantine_goal(goal.id, "corrupt state detected")
            .expect("quarantine goal");

        // Goal should no longer be in the main table
        assert!(repo.get_goal(goal.id).expect("get goal query").is_none());
    }

    #[test]
    fn list_quarantined_goals_returns_quarantined() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal1 = repo
            .create_goal(&webid, "Goal one", Visibility::Private)
            .expect("create goal 1");
        let goal2 = repo
            .create_goal(&webid, "Goal two", Visibility::Private)
            .expect("create goal 2");

        repo.quarantine_goal(goal1.id, "reason one")
            .expect("quarantine 1");
        repo.quarantine_goal(goal2.id, "reason two")
            .expect("quarantine 2");

        let quarantined = repo.list_quarantined_goals().expect("list quarantined");
        assert_eq!(quarantined.len(), 2);

        // Verify the quarantine records contain the expected data
        let ids: Vec<GoalID> = quarantined.iter().map(|q| q.id).collect();
        assert!(ids.contains(&goal1.id));
        assert!(ids.contains(&goal2.id));

        // Verify reasons are stored
        let q1 = quarantined.iter().find(|q| q.id == goal1.id).unwrap();
        assert_eq!(q1.quarantine_reason, "reason one");
        assert_eq!(q1.repair_attempts, 0);
        assert!(!q1.repaired);
    }

    #[test]
    fn repair_quarantined_goal_restores_from_original_data() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "Goal to repair", Visibility::Private)
            .expect("create goal");

        repo.quarantine_goal(goal.id, "corrupt data")
            .expect("quarantine goal");

        // Create a no-op event sink for the test
        struct NoopSink;
        impl NuEventSink for NoopSink {
            fn persist(
                &self,
                _event: &NuEvent,
            ) -> std::result::Result<(), hkask_types::InfrastructureError> {
                Ok(())
            }
        }
        let sink = NoopSink;

        let repaired = repo
            .repair_quarantined_goal(goal.id, &sink)
            .expect("repair attempt");
        assert!(repaired, "repair with original_data should return true");

        // Verify goal is back in the main goals table
        let restored = repo
            .get_goal(goal.id)
            .expect("get goal")
            .expect("goal should exist");
        assert_eq!(restored.text, "Goal to repair");

        // Verify the quarantine record is marked as repaired
        let quarantined = repo.list_quarantined_goals().expect("list quarantined");
        let q = quarantined.into_iter().find(|q| q.id == goal.id).unwrap();
        assert!(q.repaired);
    }

    #[test]
    fn repair_quarantined_goal_returns_false_for_empty_original_data() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "Legacy goal", Visibility::Private)
            .expect("create goal");
        let goal_id = goal.id;

        // Manually insert a quarantined record with empty original_data (legacy)
        let conn = repo.conn.lock().expect("lock");
        conn.execute(
            "INSERT INTO quarantined_goals (id, original_data, quarantine_reason, quarantined_at, repair_attempts, repaired)
             VALUES (?1, '', ?2, ?3, 0, 0)",
            rusqlite::params![goal_id.to_string(), "legacy corruption", chrono::Utc::now().to_rfc3339()],
        ).expect("insert legacy quarantine");
        // Remove from main table to simulate quarantine
        conn.execute("DELETE FROM goals WHERE id = ?1", [goal_id.to_string()])
            .expect("delete goal");
        drop(conn);

        struct NoopSink;
        impl NuEventSink for NoopSink {
            fn persist(
                &self,
                _event: &NuEvent,
            ) -> std::result::Result<(), hkask_types::InfrastructureError> {
                Ok(())
            }
        }
        let sink = NoopSink;

        let repaired = repo
            .repair_quarantined_goal(goal_id, &sink)
            .expect("repair attempt");
        assert!(
            !repaired,
            "repair with empty original_data should return false"
        );

        // Verify repair_attempts was incremented
        let quarantined = repo.list_quarantined_goals().expect("list quarantined");
        let q = quarantined.into_iter().find(|q| q.id == goal_id).unwrap();
        assert_eq!(q.repair_attempts, 1);
        assert!(!q.repaired);
    }

    #[test]
    fn corrupt_error_variant_carries_reason() {
        let err = GoalRepositoryError::Corrupt("bad state".to_string());
        assert!(format!("{err}").contains("bad state"));
    }

    #[test]
    fn quarantine_failed_error_variant_carries_reason() {
        let err = GoalRepositoryError::QuarantineFailed("disk error".to_string());
        assert!(format!("{err}").contains("disk error"));
    }

    #[test]
    fn try_goal_from_row_rejects_corrupt_state() {
        let db = Database::in_memory().expect("in-memory db");
        let conn = db.conn_arc();
        let c = conn.lock().expect("lock");
        let webid = WebID::new();
        let goal_id = GoalID::new();

        // Insert a goal with a corrupt state value
        c.execute(
            "INSERT INTO goals (id, webid, text, state, visibility, created_at, depth)
             VALUES (?1, ?2, ?3, 'INVALID_STATE', 'private', ?4, 0)",
            rusqlite::params![
                goal_id.to_string(),
                webid.to_string(),
                "corrupt goal",
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .expect("insert corrupt row");

        let mut stmt = c
            .prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE id = ?1")
            .expect("prepare");
        let mut rows = stmt.query([goal_id.to_string()]).expect("query");
        let row = rows.next().expect("row").expect("some row");

        let result = SqliteGoalRepository::try_goal_from_row(row);
        match result {
            Err(GoalRepositoryError::Corrupt(msg)) => {
                assert!(
                    msg.contains("state"),
                    "corrupt error should mention state: {msg}"
                );
            }
            Err(other) => panic!("expected Corrupt error, got: {other}"),
            Ok(_) => panic!("expected error for corrupt state, got a goal"),
        }
    }
}
