use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const GIT_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Deserialize)]
pub struct StdinInput {
    pub model: Option<Model>,
    pub context_window: Option<ContextWindow>,
    pub cwd: Option<String>,
    pub session: Option<Session>,
}

#[derive(Deserialize)]
pub struct Model {
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ContextWindow {
    pub context_window_size: Option<u64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Deserialize)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Deserialize)]
pub struct Session {
    pub start_time: Option<String>,
}

pub struct ContextInfo {
    pub model_name: String,
    pub context_pct: u64,
    pub project_dir: String,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub session_duration: Option<String>,
    pub effort_level: String,
}

pub fn read_stdin() -> Option<StdinInput> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    if input.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&input).ok()
}

pub fn build_context(input: &StdinInput) -> ContextInfo {
    let model_name = input
        .model
        .as_ref()
        .and_then(|m| m.display_name.clone())
        .unwrap_or_else(|| "Unknown Model".to_string());

    let context_pct = compute_context_pct(input);

    let cwd = input.cwd.clone().unwrap_or_else(|| ".".to_string());
    let project_dir = Path::new(&cwd)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| cwd.clone());

    let git_branch = get_git_branch(&cwd);
    let git_dirty = git_branch.is_some() && is_git_dirty(&cwd);

    let session_duration = input
        .session
        .as_ref()
        .and_then(|s| s.start_time.as_ref())
        .and_then(|t| format_duration(t));

    let effort_level = read_effort_level();

    ContextInfo {
        model_name,
        context_pct,
        project_dir,
        git_branch,
        git_dirty,
        session_duration,
        effort_level,
    }
}

pub(crate) fn compute_context_pct(input: &StdinInput) -> u64 {
    let cw = match &input.context_window {
        Some(cw) => cw,
        None => return 0,
    };
    let size = cw.context_window_size.unwrap_or(200_000);
    if size == 0 {
        return 0;
    }
    let usage = match &cw.current_usage {
        Some(u) => u,
        None => return 0,
    };
    let total = usage.input_tokens.unwrap_or(0)
        + usage.cache_creation_input_tokens.unwrap_or(0)
        + usage.cache_read_input_tokens.unwrap_or(0);
    ((total as f64 / size as f64) * 100.0).round() as u64
}

fn get_git_branch(cwd: &str) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd);
    let output = run_with_timeout(&mut cmd, GIT_TIMEOUT)?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn is_git_dirty(cwd: &str) -> bool {
    let mut cmd = Command::new("git");
    cmd.args(["status", "--porcelain"]).current_dir(cwd);
    run_with_timeout(&mut cmd, GIT_TIMEOUT)
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Runs `cmd`, killing and returning `None` if it doesn't finish within `timeout`.
pub(crate) fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Option<Output> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

pub(crate) fn format_duration(start_time: &str) -> Option<String> {
    let start: DateTime<Utc> = start_time.parse().ok()?;
    let now = Utc::now();
    let diff = now.signed_duration_since(start);
    let total_mins = diff.num_minutes();
    if total_mins < 0 {
        return Some("0m".to_string());
    }
    let hours = total_mins / 60;
    let mins = total_mins % 60;
    if hours > 0 {
        Some(format!("{}h {}m", hours, mins))
    } else {
        Some(format!("{}m", mins))
    }
}

fn read_effort_level() -> String {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return "default".to_string(),
    };
    let path = format!("{}/.claude/settings.json", home);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return "default".to_string(),
    };
    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return "default".to_string(),
    };
    val.get("effortLevel")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(
        input_tokens: u64,
        cache_creation: u64,
        cache_read: u64,
        window_size: u64,
    ) -> StdinInput {
        StdinInput {
            model: Some(Model {
                display_name: Some("Claude Sonnet 4".to_string()),
            }),
            context_window: Some(ContextWindow {
                context_window_size: Some(window_size),
                current_usage: Some(CurrentUsage {
                    input_tokens: Some(input_tokens),
                    cache_creation_input_tokens: Some(cache_creation),
                    cache_read_input_tokens: Some(cache_read),
                }),
            }),
            cwd: Some("/tmp/test-project".to_string()),
            session: None,
        }
    }

    #[test]
    fn test_context_pct_spec_scenario() {
        // Spec: 15000+3000+2000=20000, size=200000 → 10%
        let input = make_input(15000, 3000, 2000, 200000);
        assert_eq!(compute_context_pct(&input), 10);
    }

    #[test]
    fn test_context_pct_23_percent() {
        // 46000/200000 = 23%
        let input = make_input(30000, 5000, 11000, 200000);
        assert_eq!(compute_context_pct(&input), 23);
    }

    #[test]
    fn test_context_pct_zero_usage() {
        let input = make_input(0, 0, 0, 200000);
        assert_eq!(compute_context_pct(&input), 0);
    }

    #[test]
    fn test_context_pct_full_usage() {
        let input = make_input(200000, 0, 0, 200000);
        assert_eq!(compute_context_pct(&input), 100);
    }

    #[test]
    fn test_context_pct_zero_window_size() {
        let input = make_input(100, 0, 0, 0);
        assert_eq!(compute_context_pct(&input), 0);
    }

    #[test]
    fn test_context_pct_no_context_window() {
        let input = StdinInput {
            model: None,
            context_window: None,
            cwd: None,
            session: None,
        };
        assert_eq!(compute_context_pct(&input), 0);
    }

    #[test]
    fn test_context_pct_no_usage() {
        let input = StdinInput {
            model: None,
            context_window: Some(ContextWindow {
                context_window_size: Some(200000),
                current_usage: None,
            }),
            cwd: None,
            session: None,
        };
        assert_eq!(compute_context_pct(&input), 0);
    }

    #[test]
    fn test_context_pct_defaults_window_to_200k() {
        let input = StdinInput {
            model: None,
            context_window: Some(ContextWindow {
                context_window_size: None,
                current_usage: Some(CurrentUsage {
                    input_tokens: Some(20000),
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            }),
            cwd: None,
            session: None,
        };
        assert_eq!(compute_context_pct(&input), 10);
    }

    #[test]
    fn test_context_pct_rounding() {
        // 19999/200000 = 9.9995% → rounds to 10%
        let input = make_input(19999, 0, 0, 200000);
        assert_eq!(compute_context_pct(&input), 10);
    }

    #[test]
    fn test_format_duration_over_one_hour() {
        let start = (chrono::Utc::now() - chrono::Duration::minutes(83)).to_rfc3339();
        let result = format_duration(&start).unwrap();
        assert_eq!(result, "1h 23m");
    }

    #[test]
    fn test_format_duration_under_one_hour() {
        let start = (chrono::Utc::now() - chrono::Duration::minutes(15)).to_rfc3339();
        let result = format_duration(&start).unwrap();
        assert_eq!(result, "15m");
    }

    #[test]
    fn test_format_duration_zero() {
        let start = chrono::Utc::now().to_rfc3339();
        let result = format_duration(&start).unwrap();
        assert_eq!(result, "0m");
    }

    #[test]
    fn test_format_duration_future_time() {
        let start = (chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339();
        let result = format_duration(&start).unwrap();
        assert_eq!(result, "0m");
    }

    #[test]
    fn test_format_duration_invalid() {
        assert!(format_duration("not-a-date").is_none());
    }

    #[test]
    fn test_build_context_model_name() {
        let input = make_input(0, 0, 0, 200000);
        let ctx = build_context(&input);
        assert_eq!(ctx.model_name, "Claude Sonnet 4");
    }

    #[test]
    fn test_build_context_missing_model() {
        let input = StdinInput {
            model: None,
            context_window: None,
            cwd: Some("/tmp/foo".to_string()),
            session: None,
        };
        let ctx = build_context(&input);
        assert_eq!(ctx.model_name, "Unknown Model");
    }

    #[test]
    fn test_build_context_project_dir() {
        let input = StdinInput {
            model: None,
            context_window: None,
            cwd: Some("/Users/test/Documents/my-project".to_string()),
            session: None,
        };
        let ctx = build_context(&input);
        assert_eq!(ctx.project_dir, "my-project");
    }

    #[test]
    fn test_stdin_deserialization() {
        let json = r#"{"model":{"display_name":"Claude Sonnet 4"},"context_window":{"context_window_size":200000,"current_usage":{"input_tokens":100}},"cwd":"/tmp","session":{"start_time":"2026-01-01T00:00:00Z"}}"#;
        let input: StdinInput = serde_json::from_str(json).unwrap();
        assert_eq!(
            input.model.unwrap().display_name.unwrap(),
            "Claude Sonnet 4"
        );
        assert_eq!(
            input
                .context_window
                .unwrap()
                .current_usage
                .unwrap()
                .input_tokens
                .unwrap(),
            100
        );
    }

    #[test]
    fn test_stdin_deserialization_minimal() {
        let json = r#"{}"#;
        let input: StdinInput = serde_json::from_str(json).unwrap();
        assert!(input.model.is_none());
        assert!(input.context_window.is_none());
    }

    #[test]
    fn test_run_with_timeout_returns_output_for_fast_command() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let output = run_with_timeout(&mut cmd, std::time::Duration::from_secs(2));
        let output = output.expect("fast command should complete");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    }

    #[test]
    fn test_run_with_timeout_returns_none_for_slow_command() {
        let mut cmd = Command::new("sleep");
        cmd.arg("2");
        let start = std::time::Instant::now();
        let output = run_with_timeout(&mut cmd, std::time::Duration::from_millis(100));
        let elapsed = start.elapsed();
        assert!(output.is_none(), "slow command should time out");
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "should not wait for the full slow command, took {elapsed:?}"
        );
    }
}
