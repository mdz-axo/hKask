//! Goal storage — SQLite repository for goal persistence
//!
//! Goals are transient coordination substrates.
//! Long-term retention lives in agent memory (episodic/semantic).

use crate::{Store, now_rfc3339};
use chrono::Utc;
use hkask_types::InfrastructureError;
use hkask_types::event::NuEventSink;
use hkask_types::goal::{Goal, GoalArtifact, GoalCriterion, GoalID, GoalState};
use hkask_types::ports::git_cas::{GitCASPort, RepoId};

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
    /// Optional CAS port for write-through.
    cas_port: Option<Arc<dyn GitCASPort>>,
}

impl Store for SqliteGoalRepository {
    fn conn_arc(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    fn lock_conn(
        &self,
    ) -> std::result::Result<std::sync::MutexGuard<'_, Connection>, InfrastructureError> {
        self.conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)
    }
}

impl SqliteGoalRepository {
    /// Create a new goal repository over the given SQLite connection.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            telemetry: None,
            cas_port: None,
        }
    }

    /// Attach a CNS telemetry sink for observability.
    #[must_use = "builder returns the configured repository"]
    pub fn with_telemetry(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Attach a CAS port for write-through. Consumes and returns self.
    #[must_use = "builder returns the configured repository"]
    pub fn with_cas(mut self, port: Arc<dyn GitCASPort>) -> Self {
        self.cas_port = Some(port);
        self
    }

    /// Load a goal by ID for internal use (no authorization gate).
    fn load_goal(&self, goal_id: GoalID) -> Result<Goal> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE id = ?1")?;
        let mut rows = stmt.query([goal_id])?;
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
        let id: GoalID = row.get(0).map_err(|e| corrupt_to_repo_error(0, &e))?;
        let webid: WebID = row.get(1).map_err(|e| corrupt_to_repo_error(1, &e))?;
        let text: String = row.get(2).map_err(|e| corrupt_to_repo_error(2, &e))?;
        let state: GoalState = row.get(3).map_err(|e| corrupt_to_repo_error(3, &e))?;
        let visibility: Visibility = row.get(4).map_err(|e| corrupt_to_repo_error(4, &e))?;
        let created_at: String = row.get(5).map_err(|e| corrupt_to_repo_error(5, &e))?;
        let completed_at: Option<String> = row.get(6).map_err(|e| corrupt_to_repo_error(6, &e))?;
        let parent_goal_id: Option<GoalID> =
            row.get(7).map_err(|e| corrupt_to_repo_error(7, &e))?;
        let depth: i32 = row.get(8).map_err(|e| corrupt_to_repo_error(8, &e))?;
        let display_name: Option<String> = row.get(9).ok().unwrap_or(None);

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
        let id: GoalID = row.get(0)?;
        let webid: WebID = row.get(1)?;
        let text: String = row.get(2)?;
        let state: GoalState = row.get(3)?;
        let visibility: Visibility = row.get(4)?;
        let created_at: String = row.get(5)?;
        let completed_at: Option<String> = row.get(6)?;
        let parent_goal_id: Option<GoalID> = row.get(7)?;
        let depth: i32 = row.get(8)?;
        let display_name: Option<String> = row.get(9).unwrap_or(None);

        // Timestamps require manual parsing — DateTime<Utc> can't impl FromSql
        // here (orphan rule). Corruption must surface as an error, never be
        // silently coerced to a default (which could, e.g., reopen a terminal
        // goal or downgrade visibility).
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
            (goal.id, goal.webid, goal.text.clone(), goal.state, goal.visibility, goal.depth as i32, goal.created_at.to_rfc3339(), goal.display_name.clone()),
        )?;

        Ok(goal)
    }

    /// Create a new goal with CAS write-through: persists to SQLite, then writes to the GoalsSpecs repo.
    pub async fn create_goal_with_cas(
        &self,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal> {
        let goal = self.create_goal(webid, text, visibility)?;
        if let Some(port) = &self.cas_port {
            let bytes = serde_json::to_vec(&goal).map_err(|e| {
                GoalRepositoryError::Infra(InfrastructureError::Serialization(e.to_string()))
            })?;
            port.put_blob(&RepoId::GoalsSpecs, &bytes)
                .await
                .map_err(|e| GoalRepositoryError::Infra(InfrastructureError::Io(e.to_string())))?;
        }
        Ok(goal)
    }

    /// Get a goal by ID.
    pub fn get_goal(&self, goal_id: GoalID) -> Result<Option<Goal>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE id = ?1")?;
        let mut rows = stmt.query([goal_id])?;

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
            Some(now_rfc3339())
        } else {
            None
        };

        let conn = self.lock_conn()?;
        if let Some(completed) = completed_at {
            conn.execute(
                "UPDATE goals SET state = ?1, completed_at = ?2 WHERE id = ?3",
                (state, completed, goal_id),
            )?;
        } else {
            conn.execute(
                "UPDATE goals SET state = ?1 WHERE id = ?2",
                (state, goal_id),
            )?;
        }

        Ok(())
    }

    /// List goals for a WebID, optionally filtered by state.
    pub fn list_goals(&self, webid: &WebID, state_filter: Option<GoalState>) -> Result<Vec<Goal>> {
        let conn = self.lock_conn()?;
        let goals = match state_filter {
            Some(state) => {
                let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE webid = ?1 AND state = ?2 ORDER BY created_at DESC")?;
                collect_rows!(stmt, (webid, state), Self::goal_from_row)
            }
            None => {
                let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE webid = ?1 ORDER BY created_at DESC")?;
                collect_rows!(stmt, [webid], Self::goal_from_row)
            }
        };

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
            (criterion.id, criterion.goal_id, criterion.criterion_type, criterion.description, criterion.satisfied as i32),
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
            (artifact.id, artifact.goal_id, artifact.artifact_ref, artifact.artifact_type, artifact.created_at.to_rfc3339()),
        )?;
        Ok(())
    }

    /// Get criteria for a goal.
    pub fn get_criteria(&self, goal_id: GoalID) -> Result<Vec<GoalCriterion>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, goal_id, type, description, satisfied FROM goal_criteria WHERE goal_id = ?1")?;

        let criteria = collect_rows!(
            stmt,
            [goal_id],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<GoalCriterion> {
                Ok(GoalCriterion {
                    id: row.get(0)?,
                    goal_id: row.get(1)?,
                    criterion_type: row.get(2)?,
                    description: row.get(3)?,
                    satisfied: row.get::<_, i32>(4)? != 0,
                })
            }
        );

        Ok(criteria)
    }

    /// Get artifacts for a goal.
    pub fn get_artifacts(&self, goal_id: GoalID) -> Result<Vec<GoalArtifact>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, goal_id, artifact_ref, artifact_type, created_at FROM goal_artifacts WHERE goal_id = ?1")?;

        let artifacts = collect_rows!(
            stmt,
            [goal_id],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<GoalArtifact> {
                Ok(GoalArtifact {
                    id: row.get(0)?,
                    goal_id: row.get(1)?,
                    artifact_ref: row.get(2)?,
                    artifact_type: row.get(3)?,
                    created_at: {
                        let raw: String = row.get(4)?;
                        chrono::DateTime::parse_from_rfc3339(&raw)
                            .map(|dt| dt.with_timezone(&Utc))
                            .map_err(|_| corrupt_column(4, &raw))?
                    },
                })
            }
        );

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
            (subgoal.id, subgoal.webid, subgoal.text.clone(), subgoal.state, subgoal.visibility, parent_id, subgoal.depth as i32, subgoal.created_at.to_rfc3339(), subgoal.display_name.clone()),
        )?;

        Ok(subgoal)
    }

    /// Get subgoals of a parent goal.
    pub fn get_subgoals(&self, parent_id: GoalID) -> Result<Vec<Goal>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth, display_name FROM goals WHERE parent_goal_id = ?1 ORDER BY created_at ASC")?;
        let subgoals = collect_rows!(stmt, [parent_id], Self::goal_from_row);

        Ok(subgoals)
    }

    /// Delete a goal.
    pub fn delete_goal(&self, goal_id: GoalID) -> Result<()> {
        let _goal = self.load_goal(goal_id)?;

        self.lock_conn()?
            .execute("DELETE FROM goals WHERE id = ?1", [goal_id])?;
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
            rusqlite::params![goal_id, original_data, reason, now_rfc3339()],
        )
        .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;

        // Remove from main goals table
        conn.execute("DELETE FROM goals WHERE id = ?1", [goal_id])
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
            let mut rows = stmt.query([goal_id])?;
            match rows.next()? {
                Some(row) => QuarantinedGoal {
                    id: row.get(0)?,
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
                [goal_id],
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
                    [goal_id],
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
                goal.id,
                goal.webid,
                goal.text,
                goal.state,
                goal.visibility,
                goal.created_at.to_rfc3339(),
                goal.completed_at.map(|dt| dt.to_rfc3339()),
                goal.parent_goal_id,
                goal.depth as i32,
                goal.display_name,
            ],
        )
        .map_err(|e| GoalRepositoryError::QuarantineFailed(e.to_string()))?;

        // Mark the quarantine record as repaired.
        conn.execute(
            "UPDATE quarantined_goals SET repaired = 1, repair_attempts = repair_attempts + 1 WHERE id = ?1",
            [goal_id],
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
        let goals = collect_rows!(
            stmt,
            [],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<QuarantinedGoal> {
                Ok(QuarantinedGoal {
                    id: row.get(0)?,
                    original_data: row.get(1)?,
                    quarantine_reason: row.get(2)?,
                    quarantined_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_default(),
                    repair_attempts: row.get::<_, u32>(4)?,
                    repaired: row.get::<_, i32>(5)? != 0,
                })
            }
        );

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
    fn corrupt_error_variant_carries_reason() {
        let err = GoalRepositoryError::Corrupt("bad state".to_string());
        assert!(format!("{err}").contains("bad state"));
    }

    #[test]
    fn quarantine_failed_error_variant_carries_reason() {
        let err = GoalRepositoryError::QuarantineFailed("disk error".to_string());
        assert!(format!("{err}").contains("disk error"));
    }

    // ── Goal CRUD behavioral tests (P2) ─────────────────────────────────

    // P8 invariant: create_goal persists and retrieves a goal with correct fields
    #[test]
    fn create_goal_roundtrips_through_sqlite() {
        let repo = test_repo();
        let webid = WebID::new();

        let goal = repo
            .create_goal(&webid, "Test goal roundtrip", Visibility::Public)
            .expect("create goal");

        assert_eq!(goal.webid, webid, "webid must round-trip");
        assert_eq!(goal.text, "Test goal roundtrip", "text must round-trip");
        assert_eq!(goal.state, GoalState::Pending, "new goal must be Pending");
        assert_eq!(
            goal.visibility,
            Visibility::Public,
            "visibility must round-trip"
        );
        assert_eq!(goal.depth, 0, "root goal depth must be 0");
        assert!(
            goal.parent_goal_id.is_none(),
            "root goal must have no parent"
        );

        // Verify it persists through SQLite
        let fetched = repo
            .get_goal(goal.id)
            .expect("get goal query")
            .expect("goal exists");
        assert_eq!(fetched.id, goal.id, "id must round-trip");
        assert_eq!(fetched.webid, webid, "webid must round-trip from SQLite");
        assert_eq!(
            fetched.text, "Test goal roundtrip",
            "text must round-trip from SQLite"
        );
        assert_eq!(fetched.state, GoalState::Pending, "state must round-trip");
        assert_eq!(
            fetched.visibility,
            Visibility::Public,
            "visibility must round-trip"
        );
    }

    // P8 invariant: get_goal returns None for nonexistent ID
    #[test]
    fn get_goal_returns_none_for_nonexistent_id() {
        let repo = test_repo();
        let fake_id = GoalID::new();
        let result = repo.get_goal(fake_id).expect("query");
        assert!(result.is_none(), "nonexistent goal must return None");
    }

    // P8 invariant: update_goal_state transitions Pending → Active
    #[test]
    fn update_goal_state_pending_to_active() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "transition test", Visibility::Private)
            .expect("create");

        repo.update_goal_state(goal.id, GoalState::Active)
            .expect("transition");

        let fetched = repo.get_goal(goal.id).expect("get").expect("exists");
        assert_eq!(
            fetched.state,
            GoalState::Active,
            "state must be Active after transition"
        );
        assert!(
            fetched.completed_at.is_none(),
            "Active goal must not have completed_at"
        );
    }

    // P8 invariant: update_goal_state sets completed_at on terminal transition
    #[test]
    fn update_goal_state_sets_completed_at_on_terminal() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "terminal test", Visibility::Private)
            .expect("create");

        // Pending → Active → Completed
        repo.update_goal_state(goal.id, GoalState::Active)
            .expect("activate");
        repo.update_goal_state(goal.id, GoalState::Completed)
            .expect("complete");

        let fetched = repo.get_goal(goal.id).expect("get").expect("exists");
        assert_eq!(fetched.state, GoalState::Completed, "must be Completed");
        assert!(
            fetched.completed_at.is_some(),
            "terminal state must set completed_at"
        );
    }

    // P8 invariant: update_goal_state rejects illegal transition (Pending → Completed)
    #[test]
    fn update_goal_state_rejects_illegal_transition() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "illegal transition", Visibility::Private)
            .expect("create");

        let result = repo.update_goal_state(goal.id, GoalState::Completed);
        assert!(result.is_err(), "Pending -> Completed must fail");
        match result {
            Err(GoalRepositoryError::InvalidTransition(msg)) => {
                assert!(
                    msg.contains("Pending") || msg.contains("pending"),
                    "error must mention source state, got: {}",
                    msg
                );
            }
            Err(other) => panic!("expected InvalidTransition, got: {:?}", other),
            Ok(_) => panic!("expected error, got success"),
        }
    }

    // P8 invariant: update_goal_state rejects transition for nonexistent goal
    #[test]
    fn update_goal_state_rejects_nonexistent_goal() {
        let repo = test_repo();
        let fake_id = GoalID::new();
        let result = repo.update_goal_state(fake_id, GoalState::Active);
        assert!(result.is_err(), "transition on nonexistent goal must fail");
    }

    // P8 invariant: list_goals filters by webid and state
    #[test]
    fn list_goals_filters_by_webid_and_state() {
        let repo = test_repo();
        let webid1 = WebID::new();
        let webid2 = WebID::new();

        repo.create_goal(&webid1, "w1 goal 1", Visibility::Private)
            .expect("create");
        repo.create_goal(&webid1, "w1 goal 2", Visibility::Public)
            .expect("create");
        repo.create_goal(&webid2, "w2 goal 1", Visibility::Private)
            .expect("create");

        // Filter by webid1 only
        let all_w1 = repo.list_goals(&webid1, None).expect("list w1");
        assert_eq!(all_w1.len(), 2, "webid1 must have 2 goals");

        // Filter by webid2 only
        let all_w2 = repo.list_goals(&webid2, None).expect("list w2");
        assert_eq!(all_w2.len(), 1, "webid2 must have 1 goal");

        // Filter by state
        let pending = repo
            .list_goals(&webid1, Some(GoalState::Pending))
            .expect("list pending");
        assert_eq!(pending.len(), 2, "all new goals start as Pending");

        // Empty result for unknown webid
        let unknown = WebID::new();
        let empty = repo.list_goals(&unknown, None).expect("list unknown");
        assert!(empty.is_empty(), "unknown webid must return no goals");
    }

    // P8 invariant: add_criterion persists and retrieves criterion
    #[test]
    fn add_criterion_roundtrips() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "goal with criteria", Visibility::Private)
            .expect("create");

        let criterion = GoalCriterion::new(goal.id, "acceptance", "all tests pass");
        repo.add_criterion(goal.id, criterion)
            .expect("add criterion");

        let criteria = repo.get_criteria(goal.id).expect("get criteria");
        assert_eq!(criteria.len(), 1, "must have 1 criterion");
        assert_eq!(criteria[0].criterion_type, "acceptance");
        assert_eq!(criteria[0].description, "all tests pass");
        assert!(!criteria[0].satisfied, "new criterion must be unsatisfied");
    }

    // P8 invariant: add_criterion rejects mismatched goal_id
    #[test]
    fn add_criterion_rejects_mismatched_goal_id() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "mismatch test", Visibility::Private)
            .expect("create");
        let other_id = GoalID::new();

        let criterion = GoalCriterion::new(other_id, "type", "desc");
        let result = repo.add_criterion(goal.id, criterion);
        assert!(
            result.is_err(),
            "criterion with wrong goal_id must be rejected"
        );
    }

    // P8 invariant: add_artifact persists and retrieves artifact
    #[test]
    fn add_artifact_roundtrips() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "goal with artifact", Visibility::Private)
            .expect("create");

        let artifact = GoalArtifact::new(goal.id, "artifact-ref-001", "test-result");
        repo.add_artifact(goal.id, artifact).expect("add artifact");

        let artifacts = repo.get_artifacts(goal.id).expect("get artifacts");
        assert_eq!(artifacts.len(), 1, "must have 1 artifact");
        assert_eq!(artifacts[0].artifact_ref, "artifact-ref-001");
        assert_eq!(artifacts[0].artifact_type, "test-result");
    }

    // P8 invariant: add_artifact rejects mismatched goal_id
    #[test]
    fn add_artifact_rejects_mismatched_goal_id() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "mismatch test", Visibility::Private)
            .expect("create");
        let other_id = GoalID::new();

        let artifact = GoalArtifact::new(other_id, "ref", "type");
        let result = repo.add_artifact(goal.id, artifact);
        assert!(
            result.is_err(),
            "artifact with wrong goal_id must be rejected"
        );
    }

    // P8 invariant: create_subgoal sets parent and increments depth
    #[test]
    fn create_subgoal_sets_parent_and_depth() {
        let repo = test_repo();
        let webid = WebID::new();
        let parent = repo
            .create_goal(&webid, "parent goal", Visibility::Private)
            .expect("create");

        // Activate parent so it can have subgoals
        repo.update_goal_state(parent.id, GoalState::Active)
            .expect("activate");

        let subgoal = repo
            .create_subgoal(parent.id, &webid, "sub goal", Visibility::Private)
            .expect("subgoal");
        assert_eq!(
            subgoal.parent_goal_id,
            Some(parent.id),
            "subgoal must reference parent"
        );
        assert_eq!(subgoal.depth, 1, "subgoal depth must be parent depth + 1");

        // Verify persisted
        let fetched = repo.get_goal(subgoal.id).expect("get").expect("exists");
        assert_eq!(fetched.parent_goal_id, Some(parent.id));
        assert_eq!(fetched.depth, 1);
    }

    // P8 invariant: create_subgoal rejects nonexistent parent
    #[test]
    fn create_subgoal_rejects_nonexistent_parent() {
        let repo = test_repo();
        let webid = WebID::new();
        let fake_id = GoalID::new();

        let result = repo.create_subgoal(fake_id, &webid, "orphan", Visibility::Private);
        assert!(result.is_err(), "subgoal with nonexistent parent must fail");
        if let Err(GoalRepositoryError::NotFound(msg)) = result {
            assert!(msg.contains("not found"), "error must mention not found");
        }
    }

    // P8 invariant: get_subgoals returns children for a parent
    #[test]
    fn get_subgoals_returns_children() {
        let repo = test_repo();
        let webid = WebID::new();
        let parent = repo
            .create_goal(&webid, "parent", Visibility::Private)
            .expect("create");
        repo.update_goal_state(parent.id, GoalState::Active)
            .expect("activate");

        let sub1 = repo
            .create_subgoal(parent.id, &webid, "sub1", Visibility::Private)
            .expect("sub1");
        let sub2 = repo
            .create_subgoal(parent.id, &webid, "sub2", Visibility::Private)
            .expect("sub2");

        let subgoals = repo.get_subgoals(parent.id).expect("get subgoals");
        assert_eq!(subgoals.len(), 2, "must have 2 subgoals");

        let ids: Vec<GoalID> = subgoals.iter().map(|g| g.id).collect();
        assert!(ids.contains(&sub1.id));
        assert!(ids.contains(&sub2.id));
    }

    // P8 invariant: delete_goal removes the goal
    #[test]
    fn delete_goal_removes_goal() {
        let repo = test_repo();
        let webid = WebID::new();
        let goal = repo
            .create_goal(&webid, "to delete", Visibility::Private)
            .expect("create");

        assert!(
            repo.get_goal(goal.id).expect("get").is_some(),
            "goal must exist before delete"
        );

        repo.delete_goal(goal.id).expect("delete");
        assert!(
            repo.get_goal(goal.id).expect("get").is_none(),
            "goal must not exist after delete"
        );
    }

    // P8 invariant: delete_goal returns NotFound for nonexistent ID
    #[test]
    fn delete_goal_returns_not_found_for_nonexistent() {
        let repo = test_repo();
        let fake_id = GoalID::new();
        let result = repo.delete_goal(fake_id);
        assert!(result.is_err(), "deleting nonexistent goal must fail");
    }

    // P8 invariant: GoalCriterion::new starts unsatisfied with correct prefix
    #[test]
    fn goal_criterion_new_starts_unsatisfied() {
        let goal_id = GoalID::new();
        let criterion = GoalCriterion::new(goal_id, "acceptance", "all tests pass");
        assert!(!criterion.satisfied, "new criterion must start unsatisfied");
        assert!(
            criterion.id.starts_with("gc_"),
            "criterion ID must start with gc_"
        );
        assert_eq!(criterion.criterion_type, "acceptance");
        assert_eq!(criterion.description, "all tests pass");
    }

    // P8 invariant: GoalCriterion::mark_satisfied flips to true
    #[test]
    fn goal_criterion_mark_satisfied_flips_state() {
        let goal_id = GoalID::new();
        let mut criterion = GoalCriterion::new(goal_id, "verification", "code review done");
        assert!(!criterion.satisfied);
        criterion.mark_satisfied();
        assert!(criterion.satisfied, "mark_satisfied must flip to true");
    }

    // P8 invariant: GoalArtifact::new has correct prefix
    #[test]
    fn goal_artifact_new_has_correct_prefix() {
        let goal_id = GoalID::new();
        let artifact = GoalArtifact::new(goal_id, "ref-001", "test-output");
        assert!(
            artifact.id.starts_with("ga_"),
            "artifact ID must start with ga_"
        );
        assert_eq!(artifact.artifact_ref, "ref-001");
        assert_eq!(artifact.artifact_type, "test-output");
    }
}
