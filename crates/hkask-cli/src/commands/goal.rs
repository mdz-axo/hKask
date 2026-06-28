//! Goal coordination commands — direct calls to goal repository.
//! Formerly delegated to GoalService (removed v0.31.0 per P5).

use hkask_services_core::GoalState;
use hkask_types::WebID;
use hkask_types::visibility::Visibility;

use crate::cli::GoalAction;

/// Run a goal command.
pub fn run_goal(action: crate::cli::GoalAction) {
    let result = match action {
        GoalAction::Create { text, visibility } => create(&text, &visibility),
        GoalAction::List { state } => list(state.as_deref()),
        GoalAction::SetState { id, state } => set_state(&id, &state),
    };
    super::helpers::or_exit(result, "Goal command failed");
}

fn create(text: &str, visibility: &str) -> Result<(), String> {
    let ctx = super::helpers::build_service_context();
    let owner = WebID::from_persona(b"cli-user");
    let vis = Visibility::parse_str(visibility).ok_or_else(|| {
        format!(
            "Invalid visibility '{}': expected private | shared",
            visibility
        )
    })?;
    let repo = ctx.goal_repo();
    let goal = repo
        .create_goal(&owner, text, vis)
        .map_err(|e| format!("Failed to create goal: {e}"))?;
    println!("Created goal {}", goal.id);
    println!("  text:       {}", goal.text);
    println!("  state:      {}", goal.state.as_str());
    println!("  visibility: {}", goal.visibility.as_str());
    Ok(())
}

fn list(state: Option<&str>) -> Result<(), String> {
    let ctx = super::helpers::build_service_context();
    let owner = WebID::from_persona(b"cli-user");
    let filter = match state {
        Some(s) => Some(
            GoalState::parse_str(s).ok_or_else(|| format!("Invalid goal state filter '{}'", s))?,
        ),
        None => None,
    };
    let repo = ctx.goal_repo();
    let goals = repo
        .list_goals(&owner, filter)
        .map_err(|e| format!("Failed to list goals: {e}"))?;
    super::helpers::print_item_list(&goals, "No goals found.", "Goals", |g| {
        format!("{} [{}] {}", g.id, g.state.as_str(), g.text)
    });
    Ok(())
}

fn set_state(id: &str, state: &str) -> Result<(), String> {
    let ctx = super::helpers::build_service_context();
    let owner = WebID::from_persona(b"cli-user");
    let goal_id: hkask_types::id::GoalID = id
        .parse()
        .map_err(|_| format!("Invalid goal ID '{}'", id))?;
    let new_state =
        GoalState::parse_str(state).ok_or_else(|| format!("Invalid goal state '{}'", state))?;
    let repo = ctx.goal_repo();

    let goal = repo
        .get_goal(goal_id)
        .map_err(|e| format!("Failed to load goal: {e}"))?
        .ok_or_else(|| format!("Goal not found: {}", id))?;

    // Ownership check — the only real logic that was in the service layer
    if goal.webid != owner {
        return Err("Not authorized to transition this goal".to_string());
    }

    let from_state = goal.state.as_str().to_string();
    repo.update_goal_state(goal_id, new_state)
        .map_err(|e| format!("Failed to update goal state: {e}"))?;

    // Curation inbox notification
    if let Some(tx) = ctx.curation_inbox_tx() {
        let event = hkask_cns::types::loops::CurationInput::GoalTransition(
            hkask_cns::types::loops::GoalTransitionEvent {
                goal_id: goal_id.to_string(),
                from_state,
                to_state: new_state.as_str().to_string(),
                agent: WebID::from_persona(b"goal-service"),
            },
        );
        if let Err(e) = tx.send(event) {
            tracing::warn!(
                target: "cns.curation",
                goal_id = %goal_id,
                error = %e,
                "Goal transition event failed to reach curation inbox"
            );
        }
    }

    println!("Goal {} -> {}", goal.id, goal.state.as_str());
    Ok(())
}
