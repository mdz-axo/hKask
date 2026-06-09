//! Goal coordination commands — call goal repo directly.

use hkask_types::goal::GoalState;
use hkask_types::id::WebID;
use hkask_types::visibility::Visibility;

use crate::cli::GoalAction;
use crate::errors::RegistryError;

fn build_goal_repo() -> Result<hkask_storage::SqliteGoalRepository, RegistryError> {
    let config = hkask_services::ServiceConfig::from_env()
        .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    let db = hkask_storage::Database::open(&config.db_path, &config.db_passphrase)
        .map_err(|e| RegistryError::InitFailed(e.to_string()))?;
    Ok(hkask_storage::SqliteGoalRepository::new(db.conn_arc()))
}

fn parse_visibility(vis: &str) -> Result<Visibility, RegistryError> {
    Visibility::parse_str(vis).ok_or_else(|| {
        RegistryError::InitFailed(format!(
            "Invalid visibility '{vis}': expected private | shared | public"
        ))
    })
}

fn parse_goal_state(state: &str) -> Result<GoalState, RegistryError> {
    GoalState::parse_str(state)
        .ok_or_else(|| RegistryError::InitFailed(format!("Invalid goal state '{state}'")))
}

pub fn create(text: &str, visibility: &str) -> Result<(), RegistryError> {
    let repo = build_goal_repo()?;
    let webid = WebID::from_persona(b"cli-user");
    let vis = parse_visibility(visibility)?;
    let goal = repo
        .create_goal(&webid, text, vis)
        .map_err(|e| RegistryError::InitFailed(format!("Goal creation failed: {e}")))?;
    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state.as_str());
    println!("  visibility: {}", goal.visibility.as_str());
    Ok(())
}

pub fn list(state: Option<&str>) -> Result<(), RegistryError> {
    let repo = build_goal_repo()?;
    let webid = WebID::from_persona(b"cli-user");
    let filter = match state {
        Some(s) => Some(parse_goal_state(s)?),
        None => None,
    };
    let goals = repo
        .list_goals(&webid, filter)
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

pub fn set_state(id: &str, state: &str) -> Result<(), RegistryError> {
    let repo = build_goal_repo()?;
    let goal_id: hkask_types::id::GoalID = id
        .parse()
        .map_err(|e| RegistryError::InitFailed(format!("Invalid goal ID '{id}': {e}")))?;
    let new_state = parse_goal_state(state)?;
    repo.update_goal_state(goal_id, new_state)
        .map_err(|e| RegistryError::InitFailed(format!("Goal state change failed: {e}")))?;
    println!("Goal {} -> {}", goal_id, state);
    Ok(())
}

pub fn run_goal(action: crate::cli::GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => create(&text, &visibility),
        GoalAction::List { state } => list(state.as_deref()),
        GoalAction::SetState { id, state } => set_state(&id, &state),
    };
    super::helpers::or_exit(result, "Goal command failed");
}
