use std::io::{self, BufRead, Read, Write};

use sharo_core::protocol::{
    HookSchemaDescriptor, PrePromptComposeHookInput, ToolCallRequest, ToolCallResponse,
    expected_pre_prompt_compose_input_schema, expected_recollection_output_schema,
    validate_pre_prompt_compose_input_value,
};
use sharo_hazel_core::retrieval::HazelMemoryCore;
use sharo_hazel_mcp::normalize::normalize_recollection_output;
use sharo_hazel_mcp::wire::{WireRequestFrame, line_content_len, parse_wire_request_frame};

const MAX_REQUEST_BYTES: usize = 131_072;

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();
    let core = HazelMemoryCore::default();
    loop {
        let mut line_bytes = Vec::new();
        let read = {
            let mut limited = reader.by_ref().take((MAX_REQUEST_BYTES + 1) as u64);
            match limited.read_until(b'\n', &mut line_bytes) {
                Ok(read) => read,
                Err(_) => break,
            }
        };
        if read == 0 {
            break;
        }
        let content_len = line_content_len(&line_bytes);
        let oversized = content_len > MAX_REQUEST_BYTES;
        let terminated = line_bytes.last() == Some(&b'\n');
        if oversized {
            if !terminated && drain_until_newline(&mut reader).is_err() {
                let response = response_error("request_stream_drain_failed".to_string());
                let _ = write_response(&mut stdout, &response);
                break;
            }
            let response =
                response_error(format!("request_too_large max_bytes={MAX_REQUEST_BYTES}"));
            if write_response(&mut stdout, &response).is_err() {
                break;
            }
            continue;
        }
        let response = match parse_wire_request_frame(&line_bytes, MAX_REQUEST_BYTES) {
            WireRequestFrame::Empty => continue,
            WireRequestFrame::Oversized => {
                response_error(format!("request_too_large max_bytes={MAX_REQUEST_BYTES}"))
            }
            WireRequestFrame::InvalidUtf8 | WireRequestFrame::InvalidJson => {
                response_error("invalid_json".to_string())
            }
            WireRequestFrame::Request(call) => process_call(&core, call),
        };
        if write_response(&mut stdout, &response).is_err() {
            break;
        }
    }
}

fn drain_until_newline(reader: &mut impl BufRead) -> io::Result<()> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(());
        }
        if let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
            reader.consume(index + 1);
            return Ok(());
        }
        let consumed = buffer.len();
        reader.consume(consumed);
    }
}

fn process_call(core: &HazelMemoryCore, call: ToolCallRequest) -> ToolCallResponse {
    match call.tool.as_str() {
        "hazel.recollect" => match validate_pre_prompt_compose_input_value(&call.input) {
            Ok(input) => {
                let output = core.recollect(&PrePromptComposeHookInput {
                    top_k: Some(input.top_k.unwrap_or(3)),
                    token_budget: input.token_budget,
                    relevance_threshold: Some(input.relevance_threshold.unwrap_or(0.0)),
                    ..input
                });
                let output_value = match serde_json::to_value(output) {
                    Ok(value) => value,
                    Err(_) => return response_error("recollection_serialize_failed".to_string()),
                };
                match normalize_recollection_output(&output_value) {
                    Ok(validated) => {
                        let value = match serde_json::to_value(validated) {
                            Ok(value) => value,
                            Err(_) => {
                                return response_error(
                                    "validated_recollection_serialize_failed".to_string(),
                                );
                            }
                        };
                        ToolCallResponse {
                            ok: true,
                            output: Some(value),
                            error: None,
                        }
                    }
                    Err(error) => response_error(error),
                }
            }
            Err(error) => response_error(error),
        },
        "hazel.schema" => {
            let descriptor = HookSchemaDescriptor {
                input: expected_pre_prompt_compose_input_schema(),
                output: expected_recollection_output_schema(),
            };
            let value = match serde_json::to_value(descriptor) {
                Ok(value) => value,
                Err(_) => return response_error("schema_serialize_failed".to_string()),
            };
            ToolCallResponse {
                ok: true,
                output: Some(value),
                error: None,
            }
        }
        _ => response_error(format!("unsupported_tool tool={}", call.tool)),
    }
}

fn response_error(error: String) -> ToolCallResponse {
    ToolCallResponse {
        ok: false,
        output: None,
        error: Some(error),
    }
}

fn write_response(stdout: &mut io::Stdout, response: &ToolCallResponse) -> io::Result<()> {
    let payload = match serde_json::to_string(response) {
        Ok(payload) => payload,
        Err(_) => {
            String::from("{\"ok\":false,\"output\":null,\"error\":\"response_serialize_failed\"}")
        }
    };
    writeln!(stdout, "{payload}")?;
    stdout.flush()
}
