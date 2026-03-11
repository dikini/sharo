use sharo_core::mcp::McpServerSummary;
use sharo_core::skills::SkillCatalogEntry;

use crate::screens::sanitize_for_terminal;

pub fn render_settings(
    model_profile_id: Option<&str>,
    warnings: &[String],
    skills: &[SkillCatalogEntry],
    mcp_servers: &[McpServerSummary],
) -> String {
    let mut out = String::new();
    out.push_str("model:\n");
    out.push_str(&format!(
        "model profile: {}\n",
        sanitize_for_terminal(model_profile_id.unwrap_or("none"))
    ));

    out.push_str("warnings:\n");
    if warnings.is_empty() {
        out.push_str("none\n");
    } else {
        for warning in warnings {
            out.push_str(&format!("{}\n", sanitize_for_terminal(warning)));
        }
    }

    out.push_str("skills:\n");
    if skills.is_empty() {
        out.push_str("none\n");
    } else {
        for skill in skills {
            out.push_str(&format!(
                "{} [{}]\n",
                sanitize_for_terminal(&skill.skill_id),
                if skill.is_active {
                    "active"
                } else {
                    "inactive"
                }
            ));
        }
    }

    out.push_str("mcp servers:\n");
    if mcp_servers.is_empty() {
        out.push_str("none\n");
    } else {
        for server in mcp_servers {
            out.push_str(&format!(
                "{} [enabled={} status={}]\n",
                sanitize_for_terminal(&server.server_id),
                server.enabled,
                mcp_runtime_status_label(server.runtime_status)
            ));
        }
    }

    out
}

fn mcp_runtime_status_label(status: sharo_core::mcp::McpRuntimeStatus) -> &'static str {
    match status {
        sharo_core::mcp::McpRuntimeStatus::Disabled => "disabled",
        sharo_core::mcp::McpRuntimeStatus::Configured => "configured",
    }
}

#[cfg(test)]
mod tests {
    use sharo_core::mcp::{McpRuntimeStatus, McpServerSummary, McpTransportKind};
    use sharo_core::skills::{SkillCatalogEntry, SkillSourceScope};

    use super::render_settings;

    #[test]
    fn settings_screen_groups_skills_mcp_and_model_separately() {
        let rendered = render_settings(
            Some("id-default"),
            &["warn-1".to_string()],
            &[SkillCatalogEntry {
                skill_id: "writing/docs".to_string(),
                name: "Docs".to_string(),
                description: "Write docs".to_string(),
                source_scope: SkillSourceScope::Configured,
                trust_label: "configured".to_string(),
                is_active: true,
            }],
            &[McpServerSummary {
                server_id: "hazel".to_string(),
                display_name: "Hazel".to_string(),
                transport_kind: McpTransportKind::Stdio,
                enabled: true,
                runtime_status: McpRuntimeStatus::Configured,
                startup_timeout_ms: None,
                trust_class: "operator".to_string(),
                diagnostic_summary: None,
            }],
        );

        let model_pos = rendered.find("model:\n").expect("model section");
        let warnings_pos = rendered.find("warnings:\n").expect("warnings section");
        let skills_pos = rendered.find("skills:\n").expect("skills section");
        let mcp_pos = rendered.find("mcp servers:\n").expect("mcp section");
        assert!(model_pos < warnings_pos);
        assert!(warnings_pos < skills_pos);
        assert!(skills_pos < mcp_pos);
    }

    #[test]
    fn settings_screen_uses_runtime_status_instead_of_enabled_flag() {
        let rendered = render_settings(
            Some("id-default"),
            &[],
            &[],
            &[McpServerSummary {
                server_id: "hazel".to_string(),
                display_name: "Hazel".to_string(),
                transport_kind: McpTransportKind::Stdio,
                enabled: true,
                runtime_status: McpRuntimeStatus::Disabled,
                startup_timeout_ms: None,
                trust_class: "operator".to_string(),
                diagnostic_summary: None,
            }],
        );

        assert!(rendered.contains("hazel [enabled=true status=disabled]"));
    }
}
