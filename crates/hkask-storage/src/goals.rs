//! Goal storage — SQLite repository for goal persistence
//!
//! Goals are transient coordination substrates.
//! Long-term retention lives in agent memory (episodic/semantic).

use chrono::Utc;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span};
use hkask_types::goal::{Goal, GoalArtifact, GoalCriterion, GoalID, GoalState};
use hkask_types::goal_capability::{GoalAccess, GoalCapabilityToken, GoalOp};
use hkask_types::id::WebID;
use hkask_types::visibility::Visibility;
use rusqlite::Connection;
use serde_json::json;
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

    #[error("Subgoal depth exceeded: {0}")]
    MaxDepthExceeded(String),

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Corrupt persisted goal data: {0}")]
    Corrupt(String),
}

pub type Result<T> = std::result::Result<T, GoalRepositoryError>;

pub struct SqliteGoalRepository {
    pub conn: Arc<Mutex<Connection>>,
    pub capability_secret: Vec<u8>,
    /// Optional CNS telemetry sink. When present, capability and visibility
    /// denials emit a `cns.tool.goal.capability.denied` outcome event so the
    /// Cybernetic Nervous System can observe authority failures. Injected as a
    /// port (hexagonal seam) so storage stays decoupled from `hkask-cns`.
    telemetry: Option<Arc<dyn NuEventSink>>,
}

impl SqliteGoalRepository {
    pub fn new(conn: Arc<Mutex<Connection>>, capability_secret: Vec<u8>) -> Self {
        Self {
            conn,
            capability_secret,
            telemetry: None,
        }
    }

    /// Attach a CNS telemetry sink so authority denials are observable.
    #[must_use = "builder returns the configured repository"]
    pub fn with_telemetry(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Emit a capability/visibility denial as a CNS ν-event. Non-fatal: a sink
    /// failure must never alter or block the security decision, so errors are
    /// logged and swallowed. The span is `cns.tool.goal.capability.denied`.
    fn emit_denial(&self, holder: &WebID, op: &str, reason: &str) {
        if let Some(sink) = &self.telemetry {
            let event = NuEvent::new(
                *holder,
                Span::tool("goal.capability.denied"),
                Phase::Outcome,
                json!({
                    "holder": holder.to_string(),
                    "attempted_op": op,
                    "reason": reason,
                }),
                0,
            )
            .with_outcome(json!({ "decision": "denied" }))
            .with_visibility("private");
            if let Err(e) = sink.persist(&event) {
                tracing::warn!(
                    target: "cns.tool.goal.capability.denied",
                    error = %e,
                    "failed to persist goal capability denial event"
                );
            }
        }
    }

    pub fn verify_capability(
        &self,
        token: &GoalCapabilityToken,
        required_op: GoalOp,
    ) -> Result<()> {
        if !token.is_valid(&self.capability_secret) {
            self.emit_denial(&token.holder_webid, required_op.as_str(), "token_invalid");
            return Err(GoalRepositoryError::CapabilityDenied(
                "Token invalid or expired".to_string(),
            ));
        }
        if !token.can_perform(required_op, &self.capability_secret) {
            self.emit_denial(
                &token.holder_webid,
                required_op.as_str(),
                "operation_not_authorized",
            );
            return Err(GoalRepositoryError::CapabilityDenied(format!(
                "Missing capability for operation: {:?}",
                required_op
            )));
        }
        Ok(())
    }

    pub fn check_visibility_access(&self, goal: &Goal, requester_webid: &WebID) -> Result<()> {
        let access = GoalAccess::check(goal, requester_webid);
        if !access.can_read() {
            self.emit_denial(requester_webid, "READ", "not_visible");
            return Err(GoalRepositoryError::VisibilityDenied(format!(
                "WebID {} cannot access goal with visibility {:?}",
                requester_webid, goal.visibility
            )));
        }
        Ok(())
    }

    /// Authority must be co-located with the effect it gates: a write needs not
    /// only a valid capability for the operation, but also that the holder's
    /// WebID is the goal owner (or has been granted access). This closes the
    /// confused-deputy gap where any valid Update/Complete token could mutate
    /// *any* goal regardless of ownership.
    fn check_write_access(&self, goal: &Goal, requester_webid: &WebID) -> Result<()> {
        if !GoalAccess::check(goal, requester_webid).can_write() {
            self.emit_denial(requester_webid, "WRITE", "not_owner_or_granted");
            return Err(GoalRepositoryError::VisibilityDenied(format!(
                "WebID {} cannot modify goal {} (visibility {:?})",
                requester_webid, goal.id, goal.visibility
            )));
        }
        Ok(())
    }

    /// Admin authority (delete) is restricted to the goal owner.
    fn check_admin_access(&self, goal: &Goal, requester_webid: &WebID) -> Result<()> {
        if !GoalAccess::check(goal, requester_webid).can_admin() {
            self.emit_denial(requester_webid, "ADMIN", "not_owner");
            return Err(GoalRepositoryError::VisibilityDenied(format!(
                "WebID {} is not the owner of goal {} and cannot administer it",
                requester_webid, goal.id
            )));
        }
        Ok(())
    }

    /// Load a goal directly (no capability gate) for internal authorization
    /// checks. Callers must have already verified the capability token.
    fn load_goal(&self, goal_id: GoalID) -> Result<Goal> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT id, webid, text, state, visibility, created_at, completed_at, parent_goal_id, depth FROM goals WHERE id = ?1")?;
        let mut rows = stmt.query([goal_id.to_string()])?;
        match rows.next()? {
            Some(row) => Ok(Self::goal_from_row(row)?),
            None => Err(GoalRepositoryError::NotFound(format!(
                "Goal {} not found",
                goal_id
            ))),
        }
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

impl SqliteGoalRepository {
    pub fn create_goal(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal> {
        self.verify_capability(token, GoalOp::Create)?;

        // A holder may only create goals it will own. Creating a goal owned by
        // another WebID would manufacture authority out of thin air.
        if token.holder_webid != *webid {
            self.emit_denial(&token.holder_webid, "CREATE", "owner_mismatch");
            return Err(GoalRepositoryError::VisibilityDenied(format!(
                "Token holder {} cannot create a goal owned by {}",
                token.holder_webid, webid
            )));
        }

        let goal = Goal::new(*webid, text, visibility);

        // Persist created_at explicitly in RFC3339 so it round-trips through
        // the strict reader. The SQLite `datetime('now')` default produces a
        // non-RFC3339 string that the reader (correctly) rejects as corrupt.
        self.conn.lock().map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?.execute(
            "INSERT INTO goals (id, webid, text, state, visibility, depth, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (goal.id.to_string(), goal.webid.to_string(), goal.text.clone(), goal.state.as_str(), goal.visibility.as_str(), goal.depth as i32, goal.created_at.to_rfc3339()),
        )?;

        Ok(goal)
    }

    pub fn get_goal(&self, token: &GoalCapabilityToken, goal_id: GoalID) -> Result<Option<Goal>> {
        self.verify_capability(token, GoalOp::Read)?;

        let goal = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
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

    pub fn update_goal_state(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        state: GoalState,
    ) -> Result<()> {
        self.verify_capability(token, GoalOp::Update)?;

        let goal = self.load_goal(goal_id)?;
        self.check_write_access(&goal, &token.holder_webid)?;

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

        let conn = self
            .conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
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

    pub fn list_goals(
        &self,
        token: &GoalCapabilityToken,
        webid: &WebID,
        state_filter: Option<GoalState>,
    ) -> Result<Vec<Goal>> {
        self.verify_capability(token, GoalOp::Read)?;

        let mut goals = Vec::new();

        let conn = self
            .conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
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

    pub fn add_criterion(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        criterion: GoalCriterion,
    ) -> Result<()> {
        self.verify_capability(token, GoalOp::Update)?;

        // The criterion must target the goal named by the caller, and the
        // holder must have write access to that goal.
        if criterion.goal_id != goal_id {
            return Err(GoalRepositoryError::InvalidTransition(format!(
                "Criterion targets goal {} but operation named goal {}",
                criterion.goal_id, goal_id
            )));
        }
        let goal = self.load_goal(goal_id)?;
        self.check_write_access(&goal, &token.holder_webid)?;

        self.conn.lock().map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?.execute(
            "INSERT INTO goal_criteria (id, goal_id, type, description, satisfied) VALUES (?1, ?2, ?3, ?4, ?5)",
            (criterion.id, criterion.goal_id.to_string(), criterion.criterion_type, criterion.description, criterion.satisfied as i32),
        )?;
        Ok(())
    }

    pub fn add_artifact(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
        artifact: GoalArtifact,
    ) -> Result<()> {
        self.verify_capability(token, GoalOp::AddArtifact)?;

        if artifact.goal_id != goal_id {
            return Err(GoalRepositoryError::InvalidTransition(format!(
                "Artifact targets goal {} but operation named goal {}",
                artifact.goal_id, goal_id
            )));
        }
        let goal = self.load_goal(goal_id)?;
        self.check_write_access(&goal, &token.holder_webid)?;

        self.conn.lock().map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?.execute(
            "INSERT INTO goal_artifacts (id, goal_id, artifact_ref, artifact_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            (artifact.id, artifact.goal_id.to_string(), artifact.artifact_ref, artifact.artifact_type, artifact.created_at.to_rfc3339()),
        )?;
        Ok(())
    }

    pub fn get_criteria(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
    ) -> Result<Vec<GoalCriterion>> {
        self.verify_capability(token, GoalOp::Read)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
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

    pub fn get_artifacts(
        &self,
        token: &GoalCapabilityToken,
        goal_id: GoalID,
    ) -> Result<Vec<GoalArtifact>> {
        self.verify_capability(token, GoalOp::Read)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
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

    pub fn create_subgoal(
        &self,
        token: &GoalCapabilityToken,
        parent_id: GoalID,
        webid: &WebID,
        text: &str,
        visibility: Visibility,
    ) -> Result<Goal> {
        self.verify_capability(token, GoalOp::CreateSubgoal)?;

        // A holder may only create subgoals it will own.
        if token.holder_webid != *webid {
            self.emit_denial(&token.holder_webid, "CREATE_SUBGOAL", "owner_mismatch");
            return Err(GoalRepositoryError::VisibilityDenied(format!(
                "Token holder {} cannot create a subgoal owned by {}",
                token.holder_webid, webid
            )));
        }

        let parent = self.get_goal(token, parent_id)?.ok_or_else(|| {
            GoalRepositoryError::NotFound(format!("Parent goal {} not found", parent_id))
        })?;
        // Adding a subgoal mutates the parent's tree, so write access to the
        // parent is required (not merely read visibility).
        self.check_write_access(&parent, &token.holder_webid)?;

        if !parent.can_have_subgoals() {
            return Err(GoalRepositoryError::MaxDepthExceeded(format!(
                "Parent goal at depth {} cannot have subgoals",
                parent.depth
            )));
        }

        let subgoal = Goal::new(*webid, text, visibility).with_parent(parent_id, parent.depth);

        self.conn.lock().map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?.execute(
            "INSERT INTO goals (id, webid, text, state, visibility, parent_goal_id, depth, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (subgoal.id.to_string(), subgoal.webid.to_string(), subgoal.text.clone(), subgoal.state.as_str(), subgoal.visibility.as_str(), parent_id.to_string(), subgoal.depth as i32, subgoal.created_at.to_rfc3339()),
        )?;

        Ok(subgoal)
    }

    pub fn get_subgoals(
        &self,
        token: &GoalCapabilityToken,
        parent_id: GoalID,
    ) -> Result<Vec<Goal>> {
        self.verify_capability(token, GoalOp::Read)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?;
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

    pub fn delete_goal(&self, token: &GoalCapabilityToken, goal_id: GoalID) -> Result<()> {
        self.verify_capability(token, GoalOp::Complete)?;

        // Deletion is an administrative act reserved to the goal owner.
        let goal = self.load_goal(goal_id)?;
        self.check_admin_access(&goal, &token.holder_webid)?;

        self.conn
            .lock()
            .map_err(|e| GoalRepositoryError::LockPoisoned(e.to_string()))?
            .execute("DELETE FROM goals WHERE id = ?1", [goal_id.to_string()])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::goal_capability::GoalOp;

    const SECRET: &[u8] = b"goal-repo-test-secret-32-bytes!!";

    fn repo() -> SqliteGoalRepository {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE goals (id TEXT PRIMARY KEY, webid TEXT NOT NULL, text TEXT NOT NULL, \
             state TEXT NOT NULL DEFAULT 'pending', visibility TEXT NOT NULL DEFAULT 'private', \
             created_at TEXT DEFAULT (datetime('now')), completed_at TEXT, parent_goal_id TEXT, \
             depth INTEGER NOT NULL DEFAULT 0);\
             CREATE TABLE goal_criteria (id TEXT PRIMARY KEY, goal_id TEXT, type TEXT NOT NULL, \
             description TEXT NOT NULL, satisfied INTEGER NOT NULL DEFAULT 0);\
             CREATE TABLE goal_artifacts (id TEXT PRIMARY KEY, goal_id TEXT, artifact_ref TEXT NOT NULL, \
             artifact_type TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));",
        )
        .expect("schema");
        SqliteGoalRepository::new(Arc::new(Mutex::new(conn)), SECRET.to_vec())
    }

    fn token_for(webid: &WebID, ops: Vec<GoalOp>) -> GoalCapabilityToken {
        GoalCapabilityToken::new(GoalID::new(), *webid, ops, SECRET)
    }

    /// Test sink that records every persisted event.
    #[derive(Default, Clone)]
    struct CapturingSink {
        events: Arc<Mutex<Vec<NuEvent>>>,
    }

    impl NuEventSink for CapturingSink {
        fn persist(
            &self,
            event: &NuEvent,
        ) -> std::result::Result<(), hkask_types::event::NuEventSinkError> {
            self.events.lock().expect("sink lock").push(event.clone());
            Ok(())
        }
    }

    /// Test sink that always fails, to prove emission is non-fatal.
    struct FailingSink;

    impl NuEventSink for FailingSink {
        fn persist(
            &self,
            _event: &NuEvent,
        ) -> std::result::Result<(), hkask_types::event::NuEventSinkError> {
            Err(hkask_types::event::NuEventSinkError::Unavailable(
                "test sink down".to_string(),
            ))
        }
    }

    #[test]
    fn owner_can_create_and_read_own_goal() {
        let r = repo();
        let alice = WebID::from_string("did:web:alice");
        let token = token_for(&alice, vec![GoalOp::Create, GoalOp::Read]);
        let goal = r
            .create_goal(&token, &alice, "ship the thing", Visibility::Private)
            .expect("owner create");
        let fetched = r.get_goal(&token, goal.id).expect("read").expect("some");
        assert_eq!(fetched.id, goal.id);
        assert_eq!(fetched.webid, alice);
    }

    #[test]
    fn holder_cannot_create_goal_owned_by_another() {
        let r = repo();
        let alice = WebID::from_string("did:web:alice");
        let bob = WebID::from_string("did:web:bob");
        let token = token_for(&alice, vec![GoalOp::Create]);
        // Alice's token tries to mint a goal owned by Bob.
        let err = r
            .create_goal(&token, &bob, "forge", Visibility::Private)
            .expect_err("cross-owner create must be denied");
        assert!(matches!(err, GoalRepositoryError::VisibilityDenied(_)));
    }

    #[test]
    fn non_owner_cannot_mutate_private_goal() {
        let r = repo();
        let alice = WebID::from_string("did:web:alice");
        let mallory = WebID::from_string("did:web:mallory");
        let alice_token = token_for(&alice, vec![GoalOp::Create]);
        let goal = r
            .create_goal(&alice_token, &alice, "secret", Visibility::Private)
            .expect("create");

        // Mallory holds a perfectly valid Update token (for her own goal id),
        // but it must not let her mutate Alice's private goal.
        let mallory_token = token_for(&mallory, vec![GoalOp::Update]);
        let err = r
            .update_goal_state(&mallory_token, goal.id, GoalState::Active)
            .expect_err("confused-deputy write must be denied");
        assert!(matches!(err, GoalRepositoryError::VisibilityDenied(_)));
    }

    #[test]
    fn illegal_state_transition_is_rejected() {
        let r = repo();
        let alice = WebID::from_string("did:web:alice");
        let token = token_for(&alice, vec![GoalOp::Create, GoalOp::Update]);
        let goal = r
            .create_goal(&token, &alice, "task", Visibility::Private)
            .expect("create");

        // Pending -> Completed is illegal (must pass through Active).
        let err = r
            .update_goal_state(&token, goal.id, GoalState::Completed)
            .expect_err("illegal transition must be rejected");
        assert!(matches!(err, GoalRepositoryError::InvalidTransition(_)));

        // Legal progression succeeds.
        r.update_goal_state(&token, goal.id, GoalState::Active)
            .expect("pending -> active");
        r.update_goal_state(&token, goal.id, GoalState::Completed)
            .expect("active -> completed");

        // Terminal goal cannot be reopened.
        let err = r
            .update_goal_state(&token, goal.id, GoalState::Active)
            .expect_err("reopening terminal goal must be rejected");
        assert!(matches!(err, GoalRepositoryError::InvalidTransition(_)));
    }

    #[test]
    fn non_owner_cannot_delete_goal() {
        let r = repo();
        let alice = WebID::from_string("did:web:alice");
        let mallory = WebID::from_string("did:web:mallory");
        let goal = r
            .create_goal(
                &token_for(&alice, vec![GoalOp::Create]),
                &alice,
                "x",
                Visibility::Shared,
            )
            .expect("create");

        // Shared visibility grants Mallory write (Granted) but not admin.
        let err = r
            .delete_goal(&token_for(&mallory, vec![GoalOp::Complete]), goal.id)
            .expect_err("non-owner delete must be denied");
        assert!(matches!(err, GoalRepositoryError::VisibilityDenied(_)));

        // Owner can delete.
        r.delete_goal(&token_for(&alice, vec![GoalOp::Complete]), goal.id)
            .expect("owner delete");
    }

    #[test]
    fn confused_deputy_write_emits_denial_telemetry() {
        let sink = CapturingSink::default();
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE goals (id TEXT PRIMARY KEY, webid TEXT NOT NULL, text TEXT NOT NULL, \
             state TEXT NOT NULL DEFAULT 'pending', visibility TEXT NOT NULL DEFAULT 'private', \
             created_at TEXT DEFAULT (datetime('now')), completed_at TEXT, parent_goal_id TEXT, \
             depth INTEGER NOT NULL DEFAULT 0);",
        )
        .expect("schema");
        let r = SqliteGoalRepository::new(Arc::new(Mutex::new(conn)), SECRET.to_vec())
            .with_telemetry(Arc::new(sink.clone()));

        let alice = WebID::from_string("did:web:alice");
        let mallory = WebID::from_string("did:web:mallory");
        let goal = r
            .create_goal(
                &token_for(&alice, vec![GoalOp::Create]),
                &alice,
                "secret",
                Visibility::Private,
            )
            .expect("create");

        let _ = r
            .update_goal_state(
                &token_for(&mallory, vec![GoalOp::Update]),
                goal.id,
                GoalState::Active,
            )
            .expect_err("confused-deputy write must be denied");

        let events = sink.events.lock().expect("sink lock");
        assert!(
            events.iter().any(|e| matches!(
                &e.span,
                Span::Tool(p) if p == "cns.tool.goal.capability.denied"
            )),
            "a denial must emit a cns.tool.goal.capability.denied span, got {:?}",
            events.iter().map(|e| &e.span).collect::<Vec<_>>()
        );
        // The denial event names the holder who was rejected.
        assert!(events.iter().any(|e| e.observer_webid == mallory));
    }

    #[test]
    fn telemetry_sink_failure_is_non_fatal() {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE goals (id TEXT PRIMARY KEY, webid TEXT NOT NULL, text TEXT NOT NULL, \
             state TEXT NOT NULL DEFAULT 'pending', visibility TEXT NOT NULL DEFAULT 'private', \
             created_at TEXT DEFAULT (datetime('now')), completed_at TEXT, parent_goal_id TEXT, \
             depth INTEGER NOT NULL DEFAULT 0);",
        )
        .expect("schema");
        let r = SqliteGoalRepository::new(Arc::new(Mutex::new(conn)), SECRET.to_vec())
            .with_telemetry(Arc::new(FailingSink));

        let alice = WebID::from_string("did:web:alice");
        let bob = WebID::from_string("did:web:bob");
        // Even though the sink errors on the denial event, the security
        // decision (denial) must still be returned correctly.
        let err = r
            .create_goal(
                &token_for(&alice, vec![GoalOp::Create]),
                &bob,
                "forge",
                Visibility::Private,
            )
            .expect_err("cross-owner create must still be denied despite sink failure");
        assert!(matches!(err, GoalRepositoryError::VisibilityDenied(_)));
    }
}
