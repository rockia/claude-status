use crate::config::DisplayConfig;
use crate::context::ContextInfo;
use crate::usage::UsageResponse;
use chrono::{DateTime, Utc};

const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const ORANGE: &str = "\x1b[38;5;208m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

fn color_for(pct: f64) -> &'static str {
    if pct >= 90.0 {
        RED
    } else if pct >= 70.0 {
        YELLOW
    } else if pct >= 50.0 {
        ORANGE
    } else {
        GREEN
    }
}

fn progress_bar(pct: f64) -> String {
    let filled = (pct / 10.0).round() as usize;
    let filled = filled.min(10);
    let empty = 10 - filled;
    let color = color_for(pct);

    format!(
        "{}{}{}{}{}{}",
        color,
        "\u{25b0}".repeat(filled),
        RESET,
        DIM,
        "\u{25b1}".repeat(empty),
        RESET,
    )
}

fn format_pct(pct: f64) -> String {
    let color = color_for(pct);
    format!("{}{:.0}%{}", color, pct, RESET)
}

pub fn render(ctx: &ContextInfo, usage: Option<&UsageResponse>, cfg: &DisplayConfig) -> String {
    let mut segments: Vec<String> = Vec::new();

    // Model name
    if cfg.model_name {
        segments.push(ctx.model_name.clone());
    }

    // Context bar (% used)
    if cfg.context_bar {
        segments.push(format!("\u{1f4ac} {}%", ctx.context_pct));
    }

    // Project dir + optional git info
    let git_info = if cfg.git_info {
        match (&ctx.git_branch, ctx.git_dirty) {
            (Some(branch), true) => format!(" ({}*)", branch),
            (Some(branch), false) => format!(" ({})", branch),
            (None, _) => String::new(),
        }
    } else {
        String::new()
    };
    segments.push(format!("{}{}", ctx.project_dir, git_info));

    // Duration
    if cfg.duration
        && let Some(dur) = &ctx.session_duration
    {
        segments.push(format!("\u{23f1} {}", dur));
    }

    // Effort level
    if cfg.effort_level {
        segments.push(format!("\u{26a1} {}", ctx.effort_level));
    }

    let line1 = segments.join(" \u{2502} ");

    // Rate limit lines
    let mut rate_lines: Vec<String> = Vec::new();

    if let Some(usage) = usage {
        if cfg.rate_limit_current
            && let Some(five) = &usage.five_hour
        {
            let util = five.utilization.unwrap_or(0.0);
            let reset = five
                .resets_at
                .as_deref()
                .map(format_reset_time_short)
                .unwrap_or_default();
            rate_lines.push(format!(
                "current  {}  {} {}",
                progress_bar(util),
                format_pct(util),
                reset,
            ));
        }

        if cfg.rate_limit_weekly
            && let Some(seven) = &usage.seven_day
        {
            let util = seven.utilization.unwrap_or(0.0);
            let reset = seven
                .resets_at
                .as_deref()
                .map(format_reset_time_long)
                .unwrap_or_default();
            rate_lines.push(format!(
                "weekly   {}  {} {}",
                progress_bar(util),
                format_pct(util),
                reset,
            ));
        }

        if cfg.rate_limit_extra
            && let Some(extra) = &usage.extra_usage
            && extra.is_enabled.unwrap_or(false)
        {
            let util = extra.utilization.unwrap_or(0.0);
            let used = extra.used_credits.unwrap_or(0.0) / 100.0;
            let limit = extra.monthly_limit.unwrap_or(0.0) / 100.0;
            let reset = extra
                .resets_at
                .as_deref()
                .map(format_reset_date_only)
                .unwrap_or_default();
            rate_lines.push(format!(
                "extra    {}  ${:.2}/${:.2} {}",
                progress_bar(util),
                used,
                limit,
                reset,
            ));
        }
    }

    if rate_lines.is_empty() {
        line1
    } else {
        format!("{}\n\n{}", line1, rate_lines.join("\n"))
    }
}

fn format_reset_time_short(ts: &str) -> String {
    if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
        let local = dt.with_timezone(&chrono::Local);
        format!("\u{27f3} {}", local.format("%-I:%M%P"))
    } else {
        String::new()
    }
}

fn format_reset_time_long(ts: &str) -> String {
    if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
        let local = dt.with_timezone(&chrono::Local);
        format!("\u{27f3} {}", local.format("%b %-d, %-I:%M%P"))
    } else {
        String::new()
    }
}

fn format_reset_date_only(ts: &str) -> String {
    if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
        let local = dt.with_timezone(&chrono::Local);
        format!("\u{27f3} {}", local.format("%b %-d"))
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::{ExtraUsage, UsagePeriod, UsageResponse};

    fn make_ctx(
        model: &str,
        pct: u64,
        project: &str,
        branch: Option<&str>,
        dirty: bool,
    ) -> ContextInfo {
        ContextInfo {
            model_name: model.to_string(),
            context_pct: pct,
            project_dir: project.to_string(),
            git_branch: branch.map(|s| s.to_string()),
            git_dirty: dirty,
            session_duration: Some("1h 23m".to_string()),
            effort_level: "default".to_string(),
        }
    }

    fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip until 'm'
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    // --- Color threshold tests ---

    #[test]
    fn test_color_green_below_50() {
        assert_eq!(color_for(0.0), GREEN);
        assert_eq!(color_for(49.9), GREEN);
    }

    #[test]
    fn test_color_orange_50_to_69() {
        assert_eq!(color_for(50.0), ORANGE);
        assert_eq!(color_for(69.9), ORANGE);
    }

    #[test]
    fn test_color_yellow_70_to_89() {
        assert_eq!(color_for(70.0), YELLOW);
        assert_eq!(color_for(89.9), YELLOW);
    }

    #[test]
    fn test_color_red_90_plus() {
        assert_eq!(color_for(90.0), RED);
        assert_eq!(color_for(100.0), RED);
    }

    // --- Progress bar tests ---

    #[test]
    fn test_progress_bar_0_pct() {
        let bar = strip_ansi(&progress_bar(0.0));
        assert_eq!(bar, "▱▱▱▱▱▱▱▱▱▱");
    }

    #[test]
    fn test_progress_bar_45_pct() {
        let bar = strip_ansi(&progress_bar(45.0));
        // round(45/10) = round(4.5) = 5 (banker's rounding, but 4 or 5 both acceptable)
        let filled = bar.chars().filter(|&c| c == '▰').count();
        let empty = bar.chars().filter(|&c| c == '▱').count();
        assert_eq!(filled + empty, 10);
        assert!(filled == 4 || filled == 5); // rounding
    }

    #[test]
    fn test_progress_bar_100_pct() {
        let bar = strip_ansi(&progress_bar(100.0));
        assert_eq!(bar, "▰▰▰▰▰▰▰▰▰▰");
    }

    #[test]
    fn test_progress_bar_23_pct() {
        let bar = strip_ansi(&progress_bar(23.0));
        // round(2.3) = 2
        let filled = bar.chars().filter(|&c| c == '▰').count();
        assert_eq!(filled, 2);
    }

    #[test]
    fn test_progress_bar_no_spaces() {
        let bar = strip_ansi(&progress_bar(50.0));
        assert!(!bar.contains(' '));
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_progress_bar_has_ansi_colors() {
        let bar = progress_bar(30.0);
        assert!(bar.contains(GREEN));
        assert!(bar.contains(RESET));
    }

    #[test]
    fn test_progress_bar_red_at_92() {
        let bar = progress_bar(92.0);
        assert!(bar.contains(RED));
    }

    // --- Format percentage tests ---

    #[test]
    fn test_format_pct_whole_number() {
        let result = strip_ansi(&format_pct(45.0));
        assert_eq!(result, "45%");
    }

    #[test]
    fn test_format_pct_rounds_decimal() {
        let result = strip_ansi(&format_pct(45.7));
        assert_eq!(result, "46%");
    }

    // --- Render line 1 tests ---

    #[test]
    fn test_render_line1_full() {
        let ctx = make_ctx("Claude Sonnet 4", 23, "my-project", Some("main"), true);
        let output = render(&ctx, None, &DisplayConfig::default());
        assert!(output.contains("Claude Sonnet 4"));
        assert!(output.contains("23%"));
        assert!(output.contains("my-project (main*)"));
        assert!(output.contains("1h 23m"));
        assert!(output.contains("default"));
    }

    #[test]
    fn test_render_line1_clean_branch() {
        let ctx = make_ctx("Claude Sonnet 4", 10, "my-project", Some("main"), false);
        let output = render(&ctx, None, &DisplayConfig::default());
        assert!(output.contains("my-project (main)"));
        assert!(!output.contains("main*"));
    }

    #[test]
    fn test_render_line1_no_git() {
        let ctx = make_ctx("Claude Sonnet 4", 10, "my-project", None, false);
        let output = render(&ctx, None, &DisplayConfig::default());
        assert!(output.contains("my-project"));
        assert!(!output.contains("("));
    }

    #[test]
    fn test_render_no_usage_no_rate_lines() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let output = render(&ctx, None, &DisplayConfig::default());
        assert!(!output.contains("current"));
        assert!(!output.contains("weekly"));
        assert!(!output.contains('\n'));
    }

    // --- Render rate limit lines ---

    #[test]
    fn test_render_with_usage() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let usage = UsageResponse {
            five_hour: Some(UsagePeriod {
                utilization: Some(23.0),
                resets_at: None,
            }),
            seven_day: Some(UsagePeriod {
                utilization: Some(45.0),
                resets_at: None,
            }),
            extra_usage: None,
        };
        let output = render(&ctx, Some(&usage), &DisplayConfig::default());
        let plain = strip_ansi(&output);
        assert!(plain.contains("current"));
        assert!(plain.contains("23%"));
        assert!(plain.contains("weekly"));
        assert!(plain.contains("45%"));
    }

    #[test]
    fn test_render_blank_line_before_rates() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let usage = UsageResponse {
            five_hour: Some(UsagePeriod {
                utilization: Some(10.0),
                resets_at: None,
            }),
            seven_day: None,
            extra_usage: None,
        };
        let output = render(&ctx, Some(&usage), &DisplayConfig::default());
        assert!(output.contains("\n\n"));
    }

    #[test]
    fn test_render_extra_enabled() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let usage = UsageResponse {
            five_hour: None,
            seven_day: None,
            extra_usage: Some(ExtraUsage {
                is_enabled: Some(true),
                utilization: Some(10.0),
                used_credits: Some(500.0),
                monthly_limit: Some(5000.0),
                resets_at: None,
            }),
        };
        let output = render(&ctx, Some(&usage), &DisplayConfig::default());
        let plain = strip_ansi(&output);
        assert!(plain.contains("extra"));
        assert!(plain.contains("$5.00/$50.00"));
    }

    #[test]
    fn test_render_extra_disabled() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let usage = UsageResponse {
            five_hour: None,
            seven_day: None,
            extra_usage: Some(ExtraUsage {
                is_enabled: Some(false),
                utilization: Some(10.0),
                used_credits: Some(500.0),
                monthly_limit: Some(5000.0),
                resets_at: None,
            }),
        };
        let output = render(&ctx, Some(&usage), &DisplayConfig::default());
        assert!(!output.contains("extra"));
    }

    #[test]
    fn test_render_multiline_structure() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let usage = UsageResponse {
            five_hour: Some(UsagePeriod {
                utilization: Some(23.0),
                resets_at: None,
            }),
            seven_day: Some(UsagePeriod {
                utilization: Some(45.0),
                resets_at: None,
            }),
            extra_usage: Some(ExtraUsage {
                is_enabled: Some(true),
                utilization: Some(10.0),
                used_credits: Some(500.0),
                monthly_limit: Some(5000.0),
                resets_at: None,
            }),
        };
        let output = render(&ctx, Some(&usage), &DisplayConfig::default());
        let lines: Vec<&str> = output.split('\n').collect();
        // Line 0: info line, Line 1: blank, Lines 2+: rate lines
        assert!(lines.len() >= 4);
        assert!(lines[1].is_empty());
    }

    #[test]
    fn test_format_reset_time_long_title_case_month() {
        // 2026-03-05T15:00:00Z → should contain "Mar" not "mar"
        let result = format_reset_time_long("2026-03-05T15:00:00Z");
        assert!(!result.is_empty());
        assert!(
            result.contains("Mar")
                || result.contains("Jan")
                || result.contains("Feb")
                || result.contains("Apr")
                || result.contains("May")
                || result.contains("Jun")
                || result.contains("Jul")
                || result.contains("Aug")
                || result.contains("Sep")
                || result.contains("Oct")
                || result.contains("Nov")
                || result.contains("Dec"),
            "Expected title-case month in: {result}"
        );
        assert!(
            !result.contains("mar") && !result.contains("jan") && !result.contains("feb"),
            "Got lowercase month in: {result}"
        );
    }

    #[test]
    fn test_format_reset_date_only_title_case_month() {
        let result = format_reset_date_only("2026-03-05T15:00:00Z");
        assert!(!result.is_empty());
        assert!(result.contains("Mar"), "Expected 'Mar' in: {result}");
        assert!(!result.contains("mar"), "Got lowercase 'mar' in: {result}");
    }

    // --- Config-gating tests ---

    #[test]
    fn test_render_hidden_effort_level() {
        let ctx = make_ctx("Model", 10, "proj", Some("main"), false);
        let mut cfg = DisplayConfig::default();
        cfg.effort_level = false;
        let output = render(&ctx, None, &cfg);
        assert!(!output.contains('\u{26a1}')); // ⚡ not present
        assert!(!strip_ansi(&output).contains("default")); // effort label absent
    }

    #[test]
    fn test_render_hidden_model_name() {
        let ctx = make_ctx("Claude Sonnet 4", 10, "proj", None, false);
        let mut cfg = DisplayConfig::default();
        cfg.model_name = false;
        let output = render(&ctx, None, &cfg);
        assert!(!output.contains("Claude Sonnet 4"));
    }

    #[test]
    fn test_render_hidden_context_bar() {
        let ctx = make_ctx("Model", 55, "proj", None, false);
        let mut cfg = DisplayConfig::default();
        cfg.context_bar = false;
        let output = strip_ansi(&render(&ctx, None, &cfg));
        assert!(!output.contains("55%"));
        assert!(!output.contains('\u{1f4ac}')); // 💬 not present
    }

    #[test]
    fn test_render_hidden_git_info() {
        let ctx = make_ctx("Model", 10, "proj", Some("feature-x"), true);
        let mut cfg = DisplayConfig::default();
        cfg.git_info = false;
        let output = render(&ctx, None, &cfg);
        assert!(!output.contains("feature-x"));
        assert!(!output.contains('*'));
        assert!(output.contains("proj")); // project dir still shown
    }

    #[test]
    fn test_render_hidden_duration() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let mut cfg = DisplayConfig::default();
        cfg.duration = false;
        let output = render(&ctx, None, &cfg);
        assert!(!output.contains("1h 23m"));
        assert!(!output.contains('\u{23f1}')); // ⏱ not present
    }

    #[test]
    fn test_render_hidden_rate_limit_current() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let mut cfg = DisplayConfig::default();
        cfg.rate_limit_current = false;
        let usage = UsageResponse {
            five_hour: Some(UsagePeriod {
                utilization: Some(50.0),
                resets_at: None,
            }),
            seven_day: Some(UsagePeriod {
                utilization: Some(20.0),
                resets_at: None,
            }),
            extra_usage: None,
        };
        let output = strip_ansi(&render(&ctx, Some(&usage), &cfg));
        assert!(!output.contains("current"));
        assert!(output.contains("weekly")); // other rows still shown
    }

    #[test]
    fn test_render_all_hidden_no_blank_lines() {
        let ctx = make_ctx("Model", 10, "proj", None, false);
        let mut cfg = DisplayConfig::default();
        cfg.rate_limit_current = false;
        cfg.rate_limit_weekly = false;
        cfg.rate_limit_extra = false;
        let usage = UsageResponse {
            five_hour: Some(UsagePeriod {
                utilization: Some(10.0),
                resets_at: None,
            }),
            seven_day: Some(UsagePeriod {
                utilization: Some(10.0),
                resets_at: None,
            }),
            extra_usage: None,
        };
        let output = render(&ctx, Some(&usage), &cfg);
        assert!(!output.contains('\n'), "Expected no newlines: {output:?}");
    }
}
