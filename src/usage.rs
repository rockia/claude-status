use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const CACHE_DIR: &str = "/tmp/csl";
const CACHE_PATH: &str = "/tmp/csl/usage-cache.json";
const CACHE_TTL: Duration = Duration::from_secs(60);
const API_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const API_TIMEOUT: Duration = Duration::from_secs(3);

// ---------- public types ----------

/// Categorized usage data ready for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    /// "current" — 5-hour rolling window.
    pub current: Option<WindowLimit>,
    /// "weekly" — 7-day rolling window.
    pub weekly: Option<WindowLimit>,
    /// "extra" — bonus/overage allowance.
    pub extra: Option<ExtraLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLimit {
    /// 0–100 utilization percentage.
    pub utilization: f64,
    /// ISO 8601 reset timestamp (raw from API).
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraLimit {
    pub is_enabled: bool,
    /// Dollar amount used.
    pub used_credits: Option<f64>,
    /// Dollar cap.
    pub monthly_limit: Option<f64>,
    /// 0–100 utilization (derived or from API).
    pub utilization: Option<f64>,
    /// When the extra allowance resets (ISO 8601).
    pub resets_at: Option<String>,
}

// ---------- intermediate API response types ----------

/// Mirror of the actual JSON envelope from the Anthropic usage API.
#[derive(Deserialize)]
struct ApiResponse {
    #[serde(default)]
    five_hour: Option<ApiWindowLimit>,
    #[serde(default)]
    seven_day: Option<ApiWindowLimit>,
    #[serde(default)]
    extra_usage: Option<ApiExtraUsage>,
}

#[derive(Deserialize)]
struct ApiWindowLimit {
    #[serde(default)]
    utilization: f64,
    #[serde(default)]
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct ApiExtraUsage {
    #[serde(default)]
    is_enabled: bool,
    #[serde(default)]
    monthly_limit: Option<f64>,
    #[serde(default)]
    used_credits: Option<f64>,
    #[serde(default)]
    utilization: Option<f64>,
    #[serde(default)]
    resets_at: Option<String>,
}

// ---------- formatting helpers ----------

/// Format an ISO 8601 timestamp into a short local time string.
///
/// Examples: "6:00pm", "mar 19, 8:00am"
pub fn format_reset_time(iso: &str, short: bool) -> String {
    let parsed: DateTime<Utc> = match iso.parse() {
        Ok(dt) => dt,
        Err(_) => return iso.to_string(),
    };
    let local: DateTime<Local> = parsed.into();
    let now = Local::now();

    if short || local.date_naive() == now.date_naive() {
        // Same day → just time: "6:00pm"
        local.format("%-I:%M%P").to_string()
    } else {
        // Different day → "mar 19, 8:00am"
        local.format("%b %-d, %-I:%M%P").to_string().to_lowercase()
    }
}

/// Format a reset timestamp for the "resets <date>" line.
pub fn format_reset_date(iso: &str) -> String {
    let parsed: DateTime<Utc> = match iso.parse() {
        Ok(dt) => dt,
        Err(_) => return iso.to_string(),
    };
    let local: DateTime<Local> = parsed.into();
    local.format("%b %-d").to_string().to_lowercase()
}

// ---------- token resolution ----------

/// Resolve an OAuth token by trying, in order:
/// 1. `CLAUDE_OAUTH_TOKEN` environment variable
/// 2. System keyring (service "claude-api", user "oauth-token")
/// 3. `~/.claude/.credentials.json` (`oauthToken` or `token` field)
pub fn resolve_token() -> Option<String> {
    // 1. Environment variable
    if let Ok(val) = std::env::var("CLAUDE_OAUTH_TOKEN")
        && !val.is_empty()
    {
        return Some(val);
    }

    // 2. Keyring
    if let Some(tok) = resolve_token_keyring() {
        return Some(tok);
    }

    // 3. Credentials file
    if let Some(tok) = resolve_token_credentials_file() {
        return Some(tok);
    }

    None
}

fn resolve_token_keyring() -> Option<String> {
    let entry = keyring::Entry::new("claude-api", "oauth-token").ok()?;
    let password = entry.get_password().ok()?;
    if password.is_empty() {
        None
    } else {
        Some(password)
    }
}

fn resolve_token_credentials_file() -> Option<String> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude").join(".credentials.json");
    let contents = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let obj = value.as_object()?;

    // Try oauthToken first, then token.
    for key in &["oauthToken", "token"] {
        if let Some(tok) = obj.get(*key).and_then(|v| v.as_str())
            && !tok.is_empty()
        {
            return Some(tok.to_owned());
        }
    }

    None
}

// ---------- cache management ----------

fn cache_path() -> PathBuf {
    PathBuf::from(CACHE_PATH)
}

/// Read cached usage data. Returns `Some` only when the cache file exists,
/// is younger than `CACHE_TTL`, and contains valid JSON.
pub fn read_cache() -> Option<UsageData> {
    read_cache_inner(true)
}

/// Read the cache ignoring staleness (used as a fallback when the API fails).
fn read_stale_cache() -> Option<UsageData> {
    read_cache_inner(false)
}

fn read_cache_inner(enforce_ttl: bool) -> Option<UsageData> {
    let path = cache_path();

    let metadata = fs::metadata(&path).ok()?;

    if enforce_ttl {
        let modified = metadata.modified().ok()?;
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(CACHE_TTL);
        if age >= CACHE_TTL {
            return None;
        }
    }

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            // Unreadable — delete and bail.
            let _ = fs::remove_file(&path);
            return None;
        }
    };

    match serde_json::from_str::<UsageData>(&contents) {
        Ok(data) => Some(data),
        Err(_) => {
            // Corrupt cache — delete and bail.
            let _ = fs::remove_file(&path);
            None
        }
    }
}

pub fn write_cache(data: &UsageData) {
    let _ = fs::create_dir_all(CACHE_DIR);
    let json = match serde_json::to_string(data) {
        Ok(j) => j,
        Err(_) => return,
    };
    let _ = fs::write(cache_path(), json);
}

// ---------- API fetch ----------

/// Fetch usage data from the Anthropic API.
/// Returns `None` on any error (network, timeout, non-200, parse failure).
pub fn fetch_usage(token: &str) -> Option<UsageData> {
    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(API_TIMEOUT))
            .build(),
    );

    let mut response = agent
        .get(API_URL)
        .header("Authorization", &format!("Bearer {}", token))
        .call()
        .ok()?;

    let body: String = response.body_mut().read_to_string().ok()?;

    // Parse the actual API shape.
    if let Ok(api) = serde_json::from_str::<ApiResponse>(&body) {
        let current = api.five_hour.map(|w| WindowLimit {
            utilization: w.utilization,
            resets_at: w.resets_at,
        });
        let weekly = api.seven_day.map(|w| WindowLimit {
            utilization: w.utilization,
            resets_at: w.resets_at,
        });
        let extra = api.extra_usage.map(|e| ExtraLimit {
            is_enabled: e.is_enabled,
            used_credits: e.used_credits,
            monthly_limit: e.monthly_limit,
            utilization: e.utilization,
            resets_at: e.resets_at,
        });

        return Some(UsageData {
            current,
            weekly,
            extra,
        });
    }

    // Fallback: if the JSON is valid but shaped differently, return empty.
    if serde_json::from_str::<serde_json::Value>(&body).is_ok() {
        return Some(UsageData {
            current: None,
            weekly: None,
            extra: None,
        });
    }

    None
}

// ---------- public orchestrator ----------

/// Get current API usage data.
///
/// Resolution order:
/// 1. Fresh cache (< 60 s old) — return immediately.
/// 2. Resolve an OAuth token; if unavailable return `None`.
/// 3. Fetch from the API; on success update cache and return.
/// 4. On API failure fall back to stale cache.
/// 5. If everything fails return `None`.
pub fn get_usage() -> Option<UsageData> {
    // 1. Fresh cache
    if let Some(cached) = read_cache() {
        return Some(cached);
    }

    // 2. Resolve token
    let token = resolve_token()?;

    // 3. Fetch from API
    if let Some(data) = fetch_usage(&token) {
        write_cache(&data);
        return Some(data);
    }

    // 4. Stale cache fallback
    if let Some(stale) = read_stale_cache() {
        return Some(stale);
    }

    // 5. Nothing worked
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_actual_api_response() {
        let json = r#"{
            "five_hour": { "utilization": 23.0, "resets_at": "2026-03-12T22:59:59.000000+00:00" },
            "seven_day": { "utilization": 12.0, "resets_at": "2026-03-19T07:59:59.000000+00:00" },
            "seven_day_opus": { "utilization": 0.0, "resets_at": null },
            "extra_usage": { "is_enabled": false, "monthly_limit": null, "used_credits": null, "utilization": null, "resets_at": null }
        }"#;

        let api: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(api.five_hour.as_ref().unwrap().utilization, 23.0);
        assert_eq!(api.seven_day.as_ref().unwrap().utilization, 12.0);
        assert!(!api.extra_usage.as_ref().unwrap().is_enabled);
    }

    #[test]
    fn parse_minimal_api_response() {
        let json = r#"{
            "five_hour": { "utilization": 6.0, "resets_at": "2026-03-12T04:59:59.000000+00:00" },
            "seven_day": { "utilization": 35.0, "resets_at": "2026-03-16T03:59:59.000000+00:00" }
        }"#;

        let api: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(api.five_hour.as_ref().unwrap().utilization, 6.0);
        assert_eq!(api.seven_day.as_ref().unwrap().utilization, 35.0);
        assert!(api.extra_usage.is_none());
    }

    #[test]
    fn parse_empty_response() {
        let json = r#"{}"#;
        let api: ApiResponse = serde_json::from_str(json).unwrap();
        assert!(api.five_hour.is_none());
        assert!(api.seven_day.is_none());
        assert!(api.extra_usage.is_none());
    }

    #[test]
    fn parse_extra_with_dollar_amounts() {
        let json = r#"{
            "five_hour": { "utilization": 10.0, "resets_at": null },
            "seven_day": { "utilization": 5.0, "resets_at": null },
            "extra_usage": { "is_enabled": true, "monthly_limit": 20.0, "used_credits": 5.0, "utilization": 25.0, "resets_at": "2026-04-01T00:00:00.000000+00:00" }
        }"#;

        let api: ApiResponse = serde_json::from_str(json).unwrap();
        let extra = api.extra_usage.unwrap();
        assert!(extra.is_enabled);
        assert_eq!(extra.monthly_limit, Some(20.0));
        assert_eq!(extra.used_credits, Some(5.0));
    }

    #[test]
    fn format_reset_time_same_day() {
        // Use a time that's today — we can't easily test exact output due to timezone,
        // but we can verify it doesn't panic and returns something short.
        let result = format_reset_time("2026-03-12T22:00:00.000000+00:00", true);
        assert!(!result.is_empty());
        // Should contain am or pm
        assert!(
            result.contains("am") || result.contains("pm"),
            "got: {result}"
        );
    }

    #[test]
    fn format_reset_date_works() {
        let result = format_reset_date("2026-04-01T12:00:00.000000+00:00");
        // Should produce a short date like "apr 1" or "mar 31" depending on timezone.
        // Just verify it's a reasonable short string, not the raw ISO format.
        assert!(
            !result.contains("2026"),
            "should not contain year: {result}"
        );
        assert!(!result.contains("T"), "should not be raw ISO: {result}");
        assert!(!result.is_empty());
    }

    #[test]
    fn format_reset_time_invalid_falls_back() {
        let result = format_reset_time("not-a-date", false);
        assert_eq!(result, "not-a-date");
    }
}
