//! Goal coordination commands.
//!
//! This module wires the goal subsystem into the CLI:
//!
//! - The repository is opened over the shared, encrypted database.
//! - A CNS [`NuEventStore`] sink (built from the *same* connection) is injected
//!   via `with_telemetry`, so goal operations persist as ν-events
//!   in the same transaction store.
//!
//! Goal operations are available to anyone with DB access — no token ceremony.
//! Authority is co-located with effect: every write checks the holder's
//! ownership (see `hkask-storage::goals`).

use crate::cli::GoalAction;
use crate::errors::RegistryError;
use hkask_storage::{NuEventStore, SqliteGoalRepository};
use hkask_types::event::NuEventSink;
use hkask_types::goal::GoalState;

use hkask_types::id::{GoalID, WebID};
use hkask_types::visibility::Visibility;
use std::sync::Arc;

/// Open a goal repository over the shared database, wired with CNS telemetry.
///
/// Returns the repository plus the caller's `WebID`.
fn open_repository() -> Result<(SqliteGoalRepository, WebID), RegistryError> {
    let conn = crate::commands::config::open_registry_db()
        .map_err(|e| RegistryError::DatabaseError(e.to_string()))?;

    // The denial sink shares the database connection so telemetry lands in the
    // same ν-event store as the rest of CNS observability.
    let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));

    let repo = SqliteGoalRepository::new(conn).with_telemetry(sink);

    // The CLI user's identity. Derived deterministically so a given install has
    // a stable owner WebID for its goals.
    let webid = WebID::from_persona(b"cli-user");

    Ok((repo, webid))
}

/// `kask goal create <text> [--visibility ...]`
pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let (repo, webid) = open_repository()?;
    let vis = Visibility::parse_str(visibility).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid visibility '{visibility}' (expected private | shared | public)"
        ))
    })?;

    let goal = repo
        .create_goal(&webid, text, vis)
        .map_err(|e| RegistryError::InitFailed(format!("Goal creation failed: {e}")))?;

    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state.as_str());
    println!("  visibility: {}", goal.visibility.as_str());
    Ok(())
}

/// `kask goal list [--state ...]`
pub fn list(state: Option<&str>) -> Result<(), RegistryError> {
    let (repo, webid) = open_repository()?;
    let state_filter = match state {
        Some(s) => Some(
            GoalState::parse_str(s)
                .ok_or_else(|| RegistryError::InitFailed(format!("Invalid state filter '{s}'")))?,
        ),
        None => None,
    };

    let goals = repo
        .list_goals(&webid, state_filter)
        .map_err(|e| RegistryError::InitFailed(format!("Goal list failed: {e}")))?;

    if goals.is_empty() {
        println!("No goals found.");
        return Ok(());
    }
    println!("Goals ({}):", goals.len());
    for g in goals {
        println!("  {} [{}] {}", g.id, g.state.as_str(), g.text);
    }
    Ok(())
}

/// `kask goal set-state <id> <state>`
pub fn set_state(id: &str, state: &str) -> Result<(), RegistryError> {
    let (repo, _webid) = open_repository()?;
    let goal_id = id
        .parse::<GoalID>()
        .map_err(|e| RegistryError::InitFailed(format!("Invalid goal ID: {e}")))?;
    let new_state = GoalState::parse_str(state).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid state '{state}' (expected pending | active | completed | blocked | abandoned)"
        ))
    })?;

    repo.update_goal_state(goal_id, new_state)
        .map_err(|e| RegistryError::InitFailed(format!("Goal state change failed: {e}")))?;

    println!("Goal {} -> {}", goal_id, new_state.as_str());
    Ok(())
}

/// CLI handler for `kask goal` subcommand
pub fn run_goal(action: crate::cli::GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => create(&text, &visibility),
        GoalAction::List { state } => list(state.as_deref()),
        GoalAction::SetState { id, state } => set_state(&id, &state),
    };
    super::helpers::or_exit(result, "Goal command failed");
}
