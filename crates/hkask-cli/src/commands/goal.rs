//! Goal coordination commands.
//!
//! This module wires the hardened goal-capability subsystem into the CLI:
//!
//! - The repository is opened over the shared, encrypted database.
//! - A CNS [`NuEventStore`] sink (built from the *same* connection) is injected
//!   via `with_telemetry`, so authority denials persist as
//!   `cns.tool.goal.capability.denied` ν-events in the same transaction store.
//! - Capability tokens are minted from the resolved OCAP secret, keeping goal
//!   authority consistent with the rest of the OCAP system.
//!
//! Authority is co-located with effect: every write checks the holder's
//! ownership in addition to the capability (see `hkask-storage::goals`).

use crate::errors::RegistryError;
use hkask_storage::{NuEventStore, SqliteGoalRepository};
use hkask_types::event::NuEventSink;
use hkask_types::goal::GoalState;
use hkask_types::goal_capability::{GoalCapabilityToken, GoalOp};
use hkask_types::id::{GoalID, WebID};
use hkask_types::visibility::Visibility;
use std::sync::Arc;

/// Resolve the OCAP secret used to sign goal capability tokens.
///
/// Uses the deterministic master-key derivation chain (same secret as the rest
/// of the OCAP system), so tokens are restart-stable for a given passphrase.
fn resolve_ocap_secret() -> Result<Vec<u8>, RegistryError> {
    hkask_keystore::get_or_create_ocap_secret()
        .map(|s| s.to_vec())
        .map_err(|e| {
            RegistryError::InitFailed(format!(
                "Could not resolve OCAP secret for goal capability ({e}). \
                 Run `kask chat` to onboard, set HKASK_MASTER_KEY, or use \
                 HKASK_INSECURE_DEV=1 with `kask admin unlock`."
            ))
        })
}

/// Open a goal repository over the shared database, wired with CNS telemetry.
///
/// Returns the repository plus the caller's `WebID` and the OCAP secret used to
/// mint tokens.
fn open_repository() -> Result<(SqliteGoalRepository, WebID, Vec<u8>), RegistryError> {
    let conn = crate::commands::config::open_registry_db()
        .map_err(|e| RegistryError::DatabaseError(e.to_string()))?;

    // The denial sink shares the database connection so telemetry lands in the
    // same ν-event store as the rest of CNS observability.
    let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));

    let secret = resolve_ocap_secret()?;
    let repo = SqliteGoalRepository::new(conn).with_telemetry(sink);

    // The CLI user's identity. Derived deterministically so a given install has
    // a stable owner WebID for its goals.
    let webid = WebID::from_persona(b"cli-user");

    Ok((repo, webid, secret))
}

/// Mint a capability token for the given operations, bound to `goal_id` and
/// held by `webid`.
fn mint_token(
    goal_id: GoalID,
    webid: WebID,
    ops: Vec<GoalOp>,
    secret: &[u8],
) -> GoalCapabilityToken {
    GoalCapabilityToken::new(goal_id, webid, ops, secret)
}

/// `kask goal create <text> [--visibility ...]`
pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let (repo, webid, secret) = open_repository()?;
    let vis = Visibility::parse_str(visibility).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid visibility '{visibility}' (expected private | shared | public)"
        ))
    })?;

    // A token bound to a fresh goal id authorizes creation; the repository binds
    // the created goal's id, so the token need only carry CREATE here.
    let token = mint_token(GoalID::new(), webid, vec![GoalOp::Create], &secret);

    let goal = repo
        .create_goal(&token, &webid, text, vis)
        .map_err(|e| RegistryError::InitFailed(format!("Goal creation denied: {e}")))?;

    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state.as_str());
    println!("  visibility: {}", goal.visibility.as_str());
    Ok(())
}

/// `kask goal list [--state ...]`
pub fn list(state: Option<&str>) -> Result<(), RegistryError> {
    let (repo, webid, secret) = open_repository()?;
    let state_filter = match state {
        Some(s) => Some(
            GoalState::parse_str(s)
                .ok_or_else(|| RegistryError::InitFailed(format!("Invalid state filter '{s}'")))?,
        ),
        None => None,
    };

    let token = mint_token(GoalID::new(), webid, vec![GoalOp::Read], &secret);
    let goals = repo
        .list_goals(&token, &webid, state_filter)
        .map_err(|e| RegistryError::InitFailed(format!("Goal list denied: {e}")))?;

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
    let (repo, webid, secret) = open_repository()?;
    let goal_id = GoalID::from_string(id);
    let new_state = GoalState::parse_str(state).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid state '{state}' (expected pending | active | completed | blocked | abandoned)"
        ))
    })?;

    let token = mint_token(goal_id, webid, vec![GoalOp::Update], &secret);
    repo.update_goal_state(&token, goal_id, new_state)
        .map_err(|e| RegistryError::InitFailed(format!("Goal state change denied: {e}")))?;

    println!("Goal {} → {}", goal_id, new_state.as_str());
    Ok(())
}
