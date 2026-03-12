use sharo_core::mcp::{McpRuntimeStatus, McpServerSummary, McpTransportKind, RuntimeStatusSummary};
use sharo_core::protocol::{
    ApprovalSummary, ArtifactSummary, DaemonRequest, DaemonResponse, GetArtifactsResponse,
    GetHazelProposalBatchResponse, GetHazelSleepJobResponse, GetHazelStatusResponse,
    GetRuntimeStatusResponse, GetSessionViewResponse, GetSkillRequest, GetSkillResponse,
    GetTaskResponse, GetTraceResponse, HazelActionAvailability, HazelCardView,
    HazelLimitsSummary, HazelProposalBatchView, HazelSleepJobState, HazelSleepJobView,
    HazelStatusSummary, ListHazelCardsResponse, ListHazelProposalBatchesResponse,
    ListHazelSleepJobsResponse, ListMcpServersResponse, ListSessionsResponse, ListSkillsRequest,
    ListSkillsResponse, ProvenanceRef, RecollectionCardKind, RecollectionCardState,
    ResolveApprovalRequest, ResolveApprovalResponse, SessionSummary, SessionView,
    SetSessionSkillsRequest, SetSessionSkillsResponse, SubmitTaskRequest, SubmitTaskResponse,
    TaskState, TaskStatusRequest, TaskStatusResponse, TaskSummary, TraceEventSummary,
    TraceSummary, UpdateMcpServerStateRequest, UpdateMcpServerStateResponse,
};
use sharo_core::skills::{SkillCatalogEntry, SkillDocument, SkillSourceScope};

#[test]
fn ipc_submit_envelope_roundtrip() {
    let request = DaemonRequest::Submit(SubmitTaskRequest {
        session_id: Some("session-a".to_string()),
        goal: "read docs".to_string(),
    });

    let json = serde_json::to_string(&request).expect("serialize submit request");
    let parsed: DaemonRequest = serde_json::from_str(&json).expect("deserialize submit request");

    match parsed {
        DaemonRequest::Submit(payload) => {
            assert_eq!(payload.session_id.as_deref(), Some("session-a"));
            assert_eq!(payload.goal, "read docs");
        }
        _ => panic!("expected submit request"),
    }
}

#[test]
fn ipc_status_envelope_roundtrip() {
    let response = DaemonResponse::Status(TaskStatusResponse {
        task_id: "task-0001".to_string(),
        state: TaskState::Succeeded,
        summary: "completed".to_string(),
    });

    let json = serde_json::to_string(&response).expect("serialize status response");
    let parsed: DaemonResponse = serde_json::from_str(&json).expect("deserialize status response");

    match parsed {
        DaemonResponse::Status(payload) => {
            assert_eq!(payload.task_id, "task-0001");
            assert_eq!(payload.state, TaskState::Succeeded);
            assert_eq!(payload.summary, "completed");
        }
        _ => panic!("expected status response"),
    }
}

#[test]
fn response_variant_matches_request_kind() {
    let submit_req = DaemonRequest::Submit(SubmitTaskRequest {
        session_id: None,
        goal: "g".to_string(),
    });
    let submit_resp = DaemonResponse::Submit(SubmitTaskResponse {
        task_id: "task-1".to_string(),
        state: TaskState::Submitted,
    });

    let status_req = DaemonRequest::Status(TaskStatusRequest {
        task_id: "task-1".to_string(),
    });
    let status_resp = DaemonResponse::Status(TaskStatusResponse {
        task_id: "task-1".to_string(),
        state: TaskState::Running,
        summary: "in progress".to_string(),
    });

    assert!(matches!(submit_req, DaemonRequest::Submit(_)));
    assert!(matches!(submit_resp, DaemonResponse::Submit(_)));
    assert!(matches!(status_req, DaemonRequest::Status(_)));
    assert!(matches!(status_resp, DaemonResponse::Status(_)));
}

#[test]
fn approval_envelope_roundtrip() {
    let resolve_req = DaemonRequest::ResolveApproval(ResolveApprovalRequest {
        approval_id: "approval-000001".to_string(),
        decision: "approve".to_string(),
    });
    let resolve_req_json = serde_json::to_string(&resolve_req).expect("serialize resolve request");
    let resolve_req_parsed: DaemonRequest =
        serde_json::from_str(&resolve_req_json).expect("deserialize resolve request");
    assert!(matches!(
        resolve_req_parsed,
        DaemonRequest::ResolveApproval(_)
    ));

    let resolve_resp = DaemonResponse::ResolveApproval(ResolveApprovalResponse {
        approval_id: "approval-000001".to_string(),
        task_id: "task-000001".to_string(),
        state: "approved".to_string(),
    });
    let resolve_resp_json =
        serde_json::to_string(&resolve_resp).expect("serialize resolve response");
    let resolve_resp_parsed: DaemonResponse =
        serde_json::from_str(&resolve_resp_json).expect("deserialize resolve response");
    assert!(matches!(
        resolve_resp_parsed,
        DaemonResponse::ResolveApproval(_)
    ));

    let list_req = DaemonRequest::ListPendingApprovals;
    let list_req_json = serde_json::to_string(&list_req).expect("serialize list request");
    let list_req_parsed: DaemonRequest =
        serde_json::from_str(&list_req_json).expect("deserialize list request");
    assert!(matches!(
        list_req_parsed,
        DaemonRequest::ListPendingApprovals
    ));
}

#[test]
fn trace_and_artifact_envelopes_include_conformance_fields() {
    let task_resp = DaemonResponse::GetTask(GetTaskResponse {
        task: TaskSummary {
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            task_state: "succeeded".to_string(),
            current_step_summary: "done".to_string(),
            blocking_reason: None,
            coordination_summary: None,
            result_preview: Some("preview".to_string()),
        },
    });
    let task_json = serde_json::to_string(&task_resp).expect("serialize task response");
    let task_parsed: DaemonResponse =
        serde_json::from_str(&task_json).expect("deserialize task response");
    match task_parsed {
        DaemonResponse::GetTask(payload) => {
            assert_eq!(payload.task.result_preview.as_deref(), Some("preview"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let trace_resp = DaemonResponse::GetTrace(GetTraceResponse {
        trace: TraceSummary {
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            events: vec![TraceEventSummary {
                event_sequence: 1,
                event_kind: "task_submitted".to_string(),
                details: "goal".to_string(),
            }],
        },
    });
    let trace_json = serde_json::to_string(&trace_resp).expect("serialize trace response");
    let trace_parsed: DaemonResponse =
        serde_json::from_str(&trace_json).expect("deserialize trace response");
    match trace_parsed {
        DaemonResponse::GetTrace(payload) => {
            assert_eq!(payload.trace.session_id, "session-1");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let artifacts_resp = DaemonResponse::GetArtifacts(GetArtifactsResponse {
        artifacts: vec![ArtifactSummary {
            artifact_id: "artifact-1".to_string(),
            artifact_kind: "verification_result".to_string(),
            summary: "ok".to_string(),
            produced_by_step_id: "step-1".to_string(),
            produced_by_trace_event_sequence: 3,
        }],
    });
    let artifacts_json =
        serde_json::to_string(&artifacts_resp).expect("serialize artifacts response");
    let artifacts_parsed: DaemonResponse =
        serde_json::from_str(&artifacts_json).expect("deserialize artifacts response");
    match artifacts_parsed {
        DaemonResponse::GetArtifacts(payload) => {
            assert_eq!(payload.artifacts[0].produced_by_step_id, "step-1");
            assert_eq!(payload.artifacts[0].produced_by_trace_event_sequence, 3);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn session_control_plane_envelopes_roundtrip() {
    let list_request =
        DaemonRequest::GetSessionTasks(sharo_core::protocol::GetSessionTasksRequest {
            session_id: "session-1".to_string(),
            task_limit: Some(5),
        });
    let list_request_json =
        serde_json::to_string(&list_request).expect("serialize get session tasks request");
    let list_request_parsed: DaemonRequest =
        serde_json::from_str(&list_request_json).expect("deserialize get session tasks request");
    match list_request_parsed {
        DaemonRequest::GetSessionTasks(payload) => {
            assert_eq!(payload.session_id, "session-1");
            assert_eq!(payload.task_limit, Some(5));
        }
        other => panic!("unexpected request: {other:?}"),
    }

    let sessions_resp = DaemonResponse::ListSessions(ListSessionsResponse {
        sessions: vec![SessionSummary {
            session_id: "session-1".to_string(),
            session_label: "alpha".to_string(),
            session_status: "awaiting_approval".to_string(),
            activity_sequence: 7,
            latest_task_id: Some("task-1".to_string()),
            latest_task_state: Some("awaiting_approval".to_string()),
            latest_result_preview: None,
            has_pending_approval: true,
        }],
    });
    let sessions_json = serde_json::to_string(&sessions_resp).expect("serialize list sessions");
    let sessions_parsed: DaemonResponse =
        serde_json::from_str(&sessions_json).expect("deserialize list sessions");
    match sessions_parsed {
        DaemonResponse::ListSessions(payload) => {
            assert_eq!(payload.sessions[0].session_status, "awaiting_approval");
            assert_eq!(payload.sessions[0].activity_sequence, 7);
            assert!(payload.sessions[0].has_pending_approval);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let session_view_resp = DaemonResponse::GetSessionView(GetSessionViewResponse {
        session: SessionView {
            session_id: "session-1".to_string(),
            session_label: "alpha".to_string(),
            tasks: vec![TaskSummary {
                task_id: "task-1".to_string(),
                session_id: "session-1".to_string(),
                task_state: "awaiting_approval".to_string(),
                current_step_summary: "awaiting approval".to_string(),
                blocking_reason: Some("approval_required approval_id=approval-1".to_string()),
                coordination_summary: None,
                result_preview: None,
            }],
            pending_approvals: vec![ApprovalSummary {
                approval_id: "approval-1".to_string(),
                task_id: "task-1".to_string(),
                state: "pending".to_string(),
                reason: "policy require_approval".to_string(),
            }],
            latest_result_preview: None,
            active_blocking_task_id: Some("task-1".to_string()),
        },
    });
    let session_view_request =
        DaemonRequest::GetSessionView(sharo_core::protocol::GetSessionViewRequest {
            session_id: "session-1".to_string(),
            task_limit: Some(10),
        });
    let session_view_request_json =
        serde_json::to_string(&session_view_request).expect("serialize session view request");
    let session_view_request_parsed: DaemonRequest =
        serde_json::from_str(&session_view_request_json).expect("deserialize session view request");
    match session_view_request_parsed {
        DaemonRequest::GetSessionView(payload) => {
            assert_eq!(payload.session_id, "session-1");
            assert_eq!(payload.task_limit, Some(10));
        }
        other => panic!("unexpected request: {other:?}"),
    }
    let session_view_json =
        serde_json::to_string(&session_view_resp).expect("serialize session view");
    let session_view_parsed: DaemonResponse =
        serde_json::from_str(&session_view_json).expect("deserialize session view");
    match session_view_parsed {
        DaemonResponse::GetSessionView(payload) => {
            assert_eq!(payload.session.pending_approvals.len(), 1);
            assert_eq!(
                payload.session.active_blocking_task_id.as_deref(),
                Some("task-1")
            );
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn skills_control_plane_envelopes_roundtrip() {
    let list_request = DaemonRequest::ListSkills(ListSkillsRequest {
        session_id: Some("session-1".to_string()),
    });
    let list_request_json =
        serde_json::to_string(&list_request).expect("serialize list skills request");
    let list_request_parsed: DaemonRequest =
        serde_json::from_str(&list_request_json).expect("deserialize list skills request");
    match list_request_parsed {
        DaemonRequest::ListSkills(payload) => {
            assert_eq!(payload.session_id.as_deref(), Some("session-1"));
        }
        other => panic!("unexpected request: {other:?}"),
    }

    let list_response = DaemonResponse::ListSkills(ListSkillsResponse {
        skills: vec![SkillCatalogEntry {
            skill_id: "writing/docs/strict-plan".to_string(),
            name: "Strict Plan".to_string(),
            description: "Enforce structured planning".to_string(),
            source_scope: SkillSourceScope::Project,
            trust_label: "project".to_string(),
            is_active: true,
        }],
    });
    let list_response_json =
        serde_json::to_string(&list_response).expect("serialize list skills response");
    let list_response_parsed: DaemonResponse =
        serde_json::from_str(&list_response_json).expect("deserialize list skills response");
    match list_response_parsed {
        DaemonResponse::ListSkills(payload) => {
            assert_eq!(payload.skills[0].skill_id, "writing/docs/strict-plan");
            assert!(payload.skills[0].is_active);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let get_request = DaemonRequest::GetSkill(GetSkillRequest {
        skill_id: "writing/docs/strict-plan".to_string(),
    });
    let get_request_json =
        serde_json::to_string(&get_request).expect("serialize get skill request");
    let get_request_parsed: DaemonRequest =
        serde_json::from_str(&get_request_json).expect("deserialize get skill request");
    match get_request_parsed {
        DaemonRequest::GetSkill(payload) => {
            assert_eq!(payload.skill_id, "writing/docs/strict-plan");
        }
        other => panic!("unexpected request: {other:?}"),
    }

    let get_response = DaemonResponse::GetSkill(GetSkillResponse {
        skill: SkillDocument {
            skill_id: "writing/docs/strict-plan".to_string(),
            name: "Strict Plan".to_string(),
            description: "Enforce structured planning".to_string(),
            source_scope: SkillSourceScope::Project,
            trust_label: "project".to_string(),
            markdown: "# Strict Plan\n\nUse the plan format.".to_string(),
            has_scripts: false,
            has_references: false,
            has_assets: false,
        },
    });
    let get_response_json =
        serde_json::to_string(&get_response).expect("serialize get skill response");
    let get_response_parsed: DaemonResponse =
        serde_json::from_str(&get_response_json).expect("deserialize get skill response");
    match get_response_parsed {
        DaemonResponse::GetSkill(payload) => {
            assert!(payload.skill.markdown.contains("# Strict Plan"));
            assert_eq!(payload.skill.source_scope, SkillSourceScope::Project);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let set_request = DaemonRequest::SetSessionSkills(SetSessionSkillsRequest {
        session_id: "session-1".to_string(),
        active_skill_ids: vec![
            "writing/docs/strict-plan".to_string(),
            "brainstorming".to_string(),
        ],
    });
    let set_request_json =
        serde_json::to_string(&set_request).expect("serialize set session skills request");
    let set_request_parsed: DaemonRequest =
        serde_json::from_str(&set_request_json).expect("deserialize set session skills request");
    match set_request_parsed {
        DaemonRequest::SetSessionSkills(payload) => {
            assert_eq!(payload.session_id, "session-1");
            assert_eq!(payload.active_skill_ids.len(), 2);
        }
        other => panic!("unexpected request: {other:?}"),
    }

    let set_response = DaemonResponse::SetSessionSkills(SetSessionSkillsResponse {
        session_id: "session-1".to_string(),
        active_skill_ids: vec![
            "brainstorming".to_string(),
            "writing/docs/strict-plan".to_string(),
        ],
    });
    let set_response_json =
        serde_json::to_string(&set_response).expect("serialize set session skills response");
    let set_response_parsed: DaemonResponse =
        serde_json::from_str(&set_response_json).expect("deserialize set session skills response");
    match set_response_parsed {
        DaemonResponse::SetSessionSkills(payload) => {
            assert_eq!(payload.session_id, "session-1");
            assert_eq!(
                payload.active_skill_ids,
                vec![
                    "brainstorming".to_string(),
                    "writing/docs/strict-plan".to_string()
                ]
            );
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn mcp_control_plane_envelopes_roundtrip() {
    let list = DaemonResponse::ListMcpServers(ListMcpServersResponse {
        servers: vec![McpServerSummary {
            server_id: "hazel".to_string(),
            display_name: "Hazel".to_string(),
            transport_kind: McpTransportKind::Stdio,
            enabled: true,
            runtime_status: McpRuntimeStatus::Configured,
            startup_timeout_ms: Some(250),
            trust_class: "operator".to_string(),
            diagnostic_summary: Some("stdio command=/usr/bin/hazel-mcp".to_string()),
        }],
    });
    let json = serde_json::to_string(&list).expect("serialize list mcp");
    let parsed: DaemonResponse = serde_json::from_str(&json).expect("deserialize list mcp");
    match parsed {
        DaemonResponse::ListMcpServers(payload) => {
            assert_eq!(payload.servers[0].server_id, "hazel");
            assert!(payload.servers[0].enabled);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let update_req = DaemonRequest::UpdateMcpServerState(UpdateMcpServerStateRequest {
        server_id: "hazel".to_string(),
        enabled: false,
    });
    let update_req_json = serde_json::to_string(&update_req).expect("serialize update mcp");
    let update_req_parsed: DaemonRequest =
        serde_json::from_str(&update_req_json).expect("deserialize update mcp");
    match update_req_parsed {
        DaemonRequest::UpdateMcpServerState(payload) => {
            assert_eq!(payload.server_id, "hazel");
            assert!(!payload.enabled);
        }
        other => panic!("unexpected request: {other:?}"),
    }

    let update_resp = DaemonResponse::UpdateMcpServerState(UpdateMcpServerStateResponse {
        server: McpServerSummary {
            server_id: "hazel".to_string(),
            display_name: "Hazel".to_string(),
            transport_kind: McpTransportKind::Stdio,
            enabled: false,
            runtime_status: McpRuntimeStatus::Disabled,
            startup_timeout_ms: Some(250),
            trust_class: "operator".to_string(),
            diagnostic_summary: None,
        },
    });
    let update_resp_json =
        serde_json::to_string(&update_resp).expect("serialize update mcp response");
    let update_resp_parsed: DaemonResponse =
        serde_json::from_str(&update_resp_json).expect("deserialize update mcp response");
    match update_resp_parsed {
        DaemonResponse::UpdateMcpServerState(payload) => {
            assert_eq!(payload.server.runtime_status, McpRuntimeStatus::Disabled);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let runtime = DaemonResponse::GetRuntimeStatus(GetRuntimeStatusResponse {
        status: RuntimeStatusSummary {
            daemon_ready: true,
            config_loaded: true,
            model_profile_id: Some("id-default".to_string()),
            mcp_enabled_count: 1,
            mcp_disabled_count: 1,
            warnings: vec![],
        },
    });
    let runtime_json = serde_json::to_string(&runtime).expect("serialize runtime");
    let runtime_parsed: DaemonResponse =
        serde_json::from_str(&runtime_json).expect("deserialize runtime");
    match runtime_parsed {
        DaemonResponse::GetRuntimeStatus(payload) => {
            assert_eq!(payload.status.mcp_enabled_count, 1);
            assert_eq!(payload.status.mcp_disabled_count, 1);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn hazel_control_plane_envelopes_roundtrip() {
    let status_request = DaemonRequest::GetHazelStatus;
    let status_request_json =
        serde_json::to_string(&status_request).expect("serialize hazel status request");
    let status_request_parsed: DaemonRequest =
        serde_json::from_str(&status_request_json).expect("deserialize hazel status request");
    assert!(matches!(status_request_parsed, DaemonRequest::GetHazelStatus));

    let status_response = DaemonResponse::GetHazelStatus(GetHazelStatusResponse {
        status: HazelStatusSummary {
            available: true,
            card_count: 2,
            proposal_batch_count: 1,
            sleep_job_count: 1,
            actions: HazelActionAvailability {
                retrieval_preview: true,
                validate_batch: true,
                submit_batch: true,
                enqueue_sleep_job: true,
                cancel_sleep_job: true,
            },
            limits: HazelLimitsSummary {
                max_list_items: 64,
                max_preview_cards: 8,
                max_sleep_batches: 8,
                max_sleep_proposals_per_batch: 64,
            },
        },
    });
    let status_response_json =
        serde_json::to_string(&status_response).expect("serialize hazel status response");
    let status_response_parsed: DaemonResponse =
        serde_json::from_str(&status_response_json).expect("deserialize hazel status response");
    match status_response_parsed {
        DaemonResponse::GetHazelStatus(payload) => {
            assert!(payload.status.available);
            assert_eq!(payload.status.limits.max_list_items, 64);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let cards_request =
        DaemonRequest::ListHazelCards(sharo_core::protocol::ListHazelCardsRequest { limit: Some(8) });
    let cards_request_json =
        serde_json::to_string(&cards_request).expect("serialize list hazel cards request");
    let cards_request_parsed: DaemonRequest =
        serde_json::from_str(&cards_request_json).expect("deserialize list hazel cards request");
    match cards_request_parsed {
        DaemonRequest::ListHazelCards(payload) => assert_eq!(payload.limit, Some(8)),
        other => panic!("unexpected request: {other:?}"),
    }

    let card = HazelCardView {
        card_id: "hazel-memory-1".to_string(),
        kind: RecollectionCardKind::AssociationCue,
        state: RecollectionCardState::Active,
        subject: "hazel".to_string(),
        text: "hazel is structured memory".to_string(),
        provenance: vec![ProvenanceRef {
            source_ref: "hazel:assertion/hazel-memory-1".to_string(),
            source_excerpt: Some("support=5 contradiction=0 confidence_milli=920".to_string()),
        }],
        policy_ids: vec!["hunch.v1".to_string()],
    };
    let cards_response = DaemonResponse::ListHazelCards(ListHazelCardsResponse {
        cards: vec![card.clone()],
    });
    let cards_response_json =
        serde_json::to_string(&cards_response).expect("serialize list hazel cards response");
    let cards_response_parsed: DaemonResponse =
        serde_json::from_str(&cards_response_json).expect("deserialize list hazel cards response");
    match cards_response_parsed {
        DaemonResponse::ListHazelCards(payload) => {
            assert_eq!(
                payload.cards[0].provenance[0].source_ref,
                "hazel:assertion/hazel-memory-1"
            );
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let batch_response = DaemonResponse::GetHazelProposalBatch(GetHazelProposalBatchResponse {
        batch: HazelProposalBatchView {
            batch_id: "batch-1".to_string(),
            idempotency_key: "idemp-1".to_string(),
            source_ref: "note:hazel".to_string(),
            producer: "operator".to_string(),
            proposal_count: 2,
        },
    });
    let batch_response_json =
        serde_json::to_string(&batch_response).expect("serialize hazel proposal batch response");
    let batch_response_parsed: DaemonResponse = serde_json::from_str(&batch_response_json)
        .expect("deserialize hazel proposal batch response");
    match batch_response_parsed {
        DaemonResponse::GetHazelProposalBatch(payload) => {
            assert_eq!(payload.batch.source_ref, "note:hazel");
            assert_eq!(payload.batch.proposal_count, 2);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let jobs_response = DaemonResponse::ListHazelSleepJobs(ListHazelSleepJobsResponse {
        jobs: vec![HazelSleepJobView {
            job_id: "job-1".to_string(),
            state: HazelSleepJobState::Completed,
            run_id: Some("sleep-run-v2-1234".to_string()),
            proposal_batch_ids: vec!["batch-1".to_string()],
            summary: "completed with one batch".to_string(),
        }],
    });
    let jobs_response_json =
        serde_json::to_string(&jobs_response).expect("serialize hazel sleep jobs response");
    let jobs_response_parsed: DaemonResponse =
        serde_json::from_str(&jobs_response_json).expect("deserialize hazel sleep jobs response");
    match jobs_response_parsed {
        DaemonResponse::ListHazelSleepJobs(payload) => {
            assert_eq!(payload.jobs[0].state, HazelSleepJobState::Completed);
            assert_eq!(payload.jobs[0].proposal_batch_ids, vec!["batch-1".to_string()]);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let job_response = DaemonResponse::GetHazelSleepJob(GetHazelSleepJobResponse {
        job: HazelSleepJobView {
            job_id: "job-1".to_string(),
            state: HazelSleepJobState::Completed,
            run_id: Some("sleep-run-v2-1234".to_string()),
            proposal_batch_ids: vec!["batch-1".to_string()],
            summary: "completed with one batch".to_string(),
        },
    });
    let job_response_json =
        serde_json::to_string(&job_response).expect("serialize hazel sleep job response");
    let job_response_parsed: DaemonResponse =
        serde_json::from_str(&job_response_json).expect("deserialize hazel sleep job response");
    match job_response_parsed {
        DaemonResponse::GetHazelSleepJob(payload) => {
            assert_eq!(payload.job.run_id.as_deref(), Some("sleep-run-v2-1234"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let list_batches_response =
        DaemonResponse::ListHazelProposalBatches(ListHazelProposalBatchesResponse {
            batches: vec![HazelProposalBatchView {
                batch_id: "batch-1".to_string(),
                idempotency_key: "idemp-1".to_string(),
                source_ref: "note:hazel".to_string(),
                producer: "operator".to_string(),
                proposal_count: 2,
            }],
        });
    let list_batches_json =
        serde_json::to_string(&list_batches_response).expect("serialize hazel batches response");
    let list_batches_parsed: DaemonResponse =
        serde_json::from_str(&list_batches_json).expect("deserialize hazel batches response");
    match list_batches_parsed {
        DaemonResponse::ListHazelProposalBatches(payload) => {
            assert_eq!(payload.batches[0].producer, "operator");
        }
        other => panic!("unexpected response: {other:?}"),
    }
}
