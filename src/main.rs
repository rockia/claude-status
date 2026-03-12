mod cli;
mod format;
mod git;
mod statusline;
mod usage;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "csl", about = "Claude Code statusline")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Install csl into ~/.claude/ and configure settings.json
    Install,
    /// Uninstall csl and restore previous settings
    Uninstall,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Some(Command::Install) => {
            if let Err(e) = cli::install() {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Command::Uninstall) => {
            if let Err(e) = cli::uninstall() {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        None => {
            // Statusline mode: read stdin, output formatted line, always exit 0
            statusline::run();
        }
    }
}
