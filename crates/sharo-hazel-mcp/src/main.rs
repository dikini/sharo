use std::io::{self, BufRead, Write};

use sharo_core::protocol::{
    PrePromptComposeHookInput, ToolCallRequest, ToolCallResponse,
    validate_pre_prompt_compose_input_value,
};
use sharo_hazel_core::retrieval::HazelMemoryCore;
use sharo_hazel_mcp::normalize::normalize_recollection_output;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let core = HazelMemoryCore::default();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<ToolCallRequest>(&line) {
            Ok(call) => {
                if call.tool != "hazel.recollect" {
                    ToolCallResponse {
                        ok: false,
                        output: None,
                        error: Some(format!("unsupported_tool tool={}", call.tool)),
                    }
                } else {
                    match validate_pre_prompt_compose_input_value(&call.input) {
                        Ok(input) => {
                            let output = core.recollect(&PrePromptComposeHookInput {
                                top_k: Some(input.top_k.unwrap_or(3)),
                                token_budget: input.token_budget,
                                relevance_threshold: Some(input.relevance_threshold.unwrap_or(0.0)),
                                ..input
                            });
                            let output_value = serde_json::to_value(output)
                                .expect("serialize recollection payload");
                            match normalize_recollection_output(&output_value) {
                                Ok(validated) => ToolCallResponse {
                                    ok: true,
                                    output: Some(
                                        serde_json::to_value(validated)
                                            .expect("serialize validated recollection"),
                                    ),
                                    error: None,
                                },
                                Err(error) => ToolCallResponse {
                                    ok: false,
                                    output: None,
                                    error: Some(error),
                                },
                            }
                        }
                        Err(error) => ToolCallResponse {
                            ok: false,
                            output: None,
                            error: Some(error),
                        },
                    }
                }
            }
            Err(error) => ToolCallResponse {
                ok: false,
                output: None,
                error: Some(format!("invalid_json error={error}")),
            },
        };
        let payload = serde_json::to_string(&response).expect("serialize tool response");
        let _ = writeln!(stdout, "{payload}");
        let _ = stdout.flush();
    }
}
