//! Goal storage — SQLite repository for goal persistence
//!
//! Goals are transient coordination substrates.
//! Long-term retention lives in agent memory (episodic/semantic).

use chrono::Utc;
use hkask_types::goal::{Goal, GoalArtifact, GoalCriterion, GoalID, GoalState};
use hkask_types::goal_capability::{GoalAccess, GoalCapabilityToken, GoalOp};
use hkask_types::id::WebID;
use hkask_types::visibility::Visibility;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoalRepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    #[error("Visibility denied: {0}")]
    VisibilityDenied(String),

    #[error("Goal not found: {0}")]
    NotFound(String),

    #[error("Invalid goal state transition: {0}")]
    InvalidTransition(String),

    #[error("Subgoal depth exceeded (max 7): {0}")]
    MaxDepthExceeded(String),
}

pub type Result<T> = std::result::Result<T, GoalRepositoryError>;

pub trait GoalRepositoryPort {
    fn create_goal(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal>;
    fn get_goal(&self, token: &GoalCapabilityToken, goal_id: GoalID) -> Result<Option<Goal>>;
    fn update_goal_state(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        state: GoalState,
    ) -> Result<()>;
    fn list_goals(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        state_filter: Option<GoalState>,
    ) -> Result<Vec<Goal>>;
    fn add_criterion(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        criterion: GoalCriterion,
    ) -> Result<()>;
    fn add_artifact(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        artifact: GoalArtifact,
    ) -> Result<()>;
    fn get_criteria(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
    ) -> Result<Vec<GoalCriterion>>;
    fn get_artifacts(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
    ) -> Result<Vec<GoalArtifact>>;
    fn create_subgoal(
        &self,
        token: &GoalCapabilityToken,
        parent_id: GoalID,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal>;
    fn get_subgoals(&self, token: &GoalCapabilityToken, parent_id: GoalID) -> Result<Vec<Goal>>;
    fn delete_goal(&self, token: &GoalCapabilityToken, goal_id: GoalID) -> Result<()>;
}

pub struct SqliteGoalRepository {
    conn: Arc<Mutex<Connection>>,
    capability_secret: Vec<u8>,
}

impl SqliteGoalRepository {
    pub fn new(conn: Arc<Mutex<Connection>>, capability_secret: Vec<u8>) -> Self {
        Self {
            conn,
            capability_secret,
        }
    }

    fn verify_capability(&self, token: &GoalCapabilityToken, required_op: GoalOp) -> Result<()> {
        if !token.is_valid(&self.capability_secret) {
            return Err(GoalRepositoryError::CapabilityDenied(
                "Token invalid or expired".to_string(),
            ));
        }
        if !token.can_perform(required_op, &self.capability_secret) {
            return Err(GoalRepositoryError::CapabilityDenied(format!(
                "Missing capability for operation: {:?}",
                required_op
            )));
        }
        Ok(())
    }

    fn check_visibility_access(&self, goal: &Goal, requester_webid: &WebID) -> Result<()> {
        let access = GoalAccess::check(goal, requester_webid);
        if !access.can_read() {
            return Err(GoalRepositoryError::VisibilityDenied(format!(
                "WebID {} cannot access goal with visibility {:?}",
                requester_webid, goal.visibility
            )));
        }
        Ok(())
    }

    fn goal_from_row(row: &rusqlite::Row) -> rusqlite::Result<Goal> {
        let id_str: String = row.get(0)?;
        let webid_str: String = row.get(1)?;
        let text: String = row.get(2)?;
        let state_str: String = row.get(3)?;
        let visibility_str: String = row.get(4)?;
        let created_at: String = row.get(5)?;
        let completed_at: Option<String> = row.get(6)?;
        let parent_goal_id: Option<String> = row.get(7)?;
        let depth: i32 = row.get(8)?;

        let id = GoalID::from_string(&id_str);
        let webid = WebID::from_string(&webid_str);
        let state = GoalState::parse_str(&state_str).unwrap_or(GoalState::Pending);
        let visibility = Visibility::parse_str(&visibility_str).unwrap_or(Visibility::Private);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let completed_at = completed_at.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });
        let parent_goal_id = parent_goal_id.map(|s| GoalID::from_string(&s));

        Ok(Goal {
            id,
            webid,
            text,
            state,
            visibility,
            created_at,
            completed_at,
            parent_goal_id,
            depth: depth as u8,
        })
    }
}

impl GoalRepositoryPort for SqliteGoalRepository {
    fn create_goal(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal> {
        self.verify_capability(token, GoalOp::Create)?;

        let goal = Goal::new(*webid, text, visibility);

        self.conn.lock().unwrap().execute(
            "INSERT INTO goals (id, webid, text, state, visibility, depth) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (goal.id.to_string(), goal.webid.to_string(), goal.text.clone(), goal.state.as_str(), goal.visibility.as_str(), goal.depth as i32),
        )?;

        Ok(goal)
    }

    fn get_goal(&self, token: &GoalCapabilityToken, goal_id: GoalID) -> Result<Option<Goal>> {
        self.verify_capability(token, GoalOp::Read)?;

        let goal = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth FROM goals WHERE id = ?1")?;
            let mut rows = stmt.query([goal_id.to_string()])?;

            if let Some(row) = rows.next()? {
                Some(Self::goal_from_row(row)?)
            } else {
                None
            }
        };

        if let Some(goal) = &goal {
            self.check_visibility_access(goal, &token.holder_webid)?;
        }

        Ok(goal)
    }

    fn update_goal_state(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        state: GoalState,
    ) -> Result<()> {
        self.verify_capability(token, GoalOp::Update)?;

        let completed_at = if state.is_terminal() {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        let conn = self.conn.lock().unwrap();
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

    fn list_goals(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        state_filter: Option<GoalState>,
    ) -> Result<Vec<Goal>> {
        self.verify_capability(token, GoalOp::Read)?;

        let mut goals = Vec::new();

        let conn = self.conn.lock().unwrap();
        match state_filter {
            Some(state) => {
                let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth FROM goals WHERE webid = ?1 AND state = ?2 ORDER BY created_at DESC")?;
                let rows = stmt.query_map((webid.to_string(), state.as_str()), |row| {
                    Self::goal_from_row(row)
                })?;
                for goal in rows.flatten() {
                    if self
                        .check_visibility_access(&goal, &token.holder_webid)
                        .is_ok()
                    {
                        goals.push(goal);
                    }
                }
            }
            None => {
                let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth FROM goals WHERE webid = ?1 ORDER BY created_at DESC")?;
                let rows = stmt.query_map([webid.to_string()], Self::goal_from_row)?;
                for goal in rows.flatten() {
                    if self
                        .check_visibility_access(&goal, &token.holder_webid)
                        .is_ok()
                    {
                        goals.push(goal);
                    }
                }
            }
        }

        Ok(goals)
    }

    fn add_criterion(
        &self,
        token: &GoalCapabilityToken,
        _goal_id: GoalID,
        criterion: GoalCriterion,
    ) -> Result<()> {
        self.verify_capability(token, GoalOp::Update)?;

        self.conn.lock().unwrap().execute(
            "INSERT INTO goal_criteria (id, goal_id, type, description, satisfied) VALUES (?1, ?2, ?3, ?4, ?5)",
            (criterion.id, criterion.goal_id.to_string(), criterion.criterion_type, criterion.description, criterion.satisfied as i32),
        )?;
        Ok(())
    }

    fn add_artifact(
        &self,
        token: &GoalCapabilityToken,
        _goal_id: GoalID,
        artifact: GoalArtifact,
    ) -> Result<()> {
        self.verify_capability(token, GoalOp::AddArtifact)?;

        self.conn.lock().unwrap().execute(
            "INSERT INTO goal_artifacts (id, goal_id, artifact_ref, artifact_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            (artifact.id, artifact.goal_id.to_string(), artifact.artifact_ref, artifact.artifact_type, artifact.created_at.to_rfc3339()),
        )?;
        Ok(())
    }

    fn get_criteria(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
    ) -> Result<Vec<GoalCriterion>> {
        self.verify_capability(token, GoalOp::Read)?;

        let conn = self.conn.lock().unwrap();
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

    fn get_artifacts(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
    ) -> Result<Vec<GoalArtifact>> {
        self.verify_capability(token, GoalOp::Read)?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, goal_id, artifact_ref, artifact_type, created_at FROM goal_artifacts WHERE goal_id = ?1")?;
        let rows = stmt.query_map([goal_id.to_string()], |row| {
            Ok(GoalArtifact {
                id: row.get(0)?,
                goal_id: GoalID::from_string(&row.get::<_, String>(1)?),
                artifact_ref: row.get(2)?,
                artifact_type: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut artifacts = Vec::new();
        for artifact in rows.flatten() {
            artifacts.push(artifact);
        }

        Ok(artifacts)
    }

    fn create_subgoal(
        &self,
        token: &GoalCapabilityToken,
        parent_id: GoalID,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal> {
        self.verify_capability(token, GoalOp::CreateSubgoal)?;

        let parent = self.get_goal(token, parent_id)?.ok_or_else(|| {
            GoalRepositoryError::NotFound(format!("Parent goal {} not found", parent_id))
        })?;

        if !parent.can_have_subgoals() {
            return Err(GoalRepositoryError::MaxDepthExceeded(format!(
                "Parent goal at depth {} cannot have subgoals (max depth 7)",
                parent.depth
            )));
        }

        let subgoal = Goal::new(*webid, text, visibility).with_parent(parent_id, parent.depth);

        self.conn.lock().unwrap().execute(
            "INSERT INTO goals (id, webid, text, state, visibility, parent_goal_id, depth) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (subgoal.id.to_string(), subgoal.webid.to_string(), subgoal.text.clone(), subgoal.state.as_str(), subgoal.visibility.as_str(), parent_id.to_string(), subgoal.depth as i32),
        )?;

        Ok(subgoal)
    }

    fn get_subgoals(&self, token: &GoalCapabilityToken, parent_id: GoalID) -> Result<Vec<Goal>> {
        self.verify_capability(token, GoalOp::Read)?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth FROM goals WHERE parent_goal_id = ?1 ORDER BY created_at ASC")?;
        let rows = stmt.query_map([parent_id.to_string()], Self::goal_from_row)?;

        let mut subgoals = Vec::new();
        for goal in rows.flatten() {
            if self
                .check_visibility_access(&goal, &token.holder_webid)
                .is_ok()
            {
                subgoals.push(goal);
            }
        }

        Ok(subgoals)
    }

    fn delete_goal(&self, token: &GoalCapabilityToken, goal_id: GoalID) -> Result<()> {
        self.verify_capability(token, GoalOp::Complete)?;

        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM goals WHERE id = ?1", [goal_id.to_string()])?;
        Ok(())
    }
}
