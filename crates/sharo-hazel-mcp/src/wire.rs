use sharo_core::protocol::ToolCallRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireRequestFrame {
    Empty,
    Oversized,
    InvalidUtf8,
    InvalidJson,
    Request(ToolCallRequest),
}

pub fn line_content_len(bytes: &[u8]) -> usize {
    let mut len = bytes.len();
    if len > 0 && bytes[len - 1] == b'\n' {
        len -= 1;
        if len > 0 && bytes[len - 1] == b'\r' {
            len -= 1;
        }
    }
    len
}

pub fn parse_wire_request_frame(bytes: &[u8], max_request_bytes: usize) -> WireRequestFrame {
    if line_content_len(bytes) > max_request_bytes {
        return WireRequestFrame::Oversized;
    }
    let line = match std::str::from_utf8(bytes) {
        Ok(line) => line,
        Err(_) => return WireRequestFrame::InvalidUtf8,
    };
    if line.trim().is_empty() {
        return WireRequestFrame::Empty;
    }
    match serde_json::from_str::<ToolCallRequest>(line) {
        Ok(request) => WireRequestFrame::Request(request),
        Err(_) => WireRequestFrame::InvalidJson,
    }
}

#[cfg(test)]
mod tests {
    use super::{WireRequestFrame, line_content_len, parse_wire_request_frame};

    #[test]
    fn line_content_len_excludes_trailing_newline() {
        assert_eq!(line_content_len(b"abc\n"), 3);
    }

    #[test]
    fn line_content_len_keeps_non_terminated_length() {
        assert_eq!(line_content_len(b"abc"), 3);
    }

    #[test]
    fn line_content_len_excludes_trailing_crlf() {
        assert_eq!(line_content_len(b"abc\r\n"), 3);
    }

    #[test]
    fn parse_wire_request_frame_marks_oversized_before_json_decode() {
        let oversized = b"{\"tool\":\"hazel.schema\",\"input\":{}}\n";
        let frame = parse_wire_request_frame(oversized, 2);
        assert_eq!(frame, WireRequestFrame::Oversized);
    }
}
