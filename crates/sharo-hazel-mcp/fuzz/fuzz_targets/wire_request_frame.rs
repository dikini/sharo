#![no_main]

use libfuzzer_sys::fuzz_target;
use sharo_core::protocol::{validate_pre_prompt_compose_input_value, validate_recollection_payload_value};
use sharo_hazel_mcp::wire::{WireRequestFrame, parse_wire_request_frame};

fuzz_target!(|data: &[u8]| {
    match parse_wire_request_frame(data, 131_072) {
        WireRequestFrame::Request(request) => {
            match request.tool.as_str() {
                "hazel.recollect" => {
                    let _ = validate_pre_prompt_compose_input_value(&request.input);
                }
                "hazel.schema" => {}
                _ => {}
            }
        }
        WireRequestFrame::Empty
        | WireRequestFrame::Oversized
        | WireRequestFrame::InvalidUtf8
        | WireRequestFrame::InvalidJson => {}
    }

    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
        let _ = validate_recollection_payload_value(&value);
    }
});
