use super::*;

impl KanbanService {
    pub fn unjam_report(&self, board_id: BoardId) -> Result<Vec<UnjamItem>, KanbanError> {
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let now = chrono::Utc::now();
        let mut items = Vec::new();

        for task in &tasks {
            if (task.status == TaskStatus::InProgress || task.status == TaskStatus::Review)
                && let Some(hours) = task.estimated_hours
            {
                let elapsed = (now - task.updated_at).num_hours();
                if elapsed > (hours as i64) * 2 {
                    items.push(UnjamItem {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        issue: format!(
                            "Stuck in {} for {}h (estimated {}h)",
                            task.status, elapsed, hours
                        ),
                        suggestion: "Consider escalating or reassigning.".into(),
                    });
                }
            }

            if task.assignee.is_some()
                && (task.status == TaskStatus::Backlog || task.status == TaskStatus::Ready)
            {
                let elapsed = (now - task.updated_at).num_hours();
                if elapsed > 24 {
                    items.push(UnjamItem {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        issue: format!("Assigned but not started for {}h", elapsed),
                        suggestion: "Consider unassigning or escalating.".into(),
                    });
                }
            }

            if task.status == TaskStatus::Done && task.verification.is_none() {
                items.push(UnjamItem {
                    task_id: task.id,
                    task_title: task.title.clone(),
                    issue: "Completed without verification.".into(),
                    suggestion: "Reopen and verify, or verify retroactively.".into(),
                });
            }
        }

        Ok(items)
    }

    pub fn unjam_fix(&self, board_id: BoardId) -> Result<Vec<UnjamFix>, KanbanError> {
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let now = chrono::Utc::now();
        let mut fixes = Vec::new();

        for task in &tasks {
            if task.assignee.is_some()
                && (task.status == TaskStatus::Backlog || task.status == TaskStatus::Ready)
            {
                let elapsed = (now - task.updated_at).num_hours();
                if elapsed > 24 {
                    match self.task_unassign(task.id) {
                        Ok(_) => fixes.push(UnjamFix {
                            task_id: task.id,
                            task_title: task.title.clone(),
                            action: format!("Unassigned after {}h idle", elapsed),
                        }),
                        Err(e) => fixes.push(UnjamFix {
                            task_id: task.id,
                            task_title: task.title.clone(),
                            action: format!("Unassign failed: {}", e),
                        }),
                    }
                }
            }

            if task.status == TaskStatus::Done && task.verification.is_none() {
                match self.task_reopen(task.id) {
                    Ok(_) => fixes.push(UnjamFix {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        action: "Reopened (was Done without verification)".into(),
                    }),
                    Err(e) => fixes.push(UnjamFix {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        action: format!("Reopen failed: {}", e),
                    }),
                }
            }
        }

        Ok(fixes)
    }
}
