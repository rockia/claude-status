use serde::Deserialize;
use std::io::Read;

use crate::format;
use crate::git;
use crate::usage;

#[derive(Deserialize, Default)]
pub struct ModelInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Deserialize, Default)]
pub struct ContextWindow {
    #[serde(default)]
    pub used_percentage: Option<f64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub remaining_percentage: Option<f64>,
}

#[derive(Deserialize, Default)]
pub struct Workspace {
    #[serde(default)]
    pub current_dir: String,
    #[serde(default)]
    pub project_dir: String,
}

#[derive(Deserialize, Default)]
pub struct CostInfo {
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    #[serde(default)]
    pub total_duration_ms: Option<u64>,
}

#[derive(Deserialize)]
pub struct StdinInput {
    #[serde(default)]
    pub model: ModelInfo,
    #[serde(default)]
    pub context_window: ContextWindow,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub cost: CostInfo,
    #[serde(default)]
    #[allow(dead_code)]
    pub session_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub version: String,
}

pub fn run() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let parsed: StdinInput = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return,
    };

    let lines = render(&parsed);
    for line in &lines {
        println!("{line}");
    }
}

fn render(input: &StdinInput) -> Vec<String> {
    let mut lines = Vec::new();

    // --- Session info line ---
    let context_pct = input.context_window.used_percentage.unwrap_or(0.0);
    let bar = format::colorized_progress_bar(context_pct);

    let model_name = if !input.model.display_name.is_empty() {
        &input.model.display_name
    } else if !input.model.id.is_empty() {
        &input.model.id
    } else {
        "unknown"
    };

    let working_dir = if !input.workspace.current_dir.is_empty() {
        &input.workspace.current_dir
    } else {
        &input.workspace.project_dir
    };

    let dir = format::shorten_path(working_dir);

    let git_info = if !working_dir.is_empty() {
        git::get_git_info(working_dir)
    } else {
        None
    };

    let dir_branch = match &git_info {
        Some(info) => {
            let branch_str = if info.is_dirty {
                format::colored(&format!("({})", info.branch), format::YELLOW)
            } else {
                format!("({})", info.branch)
            };
            format!("{dir} {branch_str}")
        }
        None => dir,
    };

    let duration_str = match input.cost.total_duration_ms {
        Some(ms) if ms > 0 => format!("  \u{23F1} {}", format::format_duration(ms / 1000)),
        _ => String::new(),
    };

    let cost_str = match input.cost.total_cost_usd {
        Some(cost) if cost > 0.0 => format!("  ${:.2}", cost),
        _ => String::new(),
    };

    lines.push(format!(
        "{model_name}  {bar}  {dir_branch}{duration_str}{cost_str}",
    ));

    // --- Rate limit lines ---
    if let Some(usage_data) = usage::get_usage() {
        for limit in &usage_data.rate_limits {
            let limit_bar = format::colorized_progress_bar(limit.usage_percentage);
            lines.push(format!(
                "{}  {}  {}",
                limit.window_label, limit_bar, limit.reset_info
            ));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json() {
        let json = r#"{"model":{"id":"claude-sonnet-4-6","display_name":"Sonnet"},"context_window":{"used_percentage":48,"remaining_percentage":52},"workspace":{"current_dir":"/tmp","project_dir":"/tmp"}}"#;
        let parsed: StdinInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model.display_name, "Sonnet");
        assert_eq!(parsed.model.id, "claude-sonnet-4-6");
        assert_eq!(parsed.context_window.used_percentage, Some(48.0));
    }

    #[test]
    fn parse_missing_fields() {
        let json = r#"{"model":{"display_name":"opus"}}"#;
        let parsed: StdinInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model.display_name, "opus");
        assert_eq!(parsed.context_window.used_percentage, None);
        assert!(parsed.workspace.current_dir.is_empty());
    }

    #[test]
    fn parse_empty_object() {
        let json = r#"{}"#;
        let parsed: StdinInput = serde_json::from_str(json).unwrap();
        assert!(parsed.model.display_name.is_empty());
        assert_eq!(parsed.context_window.used_percentage, None);
    }

    #[test]
    fn parse_malformed_json() {
        let result = serde_json::from_str::<StdinInput>("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_cost() {
        let json = r#"{"model":{"display_name":"Opus"},"cost":{"total_cost_usd":1.23,"total_duration_ms":45000}}"#;
        let parsed: StdinInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.cost.total_cost_usd, Some(1.23));
        assert_eq!(parsed.cost.total_duration_ms, Some(45000));
    }

    /// Regression test: ensure the real Claude Code JSON format parses and
    /// produces non-empty output. Previously csl expected flat fields
    /// (e.g. "model": "string") which silently defaulted to empty values,
    /// producing blank statusline output.
    #[test]
    fn regression_claude_code_real_json_produces_output() {
        let json = r#"{
            "model": { "id": "claude-opus-4-6", "display_name": "Opus" },
            "context_window": { "used_percentage": 25, "remaining_percentage": 75 },
            "workspace": { "current_dir": "/tmp", "project_dir": "/tmp" },
            "cost": { "total_cost_usd": 0.45, "total_duration_ms": 60000 },
            "session_id": "abc123",
            "version": "2.1.74"
        }"#;

        let parsed: StdinInput = serde_json::from_str(json).unwrap();

        // Verify all fields parsed correctly from nested JSON
        assert_eq!(parsed.model.display_name, "Opus");
        assert_eq!(parsed.model.id, "claude-opus-4-6");
        assert_eq!(parsed.context_window.used_percentage, Some(25.0));
        assert_eq!(parsed.workspace.current_dir, "/tmp");
        assert_eq!(parsed.cost.total_cost_usd, Some(0.45));
        assert_eq!(parsed.cost.total_duration_ms, Some(60000));

        // Verify render produces non-empty output
        let lines = render(&parsed);
        assert!(!lines.is_empty(), "render must produce at least one line");
        let session_line = &lines[0];
        assert!(
            session_line.contains("Opus"),
            "session line must contain model name"
        );
        assert!(
            session_line.contains("25%"),
            "session line must contain context percentage"
        );
        assert!(
            session_line.contains("$0.45"),
            "session line must contain cost"
        );
    }

    /// Regression test: flat string "model" field (old format) must NOT parse
    /// successfully, ensuring we don't silently accept the wrong schema.
    #[test]
    fn regression_old_flat_format_does_not_silently_succeed() {
        let old_json = r#"{"model":"sonnet 4.6","contextWindow":200000,"tokensUsed":96000}"#;
        // The old flat "model": "string" should fail to deserialize into ModelInfo
        let result = serde_json::from_str::<StdinInput>(old_json);
        assert!(
            result.is_err(),
            "old flat JSON format should not parse into new struct"
        );
    }
}
