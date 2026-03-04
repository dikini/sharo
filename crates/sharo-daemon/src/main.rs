use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "sharo-daemon")]
#[command(about = "Sharo daemon")]
struct Cli {
    #[command(subcommand)]
    command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    Start {
        #[arg(long)]
        once: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        DaemonCommand::Start { once } => {
            println!("daemon_started");
            if once {
                println!("daemon_stopped");
                return;
            }

            let _ = tokio::signal::ctrl_c().await;
            println!("daemon_stopped");
        }
    }
}
