use crate::config::{DisplayConfig, ITEM_NAMES, config_path};
use dialoguer::MultiSelect;
use std::fs;

const USAGE: &str = "\
Usage: ccsl config <subcommand>

Subcommands:
  list                    Show current visibility for all display items
  set <item> <show|hide>  Update visibility for a single item
  reset                   Delete config file and restore all defaults

Items:
  effort_level, context_bar, rate_limit_current, rate_limit_weekly,
  rate_limit_extra, cost, git_info, duration, model_name";

pub fn run_config(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("list") => cmd_list(),
        Some("set") => cmd_set(&args[1..]),
        Some("reset") => cmd_reset(),
        _ => cmd_interactive(),
    }
}

fn cmd_interactive() {
    let cfg = DisplayConfig::load();
    let defaults: Vec<bool> = ITEM_NAMES
        .iter()
        .map(|n| cfg.get(n).unwrap_or(true))
        .collect();

    let selections = match MultiSelect::new()
        .with_prompt("Display items (space to toggle, enter to save)")
        .items(ITEM_NAMES)
        .defaults(&defaults)
        .interact_opt()
    {
        Ok(Some(s)) => s,
        _ => return, // cancelled
    };

    let mut new_cfg = DisplayConfig::default();
    for name in ITEM_NAMES {
        new_cfg.set(name, false);
    }
    for i in selections {
        new_cfg.set(ITEM_NAMES[i], true);
    }

    if let Err(e) = new_cfg.save() {
        eprintln!("Error: failed to save config: {}", e);
        std::process::exit(1);
    }
}

fn cmd_list() {
    let cfg = DisplayConfig::load();
    for name in ITEM_NAMES {
        let value = cfg.get(name).unwrap_or(true);
        println!("{:<22} {}", name, if value { "show" } else { "hide" });
    }
}

fn cmd_set(args: &[String]) {
    let item = match args.first() {
        Some(s) => s.as_str(),
        None => {
            eprintln!("Error: missing item name.\n{}", USAGE);
            std::process::exit(1);
        }
    };
    let value_str = match args.get(1) {
        Some(s) => s.as_str(),
        None => {
            eprintln!(
                "Error: missing value (expected 'show' or 'hide').\n{}",
                USAGE
            );
            std::process::exit(1);
        }
    };

    let value = match value_str {
        "show" => true,
        "hide" => false,
        other => {
            eprintln!(
                "Error: invalid value '{}'. Expected 'show' or 'hide'.",
                other
            );
            std::process::exit(1);
        }
    };

    let mut cfg = DisplayConfig::load();
    if !cfg.set(item, value) {
        eprintln!(
            "Error: unknown item '{}'. Valid items: {}",
            item,
            ITEM_NAMES.join(", ")
        );
        std::process::exit(1);
    }

    if let Err(e) = cfg.save() {
        eprintln!("Error: failed to save config: {}", e);
        std::process::exit(1);
    }
}

fn cmd_reset() {
    let path = config_path();
    if path.exists()
        && let Err(e) = fs::remove_file(&path)
    {
        eprintln!("Error: failed to delete config file: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;

    #[test]
    fn test_item_names_all_valid() {
        let cfg = DisplayConfig::default();
        for name in ITEM_NAMES {
            assert!(
                cfg.get(name).is_some(),
                "ITEM_NAMES contains unknown item: {name}"
            );
        }
    }

    #[test]
    fn test_set_and_get_roundtrip() {
        let mut cfg = DisplayConfig::default();
        for name in ITEM_NAMES {
            assert!(cfg.set(name, false));
            assert_eq!(cfg.get(name), Some(false));
            assert!(cfg.set(name, true));
            assert_eq!(cfg.get(name), Some(true));
        }
    }

    #[test]
    fn test_unknown_item_set_returns_false() {
        let mut cfg = DisplayConfig::default();
        assert!(!cfg.set("totally_unknown", false));
    }
}
