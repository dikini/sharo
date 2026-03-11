pub mod approvals;
pub mod artifacts;
pub mod chat;
pub mod sessions;
pub mod settings;

pub fn sanitize_for_terminal(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| {
            if ch == '\n' {
                "\\n".chars().collect::<Vec<_>>()
            } else if ch == '\r' {
                "\\r".chars().collect::<Vec<_>>()
            } else if ch.is_control() {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}
