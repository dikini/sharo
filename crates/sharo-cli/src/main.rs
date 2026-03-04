use clap::{Parser, Subcommand};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{SubmitTaskRequest, TaskStatusRequest};

#[derive(Debug, Parser)]
#[command(name = "sharo")]
#[command(about = "Sharo CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Submit {
        #[arg(long)]
        goal: String,
        #[arg(long)]
        session_id: Option<String>,
    },
    Status {
        #[arg(long)]
        task_id: String,
    },
}

fn run_with_client(client: &impl RuntimeClient, cli: Cli) {
    match cli.command {
        Command::Submit { goal, session_id } => {
            let response = client.submit(&SubmitTaskRequest { session_id, goal });
            println!("task_id={} state={:?}", response.task_id, response.state);
        }
        Command::Status { task_id } => {
            let response = client.status(&TaskStatusRequest { task_id });
            println!(
                "task_id={} state={:?} summary={}",
                response.task_id, response.state, response.summary
            );
        }
    }
}

fn main() {
    let cli = Cli::parse();
    run_with_client(&StubClient, cli);
}
