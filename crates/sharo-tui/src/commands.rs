#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    Sessions,
    Hazel,
    HazelStatus,
    HazelCards,
    HazelBatches,
    HazelJobs,
    HazelPreview { goal: String },
    HazelValidate { batch_id: String },
    HazelEnqueueJob {
        source_ref: String,
        idempotency_key: String,
        message: String,
    },
    HazelSubmit { batch_id: String },
    HazelCancelJob { job_id: String },
    SessionNew { label: Option<String> },
    SessionSwitch { session_id: String },
    Approve { approval_id: String },
    Deny { approval_id: String },
    Skills,
    SkillEnable { skill_id: String },
    SkillDisable { skill_id: String },
    Mcp,
    McpEnable { server_id: String },
    McpDisable { server_id: String },
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandError {
    pub code: &'static str,
    pub message: String,
}

impl SlashCommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub fn parse_slash_command(input: &str) -> Result<Option<SlashCommand>, SlashCommandError> {
    if !input.starts_with('/') {
        return Ok(None);
    }
    let tokens = lex_slash_tokens(input)?;
    parse_slash_tokens(&tokens).map(Some)
}

fn parse_slash_tokens(tokens: &[String]) -> Result<SlashCommand, SlashCommandError> {
    let command = tokens.first().ok_or_else(|| {
        SlashCommandError::new("slash_command_empty", "slash command cannot be empty")
    })?;
    match command.as_str() {
        "sessions" => {
            expect_no_extra_args("sessions", &tokens[1..]).map(|()| SlashCommand::Sessions)
        }
        "hazel" => parse_hazel_command(&tokens[1..]),
        "session" => parse_session_command(&tokens[1..]),
        "approve" => single_arg("approve", &tokens[1..])
            .map(|approval_id| SlashCommand::Approve { approval_id }),
        "deny" => {
            single_arg("deny", &tokens[1..]).map(|approval_id| SlashCommand::Deny { approval_id })
        }
        "skills" => expect_no_extra_args("skills", &tokens[1..]).map(|()| SlashCommand::Skills),
        "skill" => parse_skill_command(&tokens[1..]),
        "mcp" => parse_mcp_command(&tokens[1..]),
        "model" => expect_no_extra_args("model", &tokens[1..]).map(|()| SlashCommand::Model),
        other => Err(SlashCommandError::new(
            "slash_command_unknown",
            format!("unknown slash command `{other}`"),
        )),
    }
}

fn parse_hazel_command(args: &[String]) -> Result<SlashCommand, SlashCommandError> {
    if args.is_empty() {
        return Ok(SlashCommand::Hazel);
    }
    match args[0].as_str() {
        "status" => expect_no_extra_args("hazel status", &args[1..]).map(|()| SlashCommand::HazelStatus),
        "cards" => expect_no_extra_args("hazel cards", &args[1..]).map(|()| SlashCommand::HazelCards),
        "batches" => expect_no_extra_args("hazel batches", &args[1..]).map(|()| SlashCommand::HazelBatches),
        "jobs" => expect_no_extra_args("hazel jobs", &args[1..]).map(|()| SlashCommand::HazelJobs),
        "preview" => single_arg("hazel preview", &args[1..]).map(|goal| SlashCommand::HazelPreview { goal }),
        "validate" => single_arg("hazel validate", &args[1..]).map(|batch_id| SlashCommand::HazelValidate { batch_id }),
        "enqueue-job" => hazel_enqueue_job_args(&args[1..]),
        "submit" => single_arg("hazel submit", &args[1..]).map(|batch_id| SlashCommand::HazelSubmit { batch_id }),
        "cancel-job" => single_arg("hazel cancel-job", &args[1..])
            .map(|job_id| SlashCommand::HazelCancelJob { job_id }),
        other => Err(SlashCommandError::new(
            "slash_command_usage",
            format!("hazel subcommand `{other}` is not supported"),
        )),
    }
}

fn hazel_enqueue_job_args(args: &[String]) -> Result<SlashCommand, SlashCommandError> {
    if args.len() != 3 {
        return Err(SlashCommandError::new(
            "slash_command_usage",
            "`/hazel enqueue-job` requires exactly three arguments".to_string(),
        ));
    }
    Ok(SlashCommand::HazelEnqueueJob {
        source_ref: args[0].clone(),
        idempotency_key: args[1].clone(),
        message: args[2].clone(),
    })
}

fn parse_session_command(args: &[String]) -> Result<SlashCommand, SlashCommandError> {
    let subcommand = args.first().ok_or_else(|| {
        SlashCommandError::new(
            "slash_command_usage",
            "session requires `new` or `switch` subcommand",
        )
    })?;
    match subcommand.as_str() {
        "new" => {
            let label = if args.len() > 1 {
                Some(args[1..].join(" "))
            } else {
                None
            };
            Ok(SlashCommand::SessionNew { label })
        }
        "switch" => single_arg("session switch", &args[1..])
            .map(|session_id| SlashCommand::SessionSwitch { session_id }),
        other => Err(SlashCommandError::new(
            "slash_command_usage",
            format!("session subcommand `{other}` is not supported"),
        )),
    }
}

fn parse_skill_command(args: &[String]) -> Result<SlashCommand, SlashCommandError> {
    let subcommand = args.first().ok_or_else(|| {
        SlashCommandError::new(
            "slash_command_usage",
            "skill requires `enable` or `disable` subcommand",
        )
    })?;
    match subcommand.as_str() {
        "enable" => single_arg("skill enable", &args[1..])
            .map(|skill_id| SlashCommand::SkillEnable { skill_id }),
        "disable" => single_arg("skill disable", &args[1..])
            .map(|skill_id| SlashCommand::SkillDisable { skill_id }),
        other => Err(SlashCommandError::new(
            "slash_command_usage",
            format!("skill subcommand `{other}` is not supported"),
        )),
    }
}

fn parse_mcp_command(args: &[String]) -> Result<SlashCommand, SlashCommandError> {
    if args.is_empty() {
        return Ok(SlashCommand::Mcp);
    }
    match args[0].as_str() {
        "enable" => single_arg("mcp enable", &args[1..])
            .map(|server_id| SlashCommand::McpEnable { server_id }),
        "disable" => single_arg("mcp disable", &args[1..])
            .map(|server_id| SlashCommand::McpDisable { server_id }),
        other => Err(SlashCommandError::new(
            "slash_command_usage",
            format!("mcp subcommand `{other}` is not supported"),
        )),
    }
}

fn single_arg(command: &str, args: &[String]) -> Result<String, SlashCommandError> {
    if args.len() != 1 {
        return Err(SlashCommandError::new(
            "slash_command_usage",
            format!("`/{command}` requires exactly one argument"),
        ));
    }
    Ok(args[0].clone())
}

fn expect_no_extra_args(command: &str, args: &[String]) -> Result<(), SlashCommandError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(SlashCommandError::new(
            "slash_command_usage",
            format!("`/{command}` does not accept arguments"),
        ))
    }
}

fn lex_slash_tokens(input: &str) -> Result<Vec<String>, SlashCommandError> {
    let mut chars = input.chars().peekable();
    match chars.next() {
        Some('/') => {}
        _ => {
            return Err(SlashCommandError::new(
                "slash_command_invalid_prefix",
                "slash commands must start with `/`",
            ));
        }
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some(quote_char) => {
                if ch == quote_char {
                    quote = None;
                } else if ch == '\\' {
                    let next = chars.next().ok_or_else(|| {
                        SlashCommandError::new(
                            "slash_command_escape_invalid",
                            "trailing escape in quoted command argument",
                        )
                    })?;
                    current.push(next);
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                }
                '\\' => {
                    let next = chars.next().ok_or_else(|| {
                        SlashCommandError::new(
                            "slash_command_escape_invalid",
                            "trailing escape in slash command",
                        )
                    })?;
                    current.push(next);
                }
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
        }
    }

    if quote.is_some() {
        return Err(SlashCommandError::new(
            "slash_command_quote_unterminated",
            "unterminated quoted slash command argument",
        ));
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.is_empty() {
        return Err(SlashCommandError::new(
            "slash_command_empty",
            "slash command cannot be empty",
        ));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{SlashCommand, parse_slash_command};

    #[test]
    fn slash_commands_parse_slash_command_with_argument_vector() {
        let parsed = parse_slash_command("/session new \"alpha beta\"")
            .expect("parse")
            .expect("slash command");
        assert_eq!(
            parsed,
            SlashCommand::SessionNew {
                label: Some("alpha beta".to_string())
            }
        );
    }

    #[test]
    fn slash_commands_invalid_slash_command_returns_structured_error() {
        let error = parse_slash_command("/skill").expect_err("invalid command should fail");
        assert_eq!(error.code, "slash_command_usage");
        assert!(error.message.contains("skill requires"));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None,
            .. ProptestConfig::default()
        })]

        #[test]
        fn slash_commands_slash_parser_round_trips_valid_argument_boundaries(parts in prop::collection::vec("[a-z0-9_-]{1,8}", 1..4)) {
            let command = format!("/session new \"{}\"", parts.join(" "));
            let parsed = parse_slash_command(&command)
                .expect("parse")
                .expect("slash command");
            prop_assert_eq!(
                parsed,
                SlashCommand::SessionNew {
                    label: Some(parts.join(" "))
                }
            );
        }
    }
}
