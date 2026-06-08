//! Goal coordination commands.
//
//! This module wires the goal subsystem into the CLI via `ServiceContext`,
//! which provides the `goal_repo` field (already wired with CNS telemetry).
//! No direct database access.

use crate::cli::GoalAction;
use crate::errors::RegistryError;
use hkask_types::goal::GoalState;

use hkask_types::id::{GoalID, WebID};
use hkask_types::visibility::Visibility;

/// Build a ServiceContext for goal subcommands.
fn build_service_context() -> Result<hkask_services::ServiceContext, RegistryError> {
    let config = hkask_services::ServiceConfig::from_env()
        .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    rt.block_on(hkask_services::ServiceContext::build(config))
        .map_err(|e| RegistryError::InitFailed(e.to_string()))
}

/// `kask goal create <text> [--visibility ...]`
pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let ctx = build_service_context()?;
    let webid = WebID::from_persona(b"cli-user");
    let vis = Visibility::parse_str(visibility).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid visibility '{visibility}' (expected private | shared | public)"
        ))
    })?;

    let goal = ctx
        .goal_repo
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
    let ctx = build_service_context()?;
    let webid = WebID::from_persona(b"cli-user");
    let state_filter = match state {
        Some(s) => Some(
            GoalState::parse_str(s)
                .ok_or_else(|| RegistryError::InitFailed(format!("Invalid state filter '{s}'")))?,
        ),
        None => None,
    };

    let goals = ctx
        .goal_repo
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
    let ctx = build_service_context()?;
    let goal_id = id
        .parse::<GoalID>()
        .map_err(|e| RegistryError::InitFailed(format!("Invalid goal ID: {e}")))?;
    let new_state = GoalState::parse_str(state).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid state '{state}' (expected pending | active | completed | blocked | abandoned)"
        ))
    })?;

    ctx.goal_repo
        .update_goal_state(goal_id, new_state)
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
