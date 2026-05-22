//! hKask Storage — Goal Repository
//!
//! SQLite-backed persistence for goals with hLexicon integration.
//! Implements repository pattern for goal CRUD operations.

use hkask_types::goal::{Goal, GoalCommitment, GoalId, GoalSpec, GoalState};
use hkask_types::visibility::Visibility;
use rusqlite::{Connection, Result, params};
use std::path::Path;

/// Goal repository trait (hexagonal port)
pub trait GoalRepositoryPort {
    fn create(&self, spec: GoalSpec) -> Result<GoalId>;
    fn get(&self, id: GoalId) -> Result<Goal>;
    fn update(&self, id: GoalId, goal: Goal) -> Result<()>;
    fn delete(&self, id: GoalId) -> Result<()>;
    fn list_by_owner(&self, owner: &str) -> Result<Vec<Goal>>;
    fn list_by_session(&self, session: &str) -> Result<Vec<Goal>>;
}

/// SQLite goal repository implementation
pub struct SqliteGoalRepository {
    conn: Connection,
}

impl SqliteGoalRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    /// Initialize database schema
    pub fn init(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(include_str!(
            "../../../docs/storage/migrations/001_goals.sql"
        ))?;
        Ok(())
    }
}

impl GoalRepositoryPort for SqliteGoalRepository {
    fn create(&self, spec: GoalSpec) -> Result<GoalId> {
        let goal = Goal::new(spec);
        let id = goal.id.0.to_string();
        let session_id = goal.session_id.0.to_string();

        self.conn.execute(
            "INSERT INTO goals (
                id, session_id, owner_webid, goal_text, template_ref,
                commitment_level, state, turns_used,
                energy_budget, energy_used, max_turns, created_at,
                visibility
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                session_id,
                goal.owner_webid,
                goal.goal_text,
                goal.template_ref,
                goal.commitment_level.to_string(),
                "active",
                goal.turns_used,
                goal.energy_budget.map(|b| b as i64),
                goal.energy_used as i64,
                goal.max_turns,
                goal.created_at,
                match goal.visibility {
                    Visibility::Public => "public",
                    Visibility::Shared => "shared",
                    Visibility::Private => "private",
                },
            ],
        )?;

        Ok(goal.id)
    }

    fn get(&self, id: GoalId) -> Result<Goal> {
        let goal = self.conn.query_row(
            "SELECT * FROM goals WHERE id = ?1",
            params![id.0.to_string()],
            |row| {
                let id_str: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let owner_webid: String = row.get(2)?;
                let goal_text: String = row.get(3)?;
                let template_ref: Option<String> = row.get(4)?;
                let commitment_str: String = row.get(5)?;
                let state_str: String = row.get(7)?;
                let turns_used: u32 = row.get(8)?;
                let energy_budget: Option<i64> = row.get(9)?;
                let energy_used: i64 = row.get(10)?;
                let max_turns: u32 = row.get(11)?;
                let created_at: i64 = row.get(12)?;
                let last_turn_at: Option<i64> = row.get(13)?;
                let completed_at: Option<i64> = row.get(14)?;
                let visibility_str: String = row.get(17)?;

                let commitment_level = match commitment_str.as_str() {
                    "pledge" => GoalCommitment::Pledge,
                    "undertake" => GoalCommitment::Undertake,
                    "promise" => GoalCommitment::Promise,
                    _ => GoalCommitment::Commit,
                };

                let state = if let Some(reason) = state_str.strip_prefix("paused:") {
                    GoalState::Paused {
                        reason: reason.to_string(),
                    }
                } else if let Some(reason) = state_str.strip_prefix("done:") {
                    GoalState::Done {
                        reason: reason.to_string(),
                    }
                } else if let Some(reason) = state_str.strip_prefix("blocked:") {
                    GoalState::Blocked {
                        reason: reason.to_string(),
                    }
                } else if state_str == "cleared" {
                    GoalState::Cleared
                } else {
                    GoalState::Active
                };

                let visibility = match visibility_str.as_str() {
                    "public" => Visibility::Public,
                    "shared" => Visibility::Shared,
                    _ => Visibility::Private,
                };

                Ok(Goal {
                    id: GoalId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()),
                    session_id: hkask_types::id::SessionID(
                        uuid::Uuid::parse_str(&session_id).unwrap_or_default(),
                    ),
                    owner_webid,
                    goal_text,
                    template_ref,
                    state,
                    commitment_level,
                    flow: None,
                    completion_criteria: Vec::new(),
                    subgoals: Vec::new(),
                    turns_used,
                    energy_budget: energy_budget.map(|b| b as u64),
                    energy_used: energy_used as u64,
                    max_turns,
                    created_at,
                    last_turn_at,
                    completed_at,
                    visibility,
                })
            },
        )?;

        Ok(goal)
    }

    fn update(&self, id: GoalId, goal: Goal) -> Result<()> {
        let state_str = match &goal.state {
            GoalState::Active => "active".to_string(),
            GoalState::Paused { reason } => format!("paused:{}", reason),
            GoalState::Done { reason } => format!("done:{}", reason),
            GoalState::Cleared => "cleared".to_string(),
            GoalState::Blocked { reason } => format!("blocked:{}", reason),
        };

        self.conn.execute(
            "UPDATE goals SET
                state = ?1, turns_used = ?2,
                energy_budget = ?3, energy_used = ?4,
                last_turn_at = ?5, completed_at = ?6
            WHERE id = ?7",
            params![
                state_str,
                goal.turns_used,
                goal.energy_budget.map(|b| b as i64),
                goal.energy_used as i64,
                goal.last_turn_at,
                goal.completed_at,
                id.0.to_string(),
            ],
        )?;

        Ok(())
    }

    fn delete(&self, id: GoalId) -> Result<()> {
        self.conn
            .execute("DELETE FROM goals WHERE id = ?1", params![id.0.to_string()])?;

        Ok(())
    }

    fn list_by_owner(&self, owner: &str) -> Result<Vec<Goal>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM goals WHERE owner_webid = ?1 ORDER BY created_at DESC")?;

        let goals = stmt
            .query_map(params![owner], |row| self.row_to_goal(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(goals)
    }

    fn list_by_session(&self, session: &str) -> Result<Vec<Goal>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM goals WHERE session_id = ?1 ORDER BY created_at DESC")?;

        let goals = stmt
            .query_map(params![session], |row| self.row_to_goal(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(goals)
    }
}

impl SqliteGoalRepository {
    fn row_to_goal(&self, row: &rusqlite::Row) -> Result<Goal, rusqlite::Error> {
        let id_str: String = row.get(0)?;
        let session_id: String = row.get(1)?;
        let owner_webid: String = row.get(2)?;
        let goal_text: String = row.get(3)?;
        let template_ref: Option<String> = row.get(4)?;
        let commitment_str: String = row.get(5)?;
        let state_str: String = row.get(7)?;
        let turns_used: u32 = row.get(8)?;
        let energy_budget: Option<i64> = row.get(9)?;
        let energy_used: i64 = row.get(10)?;
        let max_turns: u32 = row.get(11)?;
        let created_at: i64 = row.get(12)?;
        let last_turn_at: Option<i64> = row.get(13)?;
        let completed_at: Option<i64> = row.get(14)?;
        let visibility_str: String = row.get(17)?;

        let commitment_level = match commitment_str.as_str() {
            "pledge" => GoalCommitment::Pledge,
            "undertake" => GoalCommitment::Undertake,
            "promise" => GoalCommitment::Promise,
            _ => GoalCommitment::Commit,
        };

        let state = if let Some(reason) = state_str.strip_prefix("paused:") {
            GoalState::Paused {
                reason: reason.to_string(),
            }
        } else if let Some(reason) = state_str.strip_prefix("done:") {
            GoalState::Done {
                reason: reason.to_string(),
            }
        } else if let Some(reason) = state_str.strip_prefix("blocked:") {
            GoalState::Blocked {
                reason: reason.to_string(),
            }
        } else if state_str == "cleared" {
            GoalState::Cleared
        } else {
            GoalState::Active
        };

        let visibility = match visibility_str.as_str() {
            "public" => Visibility::Public,
            "shared" => Visibility::Shared,
            _ => Visibility::Private,
        };

        Ok(Goal {
            id: GoalId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()),
            session_id: hkask_types::id::SessionID(
                uuid::Uuid::parse_str(&session_id).unwrap_or_default(),
            ),
            owner_webid,
            goal_text,
            template_ref,
            state,
            commitment_level,
            flow: None,
            completion_criteria: Vec::new(),
            subgoals: Vec::new(),
            turns_used,
            energy_budget: energy_budget.map(|b| b as u64),
            energy_used: energy_used as u64,
            max_turns,
            created_at,
            last_turn_at,
            completed_at,
            visibility,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::id::SessionID;

    #[test]
    fn test_create_and_get_goal() {
        let repo = SqliteGoalRepository::in_memory().unwrap();
        repo.init().unwrap();

        let spec = GoalSpec {
            owner_webid: "test-owner".to_string(),
            session_id: SessionID::new(),
            goal_text: "Test goal".to_string(),
            template_ref: None,
            commitment_level: GoalCommitment::Commit,
            flow: None,
            completion_criteria: Vec::new(),
            max_turns: Some(10),
            energy_budget: None,
            visibility: Visibility::Private,
        };

        let id = repo.create(spec).unwrap();
        let goal = repo.get(id).unwrap();

        assert_eq!(goal.goal_text, "Test goal");
        assert_eq!(goal.max_turns, 10);
    }
}
