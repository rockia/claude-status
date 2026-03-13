mod context;
mod display;
mod install;
mod usage;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("install") => install::install(),
        Some("uninstall") => install::uninstall(),
        Some("update") => install::update(),
        _ => run_status_line(),
    }
}

fn run_status_line() {
    let input = match context::read_stdin() {
        Some(input) => input,
        None => return,
    };

    let ctx = context::build_context(&input);
    let usage = usage::fetch_usage();

    let output = display::render(&ctx, usage.as_ref());
    print!("{}", output);
}
