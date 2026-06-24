use super::*;

impl KanbanService {
    /// Report stuck, idle, or unverified tasks on a board.
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

            // Report tasks that are out of gas
            if (task.status == TaskStatus::InProgress || task.status == TaskStatus::Review)
                && let Some(remaining) = task.gas_remaining
                && remaining == 0
            {
                items.push(UnjamItem {
                    task_id: task.id,
                    task_title: task.title.clone(),
                    issue: "Out of gas — budget exhausted.".into(),
                    suggestion: "Task will auto-complete. Reopen with more gas to continue.".into(),
                });
            }
        }

        Ok(items)
    }

    /// Auto-resolve jammed tasks: unassign idle, reopen unverified, gas-exhaust.
    pub fn unjam_fix(&self, board_id: BoardId) -> Result<Vec<UnjamFix>, KanbanError> {
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let now = chrono::Utc::now();
        let mut fixes = Vec::new();

        for task in &tasks {
            // Unassign tasks idle > 24h
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

            // Reopen Done tasks without verification
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

            // Gas exhaustion: auto-complete tasks that have run out of gas
            if (task.status == TaskStatus::InProgress || task.status == TaskStatus::Review)
                && let Some(remaining) = task.gas_remaining
                && remaining == 0
            {
                match self.task_gas_exhaust(task.id) {
                    Ok(_) => fixes.push(UnjamFix {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        action: "Auto-completed (gas budget exhausted)".into(),
                    }),
                    Err(e) => fixes.push(UnjamFix {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        action: format!("Gas-exhaust failed: {}", e),
                    }),
                }
            }
        }

        Ok(fixes)
    }

    /// Mark a task as Done due to gas exhaustion.
    ///
    /// Gas exhaustion is a completion path: subagents burn gas/rJoules from a
    /// budget explicitly set on the task. When gas hits zero mid-work, the
    /// task auto-completes. The delegator can reopen with more gas to continue.
    pub fn task_gas_exhaust(&self, task_id: TaskId) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        let v = Verification::new(
            false,
            "Gas exhausted — subagent budget consumed.".into(),
            task.owner,
        );
        task.verification = Some(v);
        task.status = TaskStatus::Done;
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;

        tracing::info!(
            target: "cns.kanban",
            operation = "task_gas_exhausted",
            task_id = %task_id,
            board_id = %task.board_id,
            "CNS"
        );

        Ok(task)
    }

    /// Deduct gas from a task's remaining budget.
    ///
    /// Called by the subagent execution framework after each inference step
    /// or tool dispatch. When gas hits zero, subsequent calls succeed but
    /// the caller should check `gas_remaining` and stop work. The unjam
    /// flow auto-completes zero-gas tasks.
    pub fn task_consume_gas(&self, task_id: TaskId, amount: u64) -> Result<u64, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        let remaining = task.gas_remaining.unwrap_or(0);
        let new_remaining = remaining.saturating_sub(amount);
        task.gas_remaining = Some(new_remaining);
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;

        Ok(new_remaining)
    }
}
