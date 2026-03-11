use sharo_core::protocol::{ArtifactSummary, TraceSummary};

use crate::screens::sanitize_for_terminal;

pub fn render_trace_artifacts(
    task_id: Option<&str>,
    trace: Option<&TraceSummary>,
    artifacts: &[ArtifactSummary],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "selected task: {}\n",
        sanitize_for_terminal(task_id.unwrap_or("none"))
    ));

    match trace {
        Some(trace) => {
            out.push_str(&format!(
                "trace: {} task={} session={}\n",
                sanitize_for_terminal(&trace.trace_id),
                sanitize_for_terminal(&trace.task_id),
                sanitize_for_terminal(&trace.session_id)
            ));
            for event in &trace.events {
                out.push_str(&format!(
                    "event {} [{}] {}\n",
                    event.event_sequence,
                    sanitize_for_terminal(&event.event_kind),
                    sanitize_for_terminal(&event.details)
                ));
            }
        }
        None => out.push_str("trace: none\n"),
    }

    if artifacts.is_empty() {
        out.push_str("artifacts: none\n");
    } else {
        out.push_str("artifacts:\n");
        for artifact in artifacts {
            out.push_str(&format!(
                "{} [{}] step={} event={} {}\n",
                sanitize_for_terminal(&artifact.artifact_id),
                sanitize_for_terminal(&artifact.artifact_kind),
                sanitize_for_terminal(&artifact.produced_by_step_id),
                artifact.produced_by_trace_event_sequence,
                sanitize_for_terminal(&artifact.summary)
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use sharo_core::protocol::{ArtifactSummary, TraceEventSummary, TraceSummary};

    use super::render_trace_artifacts;

    #[test]
    fn artifact_screen_uses_exact_record_ids_from_daemon_state() {
        let rendered = render_trace_artifacts(
            Some("task-9"),
            Some(&TraceSummary {
                trace_id: "trace-9".to_string(),
                task_id: "task-9".to_string(),
                session_id: "session-2".to_string(),
                events: vec![TraceEventSummary {
                    event_sequence: 7,
                    event_kind: "route_decision".to_string(),
                    details: "deterministic".to_string(),
                }],
            }),
            &[ArtifactSummary {
                artifact_id: "artifact-9-final".to_string(),
                artifact_kind: "final_result".to_string(),
                summary: "done".to_string(),
                produced_by_step_id: "step-task-9".to_string(),
                produced_by_trace_event_sequence: 7,
            }],
        );

        assert!(rendered.contains("trace: trace-9 task=task-9 session=session-2"));
        assert!(rendered.contains("artifact-9-final [final_result] step=step-task-9 event=7"));
    }
}
