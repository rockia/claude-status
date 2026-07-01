mod config;
mod config_cmd;
mod context;
mod display;
mod install;
mod usage;

const USAGE: &str = "\
Usage: ccsl [command]

Commands:
  (none)              Render the status line (reads context from stdin)
  install             Install claude-status as the Claude Code status line
  uninstall           Remove claude-status and restore the previous status line
  update              Update claude-status to the latest release
  config              View or change which display items are shown
  help, --help, -h    Show this help message";

#[derive(Debug, PartialEq)]
enum Cmd {
    StatusLine,
    Install,
    Uninstall,
    Update,
    Config,
    Help,
    Unknown(String),
}

fn parse_command(arg: Option<&str>) -> Cmd {
    match arg {
        None => Cmd::StatusLine,
        Some("install") => Cmd::Install,
        Some("uninstall") => Cmd::Uninstall,
        Some("update") => Cmd::Update,
        Some("config") => Cmd::Config,
        Some("help") | Some("--help") | Some("-h") => Cmd::Help,
        Some(other) => Cmd::Unknown(other.to_string()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match parse_command(args.get(1).map(|s| s.as_str())) {
        Cmd::Install => install::install(),
        Cmd::Uninstall => install::uninstall(),
        Cmd::Update => install::update(),
        Cmd::Config => config_cmd::run_config(&args[2..]),
        Cmd::Help => println!("{}", USAGE),
        Cmd::StatusLine => run_status_line(),
        Cmd::Unknown(cmd) => {
            eprintln!("Error: unknown command '{}'\n\n{}", cmd, USAGE);
            std::process::exit(1);
        }
    }
}

fn run_status_line() {
    let input = match context::read_stdin() {
        Some(input) => input,
        None => return,
    };

    let ctx = context::build_context(&input);
    let usage = usage::fetch_usage();
    let cfg = config::DisplayConfig::load();

    let output = display::render(&ctx, usage.as_ref(), &cfg);
    print!("{}", output);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_no_args_is_status_line() {
        assert_eq!(parse_command(None), Cmd::StatusLine);
    }

    #[test]
    fn test_parse_command_known_subcommands() {
        assert_eq!(parse_command(Some("install")), Cmd::Install);
        assert_eq!(parse_command(Some("uninstall")), Cmd::Uninstall);
        assert_eq!(parse_command(Some("update")), Cmd::Update);
        assert_eq!(parse_command(Some("config")), Cmd::Config);
    }

    #[test]
    fn test_parse_command_help_variants() {
        assert_eq!(parse_command(Some("help")), Cmd::Help);
        assert_eq!(parse_command(Some("--help")), Cmd::Help);
        assert_eq!(parse_command(Some("-h")), Cmd::Help);
    }

    #[test]
    fn test_parse_command_unknown_is_reported_not_status_line() {
        assert_eq!(
            parse_command(Some("bogus")),
            Cmd::Unknown("bogus".to_string())
        );
    }
}
