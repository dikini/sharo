use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::protocol::{
    SubmitTaskRequest, SubmitTaskResponse, TaskState, TaskStatusRequest, TaskStatusResponse,
};

pub trait RuntimeClient {
    fn submit(&self, request: &SubmitTaskRequest) -> SubmitTaskResponse;
    fn status(&self, request: &TaskStatusRequest) -> TaskStatusResponse;
}

#[derive(Debug, Default, Clone)]
pub struct StubClient;

impl StubClient {
    fn stable_task_id(session_id: Option<&str>, goal: &str) -> String {
        let mut hasher = DefaultHasher::new();
        session_id.unwrap_or("").hash(&mut hasher);
        goal.hash(&mut hasher);
        format!("task-{:#08x}", (hasher.finish() & 0xffff_ffff) as u32)
    }
}

impl RuntimeClient for StubClient {
    fn submit(&self, request: &SubmitTaskRequest) -> SubmitTaskResponse {
        SubmitTaskResponse {
            task_id: Self::stable_task_id(request.session_id.as_deref(), &request.goal),
            state: TaskState::Submitted,
        }
    }

    fn status(&self, request: &TaskStatusRequest) -> TaskStatusResponse {
        let state = if request.task_id.ends_with('0') || request.task_id.ends_with('5') {
            TaskState::Running
        } else {
            TaskState::Succeeded
        };

        let summary = match state {
            TaskState::Running => "in progress".to_string(),
            TaskState::Succeeded => "completed".to_string(),
            TaskState::Submitted => "submitted".to_string(),
            TaskState::Failed => "failed".to_string(),
            TaskState::Blocked => "blocked".to_string(),
        };

        TaskStatusResponse {
            task_id: request.task_id.clone(),
            state,
            summary,
        }
    }
}
