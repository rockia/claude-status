// Formatting utilities for statusline rendering: progress bars, ANSI colors,
// duration strings, and path shortening.

pub const RESET: &str = "\x1b[0m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const ORANGE: &str = "\x1b[38;5;208m";
pub const RED: &str = "\x1b[31m";

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Wrap `text` in the given ANSI `color` code, followed by a reset sequence.
pub fn colored(text: &str, color: &str) -> String {
    format!("{color}{text}{RESET}")
}

/// Choose a color based on how high `percentage` is:
///
/// | Range       | Color  |
/// |-------------|--------|
/// | < 50        | Green  |
/// | 50 .. 70    | Yellow |
/// | 70 .. 90    | Orange |
/// | >= 90       | Red    |
pub fn colorize_by_threshold(text: &str, percentage: f64) -> String {
    let color = match percentage {
        p if p < 50.0 => GREEN,
        p if p < 70.0 => YELLOW,
        p if p < 90.0 => ORANGE,
        _ => RED,
    };
    colored(text, color)
}

// ---------------------------------------------------------------------------
// Progress bar
// ---------------------------------------------------------------------------

const BAR_SEGMENTS: usize = 10;
const FILLED: char = '\u{25B0}'; // ▰
const EMPTY: char = '\u{25B1}'; // ▱

/// Render a 10-segment block progress bar.
///
/// The percentage is clamped to `0.0..=100.0` before rendering.
///
/// ```text
/// progress_bar(48.0) => "▰▰▰▰▰▱▱▱▱▱ 48%"
/// ```
pub fn progress_bar(percentage: f64) -> String {
    let clamped = percentage.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * BAR_SEGMENTS as f64).round() as usize;
    let empty = BAR_SEGMENTS - filled;

    let bar: String = std::iter::repeat_n(FILLED, filled)
        .chain(std::iter::repeat_n(EMPTY, empty))
        .collect();

    format!("{bar} {:.0}%", clamped)
}

/// Like [`progress_bar`], but the entire output is colored according to
/// [`colorize_by_threshold`].
pub fn colorized_progress_bar(percentage: f64) -> String {
    let plain = progress_bar(percentage);
    colorize_by_threshold(&plain, percentage)
}

// ---------------------------------------------------------------------------
// Duration formatting
// ---------------------------------------------------------------------------

/// Format a duration given in seconds into a compact human-readable string.
///
/// * `>= 1 hour`  : `2h15m` (hours + minutes, minutes omitted when 0)
/// * `>= 1 minute` : `45m`
/// * `< 1 minute`  : `5s`
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        if minutes > 0 {
            format!("{hours}h{minutes}m")
        } else {
            format!("{hours}h")
        }
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{secs}s")
    }
}

// ---------------------------------------------------------------------------
// Path shortening
// ---------------------------------------------------------------------------

/// Replace the leading home-directory portion of `path` with `~`.
///
/// If the path does not start with the home directory the original string is
/// returned unchanged.
pub fn shorten_path(path: &str) -> String {
    match dirs::home_dir() {
        Some(home) => {
            let home_str = home.to_string_lossy();
            if let Some(rest) = path.strip_prefix(home_str.as_ref()) {
                format!("~{rest}")
            } else {
                path.to_string()
            }
        }
        None => path.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Progress bar -------------------------------------------------------

    #[test]
    fn progress_bar_zero() {
        assert_eq!(progress_bar(0.0), "▱▱▱▱▱▱▱▱▱▱ 0%");
    }

    #[test]
    fn progress_bar_48() {
        assert_eq!(progress_bar(48.0), "▰▰▰▰▰▱▱▱▱▱ 48%");
    }

    #[test]
    fn progress_bar_100() {
        assert_eq!(progress_bar(100.0), "▰▰▰▰▰▰▰▰▰▰ 100%");
    }

    #[test]
    fn progress_bar_clamps_above_100() {
        assert_eq!(progress_bar(150.0), "▰▰▰▰▰▰▰▰▰▰ 100%");
    }

    #[test]
    fn progress_bar_clamps_below_zero() {
        assert_eq!(progress_bar(-10.0), "▱▱▱▱▱▱▱▱▱▱ 0%");
    }

    // -- Duration formatting ------------------------------------------------

    #[test]
    fn duration_seconds_only() {
        assert_eq!(format_duration(5), "5s");
    }

    #[test]
    fn duration_zero() {
        assert_eq!(format_duration(0), "0s");
    }

    #[test]
    fn duration_minutes_only() {
        assert_eq!(format_duration(180), "3m");
    }

    #[test]
    fn duration_45_minutes() {
        assert_eq!(format_duration(2700), "45m");
    }

    #[test]
    fn duration_hours_and_minutes() {
        assert_eq!(format_duration(8100), "2h15m");
    }

    #[test]
    fn duration_exact_hour() {
        assert_eq!(format_duration(3600), "1h");
    }

    // -- Color thresholds ---------------------------------------------------

    #[test]
    fn threshold_green_below_50() {
        let result = colorize_by_threshold("test", 30.0);
        assert!(result.starts_with(GREEN));
        assert!(result.ends_with(RESET));
        assert!(result.contains("test"));
    }

    #[test]
    fn threshold_yellow_at_50() {
        let result = colorize_by_threshold("test", 50.0);
        assert!(result.starts_with(YELLOW));
    }

    #[test]
    fn threshold_yellow_below_70() {
        let result = colorize_by_threshold("test", 69.9);
        assert!(result.starts_with(YELLOW));
    }

    #[test]
    fn threshold_orange_at_70() {
        let result = colorize_by_threshold("test", 70.0);
        assert!(result.starts_with(ORANGE));
    }

    #[test]
    fn threshold_orange_below_90() {
        let result = colorize_by_threshold("test", 89.9);
        assert!(result.starts_with(ORANGE));
    }

    #[test]
    fn threshold_red_at_90() {
        let result = colorize_by_threshold("test", 90.0);
        assert!(result.starts_with(RED));
    }

    #[test]
    fn threshold_red_at_100() {
        let result = colorize_by_threshold("test", 100.0);
        assert!(result.starts_with(RED));
    }

    // -- Colored helper -----------------------------------------------------

    #[test]
    fn colored_wraps_text() {
        let result = colored("hello", GREEN);
        assert_eq!(result, format!("{GREEN}hello{RESET}"));
    }

    // -- Colorized progress bar ---------------------------------------------

    #[test]
    fn colorized_progress_bar_includes_ansi() {
        let result = colorized_progress_bar(48.0);
        assert!(result.starts_with(GREEN));
        assert!(result.ends_with(RESET));
        assert!(result.contains("48%"));
    }

    // -- Path shortening ----------------------------------------------------

    #[test]
    fn shorten_path_replaces_home() {
        if let Some(home) = dirs::home_dir() {
            let full = format!("{}/projects/foo", home.display());
            assert_eq!(shorten_path(&full), "~/projects/foo");
        }
    }

    #[test]
    fn shorten_path_no_home_prefix() {
        assert_eq!(shorten_path("/tmp/something"), "/tmp/something");
    }
}
