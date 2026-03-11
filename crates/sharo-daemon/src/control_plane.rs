use sharo_core::protocol::{
    GetSessionTasksResponse, GetSessionViewResponse, ListSessionsResponse, SessionView,
};

use crate::store::Store;

const MAX_SESSION_TASKS: usize = 100;

pub fn list_sessions(store: &Store) -> ListSessionsResponse {
    ListSessionsResponse {
        sessions: store.list_sessions(),
    }
}

pub fn get_session_tasks(
    store: &Store,
    session_id: &str,
    task_limit: Option<u32>,
) -> Option<GetSessionTasksResponse> {
    store
        .has_session(session_id)
        .then(|| GetSessionTasksResponse {
            tasks: bounded_recent_tasks(store.list_session_tasks(session_id), task_limit),
        })
}

pub fn get_session_view(
    store: &Store,
    session_id: &str,
    task_limit: Option<u32>,
) -> Option<GetSessionViewResponse> {
    let session_label = store.session_label(session_id)?;
    let tasks = bounded_recent_tasks(store.list_session_tasks(session_id), task_limit);
    let pending_approvals = store.list_pending_approvals_for_session(session_id);
    let latest_task = tasks.last();
    let latest_result_preview = tasks
        .iter()
        .rev()
        .find_map(|task| task.result_preview.clone());
    let active_blocking_task_id = latest_task
        .filter(|task| {
            task.blocking_reason.is_some()
                && matches!(task.task_state.as_str(), "awaiting_approval" | "blocked")
        })
        .map(|task| task.task_id.clone());

    Some(GetSessionViewResponse {
        session: SessionView {
            session_id: session_id.to_string(),
            session_label,
            tasks,
            pending_approvals,
            latest_result_preview,
            active_blocking_task_id,
        },
    })
}

fn bounded_recent_tasks(
    mut tasks: Vec<sharo_core::protocol::TaskSummary>,
    task_limit: Option<u32>,
) -> Vec<sharo_core::protocol::TaskSummary> {
    let limit = task_limit
        .map(|limit| limit as usize)
        .unwrap_or(MAX_SESSION_TASKS)
        .min(MAX_SESSION_TASKS);
    if tasks.len() > limit {
        let keep_from = tasks.len() - limit;
        tasks.drain(0..keep_from);
    }
    tasks
}

#[cfg(test)]
mod tests {
    use std::fs;

    use proptest::prelude::*;

    use super::*;
    use crate::store::Store;
    use sharo_core::protocol::{SubmitTaskOpRequest, TaskSummary};

    fn temp_store(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}.json"))
    }

    #[test]
    fn session_view_surfaces_latest_result_preview() {
        let path = temp_store("session-view-preview");
        let mut store = Store::open(&path).expect("open store");
        let session_id = store.register_session("alpha").expect("register session");
        let preparation = store
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some(session_id.clone()),
                goal: "read alpha".to_string(),
                idempotency_key: None,
            })
            .expect("prepare submit");
        let preparation = match preparation {
            crate::store::SubmitPreparationOutcome::Ready(preparation) => preparation,
            crate::store::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        };
        store
            .submit_task_with_route(
                &preparation,
                SubmitTaskOpRequest {
                    session_id: Some(session_id.clone()),
                    goal: "read alpha".to_string(),
                    idempotency_key: None,
                },
                "local_mock",
                "alpha preview text",
                &[],
            )
            .expect("submit task");

        let session = get_session_view(&store, &session_id, None)
            .expect("session view")
            .session;
        assert_eq!(
            session.latest_result_preview,
            Some("alpha preview text".to_string())
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn derived_session_view_never_mutates_canonical_store_state() {
        let path = temp_store("session-view-invariant");
        let mut store = Store::open(&path).expect("open store");
        let session_id = store.register_session("alpha").expect("register session");
        let preparation = store
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some(session_id.clone()),
                goal: "restricted: inspect".to_string(),
                idempotency_key: None,
            })
            .expect("prepare submit");
        let preparation = match preparation {
            crate::store::SubmitPreparationOutcome::Ready(preparation) => preparation,
            crate::store::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        };
        let before_tasks = store.list_session_tasks(&session_id);
        let before_approvals = store.list_pending_approvals_for_session(&session_id);
        store
            .submit_task_with_route(
                &preparation,
                SubmitTaskOpRequest {
                    session_id: Some(session_id.clone()),
                    goal: "restricted: inspect".to_string(),
                    idempotency_key: None,
                },
                "local_mock",
                "blocked preview text",
                &[],
            )
            .expect("submit task");

        let task_count_before_view = store.list_session_tasks(&session_id).len();
        let approval_count_before_view =
            store.list_pending_approvals_for_session(&session_id).len();
        let _ = get_session_view(&store, &session_id, None).expect("session view");
        assert_eq!(
            store.list_session_tasks(&session_id).len(),
            task_count_before_view
        );
        assert_eq!(
            store.list_pending_approvals_for_session(&session_id).len(),
            approval_count_before_view
        );
        assert!(store.list_session_tasks(&session_id).len() >= before_tasks.len());
        assert!(
            store.list_pending_approvals_for_session(&session_id).len() >= before_approvals.len()
        );

        let _ = fs::remove_file(path);
    }

    fn sort_tasks(tasks: &mut [TaskSummary]) {
        tasks.sort_by_key(|task| {
            task.task_id
                .rsplit('-')
                .next()
                .and_then(|suffix| suffix.parse::<u64>().ok())
                .unwrap_or(0)
        });
    }

    proptest! {
        #[test]
        fn session_task_order_is_monotonic_under_valid_store_state(ids in proptest::collection::vec(1u64..1000, 1..16)) {
            let mut tasks = ids.into_iter().rev().map(|id| TaskSummary {
                task_id: format!("task-{id:06}"),
                session_id: "session-000001".to_string(),
                task_state: "succeeded".to_string(),
                current_step_summary: "done".to_string(),
                blocking_reason: None,
                coordination_summary: None,
                result_preview: None,
            }).collect::<Vec<_>>();

            sort_tasks(&mut tasks);

            let sequences = tasks.iter().map(|task| {
                task.task_id
                    .rsplit('-')
                    .next()
                    .and_then(|suffix| suffix.parse::<u64>().ok())
                    .unwrap_or(0)
            }).collect::<Vec<_>>();
            prop_assert!(sequences.windows(2).all(|window| window[0] <= window[1]));
        }
    }

    #[test]
    fn session_view_enforces_task_limit() {
        let path = temp_store("session-view-limit");
        let mut store = Store::open(&path).expect("open store");
        let session_id = store.register_session("alpha").expect("register session");

        for index in 0..6 {
            let goal = format!("read alpha {index}");
            let preparation = store
                .prepare_submit(&SubmitTaskOpRequest {
                    session_id: Some(session_id.clone()),
                    goal: goal.clone(),
                    idempotency_key: None,
                })
                .expect("prepare submit");
            let preparation = match preparation {
                crate::store::SubmitPreparationOutcome::Ready(preparation) => preparation,
                crate::store::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
            };
            store
                .submit_task_with_route(
                    &preparation,
                    SubmitTaskOpRequest {
                        session_id: Some(session_id.clone()),
                        goal,
                        idempotency_key: None,
                    },
                    "local_mock",
                    "alpha preview text",
                    &[],
                )
                .expect("submit task");
        }

        let session = get_session_view(&store, &session_id, Some(3))
            .expect("session view")
            .session;
        assert_eq!(session.tasks.len(), 3);
        assert_eq!(session.tasks[0].task_id, "task-000004");
        assert_eq!(session.tasks[2].task_id, "task-000006");

        let _ = fs::remove_file(path);
    }
}
