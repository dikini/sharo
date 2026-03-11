use std::path::PathBuf;

use clap::Parser;
use sharo_tui::app::{App, DaemonClient};
use sharo_tui::state::Screen;

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";

#[derive(Debug, Parser)]
#[command(name = "sharo-tui")]
#[command(about = "Sharo terminal user interface")]
struct Cli {
    #[arg(long, default_value = DEFAULT_SOCKET_PATH)]
    socket_path: PathBuf,
    #[arg(long, value_enum, default_value_t = Screen::Chat)]
    screen: Screen,
    #[arg(long)]
    once: bool,
}

fn main() {
    let cli = Cli::parse();
    let client = DaemonClient::new(&cli.socket_path);
    let mut app = App::new(client);
    app.state_mut().set_active_screen(cli.screen);
    if let Err(message) = app.initialize() {
        eprintln!("tui_error={message}");
        std::process::exit(1);
    }

    let rendered = app.render_shell();
    if cli.once {
        print!("{rendered}");
        return;
    }

    print!("{rendered}");
}
