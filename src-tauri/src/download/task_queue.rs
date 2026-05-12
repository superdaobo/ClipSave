use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::AppError;
use crate::models::{DownloadTask, TaskStatus};

/// Task queue managing download tasks with concurrency control.
pub struct TaskQueue {
    tasks: Arc<Mutex<VecDeque<DownloadTask>>>,
    max_concurrency: Arc<Mutex<u8>>,
}

impl TaskQueue {
    /// Create a new task queue with the given concurrency limit.
    pub fn new(max_concurrency: u8) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            max_concurrency: Arc::new(Mutex::new(max_concurrency)),
        }
    }

    /// Add a task to the queue.
    pub async fn enqueue(&self, task: DownloadTask) {
        let mut tasks = self.tasks.lock().await;
        tasks.push_back(task);
    }

    /// Get the number of currently downloading tasks.
    pub async fn active_count(&self) -> usize {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Downloading)
            .count()
    }

    /// Promote the next waiting task to parsing if capacity allows.
    pub async fn promote_next(&self) -> Option<String> {
        let mut tasks = self.tasks.lock().await;
        let max = *self.max_concurrency.lock().await;
        let active = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Downloading | TaskStatus::Parsing))
            .count();

        if active >= max as usize {
            return None;
        }

        // Find first waiting task (FIFO by position, which corresponds to created_at order)
        let idx = tasks.iter().position(|t| t.status == TaskStatus::Waiting)?;
        tasks[idx].status = TaskStatus::Parsing;
        Some(tasks[idx].id.clone())
    }

    /// Update a task's status with state machine validation.
    pub async fn update_status(&self, id: &str, new_status: TaskStatus) -> Result<(), AppError> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::InvalidInput {
                message: format!("Task not found: {}", id),
            })?;

        task.status.can_transition_to(&new_status)?;
        task.status = new_status;
        task.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Get all tasks as a snapshot.
    pub async fn get_all(&self) -> Vec<DownloadTask> {
        let tasks = self.tasks.lock().await;
        tasks.iter().cloned().collect()
    }

    /// Get a specific task by ID.
    pub async fn get_task(&self, id: &str) -> Option<DownloadTask> {
        let tasks = self.tasks.lock().await;
        tasks.iter().find(|t| t.id == id).cloned()
    }

    /// Remove a task from the queue (for completed/cancelled cleanup).
    pub async fn remove_task(&self, id: &str) -> Option<DownloadTask> {
        let mut tasks = self.tasks.lock().await;
        let idx = tasks.iter().position(|t| t.id == id)?;
        tasks.remove(idx)
    }

    /// Update max concurrency setting.
    pub async fn set_max_concurrency(&self, max: u8) {
        let mut mc = self.max_concurrency.lock().await;
        *mc = max.clamp(1, 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MediaType, Platform};

    fn make_task() -> DownloadTask {
        DownloadTask::new(
            "https://example.com/video.mp4".to_string(),
            Platform::Douyin,
            MediaType::Video,
        )
    }

    #[tokio::test]
    async fn test_enqueue_and_get_all() {
        let queue = TaskQueue::new(3);
        let task = make_task();
        let id = task.id.clone();
        queue.enqueue(task).await;

        let all = queue.get_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id);
    }

    #[tokio::test]
    async fn test_promote_next_respects_concurrency() {
        let queue = TaskQueue::new(1);

        let mut task1 = make_task();
        task1.status = TaskStatus::Downloading;
        queue.enqueue(task1).await;

        let task2 = make_task();
        queue.enqueue(task2).await;

        // Should not promote because max_concurrency=1 and one is already downloading
        let promoted = queue.promote_next().await;
        assert!(promoted.is_none());
    }

    #[tokio::test]
    async fn test_promote_next_fifo_order() {
        let queue = TaskQueue::new(3);

        let task1 = make_task();
        let id1 = task1.id.clone();
        queue.enqueue(task1).await;

        let task2 = make_task();
        queue.enqueue(task2).await;

        let promoted = queue.promote_next().await;
        assert_eq!(promoted, Some(id1));
    }

    #[tokio::test]
    async fn test_update_status_valid_transition() {
        let queue = TaskQueue::new(3);
        let task = make_task();
        let id = task.id.clone();
        queue.enqueue(task).await;

        // waiting -> parsing is valid
        let result = queue.update_status(&id, TaskStatus::Parsing).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_status_invalid_transition() {
        let queue = TaskQueue::new(3);
        let task = make_task();
        let id = task.id.clone();
        queue.enqueue(task).await;

        // waiting -> completed is invalid
        let result = queue.update_status(&id, TaskStatus::Completed).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrency_invariant() {
        let queue = TaskQueue::new(2);

        // Add 5 tasks
        for _ in 0..5 {
            queue.enqueue(make_task()).await;
        }

        // Promote up to max
        queue.promote_next().await;
        queue.promote_next().await;
        let third = queue.promote_next().await;

        // Third should not be promoted (only 2 allowed in parsing/downloading)
        assert!(third.is_none());
    }
}
