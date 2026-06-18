use super::*;

impl KanbanService {
    pub fn task_comment(
        &self,
        task_id: TaskId,
        author: WebID,
        body: &str,
    ) -> Result<Comment, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        let comment = Comment::new(task_id, author, body.to_string());
        task.comments.push(comment.clone());
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(comment)
    }

    pub fn task_comments(&self, task_id: TaskId) -> Result<Vec<Comment>, KanbanError> {
        let task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        Ok(task.comments)
    }

    pub fn task_add_deliverable(&self, task_id: TaskId, path: &str) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        task.deliverables.push(path.to_string());
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(task)
    }
}
