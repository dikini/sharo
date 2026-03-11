#![no_main]

use libfuzzer_sys::fuzz_target;
use sharo_tui::commands::parse_slash_command;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_slash_command(input);
    }
});
