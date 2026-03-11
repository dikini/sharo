#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use sharo_core::protocol::{
    DaemonRequest, GetSessionTasksRequest, GetSessionViewRequest, GetSkillRequest,
    ListSkillsRequest, SetSessionSkillsRequest,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = serde_json::from_slice::<DaemonRequest>(data) {
        if let Ok(encoded) = serde_json::to_vec(&request) {
            let _ = serde_json::from_slice::<DaemonRequest>(&encoded);
        }
    }

    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        if let Ok(tasks_request) = serde_json::from_value::<GetSessionTasksRequest>(value.clone()) {
            if let Ok(encoded) = serde_json::to_vec(&tasks_request) {
                let _ = serde_json::from_slice::<GetSessionTasksRequest>(&encoded);
            }
        }

        if let Ok(view_request) = serde_json::from_value::<GetSessionViewRequest>(value) {
            if let Ok(encoded) = serde_json::to_vec(&view_request) {
                let _ = serde_json::from_slice::<GetSessionViewRequest>(&encoded);
            }
        }
    }

    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        if let Ok(list_request) = serde_json::from_value::<ListSkillsRequest>(value.clone()) {
            if let Ok(encoded) = serde_json::to_vec(&list_request) {
                let _ = serde_json::from_slice::<ListSkillsRequest>(&encoded);
            }
        }

        if let Ok(get_request) = serde_json::from_value::<GetSkillRequest>(value.clone()) {
            if let Ok(encoded) = serde_json::to_vec(&get_request) {
                let _ = serde_json::from_slice::<GetSkillRequest>(&encoded);
            }
        }

        if let Ok(set_request) = serde_json::from_value::<SetSessionSkillsRequest>(value) {
            if let Ok(encoded) = serde_json::to_vec(&set_request) {
                let _ = serde_json::from_slice::<SetSessionSkillsRequest>(&encoded);
            }
        }
    }
});
